use fancy_regex::Regex;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Mutex;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

// --- EPUB 全局缓存 ---
pub(crate) struct EpubCache {
    pub(crate) epub_path: String,
    pub(crate) text_cache: HashMap<String, String>,
    pub(crate) binary_cache: HashMap<String, Vec<u8>>,
    pub(crate) temp_dir: Option<TempDir>,
}

impl EpubCache {
    pub(crate) fn new(path: String) -> Self {
        EpubCache {
            epub_path: path,
            text_cache: HashMap::new(),
            binary_cache: HashMap::new(),
            temp_dir: None,
        }
    }
}

static EPUB_CACHE: Lazy<Mutex<Option<EpubCache>>> = Lazy::new(|| Mutex::new(None));

static QUOTED_PATH_REF_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)(['"])(.*?)\1"#).expect("valid quoted path regex"));

static CSS_URL_PATH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)url\(\s*([^'"\)\s][^)]*?)\s*\)"#).expect("valid css url regex")
});

/// 安全获取 EPUB_CACHE 锁，避免 Mutex 中毒后连锁 panic。
pub(crate) fn lock_epub_cache() -> Result<std::sync::MutexGuard<'static, Option<EpubCache>>, String> {
    EPUB_CACHE
        .lock()
        .map_err(|_| "EPUB 缓存已被中毒，无法访问".to_string())
}

/// 读取 EPUB 文件字节，统一错误消息。
pub(crate) fn read_epub_bytes(source: &Path) -> Result<Vec<u8>, String> {
    fs::read(source).map_err(|e| format!("读取 EPUB 失败: {}", e))
}

pub(crate) fn hidden_process_command(program: &str) -> process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut command = process::Command::new(program);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }
    #[cfg(not(windows))]
    {
        process::Command::new(program)
    }
}

// --- 静态资源: 整理后的 CSS ---

pub(crate) fn escape_xml(input: &str) -> String {
    input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

pub(crate) fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = bytes[i + 1] as char;
            let h2 = bytes[i + 2] as char;
            if let (Some(a), Some(b)) = (h1.to_digit(16), h2.to_digit(16)) {
                out.push(((a << 4) as u8) | (b as u8));
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn normalize_local_file_path(path: &str) -> String {
    if let Some(raw) = path.strip_prefix("file://") {
        let mut decoded = percent_decode(raw);
        if cfg!(windows) && decoded.starts_with('/') {
            decoded = decoded.trim_start_matches('/').to_string();
        }
        decoded
    } else {
        path.to_string()
    }
}

pub(crate) fn is_zip_archive_bytes(data: &[u8]) -> bool {
    let cursor = std::io::Cursor::new(data);
    zip::ZipArchive::new(cursor).is_ok()
}

pub(crate) fn is_zip_archive_path(path: &Path) -> bool {
    if let Ok(file) = fs::File::open(path) {
        zip::ZipArchive::new(file).is_ok()
    } else {
        false
    }
}

pub(crate) fn clear_zip_encryption_flags(data: &mut [u8]) -> bool {
    let mut changed = false;
    let mut i = 0usize;
    while i + 10 < data.len() {
        // Local file header: PK\x03\x04, flag at +6
        if data[i] == 0x50 && data[i + 1] == 0x4B && data[i + 2] == 0x03 && data[i + 3] == 0x04 {
            let flag = u16::from_le_bytes([data[i + 6], data[i + 7]]);
            if flag & 0x0001 != 0 {
                let new_flag = (flag & !0x0001).to_le_bytes();
                data[i + 6] = new_flag[0];
                data[i + 7] = new_flag[1];
                changed = true;
            }
        }
        // Central directory header: PK\x01\x02, flag at +8
        if data[i] == 0x50 && data[i + 1] == 0x4B && data[i + 2] == 0x01 && data[i + 3] == 0x02 {
            let flag = u16::from_le_bytes([data[i + 8], data[i + 9]]);
            if flag & 0x0001 != 0 {
                let new_flag = (flag & !0x0001).to_le_bytes();
                data[i + 8] = new_flag[0];
                data[i + 9] = new_flag[1];
                changed = true;
            }
        }
        i += 1;
    }
    changed
}

pub(crate) fn xor_bytes(data: &[u8], key: u8) -> Vec<u8> {
    data.iter().map(|b| b ^ key).collect()
}

pub(crate) fn build_processed_epub_path(original: &Path, suffix: &str) -> PathBuf {
    let parent = original.parent().unwrap_or(Path::new("."));
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");
    let ext = original
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("epub");

    let mut candidate = parent.join(format!("{}{}.{}", stem, suffix, ext));
    if !candidate.exists() {
        return candidate;
    }

    let mut index = 2usize;
    loop {
        candidate = parent.join(format!("{}{}_{}.{}", stem, suffix, index, ext));
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn has_windows_invalid_component(path: &str) -> bool {
    let invalid = ['<', '>', ':', '"', '\\', '|', '?', '*'];
    for raw in path.replace('\\', "/").split('/') {
        let part = raw.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return true;
        }
        if part.chars().any(|c| c.is_control() || invalid.contains(&c)) {
            return true;
        }
        if part.ends_with(' ') || part.ends_with('.') {
            return true;
        }
    }
    false
}

pub(crate) fn sanitize_windows_component(part: &str) -> String {
    let invalid = ['<', '>', ':', '"', '\\', '|', '?', '*'];
    let mut out = String::with_capacity(part.len());
    for c in part.chars() {
        if c.is_control() || invalid.contains(&c) {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    while out.ends_with(' ') || out.ends_with('.') {
        out.pop();
    }
    if out.is_empty() {
        out = "_".to_string();
    }
    let upper = out.to_ascii_uppercase();
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if reserved.contains(&upper.as_str()) {
        out.push('_');
    }
    out
}

pub(crate) fn sanitize_zip_entry_name(name: &str) -> String {
    let normalized = name.replace('\\', "/").trim_start_matches('/').to_string();
    let is_dir = normalized.ends_with('/');
    let mut parts: Vec<String> = Vec::new();
    for raw in normalized.split('/') {
        if raw.is_empty() || raw == "." {
            continue;
        }
        if raw == ".." {
            continue;
        }
        parts.push(sanitize_windows_component(raw));
    }
    let mut out = parts.join("/");
    if out.is_empty() {
        out = "_".to_string();
    }
    if is_dir {
        out.push('/');
    }
    out
}

fn looks_obfuscated_basename(raw_name: &str) -> bool {
    let normalized = raw_name.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or("");
    let base = file_name
        .rsplit_once('.')
        .map(|(stem, _ext)| stem)
        .unwrap_or(file_name);
    if base.is_empty() {
        return true;
    }
    let mut total = 0usize;
    let mut allowed = 0usize;
    for c in base.chars() {
        total += 1;
        if c.is_ascii_alphanumeric()
            || c == '_'
            || c == '-'
            || ('\u{4e00}'..='\u{9fff}').contains(&c)
        {
            allowed += 1;
        }
    }
    if total == 0 {
        return true;
    }
    let bad_ratio = 1.0 - (allowed as f64 / total as f64);
    bad_ratio > 0.35 || base.contains("____") || base.len() > 40
}

pub(crate) fn friendly_name_for_path(path: &str, index: usize) -> String {
    let ext = Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let prefix = match ext.as_str() {
        "xhtml" | "html" | "htm" => "chapter",
        "css" => "style",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "svg" => "image",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "ncx" => "toc",
        "opf" => "content",
        "xml" => "meta",
        _ => "file",
    };
    if ext.is_empty() {
        format!("{}{:03}", prefix, index)
    } else {
        format!("{}{:03}.{}", prefix, index, ext)
    }
}

pub(crate) fn zip_join(base_dir: &str, rel: &str) -> String {
    let rel = rel.replace('\\', "/");
    let rel = rel.trim();
    if rel.is_empty() {
        return base_dir.to_string();
    }
    if rel.starts_with('/') {
        return rel.trim_start_matches('/').to_string();
    }
    let mut parts: Vec<String> = base_dir
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    for seg in rel.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            let _ = parts.pop();
        } else {
            parts.push(seg.to_string());
        }
    }
    parts.join("/")
}

fn build_name_from_id(item_id: &str, href: &str) -> String {
    let href_name = Path::new(href)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let href_ext = Path::new(href_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let id_file_name = Path::new(item_id)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(item_id);

    let (mut id_stem, id_ext_opt) = if let Some(dot) = id_file_name.rfind('.') {
        (
            id_file_name[..dot].to_string(),
            Some(id_file_name[dot + 1..].to_ascii_lowercase()),
        )
    } else {
        (id_file_name.to_string(), None)
    };

    let mut image_slim = String::new();
    if id_stem.to_ascii_lowercase().contains("slim")
        || href_name.to_ascii_lowercase().contains("slim")
    {
        let lower = id_stem.to_ascii_lowercase();
        let mut cut = None;
        for suffix in ["~slim", "-slim", "_slim", "slim"] {
            if lower.ends_with(suffix) {
                cut = Some(id_stem.len().saturating_sub(suffix.len()));
                break;
            }
        }
        if let Some(pos) = cut {
            id_stem = id_stem[..pos].to_string();
        }
        image_slim = "~slim".to_string();
    }

    if id_stem.is_empty() {
        id_stem = "file".to_string();
    }
    id_stem = sanitize_windows_component(&id_stem);

    let ext = match id_ext_opt {
        Some(id_ext) if !id_ext.is_empty() => {
            if !href_ext.is_empty() && id_ext != href_ext {
                href_ext
            } else {
                id_ext
            }
        }
        _ => {
            if href_ext.is_empty() {
                "bin".to_string()
            } else {
                href_ext
            }
        }
    };
    format!("{}{}.{}", id_stem, image_slim, ext)
}

pub(crate) fn collect_opf_id_hints(bytes: &[u8]) -> HashMap<String, String> {
    let mut hints: HashMap<String, String> = HashMap::new();
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return hints,
    };

    let mut opf_paths: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().replace('\\', "/");
            if name.to_ascii_lowercase().ends_with(".opf") {
                opf_paths.push(name);
            }
        }
    }

    for opf_path in opf_paths {
        let mut opf_data: Vec<u8> = Vec::new();
        if let Ok(mut f) = archive.by_name(&opf_path) {
            let _ = f.read_to_end(&mut opf_data);
        } else {
            continue;
        }
        let opf_text = String::from_utf8_lossy(&opf_data).to_string();
        let item_re = match Regex::new(r#"(?is)<item\b([^>]*)>"#) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let opf_dir = zip_parent(&opf_path);
        for caps in item_re.captures_iter(&opf_text).flatten() {
            let attrs_text = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let attrs = parse_xmlish_attrs(attrs_text);
            let item_id = attrs.get("id").map(|s| s.trim()).unwrap_or("");
            let href_raw = attrs.get("href").map(|s| s.trim()).unwrap_or("");
            if item_id.is_empty() || href_raw.is_empty() {
                continue;
            }
            let href_decoded = percent_decode(href_raw);
            let abs = zip_join(&opf_dir, &href_decoded);
            let name_hint = build_name_from_id(item_id, &href_decoded);
            hints.insert(abs.clone(), name_hint.clone());
            hints.insert(abs.to_ascii_lowercase(), name_hint);
        }
    }
    hints
}

pub(crate) fn ensure_unique_zip_name(name: &str, used: &mut HashSet<String>) -> String {
    if !used.contains(name) {
        used.insert(name.to_string());
        return name.to_string();
    }
    let is_dir = name.ends_with('/');
    let base_name = if is_dir {
        name.trim_end_matches('/').to_string()
    } else {
        name.to_string()
    };
    let mut stem = base_name.clone();
    let mut ext = String::new();
    if !is_dir {
        if let Some(idx) = base_name.rfind('.') {
            stem = base_name[..idx].to_string();
            ext = base_name[idx..].to_string();
        }
    }
    let mut i = 2usize;
    loop {
        let mut candidate = if is_dir {
            format!("{}_{}", stem, i)
        } else {
            format!("{}_{}{}", stem, i, ext)
        };
        if is_dir {
            candidate.push('/');
        }
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        i += 1;
    }
}

pub(crate) fn zip_parent(path: &str) -> String {
    if let Some(idx) = path.rfind('/') {
        path[..idx].to_string()
    } else {
        "".to_string()
    }
}

pub(crate) fn zip_relative_path(from_file: &str, to_file: &str) -> String {
    let from_dir = zip_parent(from_file);
    let from_parts: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to_file.split('/').filter(|s| !s.is_empty()).collect();
    let mut i = 0usize;
    while i < from_parts.len() && i < to_parts.len() && from_parts[i] == to_parts[i] {
        i += 1;
    }
    let mut rel_parts: Vec<String> = Vec::new();
    for _ in i..from_parts.len() {
        rel_parts.push("..".to_string());
    }
    for part in to_parts.iter().skip(i) {
        rel_parts.push((*part).to_string());
    }
    if rel_parts.is_empty() {
        ".".to_string()
    } else {
        rel_parts.join("/")
    }
}

pub(crate) fn is_text_like_entry(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".xhtml")
        || lower.ends_with(".html")
        || lower.ends_with(".htm")
        || lower.ends_with(".xml")
        || lower.ends_with(".opf")
        || lower.ends_with(".ncx")
        || lower.ends_with(".css")
        || lower.ends_with(".svg")
}

fn percent_encode_path_ref(path: &str, upper_hex: bool) -> String {
    let mut out = String::new();
    for b in path.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*b as char)
            }
            _ if upper_hex => out.push_str(&format!("%{:02X}", b)),
            _ => out.push_str(&format!("%{:02x}", b)),
        }
    }
    out
}

pub(crate) fn split_ref_suffix(raw_ref: &str) -> (&str, &str) {
    let query_pos = raw_ref.find('?');
    let fragment_pos = raw_ref.find('#');
    let cut = match (query_pos, fragment_pos) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    if let Some(idx) = cut {
        (&raw_ref[..idx], &raw_ref[idx..])
    } else {
        (raw_ref, "")
    }
}

pub(crate) fn is_external_or_inline_ref(raw_ref: &str) -> bool {
    let lower = raw_ref.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.starts_with('#')
        || lower.starts_with("data:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
        || lower.starts_with("mailto:")
        || lower.starts_with("javascript:")
        || lower.starts_with("urn:")
}

fn rewrite_single_path_ref(
    raw_ref: &str,
    current_old_path: &str,
    current_new_path: &str,
    path_map: &HashMap<String, String>,
    lower_path_map: &HashMap<String, String>,
) -> Option<String> {
    if is_external_or_inline_ref(raw_ref) {
        return None;
    }
    let (main_ref, suffix) = split_ref_suffix(raw_ref);
    if is_external_or_inline_ref(main_ref) {
        return None;
    }
    let decoded_ref = percent_decode(main_ref);
    let old_abs = zip_join(&zip_parent(current_old_path), &decoded_ref);
    let new_abs = path_map
        .get(&old_abs)
        .or_else(|| lower_path_map.get(&old_abs.to_ascii_lowercase()))?;
    if old_abs.ends_with('/') || new_abs.ends_with('/') {
        return None;
    }

    let new_rel = zip_relative_path(current_new_path, new_abs);
    if new_rel == "." {
        return None;
    }

    let mut rewritten = if main_ref.contains('%') {
        percent_encode_path_ref(&new_rel, true)
    } else {
        new_rel
    };
    if main_ref.starts_with("./") && !rewritten.starts_with("./") && !rewritten.starts_with("../") {
        rewritten = format!("./{}", rewritten);
    }
    rewritten.push_str(suffix);

    if rewritten == raw_ref {
        None
    } else {
        Some(rewritten)
    }
}

fn build_lower_path_map(path_map: &HashMap<String, String>) -> HashMap<String, String> {
    let mut lower_path_map = HashMap::new();
    let mut ambiguous = HashSet::new();

    for (old, new_path) in path_map {
        let lower = old.to_ascii_lowercase();
        if let Some(existing) = lower_path_map.get(&lower) {
            if existing != new_path {
                ambiguous.insert(lower);
            }
        } else {
            lower_path_map.insert(lower, new_path.clone());
        }
    }

    for lower in ambiguous {
        lower_path_map.remove(&lower);
    }
    lower_path_map
}

fn collect_text_link_rewrites(
    text: &str,
    current_old_path: &str,
    current_new_path: &str,
    path_map: &HashMap<String, String>,
) -> Vec<(String, String)> {
    let mut pairs: HashMap<String, String> = HashMap::new();
    let lower_path_map = build_lower_path_map(path_map);

    for caps in QUOTED_PATH_REF_RE.captures_iter(text).flatten() {
        let Some(raw_match) = caps.get(2) else {
            continue;
        };
        let raw_ref = raw_match.as_str();
        if let Some(rewritten) = rewrite_single_path_ref(
            raw_ref,
            current_old_path,
            current_new_path,
            path_map,
            &lower_path_map,
        ) {
            pairs.insert(raw_ref.to_string(), rewritten);
        }
    }

    for caps in CSS_URL_PATH_RE.captures_iter(text).flatten() {
        let Some(raw_match) = caps.get(1) else {
            continue;
        };
        let raw_ref = raw_match.as_str().trim();
        if let Some(rewritten) = rewrite_single_path_ref(
            raw_ref,
            current_old_path,
            current_new_path,
            path_map,
            &lower_path_map,
        ) {
            pairs.insert(raw_ref.to_string(), rewritten);
        }
    }

    let mut out: Vec<(String, String)> = pairs.into_iter().collect();
    out.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    out
}

pub(crate) fn rewrite_text_links(
    text: String,
    current_old_path: &str,
    current_new_path: &str,
    path_map: &HashMap<String, String>,
) -> String {
    let pairs = collect_text_link_rewrites(&text, current_old_path, current_new_path, path_map);
    if pairs.is_empty() {
        return text;
    }
    let mut out = text;
    for (from, to) in pairs {
        out = out.replace(&from, &to);
    }
    out
}

pub(crate) fn rebuild_epub_with_sanitized_names_from_bytes(
    source: &Path,
    bytes: &[u8],
    suffix: &str,
) -> Result<Option<PathBuf>, String> {
    let mut noop = |_stage: &str| {};
    rebuild_epub_with_sanitized_names_from_bytes_with_progress(source, bytes, suffix, &mut noop)
}

pub(crate) fn rebuild_epub_with_sanitized_names_from_bytes_with_progress<F>(
    source: &Path,
    bytes: &[u8],
    suffix: &str,
    progress: &mut F,
) -> Result<Option<PathBuf>, String>
where
    F: FnMut(&str),
{
    progress("读取 EPUB 目录");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    let mut path_map: HashMap<String, String> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    progress("解析 OPF manifest");
    let opf_name_hints = collect_opf_id_hints(bytes);
    let mut friendly_counter: usize = 1;
    let mut changed = false;

    progress("分析混淆文件名");
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        let old_name = file.name().replace('\\', "/");
        let mut sanitized = sanitize_zip_entry_name(&old_name);
        let old_invalid = has_windows_invalid_component(&old_name);
        let old_obfuscated = looks_obfuscated_basename(&old_name);

        if !sanitized.ends_with('/') {
            let parent = zip_parent(&sanitized);
            let hint = opf_name_hints
                .get(&old_name)
                .or_else(|| opf_name_hints.get(&old_name.to_ascii_lowercase()))
                .cloned();

            if old_obfuscated || old_invalid {
                if let Some(hint_name) = hint {
                    let target = if parent.is_empty() {
                        hint_name
                    } else {
                        format!("{}/{}", parent, hint_name)
                    };
                    if target != sanitized {
                        sanitized = target;
                        changed = true;
                    }
                } else if old_obfuscated {
                    let friendly = friendly_name_for_path(&sanitized, friendly_counter);
                    friendly_counter += 1;
                    sanitized = if parent.is_empty() {
                        friendly
                    } else {
                        format!("{}/{}", parent, friendly)
                    };
                    changed = true;
                }
            }
        }
        let unique = ensure_unique_zip_name(&sanitized, &mut used);
        if old_name != unique {
            changed = true;
        }
        if old_invalid {
            changed = true;
        }
        path_map.insert(old_name, unique);
    }

    if !changed {
        progress("未检测到需要修复的文件名");
        return Ok(None);
    }

    progress("创建修复后的 EPUB");
    let out_path = build_processed_epub_path(source, suffix);
    let out_file = fs::File::create(&out_path).map_err(|e| format!("创建输出 EPUB 失败: {}", e))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let mut archive2 = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    let mut opf_old_to_new: HashMap<String, String> = HashMap::new();
    for (old, newp) in &path_map {
        if old.to_ascii_lowercase().ends_with(".opf") && newp.to_ascii_lowercase().ends_with(".opf")
        {
            opf_old_to_new.insert(old.clone(), newp.clone());
        }
    }

    let total_entries = archive2.len();
    for i in 0..total_entries {
        if i == 0 || (i + 1) % 100 == 0 || i + 1 == total_entries {
            progress(&format!("重写 EPUB 条目 {}/{}", i + 1, total_entries));
        }
        let mut file = archive2
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
            text = rewrite_text_links(text, &old_name, &new_name, &path_map);
            data = text.into_bytes();
        } else if is_text_like_entry(&old_name) {
            let text = String::from_utf8_lossy(&data).to_string();
            let rewritten = rewrite_text_links(text, &old_name, &new_name, &path_map);
            data = rewritten.into_bytes();
        }

        writer
            .start_file(&new_name, options)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        writer
            .write_all(&data)
            .map_err(|e| format!("写入文件内容失败: {}", e))?;
    }

    progress("完成 EPUB 写入");
    writer
        .finish()
        .map_err(|e| format!("完成写入失败: {}", e))?;
    progress("EPUB 修复写出完成");
    Ok(Some(out_path))
}

pub(crate) fn parse_xmlish_attrs(text: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let Ok(attr_re) = Regex::new(r#"([:\w.-]+)\s*=\s*(['"])(.*?)\2"#) else {
        return attrs;
    };
    for caps in attr_re.captures_iter(text).flatten() {
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
        let value = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();
        if !key.is_empty() {
            attrs.insert(key, value);
        }
    }
    attrs
}

pub(crate) fn portable_marker_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("portable.txt"))
}

pub(crate) fn is_portable() -> bool {
    portable_marker_path().map(|p| p.exists()).unwrap_or(false)
}

pub(crate) fn portable_data_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("data"))
}

pub(crate) fn app_data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if is_portable() {
        if let Some(p) = portable_data_root() {
            return Ok(p);
        }
    }
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .map_err(|e| format!("无法获取 app_data_dir: {}", e))
}

// 文件名清洗：去掉 Windows 非法字符 + 控制字符 + 末尾点/空格，限制长度
pub(crate) fn sanitize_filename_part(s: &str) -> String {
    let mut out: String = s
        .chars()
        .filter_map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => Some('_'),
            c if (c as u32) < 0x20 => None,
            c => Some(c),
        })
        .collect();
    out = out.trim().trim_end_matches('.').trim().to_string();
    // 截断到 80 chars 避免过长 + 给「书名-作者-(2)」留余量
    if out.chars().count() > 80 {
        out = out.chars().take(80).collect::<String>().trim().to_string();
    }
    out
}

pub(crate) fn ensure_dir(p: &Path) -> Result<(), String> {
    if !p.exists() {
        fs::create_dir_all(p).map_err(|e| format!("创建目录失败 {}: {}", p.display(), e))?;
    }
    Ok(())
}

// 检测图片类型，仅依据 magic bytes，未识别则按 jpg 处理
pub(crate) fn detect_image_ext(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        "jpg"
    } else if bytes.len() >= 8
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        "png"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        "gif"
    } else {
        "jpg"
    }
}

