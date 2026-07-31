use base64::{engine::general_purpose, Engine as _};
use fancy_regex::Regex;
use md5;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::Emitter;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use crate::font::ensure_fonttools_available;
use crate::fs_utils::{
    build_processed_epub_path, detect_image_ext, ensure_unique_zip_name, escape_xml,
    friendly_name_for_path, hidden_process_command, is_external_or_inline_ref, is_text_like_entry,
    parse_xmlish_attrs, percent_decode, read_epub_bytes,
    rebuild_epub_with_sanitized_names_from_bytes_with_progress, rewrite_text_links,
    sanitize_filename_part, sanitize_windows_component, sanitize_zip_entry_name, split_ref_suffix,
    zip_join, zip_parent, zip_relative_path,
};

static TOOLBOX_BATCH_CANCEL: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

static DIAGNOSTIC_ATTR_REF_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)\b(?:href|src|poster|xlink:href|full-path)\s*=\s*(['"])(.*?)\1"#)
        .expect("valid diagnostic attr ref regex")
});

static DIAGNOSTIC_CSS_URL_REF_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)url\(\s*(['"]?)(.*?)\1\s*\)"#).expect("valid diagnostic css url regex")
});

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxEpubToolResult {
    source_path: String,
    output_path: String,
    changed: bool,
    action: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxEpubDiagnosticIssue {
    level: String,
    kind: String,
    path: Option<String>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxEpubDiagnosticResult {
    source_path: String,
    opf_path: Option<String>,
    total_entries: usize,
    manifest_items: usize,
    error_count: usize,
    warning_count: usize,
    issues: Vec<ToolboxEpubDiagnosticIssue>,
}

#[derive(Clone, Debug)]
pub(crate) struct ToolboxManifestItem {
    id: String,
    href: String,
    media_type: String,
    abs_path: String,
}

fn find_opf_path_in_epub_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    if let Ok(mut container) = archive.by_name("META-INF/container.xml") {
        let mut data = Vec::new();
        container
            .read_to_end(&mut data)
            .map_err(|e| format!("读取 container.xml 失败: {}", e))?;
        let text = String::from_utf8_lossy(&data).to_string();
        if let Ok(re) = Regex::new(r#"<rootfile[^>]*full-path\s*=\s*(['"])(?i:(.*?\.opf))\1"#) {
            if let Some(caps) = re.captures(&text).ok().flatten() {
                if let Some(path_match) = caps.get(2) {
                    return Ok(path_match.as_str().replace('\\', "/"));
                }
            }
        }
    }

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        let name = file.name().replace('\\', "/");
        if name.to_ascii_lowercase().ends_with(".opf") {
            return Ok(name);
        }
    }

    Err("无法发现 OPF 文件".to_string())
}

fn read_zip_entry_bytes(bytes: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;
    let mut file = archive
        .by_name(name)
        .map_err(|e| format!("读取 ZIP 条目失败 {}: {}", name, e))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| format!("读取 ZIP 条目数据失败 {}: {}", name, e))?;
    Ok(data)
}

fn parse_toolbox_manifest_items(
    bytes: &[u8],
    opf_path: &str,
) -> Result<Vec<ToolboxManifestItem>, String> {
    let opf_data = read_zip_entry_bytes(bytes, opf_path)?;
    let opf_text = String::from_utf8_lossy(&opf_data).to_string();
    let item_re = Regex::new(r#"(?is)<item\b([^>]*)>"#).map_err(|e| e.to_string())?;
    let opf_dir = zip_parent(opf_path);
    let mut items = Vec::new();
    for caps in item_re.captures_iter(&opf_text).flatten() {
        let attrs_text = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let attrs = parse_xmlish_attrs(attrs_text);
        let id = attrs.get("id").cloned().unwrap_or_default();
        let href = attrs.get("href").cloned().unwrap_or_default();
        if id.trim().is_empty() || href.trim().is_empty() {
            continue;
        }
        let href_decoded = percent_decode(&href);
        let abs_path = zip_join(&opf_dir, &href_decoded);
        items.push(ToolboxManifestItem {
            id,
            href: href_decoded,
            media_type: attrs.get("media-type").cloned().unwrap_or_default(),
            abs_path,
        });
    }
    Ok(items)
}

fn push_diag_issue(
    issues: &mut Vec<ToolboxEpubDiagnosticIssue>,
    level: &str,
    kind: &str,
    path: Option<String>,
    message: impl Into<String>,
) {
    issues.push(ToolboxEpubDiagnosticIssue {
        level: level.to_string(),
        kind: kind.to_string(),
        path,
        message: message.into(),
    });
}

fn resolve_epub_ref(current_path: &str, raw_ref: &str) -> Option<String> {
    let raw_ref = raw_ref.trim();
    if is_external_or_inline_ref(raw_ref) {
        return None;
    }
    let (main_ref, _) = split_ref_suffix(raw_ref);
    if is_external_or_inline_ref(main_ref) {
        return None;
    }
    let decoded_ref = percent_decode(main_ref.trim());
    if decoded_ref.is_empty() {
        return None;
    }
    Some(zip_join(&zip_parent(current_path), &decoded_ref))
}

fn collect_diagnostic_refs(text: &str) -> Vec<String> {
    let mut refs = HashSet::new();

    for caps in DIAGNOSTIC_ATTR_REF_RE.captures_iter(text).flatten() {
        let Some(raw_match) = caps.get(2) else {
            continue;
        };
        let raw_ref = raw_match.as_str().trim();
        if !raw_ref.is_empty() {
            refs.insert(raw_ref.to_string());
        }
    }

    for caps in DIAGNOSTIC_CSS_URL_REF_RE.captures_iter(text).flatten() {
        let Some(raw_match) = caps.get(2) else {
            continue;
        };
        let raw_ref = raw_match.as_str().trim().trim_matches(&['"', '\''][..]);
        if !raw_ref.is_empty() {
            refs.insert(raw_ref.to_string());
        }
    }

    let mut out: Vec<String> = refs.into_iter().collect();
    out.sort();
    out
}

fn is_diagnostic_resource_entry(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with('/') || lower.starts_with("__macosx/") || lower.starts_with("meta-inf/") {
        return false;
    }
    matches!(
        Path::new(&lower).extension().and_then(|s| s.to_str()),
        Some(
            "xhtml"
                | "html"
                | "htm"
                | "css"
                | "svg"
                | "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "gif"
                | "bmp"
                | "ttf"
                | "otf"
                | "woff"
                | "woff2"
                | "ncx"
                | "js"
                | "mp3"
                | "mp4"
                | "m4a"
                | "ogg"
                | "opus"
                | "wav"
        )
    )
}

fn case_mismatch_names<'a>(
    lower_to_names: &'a HashMap<String, Vec<String>>,
    expected: &str,
) -> Option<&'a Vec<String>> {
    let matches = lower_to_names.get(&expected.to_ascii_lowercase())?;
    if matches.iter().any(|name| name == expected) {
        None
    } else {
        Some(matches)
    }
}

#[derive(Clone)]
pub(crate) struct ToolboxManifestAddition {
    id: String,
    href: String,
    media_type: String,
}

fn toolbox_file_category(item: &ToolboxManifestItem) -> &'static str {
    let media = item.media_type.to_ascii_lowercase();
    let href = item.href.to_ascii_lowercase();
    if media == "application/xhtml+xml" || href.ends_with(".xhtml") || href.ends_with(".html") {
        "text"
    } else if media == "text/css" || href.ends_with(".css") {
        "css"
    } else if media.starts_with("image/") {
        "image"
    } else if media.starts_with("font/")
        || href.ends_with(".ttf")
        || href.ends_with(".otf")
        || href.ends_with(".woff")
        || href.ends_with(".woff2")
    {
        "font"
    } else if media.starts_with("audio/") {
        "audio"
    } else if media.starts_with("video/") {
        "video"
    } else {
        "other"
    }
}

fn toolbox_epub_tool_result(
    source: &Path,
    output: &Path,
    changed: bool,
    action: &str,
    message: String,
) -> ToolboxEpubToolResult {
    ToolboxEpubToolResult {
        source_path: source.to_string_lossy().to_string(),
        output_path: output.to_string_lossy().to_string(),
        changed,
        action: action.to_string(),
        message,
    }
}

fn encrypted_href_basename(item_id: &str, href: &str) -> String {
    let ext = Path::new(href)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let href_name = Path::new(href)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let id_stem = item_id.split('.').next().unwrap_or(item_id);
    let lower_id = id_stem.to_ascii_lowercase();
    let lower_href = href_name.to_ascii_lowercase();
    let slim_suffix = if lower_id.ends_with("slim")
        || lower_id.ends_with("_slim")
        || lower_id.ends_with("-slim")
        || lower_id.ends_with("~slim")
        || lower_href.ends_with("slim")
        || lower_href.ends_with("_slim")
        || lower_href.ends_with("-slim")
        || lower_href.ends_with("~slim")
    {
        "~slim"
    } else {
        ""
    };

    let digest = md5::compute(id_stem.as_bytes());
    let mut bits = String::with_capacity(129);
    bits.push('_');
    for byte in digest.0 {
        for shift in (0..8).rev() {
            bits.push(if (byte >> shift) & 1 == 1 { '*' } else { ':' });
        }
    }
    format!("{}{}.{}", bits, slim_suffix, ext)
}

fn write_epub_with_path_map(
    bytes: &[u8],
    output_path: &Path,
    path_map: &HashMap<String, String>,
) -> Result<(), String> {
    write_epub_with_path_map_and_manifest_additions(bytes, output_path, path_map, &HashMap::new())
}

fn inject_manifest_additions(text: String, additions: &[ToolboxManifestAddition]) -> String {
    if additions.is_empty() {
        return text;
    }

    let mut block = String::new();
    for item in additions {
        block.push_str(&format!(
            "\n    <item id=\"{}\" href=\"{}\" media-type=\"{}\"/>",
            escape_xml(&item.id),
            escape_xml(&item.href),
            escape_xml(&item.media_type)
        ));
    }

    let Ok(re) = Regex::new(r"(?i)</manifest>") else {
        return text;
    };
    if let Ok(Some(mat)) = re.find(&text) {
        let insert_at = mat.start();
        let mut out = text;
        out.insert_str(insert_at, &block);
        out
    } else {
        text
    }
}

fn write_epub_with_path_map_and_manifest_additions(
    bytes: &[u8],
    output_path: &Path,
    path_map: &HashMap<String, String>,
    manifest_additions_by_opf: &HashMap<String, Vec<ToolboxManifestAddition>>,
) -> Result<(), String> {
    let out_file =
        fs::File::create(output_path).map_err(|e| format!("创建输出 EPUB 失败: {}", e))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    let mut opf_old_to_new: HashMap<String, String> = HashMap::new();
    for (old, newp) in path_map {
        if old.to_ascii_lowercase().ends_with(".opf") && newp.to_ascii_lowercase().ends_with(".opf")
        {
            opf_old_to_new.insert(old.clone(), newp.clone());
        }
    }

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        let old_name = file.name().replace('\\', "/");
        let new_name = path_map
            .get(&old_name)
            .cloned()
            .unwrap_or_else(|| old_name.clone());

        let options = SimpleFileOptions::default().compression_method(file.compression());
        if new_name.ends_with('/') {
            writer
                .add_directory(&new_name, options)
                .map_err(|e| format!("写入目录失败: {}", e))?;
            continue;
        }

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("读取条目数据失败: {}", e))?;

        if old_name.eq_ignore_ascii_case("META-INF/container.xml") {
            let mut text = String::from_utf8_lossy(&data).to_string();
            for (opf_old, opf_new) in &opf_old_to_new {
                text = text.replace(opf_old, opf_new);
            }
            text = rewrite_text_links(text, &old_name, &new_name, path_map);
            data = text.into_bytes();
        } else if is_text_like_entry(&old_name) {
            let text = String::from_utf8_lossy(&data).to_string();
            let mut text = rewrite_text_links(text, &old_name, &new_name, path_map);
            if let Some(additions) = manifest_additions_by_opf.get(&new_name) {
                text = inject_manifest_additions(text, additions);
            }
            data = text.into_bytes();
        }

        writer
            .start_file(&new_name, options)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        writer
            .write_all(&data)
            .map_err(|e| format!("写入文件内容失败: {}", e))?;
    }

    writer
        .finish()
        .map_err(|e| format!("完成写入失败: {}", e))?;
    Ok(())
}

fn toolbox_file_encrypt_impl(source: &Path) -> Result<ToolboxEpubToolResult, String> {
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    let bytes = read_epub_bytes(source)?;
    let opf_path = find_opf_path_in_epub_bytes(&bytes)?;
    let manifest_items = parse_toolbox_manifest_items(&bytes, &opf_path)?;
    if manifest_items.is_empty() {
        return Err("OPF manifest 为空，无法执行文件加密".to_string());
    }

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;
    let mut entry_names = Vec::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        entry_names.push(file.name().replace('\\', "/"));
    }

    let mut manifest_by_abs: HashMap<String, ToolboxManifestItem> = HashMap::new();
    for item in manifest_items {
        manifest_by_abs.insert(item.abs_path.clone(), item);
    }

    let mut used: HashSet<String> = HashSet::new();
    for name in &entry_names {
        if !manifest_by_abs.contains_key(name) {
            used.insert(name.clone());
        }
    }

    let mut path_map: HashMap<String, String> = HashMap::new();
    let mut changed = false;
    for name in &entry_names {
        if let Some(item) = manifest_by_abs.get(name) {
            let parent = zip_parent(name);
            let basename = encrypted_href_basename(&item.id, &item.href);
            let target_dir = match toolbox_file_category(item) {
                // Keep the current directory layout stable. The category is kept here
                // to mirror epub_tool's manifest classification and leave room for
                // future full Sigil-style relocation without changing callers.
                _ => parent,
            };
            let target = if target_dir.is_empty() {
                basename
            } else {
                format!("{}/{}", target_dir, basename)
            };
            let unique = ensure_unique_zip_name(&target, &mut used);
            if unique != *name {
                changed = true;
            }
            path_map.insert(name.clone(), unique);
        } else {
            path_map.insert(name.clone(), name.clone());
        }
    }

    if !changed {
        return Ok(toolbox_epub_tool_result(
            source,
            source,
            false,
            "file_encrypt",
            "未检测到可混淆的 manifest 文件项".to_string(),
        ));
    }

    let output_path = build_processed_epub_path(source, "_encrypt");
    write_epub_with_path_map(&bytes, &output_path, &path_map)?;
    Ok(toolbox_epub_tool_result(
        source,
        &output_path,
        true,
        "file_encrypt",
        "文件加密完成".to_string(),
    ))
}

fn toolbox_file_decrypt_impl(source: &Path) -> Result<ToolboxEpubToolResult, String> {
    let mut noop = |_stage: &str| {};
    toolbox_file_decrypt_impl_with_progress(source, &mut noop)
}

fn toolbox_file_decrypt_impl_with_progress<F>(
    source: &Path,
    progress: &mut F,
) -> Result<ToolboxEpubToolResult, String>
where
    F: FnMut(&str),
{
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    progress("读取 EPUB 文件");
    let bytes = read_epub_bytes(source)?;
    let processed = rebuild_epub_with_sanitized_names_from_bytes_with_progress(
        source, &bytes, "_decrypt", progress,
    )?;
    if let Some(output_path) = processed {
        Ok(toolbox_epub_tool_result(
            source,
            &output_path,
            true,
            "file_decrypt",
            "文件解密完成".to_string(),
        ))
    } else {
        Ok(toolbox_epub_tool_result(
            source,
            source,
            false,
            "file_decrypt",
            "未检测到文件名混淆，无需处理".to_string(),
        ))
    }
}

fn toolbox_reformat_target_path(item: Option<&ToolboxManifestItem>, path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower == "mimetype" || lower.starts_with("meta-inf/") {
        return path.to_string();
    }
    if lower.ends_with(".opf") {
        return "OEBPS/content.opf".to_string();
    }
    if lower.ends_with(".ncx") {
        return "OEBPS/toc.ncx".to_string();
    }

    let category = item.map(toolbox_file_category).unwrap_or_else(|| {
        if lower.ends_with(".xhtml") || lower.ends_with(".html") || lower.ends_with(".htm") {
            "text"
        } else if lower.ends_with(".css") {
            "css"
        } else if lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".png")
            || lower.ends_with(".gif")
            || lower.ends_with(".webp")
            || lower.ends_with(".bmp")
            || lower.ends_with(".svg")
        {
            "image"
        } else if lower.ends_with(".ttf")
            || lower.ends_with(".otf")
            || lower.ends_with(".woff")
            || lower.ends_with(".woff2")
        {
            "font"
        } else if lower.ends_with(".mp3") || lower.ends_with(".m4a") || lower.ends_with(".aac") {
            "audio"
        } else if lower.ends_with(".mp4") || lower.ends_with(".webm") {
            "video"
        } else {
            "other"
        }
    });

    let folder = match category {
        "text" => "Text",
        "css" => "Styles",
        "image" => "Images",
        "font" => "Fonts",
        "audio" => "Audio",
        "video" => "Video",
        _ => "Misc",
    };
    let basename = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(sanitize_windows_component)
        .unwrap_or_else(|| friendly_name_for_path(path, 1));
    format!("OEBPS/{}/{}", folder, basename)
}

fn toolbox_manifest_media_type_for_path(path: &str) -> Option<&'static str> {
    let lower = path.to_ascii_lowercase();
    match Path::new(&lower).extension().and_then(|s| s.to_str()) {
        Some("xhtml") => Some("application/xhtml+xml"),
        Some("html") | Some("htm") => Some("text/html"),
        Some("css") => Some("text/css"),
        Some("svg") => Some("image/svg+xml"),
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        Some("bmp") => Some("image/bmp"),
        Some("ttf") => Some("font/ttf"),
        Some("otf") => Some("font/otf"),
        Some("woff") => Some("font/woff"),
        Some("woff2") => Some("font/woff2"),
        Some("ncx") => Some("application/x-dtbncx+xml"),
        Some("js") => Some("text/javascript"),
        Some("mp3") => Some("audio/mpeg"),
        Some("m4a") => Some("audio/mp4"),
        Some("aac") => Some("audio/aac"),
        Some("ogg") | Some("opus") => Some("audio/ogg"),
        Some("wav") => Some("audio/wav"),
        Some("mp4") => Some("video/mp4"),
        Some("webm") => Some("video/webm"),
        _ => None,
    }
}

fn unique_manifest_addition_id(path: &str, index: usize, used_ids: &mut HashSet<String>) -> String {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("resource");
    let mut cleaned: String = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    cleaned = cleaned.trim_matches(&['.', '_', '-'][..]).to_string();
    if cleaned.is_empty() {
        cleaned = "resource".to_string();
    }
    let mut candidate = format!("te-extra-{}", cleaned);
    if !candidate
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic() || ch == '_')
        .unwrap_or(false)
    {
        candidate = format!("te-extra-{:03}", index + 1);
    }
    let base = candidate.clone();
    let mut suffix = 2usize;
    while used_ids.contains(&candidate) {
        candidate = format!("{}-{}", base, suffix);
        suffix += 1;
    }
    used_ids.insert(candidate.clone());
    candidate
}

fn build_reformat_manifest_additions(
    entry_names: &[String],
    manifest_paths: &HashSet<String>,
    manifest_ids: &HashSet<String>,
    path_map: &HashMap<String, String>,
    opf_new_path: &str,
) -> Vec<ToolboxManifestAddition> {
    let mut additions = Vec::new();
    let mut used_ids = manifest_ids.clone();

    for old_name in entry_names {
        if old_name.ends_with('/')
            || manifest_paths.contains(old_name)
            || old_name.eq_ignore_ascii_case("mimetype")
            || old_name.to_ascii_lowercase().starts_with("meta-inf/")
            || old_name.to_ascii_lowercase().ends_with(".opf")
        {
            continue;
        }
        let Some(media_type) = toolbox_manifest_media_type_for_path(old_name) else {
            continue;
        };
        let new_path = path_map
            .get(old_name)
            .cloned()
            .unwrap_or_else(|| old_name.clone());
        let href = zip_relative_path(opf_new_path, &new_path);
        if href == "." {
            continue;
        }
        let id = unique_manifest_addition_id(&new_path, additions.len(), &mut used_ids);
        additions.push(ToolboxManifestAddition {
            id,
            href,
            media_type: media_type.to_string(),
        });
    }

    additions
}

fn toolbox_epub_reformat_impl(source: &Path) -> Result<ToolboxEpubToolResult, String> {
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    let bytes = read_epub_bytes(source)?;
    let opf_path = find_opf_path_in_epub_bytes(&bytes)?;
    let manifest_items = parse_toolbox_manifest_items(&bytes, &opf_path)?;
    let manifest_paths: HashSet<String> = manifest_items
        .iter()
        .map(|item| item.abs_path.clone())
        .collect();
    let manifest_ids: HashSet<String> = manifest_items.iter().map(|item| item.id.clone()).collect();
    let manifest_by_abs: HashMap<String, ToolboxManifestItem> = manifest_items
        .into_iter()
        .map(|item| (item.abs_path.clone(), item))
        .collect();

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;
    let mut used: HashSet<String> = HashSet::new();
    let mut path_map: HashMap<String, String> = HashMap::new();
    let mut entry_names: Vec<String> = Vec::new();
    let mut changed = false;

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        let old_name = file.name().replace('\\', "/");
        entry_names.push(old_name.clone());
        let is_dir = old_name.ends_with('/');
        let mut target = if is_dir {
            old_name.clone()
        } else {
            toolbox_reformat_target_path(manifest_by_abs.get(&old_name), &old_name)
        };
        target = sanitize_zip_entry_name(&target);
        let unique = ensure_unique_zip_name(&target, &mut used);
        if unique != old_name {
            changed = true;
        }
        path_map.insert(old_name, unique);
    }

    let opf_new_path = path_map
        .get(&opf_path)
        .cloned()
        .unwrap_or_else(|| opf_path.clone());
    let additions = build_reformat_manifest_additions(
        &entry_names,
        &manifest_paths,
        &manifest_ids,
        &path_map,
        &opf_new_path,
    );

    if !changed && additions.is_empty() {
        return Ok(toolbox_epub_tool_result(
            source,
            source,
            false,
            "epub_reformat",
            "EPUB 已经是规范结构，无需重构".to_string(),
        ));
    }

    let output_path = build_processed_epub_path(source, "_reformat");
    let mut manifest_additions_by_opf: HashMap<String, Vec<ToolboxManifestAddition>> =
        HashMap::new();
    if !additions.is_empty() {
        manifest_additions_by_opf.insert(opf_new_path, additions);
    }
    write_epub_with_path_map_and_manifest_additions(
        &bytes,
        &output_path,
        &path_map,
        &manifest_additions_by_opf,
    )?;
    Ok(toolbox_epub_tool_result(
        source,
        &output_path,
        true,
        "epub_reformat",
        "EPUB 重构完成".to_string(),
    ))
}

#[derive(Clone)]
pub(crate) struct ImageConversionEntry {
    new_path: String,
    data: Vec<u8>,
    media_type: String,
}

fn has_image_alpha(image: &image::DynamicImage) -> bool {
    matches!(
        image.color(),
        image::ColorType::La8
            | image::ColorType::La16
            | image::ColorType::Rgba8
            | image::ColorType::Rgba16
    )
}

fn encode_converted_image(
    source_name: &str,
    data: &[u8],
    requested_format: Option<&str>,
) -> Result<(Vec<u8>, &'static str, &'static str), String> {
    let image = image::load_from_memory(data)
        .map_err(|e| format!("转换图片失败 {}: {}", source_name, e))?;
    let format = requested_format
        .unwrap_or("auto")
        .trim()
        .to_ascii_lowercase();
    let use_png =
        format == "png" || (format != "jpg" && format != "jpeg" && has_image_alpha(&image));
    let mut out = Vec::new();
    if use_png {
        let mut cursor = std::io::Cursor::new(&mut out);
        image
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| format!("写入 PNG 失败 {}: {}", source_name, e))?;
        Ok((out, "png", "image/png"))
    } else {
        let rgb = image::DynamicImage::ImageRgb8(image.to_rgb8());
        let mut cursor = std::io::Cursor::new(&mut out);
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 90);
        rgb.write_with_encoder(encoder)
            .map_err(|e| format!("写入 JPEG 失败 {}: {}", source_name, e))?;
        Ok((out, "jpg", "image/jpeg"))
    }
}

fn set_xmlish_attr(tag: &str, attr: &str, value: &str) -> String {
    let pattern = format!(r#"(?is)\b{}\s*=\s*(['"])(.*?)\1"#, attr);
    if let Ok(re) = Regex::new(&pattern) {
        if let Some(caps) = re.captures(tag).ok().flatten() {
            if let (Some(full), Some(quote)) = (caps.get(0), caps.get(1)) {
                let mut out = String::new();
                out.push_str(&tag[..full.start()]);
                out.push_str(attr);
                out.push('=');
                out.push_str(quote.as_str());
                out.push_str(value);
                out.push_str(quote.as_str());
                out.push_str(&tag[full.end()..]);
                return out;
            }
        }
    }

    if let Some(pos) = tag.rfind("/>") {
        let mut out = String::new();
        out.push_str(&tag[..pos]);
        out.push(' ');
        out.push_str(attr);
        out.push_str("=\"");
        out.push_str(value);
        out.push('"');
        out.push_str(&tag[pos..]);
        out
    } else if let Some(pos) = tag.rfind('>') {
        let mut out = String::new();
        out.push_str(&tag[..pos]);
        out.push(' ');
        out.push_str(attr);
        out.push_str("=\"");
        out.push_str(value);
        out.push('"');
        out.push_str(&tag[pos..]);
        out
    } else {
        tag.to_string()
    }
}

fn rewrite_opf_image_media_types(
    text: String,
    opf_path: &str,
    conversions: &HashMap<String, ImageConversionEntry>,
) -> String {
    let Ok(item_re) = Regex::new(r#"(?is)<item\b[^>]*>"#) else {
        return text;
    };
    let opf_dir = zip_parent(opf_path);
    let mut out = String::new();
    let mut last = 0usize;
    for caps in item_re.captures_iter(&text).flatten() {
        let Some(m) = caps.get(0) else {
            continue;
        };
        out.push_str(&text[last..m.start()]);
        let tag = m.as_str();
        let attrs = parse_xmlish_attrs(tag);
        let href = attrs.get("href").cloned().unwrap_or_default();
        let href_path = percent_decode(href.split(['?', '#']).next().unwrap_or("").trim());
        let abs_path = zip_join(&opf_dir, &href_path);
        if let Some(entry) = conversions.get(&abs_path) {
            let new_href = zip_relative_path(opf_path, &entry.new_path);
            let updated = set_xmlish_attr(
                &set_xmlish_attr(tag, "href", &new_href),
                "media-type",
                &entry.media_type,
            );
            out.push_str(&updated);
        } else {
            out.push_str(tag);
        }
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

fn write_epub_with_image_conversions(
    bytes: &[u8],
    output_path: &Path,
    path_map: &HashMap<String, String>,
    conversions: &HashMap<String, ImageConversionEntry>,
    opf_path: &str,
) -> Result<(), String> {
    let out_file =
        fs::File::create(output_path).map_err(|e| format!("创建输出 EPUB 失败: {}", e))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        let old_name = file.name().replace('\\', "/");
        let options = SimpleFileOptions::default().compression_method(file.compression());

        if let Some(converted) = conversions.get(&old_name) {
            writer
                .start_file(&converted.new_path, options)
                .map_err(|e| format!("写入转换图片失败: {}", e))?;
            writer
                .write_all(&converted.data)
                .map_err(|e| format!("写入转换图片内容失败: {}", e))?;
            continue;
        }

        let new_name = path_map
            .get(&old_name)
            .cloned()
            .unwrap_or_else(|| old_name.clone());
        if new_name.ends_with('/') {
            writer
                .add_directory(&new_name, options)
                .map_err(|e| format!("写入目录失败: {}", e))?;
            continue;
        }

        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("读取条目数据失败: {}", e))?;

        if is_text_like_entry(&old_name) {
            let mut text = String::from_utf8_lossy(&data).to_string();
            if old_name == opf_path {
                text = rewrite_opf_image_media_types(text, &old_name, conversions);
            }
            text = rewrite_text_links(text, &old_name, &new_name, path_map);
            data = text.into_bytes();
        }

        writer
            .start_file(&new_name, options)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        writer
            .write_all(&data)
            .map_err(|e| format!("写入文件内容失败: {}", e))?;
    }

    writer
        .finish()
        .map_err(|e| format!("完成写入失败: {}", e))?;
    Ok(())
}

fn toolbox_image_convert_impl(
    source: &Path,
    image_format: Option<&str>,
) -> Result<ToolboxEpubToolResult, String> {
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    let bytes = read_epub_bytes(source)?;
    let opf_path = find_opf_path_in_epub_bytes(&bytes).unwrap_or_default();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;
    let mut entry_names = Vec::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        entry_names.push(file.name().replace('\\', "/"));
    }

    let mut used: HashSet<String> = entry_names
        .iter()
        .filter(|name| !name.to_ascii_lowercase().ends_with(".webp"))
        .cloned()
        .collect();
    let mut conversions: HashMap<String, ImageConversionEntry> = HashMap::new();
    let mut path_map: HashMap<String, String> = HashMap::new();

    for name in &entry_names {
        if name.ends_with('/') || !name.to_ascii_lowercase().ends_with(".webp") {
            path_map.insert(name.clone(), name.clone());
            continue;
        }
        let data = read_zip_entry_bytes(&bytes, name)?;
        let (converted, ext, media_type) = encode_converted_image(name, &data, image_format)?;
        let parent = zip_parent(name);
        let stem = Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        let target = if parent.is_empty() {
            format!("{}.{}", stem, ext)
        } else {
            format!("{}/{}.{}", parent, stem, ext)
        };
        let unique = ensure_unique_zip_name(&target, &mut used);
        path_map.insert(name.clone(), unique.clone());
        conversions.insert(
            name.clone(),
            ImageConversionEntry {
                new_path: unique,
                data: converted,
                media_type: media_type.to_string(),
            },
        );
    }

    if conversions.is_empty() {
        return Ok(toolbox_epub_tool_result(
            source,
            source,
            false,
            "image_convert",
            "未发现需要转换的 WebP 图片".to_string(),
        ));
    }

    let output_path = build_processed_epub_path(source, "_transfer");
    write_epub_with_image_conversions(&bytes, &output_path, &path_map, &conversions, &opf_path)?;
    Ok(toolbox_epub_tool_result(
        source,
        &output_path,
        true,
        "image_convert",
        format!("图片转换完成，转换 {} 张 WebP 图片", conversions.len()),
    ))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxBatchEvent {
    task_id: String,
    event: String,
    level: String,
    index: usize,
    total: usize,
    input_path: Option<String>,
    output_path: Option<String>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxBatchSummary {
    task_id: String,
    total: usize,
    succeeded: usize,
    failed: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxBatchScanResult {
    input_paths: Vec<String>,
    output_dir: String,
}

fn emit_toolbox_batch_event(app: &tauri::AppHandle, event: ToolboxBatchEvent) {
    let _ = app.emit("toolbox-batch-event", event);
}

fn format_toolbox_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs >= 3600 {
        format!("{}小时{}分{}秒", secs / 3600, (secs % 3600) / 60, secs % 60)
    } else if secs >= 60 {
        format!("{}分{}秒", secs / 60, secs % 60)
    } else if secs >= 1 {
        format!("{}秒", secs)
    } else {
        format!("{}毫秒", duration.as_millis())
    }
}

fn append_toolbox_duration(message: String, duration: Duration) -> String {
    format!("{}（耗时 {}）", message, format_toolbox_duration(duration))
}

fn emit_toolbox_batch_stage_with_level(
    app: &tauri::AppHandle,
    task_id: &str,
    index: usize,
    total: usize,
    input_path: &Path,
    level: &str,
    message: impl Into<String>,
) {
    emit_toolbox_batch_event(
        app,
        ToolboxBatchEvent {
            task_id: task_id.to_string(),
            event: "file-stage".to_string(),
            level: level.to_string(),
            index,
            total,
            input_path: Some(input_path.to_string_lossy().to_string()),
            output_path: None,
            message: message.into(),
        },
    );
}

fn emit_toolbox_batch_stage(
    app: &tauri::AppHandle,
    task_id: &str,
    index: usize,
    total: usize,
    input_path: &Path,
    message: impl Into<String>,
) {
    emit_toolbox_batch_stage_with_level(app, task_id, index, total, input_path, "info", message);
}

fn reset_toolbox_batch_cancel(task_id: &str) {
    if let Ok(mut set) = TOOLBOX_BATCH_CANCEL.lock() {
        set.remove(task_id);
    }
}

fn request_toolbox_batch_cancel(task_id: &str) {
    if let Ok(mut set) = TOOLBOX_BATCH_CANCEL.lock() {
        set.insert(task_id.to_string());
    }
}

fn is_toolbox_batch_cancelled(task_id: &str) -> bool {
    TOOLBOX_BATCH_CANCEL
        .lock()
        .map(|set| set.contains(task_id))
        .unwrap_or(false)
}

fn is_same_path(left: &Path, right: &Path) -> bool {
    let left_canon = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right_canon = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left_canon == right_canon
}

fn path_is_under(path: &Path, parent: &Path) -> bool {
    let path_canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let parent_canon = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    path_canon.starts_with(parent_canon)
}

fn default_batch_output_dir(input_paths: &[String]) -> PathBuf {
    let first = input_paths
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    if first.is_dir() {
        first.join("TEpub-batch-output")
    } else {
        first
            .parent()
            .unwrap_or(Path::new("."))
            .join("TEpub-batch-output")
    }
}

fn resolve_batch_output_dir(input_paths: &[String], output_dir: Option<String>) -> PathBuf {
    output_dir
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_batch_output_dir(input_paths))
}

fn build_output_path_in_dir(original_output: &Path, output_dir: &Path) -> PathBuf {
    let file_name = original_output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("output.epub");
    let candidate = output_dir.join(file_name);
    if !candidate.exists() {
        return candidate;
    }

    let stem = original_output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let ext = original_output
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("epub");
    let mut index = 2usize;
    loop {
        let candidate = output_dir.join(format!("{}_{}.{}", stem, index, ext));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn finalize_toolbox_output(
    mut result: ToolboxEpubToolResult,
    source: &Path,
    output_root: &Path,
    output_mode: &str,
    conflict_mode: &str,
    input_roots: &[String],
) -> Result<ToolboxEpubToolResult, String> {
    if !result.changed {
        return Ok(result);
    }

    // 决定最终目标目录：
    // - batch_dir（默认）：统一放到输出根目录
    // - same_dir：放到源文件所在目录（与原文件并列）
    // - preserve_structure：在输出根目录下按源文件相对输入根目录的子路径重建目录结构
    let target_dir: PathBuf = match output_mode {
        "same_dir" => source
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| output_root.to_path_buf()),
        "preserve_structure" => {
            let rel = input_roots
                .iter()
                .filter_map(|root| {
                    let root_path = Path::new(root);
                    source
                        .strip_prefix(root_path)
                        .ok()
                        .map(|rel| rel.to_path_buf())
                })
                .next();
            match rel {
                Some(rel) => {
                    let parent = rel.parent().unwrap_or_else(|| Path::new(""));
                    if parent.as_os_str().is_empty() {
                        output_root.to_path_buf()
                    } else {
                        output_root.join(parent)
                    }
                }
                None => output_root.to_path_buf(),
            }
        }
        _ => output_root.to_path_buf(),
    };

    fs::create_dir_all(&target_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;

    let source_output = PathBuf::from(&result.output_path);
    if !source_output.exists() {
        return Ok(result);
    }

    let file_name = source_output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("output.epub");
    let target = target_dir.join(file_name);

    // 已经在目标位置（例如 same_dir 模式下工具输出就在目标目录）
    if is_same_path(&source_output, &target) {
        return Ok(result);
    }

    // 冲突处理
    if target.exists() {
        match conflict_mode {
            "skip" => {
                // 目标已存在，跳过：删除刚生成的临时输出，标记未变更
                let _ = fs::remove_file(&source_output);
                result.changed = false;
                result.message = format!("跳过（目标已存在）: {}", target.to_string_lossy());
                result.output_path = target.to_string_lossy().to_string();
                return Ok(result);
            }
            "overwrite" => {
                fs::remove_file(&target).map_err(|e| format!("覆盖目标失败: {}", e))?;
            }
            _ => {
                // auto_number（默认）：自动编号避免冲突
                let alt = build_output_path_in_dir(&source_output, &target_dir);
                match fs::rename(&source_output, &alt) {
                    Ok(_) => {}
                    Err(_) => {
                        fs::copy(&source_output, &alt)
                            .map_err(|e| format!("复制到输出目录失败: {}", e))?;
                        fs::remove_file(&source_output)
                            .map_err(|e| format!("清理临时输出失败: {}", e))?;
                    }
                }
                result.output_path = alt.to_string_lossy().to_string();
                return Ok(result);
            }
        }
    }

    // 移动到目标
    match fs::rename(&source_output, &target) {
        Ok(_) => {}
        Err(_) => {
            fs::copy(&source_output, &target).map_err(|e| format!("复制到输出目录失败: {}", e))?;
            fs::remove_file(&source_output).map_err(|e| format!("清理临时输出失败: {}", e))?;
        }
    }
    result.output_path = target.to_string_lossy().to_string();
    Ok(result)
}

fn list_epub_sources(input_paths: &[String], output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for raw in input_paths {
        let path = PathBuf::from(raw);
        if path.is_file() {
            let is_epub = path
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("epub"))
                .unwrap_or(false);
            if is_epub {
                let key = path.to_string_lossy().to_ascii_lowercase();
                if seen.insert(key) {
                    result.push(path);
                }
            }
            continue;
        }
        if path.is_dir() {
            for entry in WalkDir::new(&path).into_iter().filter_map(Result::ok) {
                let item = entry.path();
                if item.is_dir() && is_same_path(item, output_dir) {
                    continue;
                }
                if !item.is_file() {
                    continue;
                }
                if path_is_under(item, output_dir) {
                    continue;
                }
                let is_epub = item
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.eq_ignore_ascii_case("epub"))
                    .unwrap_or(false);
                if is_epub {
                    let key = item.to_string_lossy().to_ascii_lowercase();
                    if seen.insert(key) {
                        result.push(item.to_path_buf());
                    }
                }
            }
            continue;
        }
        return Err(format!("路径不存在: {}", raw));
    }
    result.sort();
    Ok(result)
}

fn collect_epub_sources(
    app: &tauri::AppHandle,
    task_id: &str,
    input_paths: &[String],
    output_dir: &Path,
) -> Result<Vec<PathBuf>, String> {
    let sources = list_epub_sources(input_paths, output_dir)?;
    for (index, path) in sources.iter().enumerate() {
        emit_toolbox_batch_event(
            app,
            ToolboxBatchEvent {
                task_id: task_id.to_string(),
                event: "scan-file".to_string(),
                level: "info".to_string(),
                index: index + 1,
                total: sources.len(),
                input_path: Some(path.to_string_lossy().to_string()),
                output_path: None,
                message: format!("扫描到 EPUB: {}", path.to_string_lossy()),
            },
        );
    }
    Ok(sources)
}

fn run_toolbox_batch_impl(
    app: tauri::AppHandle,
    task_id: String,
    tool: String,
    input_paths: Vec<String>,
    image_format: Option<String>,
    output_dir: Option<String>,
    output_mode: Option<String>,
    conflict_mode: Option<String>,
) -> Result<ToolboxBatchSummary, String> {
    reset_toolbox_batch_cancel(&task_id);
    let output_dir = resolve_batch_output_dir(&input_paths, output_dir);
    let output_mode = output_mode
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "batch_dir".to_string());
    let conflict_mode = conflict_mode
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "auto_number".to_string());
    emit_toolbox_batch_event(
        &app,
        ToolboxBatchEvent {
            task_id: task_id.clone(),
            event: "scan-start".to_string(),
            level: "info".to_string(),
            index: 0,
            total: 0,
            input_path: None,
            output_path: Some(output_dir.to_string_lossy().to_string()),
            message: format!(
                "正在扫描 EPUB 文件，输出目录: {}",
                output_dir.to_string_lossy()
            ),
        },
    );
    let sources = collect_epub_sources(&app, &task_id, &input_paths, &output_dir)?;
    let total = sources.len();
    emit_toolbox_batch_event(
        &app,
        ToolboxBatchEvent {
            task_id: task_id.clone(),
            event: "started".to_string(),
            level: "info".to_string(),
            index: 0,
            total,
            input_path: None,
            output_path: None,
            message: format!("发现 {} 个 EPUB 文件", total),
        },
    );
    if total == 0 {
        emit_toolbox_batch_event(
            &app,
            ToolboxBatchEvent {
                task_id: task_id.clone(),
                event: "finished".to_string(),
                level: "warning".to_string(),
                index: 0,
                total,
                input_path: None,
                output_path: None,
                message: "未找到 EPUB 文件".to_string(),
            },
        );
        return Ok(ToolboxBatchSummary {
            task_id,
            total,
            succeeded: 0,
            failed: 0,
        });
    }

    let normalized_tool = tool.replace('-', "_");
    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let batch_started_at = Instant::now();
    for (idx, source) in sources.iter().enumerate() {
        let index = idx + 1;
        if is_toolbox_batch_cancelled(&task_id) {
            reset_toolbox_batch_cancel(&task_id);
            emit_toolbox_batch_event(
                &app,
                ToolboxBatchEvent {
                    task_id: task_id.clone(),
                    event: "finished".to_string(),
                    level: "warning".to_string(),
                    index: idx,
                    total,
                    input_path: None,
                    output_path: None,
                    message: format!(
                        "批量处理已取消：成功 {}，失败 {}，剩余 {}，总耗时 {}",
                        succeeded,
                        failed,
                        total.saturating_sub(idx),
                        format_toolbox_duration(batch_started_at.elapsed())
                    ),
                },
            );
            return Ok(ToolboxBatchSummary {
                task_id,
                total,
                succeeded,
                failed,
            });
        }
        let file_started_at = Instant::now();
        let mut slow_warned = false;
        let mut cancel_stage_warned = false;
        emit_toolbox_batch_event(
            &app,
            ToolboxBatchEvent {
                task_id: task_id.clone(),
                event: "file-start".to_string(),
                level: "info".to_string(),
                index,
                total,
                input_path: Some(source.to_string_lossy().to_string()),
                output_path: None,
                message: format!("开始处理 {}", source.to_string_lossy()),
            },
        );
        emit_toolbox_batch_stage(&app, &task_id, index, total, source, "读取文件并准备处理");

        let result = match normalized_tool.as_str() {
            "file_encrypt" => toolbox_file_encrypt_impl(source),
            "file_decrypt" => {
                let mut progress = |message: &str| {
                    let elapsed = file_started_at.elapsed();
                    if elapsed >= Duration::from_secs(30) && !slow_warned {
                        slow_warned = true;
                        emit_toolbox_batch_stage_with_level(
                            &app,
                            &task_id,
                            index,
                            total,
                            source,
                            "warning",
                            format!(
                                "当前文件处理超过 {}，仍在继续",
                                format_toolbox_duration(elapsed)
                            ),
                        );
                    }
                    if is_toolbox_batch_cancelled(&task_id) && !cancel_stage_warned {
                        cancel_stage_warned = true;
                        emit_toolbox_batch_stage_with_level(
                            &app,
                            &task_id,
                            index,
                            total,
                            source,
                            "warning",
                            "已收到取消请求，当前文件完成后停止",
                        );
                    }
                    emit_toolbox_batch_stage(
                        &app,
                        &task_id,
                        index,
                        total,
                        source,
                        format!("{}（已用 {}）", message, format_toolbox_duration(elapsed)),
                    );
                };
                toolbox_file_decrypt_impl_with_progress(source, &mut progress)
            }
            "font_encrypt" => toolbox_font_encrypt_impl(source),
            "font_decrypt" => toolbox_font_decrypt_impl(source, None),
            "epub_reformat" => toolbox_epub_reformat_impl(source),
            "epub_diagnose" => toolbox_epub_diagnose_tool_result(source),
            "image_convert" => toolbox_image_convert_impl(source, image_format.as_deref()),
            _ => Err(format!("不支持的批量工具: {}", tool)),
        }
        .and_then(|done| {
            emit_toolbox_batch_stage(&app, &task_id, index, total, source, "整理输出文件");
            finalize_toolbox_output(
                done,
                source,
                &output_dir,
                &output_mode,
                &conflict_mode,
                &input_paths,
            )
        });

        match result {
            Ok(done) => {
                let elapsed = file_started_at.elapsed();
                succeeded += 1;
                emit_toolbox_batch_event(
                    &app,
                    ToolboxBatchEvent {
                        task_id: task_id.clone(),
                        event: "file-done".to_string(),
                        level: if done.action == "epub_diagnose_clean" || done.changed {
                            "info"
                        } else {
                            "warning"
                        }
                        .to_string(),
                        index,
                        total,
                        input_path: Some(done.source_path),
                        output_path: Some(done.output_path),
                        message: append_toolbox_duration(done.message, elapsed),
                    },
                );
            }
            Err(error) => {
                let elapsed = file_started_at.elapsed();
                failed += 1;
                emit_toolbox_batch_event(
                    &app,
                    ToolboxBatchEvent {
                        task_id: task_id.clone(),
                        event: "file-error".to_string(),
                        level: "error".to_string(),
                        index,
                        total,
                        input_path: Some(source.to_string_lossy().to_string()),
                        output_path: None,
                        message: append_toolbox_duration(error, elapsed),
                    },
                );
            }
        }
    }

    emit_toolbox_batch_event(
        &app,
        ToolboxBatchEvent {
            task_id: task_id.clone(),
            event: "finished".to_string(),
            level: if failed == 0 { "info" } else { "warning" }.to_string(),
            index: total,
            total,
            input_path: None,
            output_path: None,
            message: format!(
                "批量处理完成：成功 {}，失败 {}，总耗时 {}",
                succeeded,
                failed,
                format_toolbox_duration(batch_started_at.elapsed())
            ),
        },
    );
    reset_toolbox_batch_cancel(&task_id);
    Ok(ToolboxBatchSummary {
        task_id,
        total,
        succeeded,
        failed,
    })
}

const TOOLBOX_FONT_OBFUSCATION_SCRIPT: &str = r##"
import json
import html as html_lib
import random
import re
import sys
import unicodedata
import zipfile
from collections import Counter, defaultdict
from io import BytesIO
from urllib.parse import unquote

MAP_NAME = "META-INF/tepub-font-obfuscation.json"

def is_html_name(name):
    lower = name.lower()
    return lower.endswith(".html") or lower.endswith(".xhtml") or lower.endswith(".htm")

def is_font_name(name):
    lower = name.lower()
    return lower.endswith(".ttf") or lower.endswith(".otf") or lower.endswith(".woff") or lower.endswith(".woff2")

def decode_text(data):
    for enc in ("utf-8", "utf-8-sig", "gb18030", "latin-1"):
        try:
            return data.decode(enc)
        except UnicodeDecodeError:
            pass
    return data.decode("utf-8", errors="replace")

def is_private_use_char(ch):
    code = ord(ch)
    return 0xE000 <= code <= 0xF8FF or 0xF0000 <= code <= 0x10FFFD

def is_cjk_ideograph(ch):
    code = ord(ch)
    return (
        0x3400 <= code <= 0x4DBF
        or 0x4E00 <= code <= 0x9FFF
        or 0xF900 <= code <= 0xFAFF
        or 0x20000 <= code <= 0x3FFFF
    )

def should_map_char(ch):
    if ch in "<>&":
        return False
    cat = unicodedata.category(ch)
    if cat.startswith(("C", "Z", "P", "S", "N")):
        return False
    if is_private_use_char(ch):
        return False
    return is_cjk_ideograph(ch)

def tag_identity(tag):
    body = tag.strip("<> \t\r\n")
    closing = body.startswith("/")
    if closing:
        body = body[1:].lstrip()
    if not body:
        return closing, ""
    name = body.split(None, 1)[0].split("/", 1)[0].lower()
    return closing, name

def iter_text_chars(html):
    raw_tag = None
    i = 0
    while i < len(html):
        ch = html[i]
        if ch == "<":
            end = html.find(">", i + 1)
            if end == -1:
                break
            tag = html[i:end + 1]
            closing, name = tag_identity(tag)
            if name in ("script", "style"):
                raw_tag = None if closing else name
            i = end + 1
            continue
        if not raw_tag:
            if ch == "&":
                semi = html.find(";", i + 1, i + 16)
                if semi != -1:
                    i = semi + 1
                    continue
            if should_map_char(ch):
                yield ch
        i += 1

def transform_text(html, table):
    out = []
    raw_tag = None
    i = 0
    while i < len(html):
        ch = html[i]
        if ch == "<":
            end = html.find(">", i + 1)
            if end == -1:
                out.append(html[i:])
                break
            tag = html[i:end + 1]
            out.append(tag)
            closing, name = tag_identity(tag)
            if name in ("script", "style"):
                raw_tag = None if closing else name
            i = end + 1
            continue
        if raw_tag:
            out.append(ch)
            i += 1
            continue
        if ch == "&":
            semi = html.find(";", i + 1, i + 16)
            if semi != -1:
                out.append(html[i:semi + 1])
                i = semi + 1
                continue
        out.append(table.get(ch, ch))
        i += 1
    return "".join(out)

def parse_attrs(tag_body):
    attrs = {}
    for m in re.finditer(r"([:\w.-]+)\s*=\s*(['\"])(.*?)\2", tag_body, flags=re.S):
        attrs[m.group(1).lower()] = m.group(3)
    return attrs

def parse_tag(tag):
    body = tag.strip("<> \t\r\n")
    if not body or body.startswith("!") or body.startswith("?"):
        return True, "", True, {}
    closing = body.startswith("/")
    if closing:
        body = body[1:].lstrip()
    self_closing = body.endswith("/")
    if self_closing:
        body = body[:-1].rstrip()
    if not body:
        return closing, "", self_closing, {}
    name = body.split(None, 1)[0].split("/", 1)[0].lower()
    rest = body[len(name):]
    return closing, name, self_closing, parse_attrs(rest)

def first_font_family(value):
    if not value:
        return ""
    parts = []
    buf = []
    quote = None
    for ch in value:
        if quote:
            if ch == quote:
                quote = None
            else:
                buf.append(ch)
            continue
        if ch in ("'", '"'):
            quote = ch
            continue
        if ch == ",":
            part = "".join(buf).strip().strip("'\"")
            if part:
                parts.append(part)
            buf = []
            continue
        buf.append(ch)
    part = "".join(buf).strip().strip("'\"")
    if part:
        parts.append(part)
    return parts[0].lower() if parts else ""

def font_family_from_style(style):
    if not style:
        return ""
    m = re.search(r"(?is)(?:^|;)\s*font-family\s*:\s*([^;}{]+)", style)
    if not m:
        return ""
    return first_font_family(m.group(1))

def simple_selector_key(selector):
    selector = selector.strip()
    selector = selector.split(":", 1)[0].strip()
    if not selector or any(token in selector for token in (" ", ">", "+", "~", "[", "]", "*")):
        return None
    if selector.startswith(".") or selector.startswith("#"):
        return selector.lower()
    if re.fullmatch(r"[a-zA-Z][\w-]*(?:[.#][\w-]+)?", selector):
        return selector.lower()
    return None

def build_selector_font_rules(infos, contents):
    selector_fonts = {}
    for info in infos:
        name = info.filename
        if not name.lower().endswith(".css") or name.endswith("/"):
            continue
        css = decode_text(contents.get(name, b""))
        css = re.sub(r"(?s)/\*.*?\*/", "", css)
        for match in re.finditer(r"(?s)([^{}]+)\{([^{}]*)\}", css):
            selectors = match.group(1).strip()
            body = match.group(2)
            if selectors.startswith("@"):
                continue
            family = font_family_from_style(body)
            if not family:
                continue
            for selector in selectors.split(","):
                key = simple_selector_key(selector)
                if key:
                    selector_fonts[key] = family
    return selector_fonts

def font_for_tag(tag_name, attrs, inherited, selector_fonts):
    inline_family = font_family_from_style(attrs.get("style", ""))
    if inline_family:
        return inline_family
    tag = (tag_name or "").lower()
    elem_id = attrs.get("id", "").strip()
    if elem_id:
        for key in (f"{tag}#{elem_id.lower()}", f"#{elem_id.lower()}"):
            if key in selector_fonts:
                return selector_fonts[key]
    classes = attrs.get("class", "").split()
    for cls in classes:
        cls = cls.lower()
        for key in (f"{tag}.{cls}", f".{cls}"):
            if key in selector_fonts:
                return selector_fonts[key]
    if tag in selector_fonts:
        return selector_fonts[tag]
    return inherited

def zip_parent(name):
    name = name.replace("\\", "/")
    return name.rsplit("/", 1)[0] if "/" in name else ""

def zip_join(base_dir, rel):
    rel = unquote((rel or "").split("#", 1)[0].split("?", 1)[0]).replace("\\", "/").strip()
    if not rel:
        return ""
    if rel.startswith("/"):
        return rel.lstrip("/")
    parts = [p for p in base_dir.split("/") if p]
    for seg in rel.split("/"):
        if not seg or seg == ".":
            continue
        if seg == "..":
            if parts:
                parts.pop()
        else:
            parts.append(seg)
    return "/".join(parts)

def strip_markup_text(text):
    text = re.sub(r"(?is)<(script|style)\b.*?</\1>", "", text)
    text = re.sub(r"(?is)<[^>]+>", "", text)
    text = html_lib.unescape(text)
    return re.sub(r"\s+", " ", text).strip()

def extract_title_text(text):
    m = re.search(r"(?is)<title\b[^>]*>(.*?)</title>", text)
    return strip_markup_text(m.group(1)) if m else ""

def collect_navigation_data(infos, contents):
    opf_paths = [info.filename for info in infos if info.filename.lower().endswith(".opf")]
    nav_paths = set()
    ncx_paths = set()
    title_by_target = {}
    for opf_path in opf_paths:
        opf_text = decode_text(contents.get(opf_path, b""))
        opf_dir = zip_parent(opf_path)
        toc_ids = set()
        for spine in re.finditer(r"(?is)<spine\b([^>]*)>", opf_text):
            toc_id = parse_attrs(spine.group(1)).get("toc", "")
            if toc_id:
                toc_ids.add(toc_id)
        for item in re.finditer(r"(?is)<item\b([^>]*)>", opf_text):
            attrs = parse_attrs(item.group(1))
            item_id = attrs.get("id", "")
            href = attrs.get("href", "")
            if not href:
                continue
            media = attrs.get("media-type", "").lower()
            properties = attrs.get("properties", "").lower().split()
            abs_path = zip_join(opf_dir, href)
            lower = abs_path.lower()
            if "nav" in properties:
                nav_paths.add(abs_path)
            if item_id in toc_ids or media == "application/x-dtbncx+xml" or lower.endswith(".ncx"):
                ncx_paths.add(abs_path)

    for nav_path in list(nav_paths):
        nav_text = decode_text(contents.get(nav_path, b""))
        nav_dir = zip_parent(nav_path)
        for a in re.finditer(r"(?is)<a\b([^>]*)>(.*?)</a>", nav_text):
            href = parse_attrs(a.group(1)).get("href", "")
            label = strip_markup_text(a.group(2))
            target = zip_join(nav_dir, href)
            if target and label:
                title_by_target.setdefault(target, label)

    for ncx_path in list(ncx_paths):
        ncx_text = decode_text(contents.get(ncx_path, b""))
        ncx_dir = zip_parent(ncx_path)
        for point in re.finditer(r"(?is)<navPoint\b.*?</navPoint>", ncx_text):
            block = point.group(0)
            content = re.search(r"(?is)<content\b([^>]*)>", block)
            label = re.search(r"(?is)<text\b[^>]*>(.*?)</text>", block)
            if not content or not label:
                continue
            src = parse_attrs(content.group(1)).get("src", "")
            target = zip_join(ncx_dir, src)
            text = strip_markup_text(label.group(1))
            if target and text:
                title_by_target.setdefault(target, text)
    return nav_paths, ncx_paths, title_by_target

def collect_title_pairs(contents, title_by_target):
    pairs = []
    for target, plain_title in title_by_target.items():
        html = contents.get(target)
        if html is None:
            continue
        cipher_title = extract_title_text(decode_text(html))
        if cipher_title and plain_title and cipher_title != plain_title:
            pairs.append((cipher_title, plain_title))
    return pairs

def is_ignored_text_char(ch):
    cat = unicodedata.category(ch)
    return ch.isspace() or cat.startswith("C") or cat.startswith("Z")

def is_cjk_or_private(ch):
    code = ord(ch)
    return (
        0x3400 <= code <= 0x4DBF
        or 0x4E00 <= code <= 0x9FFF
        or 0xF900 <= code <= 0xFAFF
        or 0x20000 <= code <= 0x3FFFF
        or 0xE000 <= code <= 0xF8FF
        or 0xF0000 <= code <= 0x10FFFD
    )

def is_alignment_stable(ch):
    if is_ignored_text_char(ch):
        return False
    if is_cjk_or_private(ch):
        return False
    cat = unicodedata.category(ch)
    return ord(ch) < 128 or cat[0] in ("P", "S", "N") or 0xFF00 <= ord(ch) <= 0xFFEF

def is_mapping_candidate(ch):
    return not is_ignored_text_char(ch) and not is_alignment_stable(ch)

def iter_text_records(html, selector_fonts=None):
    selector_fonts = selector_fonts or {}
    stack = []
    raw_tag = None
    i = 0
    while i < len(html):
        ch = html[i]
        if ch == "<":
            end = html.find(">", i + 1)
            if end == -1:
                break
            tag = html[i:end + 1]
            closing, name, self_closing, attrs = parse_tag(tag)
            if name in ("script", "style", "title"):
                raw_tag = None if closing else name
                i = end + 1
                continue
            if name and not raw_tag:
                if closing:
                    while stack:
                        popped, _font = stack.pop()
                        if popped == name:
                            break
                elif not self_closing:
                    inherited = stack[-1][1] if stack else ""
                    stack.append((name, font_for_tag(name, attrs, inherited, selector_fonts)))
            elif name and closing:
                while stack:
                    popped, _font = stack.pop()
                    if popped == name:
                        break
            i = end + 1
            continue
        if raw_tag:
            i += 1
            continue
        if ch == "&":
            semi = html.find(";", i + 1, i + 16)
            if semi != -1:
                i = semi + 1
                continue
        if not is_ignored_text_char(ch):
            yield ch, (stack[-1][1] if stack else "")
        i += 1

def transform_text_font_aware(html, global_table, font_table, selector_fonts=None):
    selector_fonts = selector_fonts or {}
    out = []
    stack = []
    raw_tag = None
    i = 0
    while i < len(html):
        ch = html[i]
        if ch == "<":
            end = html.find(">", i + 1)
            if end == -1:
                out.append(html[i:])
                break
            tag = html[i:end + 1]
            out.append(tag)
            closing, name, self_closing, attrs = parse_tag(tag)
            if name in ("script", "style"):
                raw_tag = None if closing else name
                i = end + 1
                continue
            if name and not raw_tag:
                if closing:
                    while stack:
                        popped, _font = stack.pop()
                        if popped == name:
                            break
                elif not self_closing:
                    inherited = stack[-1][1] if stack else ""
                    stack.append((name, font_for_tag(name, attrs, inherited, selector_fonts)))
            elif name and closing:
                while stack:
                    popped, _font = stack.pop()
                    if popped == name:
                        break
            i = end + 1
            continue
        if raw_tag:
            out.append(ch)
            i += 1
            continue
        if ch == "&":
            semi = html.find(";", i + 1, i + 16)
            if semi != -1:
                out.append(html[i:semi + 1])
                i = semi + 1
                continue
        font_key = stack[-1][1] if stack else ""
        out.append(font_table.get((font_key, ch), global_table.get(ch, ch)))
        i += 1
    return "".join(out)

def normalize_plain_chars(text):
    return [ch for ch in text if not is_ignored_text_char(ch)]

def char_at(seq, index):
    item = seq[index]
    return item[0] if isinstance(item, tuple) else item

def stable_signature(seq, pos, want=8, max_scan=1600):
    sig = []
    end = min(len(seq), pos + max_scan)
    while pos < end and len(sig) < want:
        ch = char_at(seq, pos)
        if is_alignment_stable(ch):
            sig.append(ch)
        pos += 1
    return "".join(sig)

def find_resync(cipher_records, ci, plain_chars, pi, window=2400, anchors=8):
    plain_sigs = {}
    plain_end = min(len(plain_chars), pi + window)
    for j in range(pi, plain_end):
        sig = stable_signature(plain_chars, j, anchors)
        if len(sig) >= anchors and sig not in plain_sigs:
            plain_sigs[sig] = j
    cipher_end = min(len(cipher_records), ci + window)
    best = None
    best_cost = None
    for i in range(ci, cipher_end):
        sig = stable_signature(cipher_records, i, anchors)
        if len(sig) < anchors:
            continue
        j = plain_sigs.get(sig)
        if j is None:
            continue
        cost = (i - ci) + (j - pi)
        if best is None or cost < best_cost:
            best = (i, j)
            best_cost = cost
            if cost == 0:
                break
    return best

def decode_txt_file(txt_path):
    with open(txt_path, "rb") as f:
        return decode_text(f.read())

def pick_mapping(counter):
    if not counter:
        return None, 0, 0, 0.0
    total = sum(counter.values())
    plain, top = counter.most_common(1)[0]
    ratio = top / total if total else 0.0
    return plain, top, total, ratio

def choose_mapping_table(counts):
    table = {}
    conflicts = 0
    for key, counter in counts.items():
        plain, top, total, ratio = pick_mapping(counter)
        if plain is None:
            continue
        if total >= 5 and ratio < 0.78:
            conflicts += 1
        if ratio >= 0.66 and (top >= 2 or top == total):
            table[key] = plain
    return table, conflicts

def add_segment_pairs(cipher_records, ci0, ci1, plain_chars, pi0, pi1, global_counts, font_counts):
    cipher_idxs = [idx for idx in range(ci0, ci1) if is_mapping_candidate(cipher_records[idx][0])]
    plain_idxs = [idx for idx in range(pi0, pi1) if is_mapping_candidate(plain_chars[idx])]
    if len(cipher_idxs) != len(plain_idxs):
        return 0
    added = 0
    for ci, pi in zip(cipher_idxs, plain_idxs):
        cipher, font_key = cipher_records[ci]
        plain = plain_chars[pi]
        global_counts[cipher][plain] += 1
        if font_key:
            font_counts[(font_key, cipher)][plain] += 1
        added += 1
    return added

def add_direct_title_pair(cipher_title, plain_title, global_counts, font_counts, weight=4):
    cipher_chars = normalize_plain_chars(cipher_title)
    plain_chars = normalize_plain_chars(plain_title)
    if len(cipher_chars) != len(plain_chars):
        return 0
    added = 0
    for cipher, plain in zip(cipher_chars, plain_chars):
        if cipher == plain:
            continue
        if is_mapping_candidate(cipher) and is_mapping_candidate(plain):
            global_counts[cipher][plain] += weight
            added += weight
    return added

def build_txt_alignment_tables(cipher_records, plain_chars, direct_title_pairs=None):
    global_counts = defaultdict(Counter)
    font_counts = defaultdict(Counter)
    ci = 0
    pi = 0
    pairs = 0
    stable_matches = 0
    skipped_cipher = 0
    skipped_plain = 0
    while ci < len(cipher_records) and pi < len(plain_chars):
        cipher, font_key = cipher_records[ci]
        plain = plain_chars[pi]
        cipher_stable = is_alignment_stable(cipher)
        plain_stable = is_alignment_stable(plain)
        if cipher_stable or plain_stable:
            if cipher_stable and plain_stable and cipher == plain:
                ci += 1
                pi += 1
                stable_matches += 1
                continue
            resync = find_resync(cipher_records, ci, plain_chars, pi)
            if resync and (resync[0] > ci or resync[1] > pi):
                pairs += add_segment_pairs(
                    cipher_records, ci, resync[0], plain_chars, pi, resync[1], global_counts, font_counts
                )
                skipped_cipher += max(0, resync[0] - ci)
                skipped_plain += max(0, resync[1] - pi)
                ci, pi = resync
                continue
            if cipher_stable and not plain_stable:
                ci += 1
                skipped_cipher += 1
            elif plain_stable and not cipher_stable:
                pi += 1
                skipped_plain += 1
            else:
                ci += 1
                pi += 1
                skipped_cipher += 1
                skipped_plain += 1
            continue
        if is_mapping_candidate(cipher) and is_mapping_candidate(plain):
            global_counts[cipher][plain] += 1
            if font_key:
                font_counts[(font_key, cipher)][plain] += 1
            pairs += 1
        ci += 1
        pi += 1
    title_pairs = 0
    title_votes = 0
    for cipher_title, plain_title in direct_title_pairs or []:
        added = add_direct_title_pair(cipher_title, plain_title, global_counts, font_counts)
        if added:
            title_pairs += 1
            title_votes += added
    global_table, global_conflicts = choose_mapping_table(global_counts)
    font_table, font_conflicts = choose_mapping_table(font_counts)
    stats = {
        "pairs": pairs,
        "stable": stable_matches,
        "skippedCipher": skipped_cipher,
        "skippedPlain": skipped_plain,
        "globalChars": len(global_table),
        "fontChars": len(font_table),
        "globalConflicts": global_conflicts,
        "fontConflicts": font_conflicts,
        "titlePairs": title_pairs,
        "titleVotes": title_votes,
    }
    return global_table, font_table, stats

def neutralize_css_fonts(text):
    text = re.sub(r"(?is)@font-face\s*\{.*?\}", "", text)
    text = re.sub(r"(?is)font-family\s*:\s*[^;}{]+;?", "font-family: serif;", text)
    return text

def neutralize_html_inline_fonts(text):
    return re.sub(r"(?is)font-family\s*:\s*[^;\"'}]+;?", "font-family: serif;", text)

def private_chars(count, excluded=None):
    excluded = set(excluded or ())
    ranges = [(0xE000, 0xF8FF), (0xF0000, 0xFFFFD), (0x100000, 0x10FFFD)]
    available = []
    for start, end in ranges:
        for code in range(start, end + 1):
            ch = chr(code)
            if ch not in excluded:
                available.append(ch)
    if len(available) >= count:
        random.SystemRandom().shuffle(available)
        return available[:count]
    raise RuntimeError("可用私用区字符不足")

def collect_font_chars(font_items):
    from fontTools.ttLib import TTFont

    supported = set()
    occupied_private = set()
    coverage_by_name = {}
    for name, font_data in font_items:
        try:
            font = TTFont(BytesIO(font_data))
            best = font.getBestCmap() or {}
            covered = set()
            for code in best.keys():
                ch = chr(code)
                if should_map_char(ch):
                    supported.add(ch)
                    covered.add(ch)
                if is_private_use_char(ch):
                    occupied_private.add(ch)
            coverage_by_name[name.lower()] = len(covered)
        except Exception:
            continue
    return supported, occupied_private, coverage_by_name

def split_font_families(value):
    families = []
    current = []
    quote = None
    for ch in value:
        if quote:
            current.append(ch)
            if ch == quote:
                quote = None
        elif ch in ("'", '"'):
            quote = ch
            current.append(ch)
        elif ch == ",":
            item = "".join(current).strip()
            if item:
                families.append(item)
            current = []
        else:
            current.append(ch)
    item = "".join(current).strip()
    if item:
        families.append(item)
    return families

def unquote_font_family(value):
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        value = value[1:-1]
    return value

def normalize_font_family(value):
    return unquote_font_family(value).lower()

def css_font_family(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'

def append_font_fallbacks(value, fallback_families):
    important = ""
    important_match = re.search(r"\s*!important\s*$", value, flags=re.I)
    if important_match:
        important = important_match.group(0)
        value = value[:important_match.start()]
    families = split_font_families(value)
    existing = {normalize_font_family(family) for family in families}
    additions = [css_font_family(family) for family in fallback_families if family.lower() not in existing]
    if not additions:
        return value + important
    generics = {
        "serif", "sans-serif", "monospace", "cursive", "fantasy", "system-ui", "ui-serif",
        "ui-sans-serif", "ui-monospace", "ui-rounded", "emoji", "math", "fangsong"
    }
    insert_at = next((index for index, family in enumerate(families) if normalize_font_family(family) in generics), len(families))
    families[insert_at:insert_at] = additions
    return ", ".join(families) + important

def rewrite_css_font_fallbacks(source, fallback_families):
    if not fallback_families:
        return source
    font_faces = []
    def mask_face(match):
        marker = "__TEPUB_FONT_FACE_{}__".format(len(font_faces))
        font_faces.append(match.group(0))
        return marker
    masked = re.sub(r"(?is)@font-face\s*\{[^{}]*\}", mask_face, source)
    rewritten = re.sub(
        r"(?is)(font-family\s*:\s*)([^;}{]+)",
        lambda match: match.group(1) + append_font_fallbacks(match.group(2), fallback_families),
        masked,
    )
    return re.sub(
        r"__TEPUB_FONT_FACE_(\d+)__",
        lambda match: font_faces[int(match.group(1))] if int(match.group(1)) < len(font_faces) else "",
        rewritten,
    )

def rewrite_inline_font_fallbacks(source, fallback_families):
    if not fallback_families:
        return source
    def rewrite_style(match):
        style = re.sub(
            r"(?is)(font-family\s*:\s*)([^;]+)",
            lambda declaration: declaration.group(1) + append_font_fallbacks(declaration.group(2), fallback_families),
            match.group(3),
        )
        return match.group(1) + match.group(2) + style + match.group(2)
    return re.sub(r"(?is)(style\s*=\s*)(['\"])(.*?)\2", rewrite_style, source)

def inject_obfuscated_text_compatibility(source, fallback_families):
    if re.search(r"data-tepub-font-obfuscation\s*=", source, flags=re.I):
        return source
    default_family = ""
    if fallback_families:
        default_family = "html { font-family: " + ", ".join(css_font_family(family) for family in fallback_families) + ", serif; }\n"
    style = '<style type="text/css" data-tepub-font-obfuscation="1">' + default_family + 'html, body { overflow-wrap: anywhere; word-break: break-all; }</style>'
    if re.search(r"</head\s*>", source, flags=re.I):
        return re.sub(r"</head\s*>", lambda _match: style + "</head>", source, count=1, flags=re.I)
    return style + source

def collect_embedded_font_families(infos, contents, coverage_by_name):
    ranked = []
    seen = set()
    for info in infos:
        name = info.filename
        if not name.lower().endswith(".css") or name.endswith("/"):
            continue
        source = decode_text(contents.get(name, b""))
        for face in re.finditer(r"(?is)@font-face\s*\{([^{}]*)\}", source):
            body = face.group(1)
            family_match = re.search(r"(?is)font-family\s*:\s*([^;}{]+)", body)
            if not family_match:
                continue
            family = unquote_font_family(family_match.group(1))
            family_key = family.lower()
            if not family or family_key in seen:
                continue
            scores = []
            for url_match in re.finditer(r"(?is)url\(\s*(['\"]?)(.*?)\1\s*\)", body):
                target = zip_join(zip_parent(name), url_match.group(2))
                if target.lower() in coverage_by_name:
                    scores.append(coverage_by_name[target.lower()])
            if scores:
                seen.add(family_key)
                ranked.append((family, max(scores), len(ranked)))
    ranked.sort(key=lambda item: (-item[1], item[2]))
    return [family for family, _score, _order in ranked]

def add_private_cmap(font_data, mapping):
    from fontTools.ttLib import TTFont

    font = TTFont(BytesIO(font_data))
    best = font.getBestCmap() or {}
    if "cmap" not in font:
        return font_data
    added = 0
    for plain, shadow in mapping.items():
        glyph = best.get(ord(plain))
        if not glyph:
            continue
        for table in font["cmap"].tables:
            if table.isUnicode():
                table.cmap[ord(shadow)] = glyph
                added += 1
    if added == 0:
        return font_data
    out = BytesIO()
    font.save(out)
    return out.getvalue()

def write_entries(output_path, infos, contents, extra_map=None, skip_map=False):
    with zipfile.ZipFile(output_path, "w") as zout:
        for info in infos:
            if info.filename == "mimetype":
                zi = zipfile.ZipInfo(info.filename)
                zi.compress_type = zipfile.ZIP_STORED
                zout.writestr(zi, contents[info.filename])
                break
        for info in infos:
            name = info.filename
            if name == "mimetype":
                continue
            if skip_map and name == MAP_NAME:
                continue
            zi = zipfile.ZipInfo(name)
            zi.compress_type = info.compress_type if info.compress_type is not None else zipfile.ZIP_DEFLATED
            zi.external_attr = info.external_attr
            zi.date_time = info.date_time
            if name.endswith("/"):
                zout.writestr(zi, b"")
            else:
                zout.writestr(zi, contents[name])
        if extra_map is not None:
            zi = zipfile.ZipInfo(MAP_NAME)
            zi.compress_type = zipfile.ZIP_DEFLATED
            zout.writestr(zi, json.dumps(extra_map, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))

BODY_ENCRYPTION_CLASS = "tepub-font-encrypted-body"
BODY_ENCRYPTION_FAMILY = "TEpubEncryptedBodyFont"

def update_body_tag_class(tag, add):
    match = re.search(r"\bclass\s*=\s*(['\"])(.*?)\1", tag, flags=re.I | re.S)
    if match:
        classes = [name for name in match.group(2).split() if name != BODY_ENCRYPTION_CLASS]
        if add:
            classes.append(BODY_ENCRYPTION_CLASS)
        if not classes:
            return tag[:match.start()] + tag[match.end():]
        replacement = "class=" + match.group(1) + " ".join(classes) + match.group(1)
        return tag[:match.start()] + replacement + tag[match.end():]
    if not add:
        return tag
    return re.sub(r"\s*(/?)>$", ' class="' + BODY_ENCRYPTION_CLASS + r'"\1>', tag)

def is_title_like_tag(tag, name):
    if re.fullmatch(r"h[1-6]", name or "") or name in ("title", "header", "nav"):
        return True
    attrs = parse_attrs(tag)
    marker = (attrs.get("class", "") + " " + attrs.get("id", "")).lower()
    return bool(
        re.search(r"(^|[\s_-])(title|subtitle|heading|headline)([\s_-]|$)", marker)
        or re.search(r"(^|[\s_-])(chapter|volume|part|section)[\s_-]*(title|subtitle|number|name)([\s_-]|$)", marker)
    )

def transform_body_paragraphs(source, mapping, mode="encrypt"):
    out = []
    paragraph_depth = 0
    title_depth = 0
    element_stack = []
    raw_tag = None
    i = 0
    while i < len(source):
        if source[i] == "<":
            end = source.find(">", i + 1)
            if end < 0:
                out.append(source[i:])
                break
            tag = source[i:end + 1]
            closing, name = tag_identity(tag)
            self_closing = bool(re.search(r"/\s*>$", tag))
            class_match = re.search(r"\bclass\s*=\s*(['\"])(.*?)\1", tag, flags=re.I | re.S)
            marked = bool(class_match and BODY_ENCRYPTION_CLASS in class_match.group(2).split())
            if closing:
                while element_stack:
                    popped_name, popped_paragraph, popped_title = element_stack.pop()
                    paragraph_depth -= int(popped_paragraph)
                    title_depth -= int(popped_title)
                    if popped_name == name:
                        break
            if not closing and not self_closing:
                title_like = is_title_like_tag(tag, name)
                target = (name == "p" and title_depth == 0 and not title_like) if mode == "encrypt" else marked
                if target:
                    paragraph_depth += 1
                    tag = update_body_tag_class(tag, mode == "encrypt")
                if title_like:
                    title_depth += 1
                element_stack.append((name, target, title_like))
            if name in ("script", "style"):
                raw_tag = None if closing else name
            out.append(tag)
            i = end + 1
            continue
        if not raw_tag and source[i] == "&":
            end = source.find(";", i + 1, i + 16)
            if end >= 0:
                out.append(source[i:end + 1])
                i = end + 1
                continue
        ch = source[i]
        out.append(mapping.get(ch, ch) if not raw_tag and paragraph_depth > 0 else ch)
        i += 1
    return "".join(out)

def collect_body_and_protected_chars(source):
    body_chars = []
    protected_chars = []
    paragraph_depth = 0
    title_depth = 0
    element_stack = []
    raw_tag = None
    i = 0
    while i < len(source):
        if source[i] == "<":
            end = source.find(">", i + 1)
            if end < 0:
                break
            tag = source[i:end + 1]
            closing, name = tag_identity(tag)
            self_closing = bool(re.search(r"/\s*>$", tag))
            if closing:
                while element_stack:
                    popped_name, popped_paragraph, popped_title = element_stack.pop()
                    paragraph_depth -= int(popped_paragraph)
                    title_depth -= int(popped_title)
                    if popped_name == name:
                        break
            elif not self_closing:
                title_like = is_title_like_tag(tag, name)
                paragraph = name == "p" and title_depth == 0 and not title_like
                paragraph_depth += int(paragraph)
                title_depth += int(title_like)
                element_stack.append((name, paragraph, title_like))
            if name in ("script", "style"):
                raw_tag = None if closing else name
            i = end + 1
            continue
        if not raw_tag and source[i] == "&":
            end = source.find(";", i + 1, i + 16)
            if end >= 0:
                i = end + 1
                continue
        ch = source[i]
        if not raw_tag and should_map_char(ch):
            if paragraph_depth > 0 and title_depth == 0:
                body_chars.append(ch)
            else:
                protected_chars.append(ch)
        i += 1
    return body_chars, protected_chars

def collect_spine_html_names(infos, contents):
    names = {info.filename for info in infos}
    for info in infos:
        opf_path = info.filename
        if not opf_path.lower().endswith(".opf"):
            continue
        opf = decode_text(contents.get(opf_path, b""))
        opf_dir = zip_parent(opf_path)
        manifest = {}
        for item in re.finditer(r"(?is)<item\b([^>]*)>", opf):
            attrs = parse_attrs(item.group(1))
            if attrs.get("id") and attrs.get("href"):
                manifest[attrs["id"]] = zip_join(opf_dir, attrs["href"])
        spine = []
        for itemref in re.finditer(r"(?is)<itemref\b([^>]*)>", opf):
            target = manifest.get(parse_attrs(itemref.group(1)).get("idref", ""), "")
            if target in names and is_html_name(target):
                spine.append(target)
        if spine:
            return spine
    return [info.filename for info in infos if is_html_name(info.filename)]

def relative_zip_path(from_file, target):
    source_parts = [part for part in zip_parent(from_file).split("/") if part]
    target_parts = [part for part in target.replace("\\", "/").split("/") if part]
    while source_parts and target_parts and source_parts[0].lower() == target_parts[0].lower():
        source_parts.pop(0)
        target_parts.pop(0)
    return "/".join([".."] * len(source_parts) + target_parts)

def inject_body_font_style(source, html_path, font_path):
    source = re.sub(
        r"(?is)<style[^>]*data-tepub-body-font-encryption\s*=\s*(['\"])1\1[^>]*>.*?</style>",
        "",
        source,
    )
    font_url = relative_zip_path(html_path, font_path).replace('"', "%22")
    style = (
        '<style type="text/css" data-tepub-body-font-encryption="1">'
        '@font-face { font-family: "' + BODY_ENCRYPTION_FAMILY + '"; src: url("' + font_url + '"); } '
        '.' + BODY_ENCRYPTION_CLASS + ', .' + BODY_ENCRYPTION_CLASS + ' * { font-family: "' + BODY_ENCRYPTION_FAMILY + '" !important; }'
        '</style>'
    )
    if re.search(r"</head\s*>", source, flags=re.I):
        return re.sub(r"</head\s*>", lambda _match: style + "</head>", source, count=1, flags=re.I)
    return style + source

def random_derangement(chars):
    shuffled = list(chars)
    random_source = random.SystemRandom()
    for index in range(len(shuffled) - 1, 0, -1):
        target = random_source.randrange(index)
        shuffled[index], shuffled[target] = shuffled[target], shuffled[index]
    return shuffled

def permute_font_cmap(font_data, mapping):
    from fontTools.ttLib import TTFont

    font = TTFont(BytesIO(font_data))
    best = font.getBestCmap() or {}
    if "cmap" not in font:
        return font_data
    plain_codes = {ord(ch) for ch in mapping}
    changed = 0
    for table in font["cmap"].tables:
        if not table.isUnicode():
            continue
        for code in plain_codes:
            table.cmap.pop(code, None)
        for plain, cipher in mapping.items():
            glyph = best.get(ord(plain))
            cipher_code = ord(cipher)
            if glyph and (table.format != 4 or cipher_code <= 0xFFFF):
                table.cmap[cipher_code] = glyph
                changed += 1
    if not changed:
        return font_data
    out = BytesIO()
    font.save(out)
    return out.getvalue()

def encrypt_epub_body_permutation(input_path, output_path):
    from fontTools.ttLib import TTFont

    with zipfile.ZipFile(input_path, "r") as zin:
        infos = zin.infolist()
        contents = {info.filename: zin.read(info.filename) for info in infos if not info.filename.endswith("/")}
        for info in infos:
            if info.filename.endswith("/"):
                contents[info.filename] = b""
        font_names = [info.filename for info in infos if is_font_name(info.filename)]
        html_names = collect_spine_html_names(infos, contents)
        if not font_names:
            raise RuntimeError("EPUB 内未找到字体文件")
        if not html_names:
            raise RuntimeError("EPUB 内未找到正文 HTML/XHTML 文件")
        html_sources = {name: decode_text(contents[name]) for name in html_names}
        body_chars = set()
        protected_chars = set()
        for source in html_sources.values():
            body, protected = collect_body_and_protected_chars(source)
            body_chars.update(body)
            protected_chars.update(protected)
        body_chars.difference_update(protected_chars)
        if len(body_chars) < 2:
            raise RuntimeError("未找到可加密的正文段落汉字")
        candidates = []
        for name in font_names:
            try:
                font = TTFont(BytesIO(contents[name]))
                supported = {chr(code) for code in (font.getBestCmap() or {}) if is_cjk_ideograph(chr(code))}
                chars = [ch for ch in body_chars if ch in supported]
                candidates.append((len(chars), name, chars))
            except Exception:
                continue
        if not candidates:
            raise RuntimeError("正文字体中未找到可加密汉字")
        _score, selected_font, chars = max(candidates, key=lambda item: item[0])
        if len(chars) < 2:
            raise RuntimeError("正文字体中未找到足够的可加密汉字")
        chars = sorted(chars)
        mapping = dict(zip(chars, random_derangement(chars)))
        changed_files = 0
        for name, source in html_sources.items():
            transformed = transform_body_paragraphs(source, mapping, "encrypt")
            if transformed != source:
                contents[name] = inject_body_font_style(transformed, name, selected_font).encode("utf-8")
                changed_files += 1
        if not changed_files:
            raise RuntimeError("未发现可加密的正文段落")
        new_font = permute_font_cmap(contents[selected_font], mapping)
        if new_font == contents[selected_font]:
            raise RuntimeError("正文字体 cmap 未能完成随机置换")
        contents[selected_font] = new_font
        write_entries(output_path, infos, contents, extra_map=None, skip_map=True)
        return len(mapping), 1

def encrypt_epub(input_path, output_path):
    with zipfile.ZipFile(input_path, "r") as zin:
        infos = zin.infolist()
        contents = {info.filename: zin.read(info.filename) for info in infos if not info.filename.endswith("/")}
        for info in infos:
            if info.filename.endswith("/"):
                contents[info.filename] = b""
        font_names = [info.filename for info in infos if is_font_name(info.filename)]
        html_names = [info.filename for info in infos if is_html_name(info.filename)]
        if not font_names:
            raise RuntimeError("EPUB 内未找到字体文件")
        if not html_names:
            raise RuntimeError("EPUB 内未找到 HTML/XHTML 文件")
        supported_chars, existing_private_chars, coverage_by_name = collect_font_chars(
            (name, contents[name]) for name in font_names
        )
        fallback_families = collect_embedded_font_families(infos, contents, coverage_by_name)
        if not supported_chars:
            raise RuntimeError("内嵌字体未发现可加密汉字")
        chars = []
        seen = set()
        for name in html_names:
            html = decode_text(contents[name])
            existing_private_chars.update(ch for ch in html if is_private_use_char(ch))
            for ch in iter_text_chars(html):
                if ch in supported_chars and ch not in seen:
                    seen.add(ch)
                    chars.append(ch)
        if not chars:
            raise RuntimeError("未找到可加密的正文汉字")
        shadows = private_chars(len(chars), existing_private_chars)
        mapping = dict(zip(chars, shadows))
        for name in html_names:
            transformed = transform_text(decode_text(contents[name]), mapping)
            transformed = rewrite_inline_font_fallbacks(transformed, fallback_families)
            transformed = inject_obfuscated_text_compatibility(transformed, fallback_families)
            contents[name] = transformed.encode("utf-8")
        for info in infos:
            name = info.filename
            if name.lower().endswith(".css") and not name.endswith("/"):
                contents[name] = rewrite_css_font_fallbacks(
                    decode_text(contents[name]), fallback_families
                ).encode("utf-8")
        changed_fonts = 0
        for name in font_names:
            new_data = add_private_cmap(contents[name], mapping)
            if new_data != contents[name]:
                changed_fonts += 1
                contents[name] = new_data
        if changed_fonts == 0:
            raise RuntimeError("字体 cmap 未能写入私用区映射，无法完成字体加密")
        write_entries(output_path, infos, contents, extra_map=None, skip_map=True)
        return len(chars), changed_fonts

def decrypt_epub_with_saved_map(input_path, output_path):
    with zipfile.ZipFile(input_path, "r") as zin:
        infos = zin.infolist()
        names = [info.filename for info in infos]
        if MAP_NAME not in names:
            raise RuntimeError("未找到字体加密映射文件，无法自动解密")
        payload = json.loads(zin.read(MAP_NAME).decode("utf-8"))
        pairs = payload.get("map") or []
        reverse = {shadow: plain for plain, shadow in pairs}
        if not reverse:
            raise RuntimeError("字体加密映射为空")
        contents = {info.filename: zin.read(info.filename) for info in infos if not info.filename.endswith("/")}
        for info in infos:
            if info.filename.endswith("/"):
                contents[info.filename] = b""
        restored = 0
        for info in infos:
            name = info.filename
            if is_html_name(name):
                text = decode_text(contents[name])
                new_text = transform_text(text, reverse)
                if new_text != text:
                    restored += 1
                    contents[name] = new_text.encode("utf-8")
        if restored == 0:
            raise RuntimeError("未发现可恢复的字体加密正文")
        write_entries(output_path, infos, contents, extra_map=None, skip_map=True)
        return {"mode": "saved-map", "files": restored, "chars": len(reverse)}

def decrypt_epub_with_txt_alignment(input_path, output_path, txt_path):
    if not txt_path:
        raise RuntimeError("未找到 TEpub 字体映射文件，请选择与 EPUB 对应的明文 TXT 进行对齐解密")
    with zipfile.ZipFile(input_path, "r") as zin:
        infos = zin.infolist()
        contents = {info.filename: zin.read(info.filename) for info in infos if not info.filename.endswith("/")}
        for info in infos:
            if info.filename.endswith("/"):
                contents[info.filename] = b""
        html_names = [info.filename for info in infos if is_html_name(info.filename)]
        if not html_names:
            raise RuntimeError("EPUB 内未找到 HTML/XHTML 文件")
        nav_paths, ncx_paths, title_by_target = collect_navigation_data(infos, contents)
        nav_path_keys = {path.lower() for path in nav_paths}
        content_html_names = [name for name in html_names if name.lower() not in nav_path_keys]
        direct_title_pairs = collect_title_pairs(contents, title_by_target)
        selector_fonts = build_selector_font_rules(infos, contents)
        cipher_records = []
        for name in content_html_names:
            cipher_records.extend(iter_text_records(decode_text(contents[name]), selector_fonts))
        plain_chars = normalize_plain_chars(decode_txt_file(txt_path))
        if len(cipher_records) < 50 or len(plain_chars) < 50:
            raise RuntimeError("EPUB 或 TXT 文本过短，无法可靠对齐")
        global_table, font_table, stats = build_txt_alignment_tables(cipher_records, plain_chars, direct_title_pairs)
        if stats["pairs"] < 30 or (not global_table and not font_table):
            raise RuntimeError(
                "TXT 与 EPUB 对齐失败，未能生成可靠映射。请确认 TXT 与 EPUB 是同一版本。"
            )
        changed = 0
        for info in infos:
            name = info.filename
            if name.endswith("/"):
                continue
            lower = name.lower()
            if is_html_name(name) and name.lower() not in nav_path_keys:
                old_text = decode_text(contents[name])
                new_text = transform_text_font_aware(old_text, global_table, font_table, selector_fonts)
                new_text = neutralize_html_inline_fonts(new_text)
                if new_text != old_text:
                    contents[name] = new_text.encode("utf-8")
                    changed += 1
            elif lower.endswith(".css"):
                old_text = decode_text(contents[name])
                new_text = neutralize_css_fonts(old_text)
                if new_text != old_text:
                    contents[name] = new_text.encode("utf-8")
        if changed == 0:
            raise RuntimeError("未发现可替换的字体混淆正文")
        write_entries(output_path, infos, contents, extra_map=None, skip_map=True)
        stats["mode"] = "txt-alignment-font-aware" if font_table else "txt-alignment-global"
        stats["files"] = changed
        stats["skippedNavFiles"] = len(nav_paths) + len(ncx_paths)
        return stats

def decrypt_epub(input_path, output_path, txt_path=None):
    with zipfile.ZipFile(input_path, "r") as zin:
        names = zin.namelist()
    if MAP_NAME in names:
        return decrypt_epub_with_saved_map(input_path, output_path)
    return decrypt_epub_with_txt_alignment(input_path, output_path, txt_path)

def main():
    if len(sys.argv) not in (4, 5):
        raise SystemExit("usage: font_obfuscation.py encrypt|decrypt input.epub output.epub [plain.txt]")
    mode, input_path, output_path = sys.argv[1:4]
    txt_path = sys.argv[4] if len(sys.argv) == 5 else None
    if mode == "encrypt":
        chars, fonts = encrypt_epub_body_permutation(input_path, output_path)
        print(json.dumps({"chars": chars, "fonts": fonts}, ensure_ascii=False))
    elif mode == "decrypt":
        result = decrypt_epub(input_path, output_path, txt_path)
        print(json.dumps(result, ensure_ascii=False))
    else:
        raise SystemExit("unknown mode: " + mode)

if __name__ == "__main__":
    main()
"##;

fn run_toolbox_font_obfuscation(
    source: &Path,
    output_path: &Path,
    mode: &str,
    txt_path: Option<&Path>,
) -> Result<String, String> {
    if mode == "encrypt" {
        ensure_fonttools_available()?;
    }
    let temp_dir = tempfile::tempdir().map_err(|e| format!("创建字体工具临时目录失败: {}", e))?;
    let script_path = temp_dir.path().join("tepub_font_obfuscation.py");
    fs::write(&script_path, TOOLBOX_FONT_OBFUSCATION_SCRIPT)
        .map_err(|e| format!("写入字体工具脚本失败: {}", e))?;
    let mut command = hidden_process_command("python");
    command
        .arg(&script_path)
        .arg(mode)
        .arg(source)
        .arg(output_path);
    if let Some(txt_path) = txt_path {
        command.arg(txt_path);
    }
    let output = command
        .output()
        .map_err(|e| format!("无法启动 Python 字体工具: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn toolbox_font_encrypt_impl(source: &Path) -> Result<ToolboxEpubToolResult, String> {
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    let output_path = build_processed_epub_path(source, "_font_encrypt");
    let details = run_toolbox_font_obfuscation(source, &output_path, "encrypt", None)?;
    Ok(toolbox_epub_tool_result(
        source,
        &output_path,
        true,
        "font_encrypt",
        if details.is_empty() {
            "字体加密完成".to_string()
        } else {
            format!("字体加密完成: {}", details)
        },
    ))
}

fn toolbox_font_decrypt_impl(
    source: &Path,
    txt_path: Option<&Path>,
) -> Result<ToolboxEpubToolResult, String> {
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    if let Some(txt_path) = txt_path {
        if !txt_path.exists() {
            return Err(format!("TXT 文件不存在: {}", txt_path.to_string_lossy()));
        }
    }
    let output_path = build_processed_epub_path(source, "_font_decrypt");
    let details = run_toolbox_font_obfuscation(source, &output_path, "decrypt", txt_path)?;
    Ok(toolbox_epub_tool_result(
        source,
        &output_path,
        true,
        "font_decrypt",
        if details.is_empty() {
            "字体解密完成".to_string()
        } else {
            format!("字体解密完成: {}", details)
        },
    ))
}

#[tauri::command]
pub fn toolbox_file_encrypt(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_file_encrypt_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
pub fn toolbox_file_decrypt(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_file_decrypt_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
pub fn toolbox_font_encrypt(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_font_encrypt_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
pub fn toolbox_font_decrypt(
    epub_path: String,
    txt_path: Option<String>,
) -> Result<ToolboxEpubToolResult, String> {
    let txt_path_buf = txt_path.map(PathBuf::from);
    toolbox_font_decrypt_impl(&PathBuf::from(epub_path), txt_path_buf.as_deref())
}

#[tauri::command]
pub fn toolbox_epub_reformat(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_epub_reformat_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
pub fn toolbox_image_convert(
    epub_path: String,
    image_format: Option<String>,
) -> Result<ToolboxEpubToolResult, String> {
    toolbox_image_convert_impl(&PathBuf::from(epub_path), image_format.as_deref())
}

const AI_IMAGE_MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;

const AI_IMAGE_MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

const AI_IMAGE_USER_AGENT: &str = "TEpub-Editor/image-tools";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxAiImageRequest {
    base_url: String,
    api_key: String,
    model: String,
    target: String,
    title: Option<String>,
    author: Option<String>,
    prompt: String,
    size: String,
    reference_data: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolboxAiImageResult {
    bytes: Vec<u8>,
    mime: String,
    extension: String,
    file_name: String,
    prompt: String,
    size: String,
}

fn normalize_ai_base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn remove_suffix_ignore_case(value: &str, suffix: &str) -> String {
    if value
        .to_ascii_lowercase()
        .ends_with(&suffix.to_ascii_lowercase())
    {
        value[..value.len() - suffix.len()].to_string()
    } else {
        value.to_string()
    }
}

fn is_versioned_ai_base_url(base_url: &str) -> bool {
    let lower = base_url.to_ascii_lowercase();
    let Some((prefix, tail)) = lower.rsplit_once("/api/v") else {
        return false;
    };
    !prefix.is_empty() && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
}

fn ai_image_generation_url(base_url: &str) -> Result<String, String> {
    let base = normalize_ai_base_url(base_url);
    if base.is_empty() {
        return Err("Base URL 不能为空".to_string());
    }
    let lower = base.to_ascii_lowercase();
    Ok(if lower.ends_with("/images/generations") {
        base
    } else if lower.ends_with("/chat/completions") {
        remove_suffix_ignore_case(&base, "/chat/completions") + "/images/generations"
    } else if lower.ends_with("/v1") || is_versioned_ai_base_url(&base) {
        format!("{}/images/generations", base)
    } else {
        format!("{}/v1/images/generations", base)
    })
}

fn ai_image_edits_url(base_url: &str) -> Result<String, String> {
    let base = normalize_ai_base_url(base_url);
    if base.is_empty() {
        return Err("Base URL 不能为空".to_string());
    }
    let lower = base.to_ascii_lowercase();
    Ok(if lower.ends_with("/images/edits") {
        base
    } else if lower.ends_with("/images/generations") {
        remove_suffix_ignore_case(&base, "/images/generations") + "/images/edits"
    } else if lower.ends_with("/chat/completions") {
        remove_suffix_ignore_case(&base, "/chat/completions") + "/images/edits"
    } else if lower.ends_with("/v1") || is_versioned_ai_base_url(&base) {
        format!("{}/images/edits", base)
    } else {
        format!("{}/v1/images/edits", base)
    })
}

fn ai_image_mime_from_bytes(bytes: &[u8]) -> (&'static str, &'static str) {
    match detect_image_ext(bytes) {
        "png" => ("png", "image/png"),
        "webp" => ("webp", "image/webp"),
        "gif" => ("gif", "image/gif"),
        "jpg" | "jpeg" => ("jpg", "image/jpeg"),
        _ => ("jpg", "image/jpeg"),
    }
}

fn validate_ai_image_bytes(bytes: &[u8], label: &str) -> Result<(), String> {
    if bytes.is_empty() {
        return Err(format!("{}为空", label));
    }
    if bytes.len() > AI_IMAGE_MAX_RESPONSE_BYTES {
        return Err(format!("{}过大", label));
    }
    image::load_from_memory(bytes)
        .map(|_| ())
        .map_err(|_| format!("无法识别{}的图片内容", label))
}

fn decode_ai_base64_image(value: &str) -> Option<Vec<u8>> {
    let trimmed = value.trim();
    let encoded = if trimmed.to_ascii_lowercase().starts_with("data:image/") {
        trimmed.split_once(',')?.1.trim()
    } else {
        trimmed
    };
    if encoded.len() < 80 || encoded.contains(char::is_whitespace) {
        return None;
    }
    let bytes = general_purpose::STANDARD.decode(encoded).ok()?;
    validate_ai_image_bytes(&bytes, "返回图片").ok()?;
    Some(bytes)
}

fn decode_ai_data_url(value: &str) -> Option<Vec<u8>> {
    if !value.trim().to_ascii_lowercase().starts_with("data:image/") {
        return None;
    }
    decode_ai_base64_image(value)
}

fn is_remote_ai_url(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("https://") || lower.starts_with("http://")
}

fn ai_json_error_preview(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message").or(Some(error)))
                .and_then(|message| message.as_str().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| text.chars().take(240).collect())
}

fn image_size_candidates(requested: &str, target: &str) -> Vec<String> {
    let fallback = match target {
        "banner" => "1536x768",
        "standard" => "1200x1600",
        _ => "1024x1792",
    };
    let secondary_fallback = match target {
        "banner" => "1280x640",
        "standard" => "1024x1365",
        _ => "1400x2400",
    };
    let mut values = vec![
        requested.trim().to_string(),
        fallback.to_string(),
        secondary_fallback.to_string(),
        String::new(),
    ];
    values.dedup();
    values
}

fn render_ai_image_prompt(template: &str, title: Option<&str>, author: Option<&str>) -> String {
    let title = title.unwrap_or("").trim();
    let title = if title.is_empty() {
        "当前书籍"
    } else {
        title
    };
    let raw_author = author.unwrap_or("").trim();
    let mut prompt = template.trim().to_string();
    if raw_author.is_empty() {
        for marker in [
            "作者{author}的",
            "作者{{author}}的",
            "作者 {author} 的",
            "作者 {{author}} 的",
        ] {
            prompt = prompt.replace(marker, "");
        }
    }
    let author = if raw_author.is_empty() {
        ""
    } else {
        raw_author
    };
    let replaced = prompt
        .replace("{title}", title)
        .replace("{{title}}", title)
        .replace("{{text}}", title)
        .replace("{author}", author)
        .replace("{{author}}", author);
    if prompt.contains("{title}")
        || prompt.contains("{{title}}")
        || prompt.contains("{{text}}")
        || prompt.contains("{author}")
        || prompt.contains("{{author}}")
    {
        replaced
    } else {
        let book_info = if author.is_empty() {
            title.to_string()
        } else {
            format!("{} / {}", title, author)
        };
        format!("{}\n\n书籍信息：{}", replaced, book_info)
    }
}

fn build_ai_image_request_bodies(
    model: &str,
    prompt: &str,
    size: &str,
    reference_data_url: Option<&str>,
) -> Vec<serde_json::Value> {
    fn base_body(
        model: &str,
        prompt: &str,
        size: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();
        map.insert(
            "model".to_string(),
            serde_json::Value::String(model.to_string()),
        );
        map.insert(
            "prompt".to_string(),
            serde_json::Value::String(prompt.to_string()),
        );
        map.insert("n".to_string(), serde_json::Value::Number(1.into()));
        if !size.trim().is_empty() {
            map.insert(
                "size".to_string(),
                serde_json::Value::String(size.trim().to_string()),
            );
        }
        map
    }

    let mut bodies = Vec::new();
    if let Some(reference) = reference_data_url.filter(|value| !value.trim().is_empty()) {
        for variant in [
            "image_array",
            "image_string",
            "image_urls_array",
            "image_array_no_format",
            "image_string_no_format",
            "image_urls_array_no_format",
        ] {
            let mut map = base_body(model, prompt, size);
            match variant {
                "image_array" | "image_array_no_format" => {
                    map.insert("image".to_string(), serde_json::json!([reference]));
                }
                "image_string" | "image_string_no_format" => {
                    map.insert(
                        "image".to_string(),
                        serde_json::Value::String(reference.to_string()),
                    );
                }
                _ => {
                    map.insert("image_urls".to_string(), serde_json::json!([reference]));
                }
            }
            if !variant.ends_with("_no_format") {
                map.insert(
                    "response_format".to_string(),
                    serde_json::Value::String("b64_json".to_string()),
                );
            }
            bodies.push(serde_json::Value::Object(map));
        }
    }

    let mut with_format = base_body(model, prompt, size);
    with_format.insert(
        "response_format".to_string(),
        serde_json::Value::String("b64_json".to_string()),
    );
    bodies.push(serde_json::Value::Object(with_format));
    bodies.push(serde_json::Value::Object(base_body(model, prompt, size)));

    let mut seen = HashSet::new();
    bodies
        .into_iter()
        .filter(|body| seen.insert(body.to_string()))
        .collect()
}

async fn download_ai_image_url(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(url.trim())
        .header("Accept", "image/*,*/*;q=0.8")
        .send()
        .await
        .map_err(|e| format!("下载生成图片失败: {}", e))?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取生成图片失败: {}", e))?;
    if !status.is_success() {
        return Err(format!("下载生成图片返回 {}", status));
    }
    let bytes = bytes.to_vec();
    validate_ai_image_bytes(&bytes, "生成图片")?;
    Ok(bytes)
}

const REMOTE_IMAGE_MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteImageDownloadResponse {
    data_base64: String,
    content_type: String,
    final_url: String,
}

fn validate_remote_image_url(url: &reqwest::Url) -> Result<(), String> {
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("仅支持 HTTP 或 HTTPS 图片地址".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("图片地址不能包含用户名或密码".to_string());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "图片地址缺少主机名".to_string())?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return Err("不允许下载本机或局域网图片地址".to_string());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        let blocked = match ip {
            IpAddr::V4(ip) => {
                ip.is_private()
                    || ip.is_loopback()
                    || ip.is_link_local()
                    || ip.is_broadcast()
                    || ip.is_documentation()
                    || ip.is_unspecified()
            }
            IpAddr::V6(ip) => {
                ip.is_loopback()
                    || ip.is_unspecified()
                    || (ip.segments()[0] & 0xfe00) == 0xfc00
                    || (ip.segments()[0] & 0xffc0) == 0xfe80
                    || ip.to_ipv4().map_or(false, |mapped| {
                        mapped.is_private()
                            || mapped.is_loopback()
                            || mapped.is_link_local()
                            || mapped.is_broadcast()
                            || mapped.is_documentation()
                            || mapped.is_unspecified()
                    })
            }
        };
        if blocked {
            return Err("不允许下载本机、私有网络或链路本地图片地址".to_string());
        }
    }
    Ok(())
}

fn raster_image_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg");
    }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some("image/png");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

#[tauri::command]
pub async fn download_remote_image(url: String) -> Result<RemoteImageDownloadResponse, String> {
    let mut current =
        reqwest::Url::parse(url.trim()).map_err(|e| format!("图片地址无效: {}", e))?;
    validate_remote_image_url(&current)?;

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) TEpub-Editor/1.0")
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("初始化图片下载失败: {}", e))?;

    for redirect_count in 0..=5 {
        validate_remote_image_url(&current)?;
        let mut referer = current.clone();
        referer.set_path("/");
        referer.set_query(None);
        referer.set_fragment(None);
        let response = client
            .get(current.clone())
            .header(
                reqwest::header::ACCEPT,
                "image/avif,image/webp,image/png,image/jpeg,image/gif,image/*;q=0.8,*/*;q=0.5",
            )
            .header(reqwest::header::REFERER, referer.as_str())
            .send()
            .await
            .map_err(|e| format!("下载图片失败: {}", e))?;

        if response.status().is_redirection() {
            if redirect_count == 5 {
                return Err("图片地址重定向次数超过 5 次".to_string());
            }
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| format!("图片服务器返回了无目标的重定向 {}", response.status()))?;
            current = current
                .join(location)
                .map_err(|e| format!("图片重定向地址无效: {}", e))?;
            continue;
        }

        if !response.status().is_success() {
            return Err(format!("图片服务器返回 {}", response.status()));
        }
        if response.content_length().unwrap_or(0) > REMOTE_IMAGE_MAX_BYTES as u64 {
            return Err("图片超过 32 MB 限制".to_string());
        }

        let final_url = response.url().to_string();
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("读取图片响应失败: {}", e))?
        {
            if bytes.len() + chunk.len() > REMOTE_IMAGE_MAX_BYTES {
                return Err("图片超过 32 MB 限制".to_string());
            }
            bytes.extend_from_slice(&chunk);
        }
        if bytes.is_empty() {
            return Err("图片响应为空".to_string());
        }
        let content_type = raster_image_content_type(&bytes)
            .ok_or_else(|| "响应不是支持的 JPG、PNG、WebP 或 GIF 图片".to_string())?;
        return Ok(RemoteImageDownloadResponse {
            data_base64: general_purpose::STANDARD.encode(bytes),
            content_type: content_type.to_string(),
            final_url,
        });
    }

    Err("图片下载失败".to_string())
}

async fn extract_ai_image_bytes(
    client: &reqwest::Client,
    value: &serde_json::Value,
    key_hint: Option<&str>,
) -> Option<Vec<u8>> {
    match value {
        serde_json::Value::String(text) => {
            if let Some(bytes) = decode_ai_data_url(text).or_else(|| decode_ai_base64_image(text)) {
                return Some(bytes);
            }
            let key = key_hint.unwrap_or("").to_ascii_lowercase();
            if (key.contains("url") || key == "uri" || key == "href") && is_remote_ai_url(text) {
                return download_ai_image_url(client, text).await.ok();
            }
            None
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(bytes) = Box::pin(extract_ai_image_bytes(client, item, key_hint)).await
                {
                    return Some(bytes);
                }
            }
            None
        }
        serde_json::Value::Object(map) => {
            for key in [
                "b64_json",
                "base64",
                "image_base64",
                "data",
                "content",
                "url",
                "image_url",
                "uri",
                "href",
            ] {
                if let Some(item) = map.get(key) {
                    if let Some(bytes) =
                        Box::pin(extract_ai_image_bytes(client, item, Some(key))).await
                    {
                        return Some(bytes);
                    }
                }
            }
            for (key, item) in map {
                if let Some(bytes) = Box::pin(extract_ai_image_bytes(client, item, Some(key))).await
                {
                    return Some(bytes);
                }
            }
            None
        }
        _ => None,
    }
}

async fn request_ai_image_generation(
    client: &reqwest::Client,
    request: &ToolboxAiImageRequest,
    body: &serde_json::Value,
) -> Result<Vec<u8>, String> {
    let url = ai_image_generation_url(&request.base_url)?;
    let response = client
        .post(url)
        .bearer_auth(request.api_key.trim())
        .header("Accept", "application/json,image/*")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("请求生图接口失败: {}", e))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取生图响应失败: {}", e))?;
    if bytes.len() > AI_IMAGE_MAX_RESPONSE_BYTES {
        return Err("生图响应过大".to_string());
    }
    if !status.is_success() {
        let text = String::from_utf8_lossy(&bytes).to_string();
        return Err(format!(
            "生图接口返回 {}: {}",
            status,
            ai_json_error_preview(&text)
        ));
    }
    if content_type.starts_with("image/") {
        let out = bytes.to_vec();
        validate_ai_image_bytes(&out, "生成图片")?;
        return Ok(out);
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        format!(
            "生图接口未返回有效 JSON: {}; {}",
            e,
            text.chars().take(180).collect::<String>()
        )
    })?;
    extract_ai_image_bytes(client, &value, None)
        .await
        .ok_or_else(|| "生图接口返回中没有图片数据".to_string())
}

async fn request_ai_image_edit(
    client: &reqwest::Client,
    request: &ToolboxAiImageRequest,
    prompt: &str,
    size: &str,
    reference_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let url = ai_image_edits_url(&request.base_url)?;
    let (ext, mime) = ai_image_mime_from_bytes(reference_bytes);
    let mut form = reqwest::multipart::Form::new()
        .text("model", request.model.trim().to_string())
        .text("prompt", prompt.to_string())
        .text("n", "1")
        .text("response_format", "b64_json");
    if !size.trim().is_empty() {
        form = form.text("size", size.trim().to_string());
    }
    form = form.part(
        "image",
        reqwest::multipart::Part::bytes(reference_bytes.to_vec())
            .file_name(format!("reference.{}", ext))
            .mime_str(mime)
            .map_err(|e| format!("构造参考图失败: {}", e))?,
    );

    let response = client
        .post(url)
        .bearer_auth(request.api_key.trim())
        .header("Accept", "application/json,image/*")
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("请求图片编辑接口失败: {}", e))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取图片编辑响应失败: {}", e))?;
    if bytes.len() > AI_IMAGE_MAX_RESPONSE_BYTES {
        return Err("图片编辑响应过大".to_string());
    }
    if !status.is_success() {
        let text = String::from_utf8_lossy(&bytes).to_string();
        return Err(format!(
            "图片编辑接口返回 {}: {}",
            status,
            ai_json_error_preview(&text)
        ));
    }
    if content_type.starts_with("image/") {
        let out = bytes.to_vec();
        validate_ai_image_bytes(&out, "生成图片")?;
        return Ok(out);
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        format!(
            "图片编辑接口未返回有效 JSON: {}; {}",
            e,
            text.chars().take(180).collect::<String>()
        )
    })?;
    extract_ai_image_bytes(client, &value, None)
        .await
        .ok_or_else(|| "图片编辑接口返回中没有图片数据".to_string())
}

#[tauri::command]
pub async fn toolbox_generate_ai_image(
    request: ToolboxAiImageRequest,
) -> Result<ToolboxAiImageResult, String> {
    if request.base_url.trim().is_empty()
        || request.api_key.trim().is_empty()
        || request.model.trim().is_empty()
    {
        return Err("请先填写 Base URL、API Key 和模型".to_string());
    }
    if request.prompt.trim().is_empty() {
        return Err("提示词不能为空".to_string());
    }
    let target = match request.target.trim() {
        "duokan" | "cover" => "duokan",
        "standard" => "standard",
        "banner" => "banner",
        _ => "duokan",
    };
    let reference_data = request.reference_data.as_deref();
    if let Some(bytes) = reference_data {
        if bytes.len() > AI_IMAGE_MAX_INPUT_BYTES {
            return Err("参考封面图片过大".to_string());
        }
        validate_ai_image_bytes(bytes, "参考封面")?;
    }

    let prompt = render_ai_image_prompt(
        &request.prompt,
        request.title.as_deref(),
        request.author.as_deref(),
    );
    let client = reqwest::Client::builder()
        .user_agent(AI_IMAGE_USER_AGENT)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("初始化生图客户端失败: {}", e))?;

    let reference_data_url = reference_data.map(|bytes| {
        let (_, mime) = ai_image_mime_from_bytes(bytes);
        format!(
            "data:{};base64,{}",
            mime,
            general_purpose::STANDARD.encode(bytes)
        )
    });
    let sizes = image_size_candidates(&request.size, target);
    let is_gpt_image = request.model.to_ascii_lowercase().contains("gpt-image");
    let mut last_error = None;

    if is_gpt_image {
        if let Some(bytes) = reference_data {
            for size in &sizes {
                match request_ai_image_edit(&client, &request, &prompt, size, bytes).await {
                    Ok(out) => {
                        let (ext, mime) = ai_image_mime_from_bytes(&out);
                        let stem =
                            sanitize_filename_part(request.title.as_deref().unwrap_or(target));
                        let stem = if stem.is_empty() {
                            target.to_string()
                        } else {
                            stem
                        };
                        let file_name = format!("{}-{}.{}", stem, target, ext);
                        return Ok(ToolboxAiImageResult {
                            bytes: out,
                            mime: mime.to_string(),
                            extension: ext.to_string(),
                            file_name,
                            prompt,
                            size: size.clone(),
                        });
                    }
                    Err(e) => last_error = Some(e),
                }
            }
        }
    }

    for size in &sizes {
        for body in build_ai_image_request_bodies(
            &request.model,
            &prompt,
            size,
            reference_data_url.as_deref(),
        ) {
            match request_ai_image_generation(&client, &request, &body).await {
                Ok(out) => {
                    let (ext, mime) = ai_image_mime_from_bytes(&out);
                    let stem = sanitize_filename_part(request.title.as_deref().unwrap_or(target));
                    let stem = if stem.is_empty() {
                        target.to_string()
                    } else {
                        stem
                    };
                    let file_name = format!("{}-{}.{}", stem, target, ext);
                    return Ok(ToolboxAiImageResult {
                        bytes: out,
                        mime: mime.to_string(),
                        extension: ext.to_string(),
                        file_name,
                        prompt,
                        size: size.clone(),
                    });
                }
                Err(e) => last_error = Some(e),
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "未获取到生图结果".to_string()))
}

#[tauri::command]
pub fn toolbox_epub_diagnose(epub_path: String) -> Result<ToolboxEpubDiagnosticResult, String> {
    let source = PathBuf::from(&epub_path);
    if !source.exists() {
        return Err(format!("File does not exist: {}", source.to_string_lossy()));
    }

    let bytes = fs::read(&source).map_err(|e| format!("Read EPUB failed: {}", e))?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone()))
        .map_err(|e| format!("Read EPUB failed: {}", e))?;
    let mut issues = Vec::new();
    let mut entry_names = Vec::new();

    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("Read ZIP entry failed: {}", e))?;
        entry_names.push(file.name().replace('\\', "/"));
    }

    let entry_set: HashSet<String> = entry_names.iter().cloned().collect();
    let mut lower_to_names: HashMap<String, Vec<String>> = HashMap::new();
    for name in &entry_names {
        lower_to_names
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push(name.clone());
    }

    if !entry_set.contains("mimetype") {
        push_diag_issue(
            &mut issues,
            "warning",
            "missing-mimetype",
            Some("mimetype".to_string()),
            "EPUB has no top-level mimetype entry.",
        );
    }

    if !entry_set.contains("META-INF/container.xml") {
        if let Some(matches) = case_mismatch_names(&lower_to_names, "META-INF/container.xml") {
            push_diag_issue(
                &mut issues,
                "warning",
                "case-mismatch",
                Some("META-INF/container.xml".to_string()),
                format!(
                    "container.xml exists with different case: {}",
                    matches.join(", ")
                ),
            );
        } else {
            push_diag_issue(
                &mut issues,
                "error",
                "missing-container",
                Some("META-INF/container.xml".to_string()),
                "EPUB has no META-INF/container.xml entry.",
            );
        }
    }

    let opf_path = match find_opf_path_in_epub_bytes(&bytes) {
        Ok(path) => path,
        Err(e) => {
            push_diag_issue(
                &mut issues,
                "error",
                "missing-opf",
                None,
                format!("Cannot locate OPF: {}", e),
            );
            let error_count = issues.iter().filter(|issue| issue.level == "error").count();
            let warning_count = issues
                .iter()
                .filter(|issue| issue.level == "warning")
                .count();
            return Ok(ToolboxEpubDiagnosticResult {
                source_path: source.to_string_lossy().to_string(),
                opf_path: None,
                total_entries: entry_names.len(),
                manifest_items: 0,
                error_count,
                warning_count,
                issues,
            });
        }
    };

    if !entry_set.contains(&opf_path) {
        if let Some(matches) = case_mismatch_names(&lower_to_names, &opf_path) {
            push_diag_issue(
                &mut issues,
                "warning",
                "case-mismatch",
                Some(opf_path.clone()),
                format!("OPF exists with different case: {}", matches.join(", ")),
            );
        } else {
            push_diag_issue(
                &mut issues,
                "error",
                "missing-opf",
                Some(opf_path.clone()),
                "container.xml points to an OPF entry that is not in the ZIP.",
            );
        }
    }

    let manifest_items = match parse_toolbox_manifest_items(&bytes, &opf_path) {
        Ok(items) => items,
        Err(e) => {
            push_diag_issue(
                &mut issues,
                "error",
                "manifest-parse",
                Some(opf_path.clone()),
                format!("Parse OPF manifest failed: {}", e),
            );
            let error_count = issues.iter().filter(|issue| issue.level == "error").count();
            let warning_count = issues
                .iter()
                .filter(|issue| issue.level == "warning")
                .count();
            return Ok(ToolboxEpubDiagnosticResult {
                source_path: source.to_string_lossy().to_string(),
                opf_path: Some(opf_path),
                total_entries: entry_names.len(),
                manifest_items: 0,
                error_count,
                warning_count,
                issues,
            });
        }
    };

    let manifest_set: HashSet<String> = manifest_items
        .iter()
        .map(|item| item.abs_path.clone())
        .collect();
    let mut required_entries: HashSet<String> = HashSet::new();
    required_entries.insert("mimetype".to_string());
    required_entries.insert("META-INF/container.xml".to_string());
    required_entries.insert(opf_path.clone());

    for item in &manifest_items {
        if entry_set.contains(&item.abs_path) {
            continue;
        }
        if let Some(matches) = case_mismatch_names(&lower_to_names, &item.abs_path) {
            push_diag_issue(
                &mut issues,
                "warning",
                "case-mismatch",
                Some(item.abs_path.clone()),
                format!(
                    "Manifest item '{}' exists with different case: {}",
                    item.id,
                    matches.join(", ")
                ),
            );
        } else {
            push_diag_issue(
                &mut issues,
                "error",
                "manifest-missing",
                Some(item.abs_path.clone()),
                format!("Manifest item '{}' points to a missing resource.", item.id),
            );
        }
    }

    for name in &entry_names {
        if manifest_set.contains(name) || required_entries.contains(name) {
            continue;
        }
        if is_diagnostic_resource_entry(name) {
            push_diag_issue(
                &mut issues,
                "warning",
                "unregistered-resource",
                Some(name.clone()),
                "Resource exists in ZIP but is not registered in OPF manifest.",
            );
        }
    }

    let mut seen_refs = HashSet::new();
    for name in &entry_names {
        if !is_text_like_entry(name) || name.ends_with('/') {
            continue;
        }
        let data = match read_zip_entry_bytes(&bytes, name) {
            Ok(data) => data,
            Err(e) => {
                push_diag_issue(
                    &mut issues,
                    "warning",
                    "read-entry",
                    Some(name.clone()),
                    format!("Cannot read text-like entry: {}", e),
                );
                continue;
            }
        };
        let text = String::from_utf8_lossy(&data).to_string();
        for raw_ref in collect_diagnostic_refs(&text) {
            let Some(abs_ref) = resolve_epub_ref(name, &raw_ref) else {
                continue;
            };
            if !seen_refs.insert((name.clone(), raw_ref.clone(), abs_ref.clone())) {
                continue;
            }
            if entry_set.contains(&abs_ref) {
                continue;
            }
            if let Some(matches) = case_mismatch_names(&lower_to_names, &abs_ref) {
                push_diag_issue(
                    &mut issues,
                    "warning",
                    "ref-case-mismatch",
                    Some(name.clone()),
                    format!(
                        "Reference '{}' resolves to '{}' but ZIP contains different case: {}",
                        raw_ref,
                        abs_ref,
                        matches.join(", ")
                    ),
                );
            } else {
                push_diag_issue(
                    &mut issues,
                    "error",
                    "missing-reference",
                    Some(name.clone()),
                    format!(
                        "Reference '{}' resolves to missing resource '{}'.",
                        raw_ref, abs_ref
                    ),
                );
            }
        }
    }

    let error_count = issues.iter().filter(|issue| issue.level == "error").count();
    let warning_count = issues
        .iter()
        .filter(|issue| issue.level == "warning")
        .count();

    Ok(ToolboxEpubDiagnosticResult {
        source_path: source.to_string_lossy().to_string(),
        opf_path: Some(opf_path),
        total_entries: entry_names.len(),
        manifest_items: manifest_items.len(),
        error_count,
        warning_count,
        issues,
    })
}

fn toolbox_epub_diagnose_tool_result(source: &Path) -> Result<ToolboxEpubToolResult, String> {
    let result = toolbox_epub_diagnose(source.to_string_lossy().to_string())?;
    let action = if result.error_count > 0 || result.warning_count > 0 {
        "epub_diagnose_issue"
    } else {
        "epub_diagnose_clean"
    };
    let message = if result.error_count == 0 && result.warning_count == 0 {
        format!(
            "EPUB diagnosis completed: no structural issues found; {} ZIP entries, {} manifest items",
            result.total_entries, result.manifest_items
        )
    } else {
        let mut lines = vec![format!(
            "EPUB diagnosis completed: {} errors, {} warnings; {} ZIP entries, {} manifest items",
            result.error_count, result.warning_count, result.total_entries, result.manifest_items
        )];
        for issue in result.issues.iter().take(6) {
            let path = issue
                .path
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(|value| format!(" [{}]", value))
                .unwrap_or_default();
            lines.push(format!(
                "{} {}{}: {}",
                issue.level, issue.kind, path, issue.message
            ));
        }
        if result.issues.len() > 6 {
            lines.push(format!("... and {} more issues", result.issues.len() - 6));
        }
        lines.join("\n")
    };

    Ok(toolbox_epub_tool_result(
        source, source, false, action, message,
    ))
}

#[tauri::command]
pub async fn toolbox_scan_batch_inputs(
    input_paths: Vec<String>,
    output_dir: Option<String>,
) -> Result<ToolboxBatchScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let resolved_output_dir = resolve_batch_output_dir(&input_paths, output_dir);
        let sources = list_epub_sources(&input_paths, &resolved_output_dir)?;
        Ok(ToolboxBatchScanResult {
            input_paths: sources
                .into_iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            output_dir: resolved_output_dir.to_string_lossy().to_string(),
        })
    })
    .await
    .map_err(|e| format!("批量目录扫描失败: {}", e))?
}

#[tauri::command]
pub fn toolbox_cancel_batch(task_id: String) -> Result<(), String> {
    if task_id.trim().is_empty() {
        return Err("缺少批量任务 ID".to_string());
    }
    request_toolbox_batch_cancel(&task_id);
    Ok(())
}

#[tauri::command]
pub async fn toolbox_run_batch(
    app: tauri::AppHandle,
    task_id: String,
    tool: String,
    input_paths: Vec<String>,
    image_format: Option<String>,
    output_dir: Option<String>,
    output_mode: Option<String>,
    conflict_mode: Option<String>,
) -> Result<ToolboxBatchSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_toolbox_batch_impl(
            app,
            task_id,
            tool,
            input_paths,
            image_format,
            output_dir,
            output_mode,
            conflict_mode,
        )
    })
    .await
    .map_err(|e| format!("批量任务执行失败: {}", e))?
}

// ============================================================
// ===== App Data Layout (绿色版 vs 安装版自动路由) =====
// ============================================================
//
// 判定方式：exe 同级目录存在 `portable.txt` 标记文件即视为绿色版。
// 绿色版数据目录：<exe_dir>/data/
// 安装版数据目录：Tauri app_data_dir (%APPDATA%/<bundle-id>)
//
// 凡是落到这个目录的内容：app 配置、TXT 编辑器 .history、library.pointer 等。
// 书库的 epub 副本与封面是另一回事，由用户在书库设置里指定的 customWorkDir 决定。

#[cfg(test)]
mod toolbox_tests {
    use super::*;
    use crate::fs_utils::collect_opf_id_hints;

    fn write_zip_entry(
        writer: &mut zip::ZipWriter<fs::File>,
        name: &str,
        content: &[u8],
    ) -> Result<(), String> {
        writer
            .start_file(name, SimpleFileOptions::default())
            .map_err(|e| e.to_string())?;
        writer.write_all(content).map_err(|e| e.to_string())
    }

    fn read_epub_entry(epub_path: &Path, name: &str) -> Result<String, String> {
        let file = fs::File::open(epub_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut entry = archive.by_name(name).map_err(|e| e.to_string())?;
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
        Ok(text)
    }

    fn epub_names(epub_path: &Path) -> Result<Vec<String>, String> {
        let file = fs::File::open(epub_path).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut names = Vec::new();
        for i in 0..archive.len() {
            names.push(
                archive
                    .by_index(i)
                    .map_err(|e| e.to_string())?
                    .name()
                    .replace('\\', "/"),
            );
        }
        Ok(names)
    }

    fn create_minimal_epub(epub_path: &Path) -> Result<(), String> {
        let file = fs::File::create(epub_path).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        write_zip_entry(&mut writer, "mimetype", b"application/epub+zip")?;
        write_zip_entry(
            &mut writer,
            "META-INF/container.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/content.opf",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item href="Text/chapter one.xhtml" id="chapter" media-type="application/xhtml+xml"/>
    <item media-type="text/css" href="Styles/main.css" id="style"/>
  </manifest>
  <spine>
    <itemref idref="chapter"/>
  </spine>
</package>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Text/chapter one.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><link rel="stylesheet" href="../Styles/main.css"/></head>
  <body><p>Hello</p></body>
</html>"#,
        )?;
        write_zip_entry(&mut writer, "OPS/Styles/main.css", b"body { color: #111; }")?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn create_percent_encoded_obfuscated_epub(epub_path: &Path) -> Result<(), String> {
        let file = fs::File::create(epub_path).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        write_zip_entry(&mut writer, "mimetype", b"application/epub+zip")?;
        write_zip_entry(
            &mut writer,
            "META-INF/container.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/content.opf",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item id="gyzw0001.xhtml" href="Text/%7C_%3A%2Achapter.xhtml" media-type="application/xhtml+xml"/>
    <item id="main.css" href="Styles/%7C_%2Amain.css" media-type="text/css"/>
  </manifest>
  <spine>
    <itemref idref="gyzw0001.xhtml"/>
  </spine>
</package>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Text/|_:*chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><link rel="stylesheet" href="../Styles/%7C_%2Amain.css"/></head>
  <body><p>Hello</p></body>
</html>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Styles/|_*main.css",
            b"body { color: #111; }",
        )?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn create_image_bytes() -> Result<Vec<u8>, String> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            1,
            1,
            image::Rgb([12, 34, 56]),
        ));
        let mut bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut bytes);
        image
            .write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(bytes)
    }

    fn create_webp_reference_epub(epub_path: &Path) -> Result<(), String> {
        let image_bytes = create_image_bytes()?;
        let file = fs::File::create(epub_path).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        write_zip_entry(&mut writer, "mimetype", b"application/epub+zip")?;
        write_zip_entry(
            &mut writer,
            "META-INF/container.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/content.opf",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item href="Text/chapter.xhtml" id="chapter" media-type="application/xhtml+xml"/>
    <item href="Styles/main.css" id="style" media-type="text/css"/>
    <item href="Images/cover.webp" id="cover" media-type="image/webp"/>
  </manifest>
  <metadata>
    <meta name="cover" content="cover"/>
  </metadata>
  <spine>
    <itemref idref="chapter"/>
  </spine>
</package>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Text/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><link rel="stylesheet" href="../Styles/main.css"/></head>
  <body><img src="../Images/cover.webp?rev=1"/></body>
</html>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Styles/main.css",
            b".cover { background-image: url('../Images/cover.webp'); }",
        )?;
        write_zip_entry(&mut writer, "OPS/Images/cover.webp", &image_bytes)?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn create_diagnostic_problem_epub(epub_path: &Path) -> Result<(), String> {
        let file = fs::File::create(epub_path).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        write_zip_entry(&mut writer, "mimetype", b"application/epub+zip")?;
        write_zip_entry(
            &mut writer,
            "META-INF/container.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/content.opf",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item href="Text/chapter.xhtml" id="chapter" media-type="application/xhtml+xml"/>
    <item href="Images/Cover.PNG" id="cover" media-type="image/png"/>
  </manifest>
  <spine>
    <itemref idref="chapter"/>
  </spine>
</package>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Text/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body><img src="../Images/missing.png"/></body>
</html>"#,
        )?;
        write_zip_entry(&mut writer, "OPS/Images/cover.png", &[0, 1, 2, 3])?;
        write_zip_entry(
            &mut writer,
            "OPS/Styles/unused.css",
            b"body { color: red; }",
        )?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn create_unregistered_resource_epub(epub_path: &Path) -> Result<(), String> {
        let image_bytes = create_image_bytes()?;
        let file = fs::File::create(epub_path).map_err(|e| e.to_string())?;
        let mut writer = zip::ZipWriter::new(file);
        write_zip_entry(&mut writer, "mimetype", b"application/epub+zip")?;
        write_zip_entry(
            &mut writer,
            "META-INF/container.xml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/content.opf",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" xmlns="http://www.idpf.org/2007/opf">
  <manifest>
    <item href="Text/chapter.xhtml" id="chapter" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="chapter"/>
  </spine>
</package>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Text/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><link rel="stylesheet" href="../Extra/unlisted.css"/></head>
  <body><img src="../Extra/pic.png"/></body>
</html>"#,
        )?;
        write_zip_entry(
            &mut writer,
            "OPS/Extra/unlisted.css",
            b"body { color: red; }",
        )?;
        write_zip_entry(&mut writer, "OPS/Extra/pic.png", &image_bytes)?;
        writer.finish().map_err(|e| e.to_string())?;
        Ok(())
    }

    #[test]
    fn toolbox_epub_diagnose_reports_common_package_issues() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let source = temp.path().join("problem.epub");
        create_diagnostic_problem_epub(&source)?;

        let result = toolbox_epub_diagnose(source.to_string_lossy().to_string())?;
        let kinds: HashSet<String> = result
            .issues
            .iter()
            .map(|issue| issue.kind.clone())
            .collect();

        assert_eq!(result.opf_path.as_deref(), Some("OPS/content.opf"));
        assert!(result.error_count >= 1, "{:?}", kinds);
        assert!(result.warning_count >= 2, "{:?}", kinds);
        assert!(kinds.contains("missing-reference"), "{:?}", kinds);
        assert!(kinds.contains("case-mismatch"), "{:?}", kinds);
        assert!(kinds.contains("unregistered-resource"), "{:?}", kinds);
        Ok(())
    }

    #[test]
    fn toolbox_epub_reformat_adds_unregistered_resources_to_manifest() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let source = temp.path().join("unregistered.epub");
        create_unregistered_resource_epub(&source)?;

        let reformatted = toolbox_epub_reformat_impl(&source)?;
        assert!(reformatted.changed);
        let reformatted_path = PathBuf::from(&reformatted.output_path);
        let names = epub_names(&reformatted_path)?;
        assert!(
            names.iter().any(|name| name == "OEBPS/Styles/unlisted.css"),
            "{:?}",
            names
        );
        assert!(
            names.iter().any(|name| name == "OEBPS/Images/pic.png"),
            "{:?}",
            names
        );

        let opf = read_epub_entry(&reformatted_path, "OEBPS/content.opf")?;
        assert!(
            opf.contains(r#"href="Styles/unlisted.css" media-type="text/css""#),
            "{}",
            opf
        );
        assert!(
            opf.contains(r#"href="Images/pic.png" media-type="image/png""#),
            "{}",
            opf
        );

        let chapter = read_epub_entry(&reformatted_path, "OEBPS/Text/chapter.xhtml")?;
        assert!(
            chapter.contains(r#"href="../Styles/unlisted.css""#),
            "{}",
            chapter
        );
        assert!(
            chapter.contains(r#"src="../Images/pic.png""#),
            "{}",
            chapter
        );
        Ok(())
    }

    #[test]
    fn toolbox_file_encrypt_decrypt_round_trip_uses_manifest_ids() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let source = temp.path().join("sample.epub");
        create_minimal_epub(&source)?;

        let encrypted = toolbox_file_encrypt_impl(&source)?;
        assert!(encrypted.changed);
        let encrypted_path = PathBuf::from(&encrypted.output_path);
        let encrypted_names = epub_names(&encrypted_path)?;
        assert!(encrypted_names
            .iter()
            .any(|name| name.contains('*') || name.contains(':')));

        let decrypted = toolbox_file_decrypt_impl(&encrypted_path)?;
        assert!(decrypted.changed);
        let decrypted_path = PathBuf::from(&decrypted.output_path);
        let decrypted_names = epub_names(&decrypted_path)?;
        assert!(decrypted_names
            .iter()
            .any(|name| name == "OPS/Text/chapter.xhtml"));
        assert!(decrypted_names
            .iter()
            .any(|name| name == "OPS/Styles/style.css"));

        let opf = read_epub_entry(&decrypted_path, "OPS/content.opf")?;
        assert!(opf.contains(r#"href="Text/chapter.xhtml""#));
        assert!(opf.contains(r#"href="Styles/style.css""#));
        let chapter = read_epub_entry(&decrypted_path, "OPS/Text/chapter.xhtml")?;
        assert!(chapter.contains(r#"href="../Styles/style.css""#));
        Ok(())
    }

    #[test]
    fn toolbox_file_decrypt_rewrites_percent_encoded_refs() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let source = temp.path().join("encoded.epub");
        create_percent_encoded_obfuscated_epub(&source)?;
        let source_bytes = fs::read(&source).map_err(|e| e.to_string())?;
        let hints = collect_opf_id_hints(&source_bytes);
        assert_eq!(
            hints.get("OPS/Text/|_:*chapter.xhtml").map(String::as_str),
            Some("gyzw0001.xhtml"),
            "{:?}",
            hints
        );

        let decrypted = toolbox_file_decrypt_impl(&source)?;
        assert!(decrypted.changed);
        let decrypted_path = PathBuf::from(&decrypted.output_path);
        let decrypted_names = epub_names(&decrypted_path)?;
        assert!(
            decrypted_names
                .iter()
                .any(|name| name == "OPS/Text/gyzw0001.xhtml"),
            "{:?}",
            decrypted_names
        );
        assert!(
            decrypted_names
                .iter()
                .any(|name| name == "OPS/Styles/main.css"),
            "{:?}",
            decrypted_names
        );

        let opf = read_epub_entry(&decrypted_path, "OPS/content.opf")?;
        assert!(opf.contains(r#"href="Text/gyzw0001.xhtml""#));
        assert!(opf.contains(r#"href="Styles/main.css""#));
        assert!(!opf.contains("%7C"));
        assert!(!opf.contains("%3A"));
        assert!(!opf.contains("%2A"));

        let chapter = read_epub_entry(&decrypted_path, "OPS/Text/gyzw0001.xhtml")?;
        assert!(chapter.contains(r#"href="../Styles/main.css""#));
        Ok(())
    }

    #[test]
    fn optimized_link_rewrite_preserves_suffixes_and_css_urls() -> Result<(), String> {
        let mut path_map = HashMap::new();
        path_map.insert(
            "OPS/Styles/|_*main.css".to_string(),
            "OPS/Styles/main.css".to_string(),
        );
        path_map.insert(
            "OPS/Images/cover.webp".to_string(),
            "OPS/Images/cover.png".to_string(),
        );
        path_map.insert(
            "OPS/Text/next.xhtml".to_string(),
            "OPS/Text/next.xhtml".to_string(),
        );

        let text = r#"
<html>
  <head><link href="../Styles/%7C_%2Amain.css?v=1#sheet"/></head>
  <body>
    <a href="next.xhtml#p1">next</a>
    <img src="../Images/cover.webp?rev=1"/>
    <style>.cover { background: url(../Images/cover.webp?rev=2); }</style>
  </body>
</html>"#
            .to_string();

        let rewritten = rewrite_text_links(
            text,
            "OPS/Text/chapter.xhtml",
            "OPS/Text/chapter.xhtml",
            &path_map,
        );
        assert!(rewritten.contains(r#"href="../Styles/main.css?v=1#sheet""#));
        assert!(rewritten.contains(r#"href="next.xhtml#p1""#));
        assert!(rewritten.contains(r#"src="../Images/cover.png?rev=1""#));
        assert!(rewritten.contains("url(../Images/cover.png?rev=2)"));
        assert!(!rewritten.contains("%7C"));
        assert!(!rewritten.contains("cover.webp"));
        Ok(())
    }

    #[test]
    fn optimized_link_rewrite_corrects_case_mismatched_refs() -> Result<(), String> {
        let mut path_map = HashMap::new();
        path_map.insert(
            "OPS/Images/Cover.PNG".to_string(),
            "OEBPS/Images/Cover.PNG".to_string(),
        );
        path_map.insert(
            "OPS/Styles/Main.CSS".to_string(),
            "OEBPS/Styles/Main.CSS".to_string(),
        );

        let text = r#"
<html>
  <head><link href="../styles/main.css"/></head>
  <body>
    <img src="../images/cover.png?rev=1"/>
    <style>.cover { background: url(../images/cover.png); }</style>
  </body>
</html>
"#;
        let rewritten = rewrite_text_links(
            text.to_string(),
            "OPS/Text/chapter.xhtml",
            "OEBPS/Text/chapter.xhtml",
            &path_map,
        );

        assert!(
            rewritten.contains(r#"href="../Styles/Main.CSS""#),
            "{}",
            rewritten
        );
        assert!(
            rewritten.contains(r#"src="../Images/Cover.PNG?rev=1""#),
            "{}",
            rewritten
        );
        assert!(
            rewritten.contains("url(../Images/Cover.PNG)"),
            "{}",
            rewritten
        );
        Ok(())
    }

    #[test]
    fn toolbox_epub_reformat_moves_manifest_files_to_standard_dirs() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let source = temp.path().join("sample.epub");
        create_minimal_epub(&source)?;

        let reformatted = toolbox_epub_reformat_impl(&source)?;
        assert!(reformatted.changed);
        let reformatted_path = PathBuf::from(&reformatted.output_path);
        let names = epub_names(&reformatted_path)?;
        assert!(
            names.iter().any(|name| name == "OEBPS/content.opf"),
            "{:?}",
            names
        );
        assert!(
            names
                .iter()
                .any(|name| name == "OEBPS/Text/chapter one.xhtml"),
            "{:?}",
            names
        );
        assert!(
            names.iter().any(|name| name == "OEBPS/Styles/main.css"),
            "{:?}",
            names
        );

        let container = read_epub_entry(&reformatted_path, "META-INF/container.xml")?;
        assert!(container.contains(r#"full-path="OEBPS/content.opf""#));
        let opf = read_epub_entry(&reformatted_path, "OEBPS/content.opf")?;
        assert!(opf.contains(r#"href="Text/chapter one.xhtml""#));
        assert!(opf.contains(r#"href="Styles/main.css""#));
        let chapter = read_epub_entry(&reformatted_path, "OEBPS/Text/chapter one.xhtml")?;
        assert!(chapter.contains(r#"href="../Styles/main.css""#));
        Ok(())
    }

    #[test]
    fn toolbox_image_convert_rewrites_opf_html_and_css_refs() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let source = temp.path().join("webp.epub");
        create_webp_reference_epub(&source)?;

        let converted = toolbox_image_convert_impl(&source, Some("png"))?;
        assert!(converted.changed);
        let converted_path = PathBuf::from(&converted.output_path);
        let names = epub_names(&converted_path)?;
        assert!(
            names.iter().any(|name| name == "OPS/Images/cover.png"),
            "{:?}",
            names
        );
        assert!(
            !names.iter().any(|name| name == "OPS/Images/cover.webp"),
            "{:?}",
            names
        );

        let opf = read_epub_entry(&converted_path, "OPS/content.opf")?;
        assert!(opf.contains(r#"href="Images/cover.png""#), "{}", opf);
        assert!(opf.contains(r#"media-type="image/png""#), "{}", opf);
        assert!(!opf.contains("image/webp"), "{}", opf);

        let chapter = read_epub_entry(&converted_path, "OPS/Text/chapter.xhtml")?;
        assert!(
            chapter.contains(r#"src="../Images/cover.png?rev=1""#),
            "{}",
            chapter
        );
        let css = read_epub_entry(&converted_path, "OPS/Styles/main.css")?;
        assert!(css.contains("url('../Images/cover.png')"), "{}", css);
        Ok(())
    }
}
