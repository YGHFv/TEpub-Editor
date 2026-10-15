use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::epub::EpubMetadata;
use crate::fs_utils::{
    ensure_dir, hidden_process_command, lock_epub_cache, normalize_local_file_path,
    sanitize_filename_part,
};
use crate::library::library_data_dir;
use crate::library::library_fonts_dir;

#[derive(Serialize, Debug, Clone)]
pub(crate) struct LibraryFontInfo {
    family: String,
    css_value: String,
    file_name: String,
    path: String,
}

pub(crate) type LibraryFontAliasMap = HashMap<String, String>;

#[derive(Serialize, Debug)]
pub(crate) struct FontGlyphAnalyzeResult {
    internal_names: Vec<String>,
    missing_chars: Vec<String>,
    unsupported_reason: Option<String>,
}

// --- 辅助函数 ---

pub(crate) fn build_font_subset_text(content: &str, metadata: &EpubMetadata) -> String {
    let mut seen = HashSet::new();
    let mut out = String::new();

    for chunk in [
        metadata.title.as_str(),
        metadata.creator.as_str(),
        metadata.publisher.as_str(),
        metadata.description.as_str(),
        content,
    ] {
        for ch in chunk.chars() {
            if seen.insert(ch) {
                out.push(ch);
            }
        }
    }

    for tag in &metadata.tags {
        for ch in tag.chars() {
            if seen.insert(ch) {
                out.push(ch);
            }
        }
    }

    out
}

pub(crate) fn ensure_fonttools_available() -> Result<(), String> {
    let check = hidden_process_command("python")
        .args(["-m", "fontTools.subset", "--help"])
        .output();
    if let Ok(output) = check {
        if output.status.success() {
            return Ok(());
        }
    }

    let install = hidden_process_command("python")
        .args([
            "-m",
            "pip",
            "install",
            "--user",
            "fonttools",
            "brotli",
            "zopfli",
        ])
        .output()
        .map_err(|e| format!("无法启动 Python 安装 fontTools: {}", e))?;

    if install.status.success() {
        Ok(())
    } else {
        Err(format!(
            "自动安装 fontTools 失败: {}",
            String::from_utf8_lossy(&install.stderr)
        ))
    }
}

pub(crate) fn try_subset_font_bytes(
    bytes: &[u8],
    ext: &str,
    subset_text: &str,
) -> Result<Vec<u8>, String> {
    let ext = ext.to_lowercase();
    if subset_text.trim().is_empty() || !matches!(ext.as_str(), "ttf" | "otf" | "woff" | "woff2") {
        return Ok(bytes.to_vec());
    }

    ensure_fonttools_available()?;

    let temp_dir = tempfile::tempdir().map_err(|e| format!("创建字体子集临时目录失败: {}", e))?;
    let input_path = temp_dir.path().join(format!("input.{}", ext));
    let text_path = temp_dir.path().join("subset.txt");
    let output_path = temp_dir.path().join(format!("output.{}", ext));

    fs::write(&input_path, bytes).map_err(|e| format!("写入原始字体临时文件失败: {}", e))?;
    fs::write(&text_path, subset_text).map_err(|e| format!("写入子集字符集失败: {}", e))?;

    let mut args = vec![
        "-m".to_string(),
        "fontTools.subset".to_string(),
        input_path.to_string_lossy().to_string(),
        format!("--text-file={}", text_path.to_string_lossy()),
        format!("--output-file={}", output_path.to_string_lossy()),
        "--layout-features=*".to_string(),
        "--glyph-names".to_string(),
        "--symbol-cmap".to_string(),
        "--legacy-cmap".to_string(),
        "--notdef-glyph".to_string(),
        "--notdef-outline".to_string(),
        "--recommended-glyphs".to_string(),
        "--name-IDs=*".to_string(),
        "--name-legacy".to_string(),
        "--name-languages=*".to_string(),
    ];

    if ext == "woff" || ext == "woff2" {
        args.push(format!("--flavor={}", ext));
    }

    let output = hidden_process_command("python")
        .args(&args)
        .output()
        .map_err(|e| format!("无法启动字体子集化命令: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "fontTools 子集化失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    fs::read(&output_path).map_err(|e| format!("读取子集化字体失败: {}", e))
}

fn library_font_aliases_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("font-aliases.json"))
}

fn read_library_font_aliases(app: &tauri::AppHandle) -> Result<LibraryFontAliasMap, String> {
    let path = library_font_aliases_path(app)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取字体别名失败: {}", e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("解析字体别名失败: {}", e))
}

fn write_library_font_aliases(
    app: &tauri::AppHandle,
    aliases: &LibraryFontAliasMap,
) -> Result<(), String> {
    let dir = library_data_dir(app)?;
    ensure_dir(&dir)?;
    let path = library_font_aliases_path(app)?;
    let bytes =
        serde_json::to_vec_pretty(aliases).map_err(|e| format!("序列化字体别名失败: {}", e))?;
    fs::write(&path, bytes).map_err(|e| format!("保存字体别名失败: {}", e))
}

fn pick_font_family_name(font_data: &[u8], source_path: &Path) -> String {
    if let Some(name) = pick_preferred_font_name(parse_font_internal_names(font_data)) {
        return name;
    }

    source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "CustomFont".to_string())
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch as u32,
            0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xF900..=0xFAFF
                | 0x20000..=0x2A6DF
                | 0x2A700..=0x2B73F
                | 0x2B740..=0x2B81F
                | 0x2B820..=0x2CEAF
                | 0x2CEB0..=0x2EBEF
        )
    })
}

fn font_name_preference_score(name: &str) -> i32 {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return i32::MIN;
    }

    let mut score = 0i32;
    if contains_cjk(trimmed) {
        score += 1000;
    }
    if trimmed.contains(' ') {
        score += 40;
    }
    if trimmed.len() >= 2 {
        score += 10;
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '+' | '.'))
    {
        score -= 120;
    }
    if trimmed.eq_ignore_ascii_case("regular")
        || trimmed.eq_ignore_ascii_case("bold")
        || trimmed.eq_ignore_ascii_case("italic")
    {
        score -= 400;
    }
    score - trimmed.len() as i32 / 4
}

fn pick_preferred_font_name(names: Vec<String>) -> Option<String> {
    names
        .into_iter()
        .filter(|name| !name.trim().is_empty())
        .max_by_key(|name| font_name_preference_score(name))
}

fn build_library_font_info(
    app: &tauri::AppHandle,
    font_path: &Path,
) -> Result<LibraryFontInfo, String> {
    let font_data = fs::read(font_path).map_err(|e| format!("读取字体失败: {}", e))?;
    let file_name = font_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "字体文件名无效".to_string())?
        .to_string();
    let aliases = read_library_font_aliases(app).unwrap_or_default();
    let family = aliases
        .get(&file_name)
        .cloned()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| pick_font_family_name(&font_data, font_path));
    Ok(LibraryFontInfo {
        css_value: format!(r#""{}", serif"#, family),
        family,
        file_name,
        path: font_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn list_library_fonts(app: tauri::AppHandle) -> Result<Vec<LibraryFontInfo>, String> {
    let dir = library_fonts_dir(&app)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut fonts = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("读取字体目录失败: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取字体目录项失败: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !matches!(ext.as_str(), "ttf" | "otf" | "woff" | "woff2") {
            continue;
        }
        if let Ok(info) = build_library_font_info(&app, &path) {
            fonts.push(info);
        }
    }

    fonts.sort_by(|a, b| a.family.to_lowercase().cmp(&b.family.to_lowercase()));
    Ok(fonts)
}

#[tauri::command]
pub fn import_library_font(app: tauri::AppHandle, path: String) -> Result<LibraryFontInfo, String> {
    let normalized = normalize_local_file_path(&path);
    let source = PathBuf::from(&normalized);
    if !source.exists() || !source.is_file() {
        return Err("字体文件不存在".to_string());
    }

    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !matches!(ext.as_str(), "ttf" | "otf" | "woff" | "woff2") {
        return Err("仅支持导入 ttf / otf / woff / woff2 字体".to_string());
    }

    let font_data = fs::read(&source).map_err(|e| format!("读取字体失败: {}", e))?;
    let family = pick_font_family_name(&font_data, &source);
    let file_stem = sanitize_filename_part(&family);
    let safe_stem = if file_stem.is_empty() {
        "font".to_string()
    } else {
        file_stem
    };

    let target_dir = library_fonts_dir(&app)?;
    ensure_dir(&target_dir)?;

    let mut target = target_dir.join(format!("{}.{}", safe_stem, ext));
    let mut seq = 2usize;
    while target.exists() {
        let existing = fs::read(&target).unwrap_or_default();
        if existing == font_data {
            return build_library_font_info(&app, &target);
        }
        target = target_dir.join(format!("{}-{}.{}", safe_stem, seq, ext));
        seq += 1;
    }

    fs::write(&target, &font_data).map_err(|e| format!("写入字体失败: {}", e))?;
    build_library_font_info(&app, &target)
}

#[tauri::command]
pub fn rename_library_font(
    app: tauri::AppHandle,
    file_name: String,
    family: String,
) -> Result<LibraryFontInfo, String> {
    let trimmed_family = family.trim();
    if trimmed_family.is_empty() {
        return Err("字体名称不能为空".to_string());
    }

    let font_path = library_fonts_dir(&app)?.join(&file_name);
    if !font_path.exists() || !font_path.is_file() {
        return Err("字体文件不存在".to_string());
    }

    let mut aliases = read_library_font_aliases(&app)?;
    aliases.insert(file_name.clone(), trimmed_family.to_string());
    write_library_font_aliases(&app, &aliases)?;
    build_library_font_info(&app, &font_path)
}

#[tauri::command]
pub fn delete_library_font(app: tauri::AppHandle, file_name: String) -> Result<(), String> {
    let font_path = library_fonts_dir(&app)?.join(&file_name);
    if !font_path.exists() || !font_path.is_file() {
        return Err("字体文件不存在".to_string());
    }

    fs::remove_file(&font_path).map_err(|e| format!("删除字体失败: {}", e))?;
    let mut aliases = read_library_font_aliases(&app)?;
    if aliases.remove(&file_name).is_some() {
        write_library_font_aliases(&app, &aliases)?;
    }
    Ok(())
}

fn be_u16(data: &[u8], offset: usize) -> Option<u16> {
    if offset + 2 > data.len() {
        return None;
    }
    Some(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

fn be_u32(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > data.len() {
        return None;
    }
    Some(u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

fn sfnt_table<'a>(font_data: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    if font_data.len() < 12 {
        return None;
    }
    let num_tables = be_u16(font_data, 4)? as usize;
    let mut rec_off = 12usize;
    for _ in 0..num_tables {
        if rec_off + 16 > font_data.len() {
            return None;
        }
        if &font_data[rec_off..rec_off + 4] == tag {
            let table_offset = be_u32(font_data, rec_off + 8)? as usize;
            let table_len = be_u32(font_data, rec_off + 12)? as usize;
            if table_offset + table_len > font_data.len() {
                return None;
            }
            return Some(&font_data[table_offset..table_offset + table_len]);
        }
        rec_off += 16;
    }
    None
}

fn decode_utf16be(raw: &[u8]) -> Option<String> {
    if raw.is_empty() || raw.len() % 2 != 0 {
        return None;
    }
    let mut units = Vec::with_capacity(raw.len() / 2);
    let mut idx = 0usize;
    while idx + 1 < raw.len() {
        units.push(u16::from_be_bytes([raw[idx], raw[idx + 1]]));
        idx += 2;
    }
    String::from_utf16(&units).ok()
}

fn parse_font_internal_names(font_data: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut dedup = HashSet::new();
    let table = match sfnt_table(font_data, b"name") {
        Some(t) => t,
        None => return names,
    };
    if table.len() < 6 {
        return names;
    }

    let count = match be_u16(table, 2) {
        Some(v) => v as usize,
        None => return names,
    };
    let string_offset = match be_u16(table, 4) {
        Some(v) => v as usize,
        None => return names,
    };

    for i in 0..count {
        let rec_off = 6 + i * 12;
        if rec_off + 12 > table.len() {
            break;
        }
        let platform_id = be_u16(table, rec_off).unwrap_or(0);
        let name_id = be_u16(table, rec_off + 6).unwrap_or(0);
        if name_id != 1 && name_id != 4 && name_id != 16 {
            continue;
        }
        let length = be_u16(table, rec_off + 8).unwrap_or(0) as usize;
        let offset = be_u16(table, rec_off + 10).unwrap_or(0) as usize;
        let start = string_offset + offset;
        let end = start + length;
        if end > table.len() || start >= end {
            continue;
        }
        let raw = &table[start..end];
        let value = if platform_id == 0 || platform_id == 3 {
            decode_utf16be(raw)
        } else {
            Some(String::from_utf8_lossy(raw).to_string())
        };
        if let Some(mut v) = value {
            v = v.trim().to_string();
            if !v.is_empty() && dedup.insert(v.clone()) {
                names.push(v);
            }
        }
    }
    names
}

fn cmap_has_glyph_format4(subtable: &[u8], codepoint: u16) -> Option<bool> {
    if subtable.len() < 16 {
        return None;
    }
    let seg_count_x2 = be_u16(subtable, 6)? as usize;
    if seg_count_x2 == 0 || seg_count_x2 % 2 != 0 {
        return None;
    }
    let seg_count = seg_count_x2 / 2;
    let end_codes_off = 14usize;
    let start_codes_off = end_codes_off + seg_count * 2 + 2;
    let id_delta_off = start_codes_off + seg_count * 2;
    let id_range_off = id_delta_off + seg_count * 2;
    if id_range_off + seg_count * 2 > subtable.len() {
        return None;
    }

    for i in 0..seg_count {
        let end_code = be_u16(subtable, end_codes_off + i * 2)? as u32;
        if (codepoint as u32) > end_code {
            continue;
        }
        let start_code = be_u16(subtable, start_codes_off + i * 2)? as u32;
        if (codepoint as u32) < start_code {
            return Some(false);
        }
        let id_delta = be_u16(subtable, id_delta_off + i * 2)?;
        let id_range_offset = be_u16(subtable, id_range_off + i * 2)? as usize;
        if id_range_offset == 0 {
            let glyph = codepoint.wrapping_add(id_delta);
            return Some(glyph != 0);
        }

        let glyph_index_off =
            id_range_off + i * 2 + id_range_offset + ((codepoint as u32 - start_code) as usize) * 2;
        if glyph_index_off + 2 > subtable.len() {
            return Some(false);
        }
        let glyph_index = be_u16(subtable, glyph_index_off)?;
        if glyph_index == 0 {
            return Some(false);
        }
        let glyph = glyph_index.wrapping_add(id_delta);
        return Some(glyph != 0);
    }
    Some(false)
}

fn cmap_has_glyph_format12(subtable: &[u8], codepoint: u32) -> Option<bool> {
    if subtable.len() < 16 {
        return None;
    }
    let n_groups = be_u32(subtable, 12)? as usize;
    let groups_off = 16usize;
    for i in 0..n_groups {
        let off = groups_off + i * 12;
        if off + 12 > subtable.len() {
            break;
        }
        let start_char = be_u32(subtable, off)?;
        let end_char = be_u32(subtable, off + 4)?;
        let start_glyph = be_u32(subtable, off + 8)?;
        if codepoint < start_char {
            return Some(false);
        }
        if codepoint <= end_char {
            return Some(start_glyph + (codepoint - start_char) != 0);
        }
    }
    Some(false)
}

fn cmap_has_glyph_format13(subtable: &[u8], codepoint: u32) -> Option<bool> {
    if subtable.len() < 16 {
        return None;
    }
    let n_groups = be_u32(subtable, 12)? as usize;
    let groups_off = 16usize;
    for i in 0..n_groups {
        let off = groups_off + i * 12;
        if off + 12 > subtable.len() {
            break;
        }
        let start_char = be_u32(subtable, off)?;
        let end_char = be_u32(subtable, off + 4)?;
        let glyph_id = be_u32(subtable, off + 8)?;
        if codepoint < start_char {
            return Some(false);
        }
        if codepoint <= end_char {
            return Some(glyph_id != 0);
        }
    }
    Some(false)
}

fn font_has_glyph(font_data: &[u8], codepoint: u32) -> Result<bool, String> {
    let cmap = sfnt_table(font_data, b"cmap").ok_or_else(|| "字体缺少 cmap 表".to_string())?;
    if cmap.len() < 4 {
        return Err("字体 cmap 表无效".to_string());
    }
    let num_tables = be_u16(cmap, 2).ok_or_else(|| "字体 cmap 表无效".to_string())? as usize;
    let mut checked_any = false;

    for i in 0..num_tables {
        let rec_off = 4 + i * 8;
        if rec_off + 8 > cmap.len() {
            break;
        }
        let sub_offset = match be_u32(cmap, rec_off + 4) {
            Some(v) => v as usize,
            None => continue,
        };
        if sub_offset + 2 > cmap.len() {
            continue;
        }
        let format = be_u16(cmap, sub_offset).unwrap_or(0);
        match format {
            4 => {
                if codepoint > 0xFFFF {
                    continue;
                }
                checked_any = true;
                let res = cmap_has_glyph_format4(&cmap[sub_offset..], codepoint as u16)
                    .ok_or_else(|| "字体 cmap format 4 解析失败".to_string())?;
                if res {
                    return Ok(true);
                }
            }
            12 => {
                checked_any = true;
                let res = cmap_has_glyph_format12(&cmap[sub_offset..], codepoint)
                    .ok_or_else(|| "字体 cmap format 12 解析失败".to_string())?;
                if res {
                    return Ok(true);
                }
            }
            13 => {
                checked_any = true;
                let res = cmap_has_glyph_format13(&cmap[sub_offset..], codepoint)
                    .ok_or_else(|| "字体 cmap format 13 解析失败".to_string())?;
                if res {
                    return Ok(true);
                }
            }
            _ => {}
        }
    }

    if !checked_any {
        return Err("字体 cmap 子表格式暂不支持".to_string());
    }
    Ok(false)
}

fn read_epub_binary_cached(epub_path: &str, file_path: &str) -> Result<Vec<u8>, String> {
    {
        let cache_guard = lock_epub_cache()?;
        if let Some(cache) = cache_guard.get(epub_path) {
            if let Some(data) = cache.binary_cache.get(file_path) {
                return Ok(data.clone());
            }
        }
    }

    let file = fs::File::open(epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("无效 EPUB 文件: {}", e))?;
    let mut zip_file = archive
        .by_name(file_path)
        .map_err(|e| format!("字体文件未找到: {}", e))?;

    let mut buffer = Vec::new();
    zip_file
        .read_to_end(&mut buffer)
        .map_err(|e| format!("读取字体文件失败: {}", e))?;

    {
        let mut cache_guard = lock_epub_cache()?;
        if let Some(cache) = cache_guard.get_mut(epub_path) {
            cache
                .binary_cache
                .insert(file_path.to_string(), buffer.clone());
        }
    }

    Ok(buffer)
}

#[tauri::command]
pub async fn analyze_epub_font_glyphs(
    epub_path: String,
    file_path: String,
    chars: Vec<String>,
) -> Result<FontGlyphAnalyzeResult, String> {
    let font_data = read_epub_binary_cached(&epub_path, &file_path)?;
    let internal_names = parse_font_internal_names(&font_data);

    if font_data.starts_with(b"wOFF") || font_data.starts_with(b"wOF2") {
        return Ok(FontGlyphAnalyzeResult {
            internal_names,
            missing_chars: Vec::new(),
            unsupported_reason: Some("当前暂不支持 WOFF/WOFF2 的字形统计".to_string()),
        });
    }

    if font_data.starts_with(b"ttcf") {
        return Ok(FontGlyphAnalyzeResult {
            internal_names,
            missing_chars: Vec::new(),
            unsupported_reason: Some("当前暂不支持 TTC 字体集合的字形统计".to_string()),
        });
    }

    let mut missing_chars: Vec<String> = Vec::new();
    for ch in chars {
        let c = match ch.chars().next() {
            Some(v) => v,
            None => continue,
        };
        if c.is_whitespace() {
            continue;
        }
        let codepoint = c as u32;
        match font_has_glyph(&font_data, codepoint) {
            Ok(true) => {}
            Ok(false) => missing_chars.push(ch),
            Err(reason) => {
                return Ok(FontGlyphAnalyzeResult {
                    internal_names,
                    missing_chars: Vec::new(),
                    unsupported_reason: Some(reason),
                });
            }
        }
    }

    Ok(FontGlyphAnalyzeResult {
        internal_names,
        missing_chars,
        unsupported_reason: None,
    })
}
