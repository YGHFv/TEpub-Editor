use chardetng::EncodingDetector;
use fancy_regex::Regex;
use md5;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process; // 引入进程控制
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use walkdir::WalkDir;
use zip::write::FileOptions;

// --- EPUB 全局缓存 ---
struct EpubCache {
    epub_path: String,
    text_cache: HashMap<String, String>,
    binary_cache: HashMap<String, Vec<u8>>,
    temp_dir: Option<TempDir>,
}

impl EpubCache {
    fn new(path: String) -> Self {
        EpubCache {
            epub_path: path,
            text_cache: HashMap::new(),
            binary_cache: HashMap::new(),
            temp_dir: None,
        }
    }
}

static EPUB_CACHE: Lazy<Mutex<Option<EpubCache>>> = Lazy::new(|| Mutex::new(None));

// --- 静态资源: 整理后的 CSS ---

const CSS_FONT: &str = r#"@charset "utf-8";
/*正文字体*/
@font-face {
    font-family: "Maintext";
    src: url("../Fonts/Maintext.ttf");
}

/*标题字体*/
@font-face {
    font-family: "Title";
    src: url("../Fonts/Title.ttf"); 
}"#;

const CSS_MAIN: &str = r#"@charset "utf-8";

@import url("fonts.css");

/* Global Setting */

body {
    padding: 0%;
    margin-top: 0%;
    margin-bottom: 0%;
    margin-left: 0.5%;
    margin-right: 0.5%;
    line-height: 130%;
    text-align: justify;
    font-family: "Maintext", "DK-SONGTI", "st", "宋体", "zw", sans-serif;
}

p {
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 150%;
    margin-right: 0.5%;
    margin-left: 0.5%;
    font-family: "Maintext";
}

div {
    margin: 0;
    padding: 0;
    line-height: 130%;
    text-align: justify;
    font-family: "zw";
}

/*————————————————————制作说明————————————————————*/
.copyright {
    margin: 10% 7.25% 2.75% 7.25%;
    padding: 5.25% 5.25%;
    border: 1.5px solid #6C322D;
    background-size: 35% auto;
    border-radius: 5px;
}

.line {
    border: dotted #333;
    border-width: 1px 0 0 0;
    margin: 5% 0 5% 0;
}

h1.copyright-title {
    font-family: "Title";
    font-size: 121%;
    font-weight: normal;
    color: #00008B;
    margin: 1em 0 0.77em 0;
    text-align: center;
}

body.full {
    background: no-repeat center;
    background-size: cover;
    background-attachment: fixed;
    background-repeat: no-repeat;
    background-position: bottom center;
    background-image: url(../Images/back.jpg);
    transform: scale(1.0) translate(0px, 0px);
}

.copyright-text1 {
    font-family: "Title";
    font-size: 80%;
    color: #220;
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    margin: 0 0 2.5% 0;
}

.copyright-text2 {
    font-family: "cc", "kt", sans-serif;
    font-size: 65%;
    color: #000;
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    margin: 2.5% 0 0 0;
}

div.logo {
    margin: 0 24% 0 24%;
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
}

img.logo {
    width: 66%;
}

/*————————————————————内容简介————————————————————*/

body.introduction {
    border-color: rgba(83, 83, 83, 0.5);
    border-width: 0.4em;
}

div.cover {
    margin: 2em 0 1em 0;
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0;
    width: 100%;
}

img.cover {
    width: 40%;
    box-shadow: 3px 3px 3px #535353;
    margin: 0 0 0.5em 0;
}

h1.nrjj-title {
    font-family: "Title";
    font-size: 160%;
    font-weight: normal;
    color: #00008B;
    margin: 2em 0 1.6em 0;
    text-align: center;
}

span.book-name {
    font-family: "楷体", sans-serif;
    color: #DC143C
}

span.author {
    font-family: "小标宋", sans-serif;
}

h1.introduction-title {
    margin: 0.3em 0 0.5em 0;
    text-align: left;
    text-indent: 0;
    duokan-text-indent: 0;
    font-size: 110%;
    color: #00008B;
    font-family: "Title";
}

h1.introduction-title span {
    padding: 0.4em 2em 0.2em 0.4em;
}

div.book-introduction p {
    font-family: "DK-XIHEITI", "黑体", sans-serif;
}

h1.PrefacehA1 {
    font-family: "Title", "黑体", sans-serif;
    text-align: center;
    font-weight: 600;
    font-size: 1.2em;
    margin: 7em 0em 1em 0em;
    color: #f972bd;
    line-height: 130%;
}

h1.PrefacehA1 b {
    font-family: "Title", "黑体", sans-serif;
    font-size: 1.1em;
    font-weight: 900;
    color: #dd3e3f;
}

p.PrefacepA1 {
    font-family: "Title";
    color: #5577c1;
    font-size: 1.7em;
    margin: 0em 0em 0.2em 0em;
    text-indent: 0em;
    text-align: center;
    line-height: 110%;
}

/* Header Image */

div.logo {
    margin: 0.5em;
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
    duokan-bleed: lefttopright;
}

img.logo {
    width: 70%;
}

/* Chapter Title */

h3.head {
    font-size: 1.2em;
    color: #5577c1;
    text-align: center;
    line-height: 130%;
    padding: 35px 4px 0 4px;
    margin: 0em auto 2em auto;
    font-family: "Title";
}

h3.neirong {
    font-size: 1.1em;
    color: #5577c1;
    text-align: right;
    line-height: 130%;
    padding: 0 4px 0 4px;
    margin: -1em 0em 0em 2em;
    font-family: "Maintext";
}

span.num {
    font-family: "Maintext";
    padding: 2px 4px 1px 4px;
    text-align: center;
    font-size: 0.81em;
    background-color: #f972bd;
    border-radius: 10px;
    color: #fff;
}

span.num2 {
    font-size: 0.95em;
    color: white;
    background-color: #20626d;
    padding: 0.2em 0.4em 0.1em;
    border-radius: 0.2em;
    font-family: "Maintext";
}

span.num3 {
    color: #b50a02;
    font-family: "Maintext";
}

h2.head5 {
    padding: 0 4px 0 4px;
    margin: 1em auto 2em auto;
    font-size: 1.6em;
    color: #a36141;
    text-align: center;
    line-height: 130%;
    font-family: "Title";
    text-indent: 0em;
    duokan-text-indent: 0em;
}

h2.head {
    font-size: 2.1em;
    color: #59bde6;
    text-align: center;
    line-height: 130%;
    padding: 64px 4px 0 4px;
    margin: 0em auto 2em auto;
    font-family: "Title";
}

span.num {
    font-family: "Maintext";
    padding: 2px 4px 2px 4px;
    text-align: center;
    font-size: x-small;
    background-color: #f972bd;
    border-radius: 16px;
    color: #fff;
}

span.num2 {
    font-size: 0.95em;
    color: white;
    background-color: #20626d;
    padding: 0.2em 0.4em 0.1em;
    border-radius: 0.2em;
    font-family: "Maintext";
}

/* 分割线 */
p.fg1 {
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0em;
}

/*全面屏*/
body.fy {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: center;
    background-image: url('../Images/fy.jpg');
}

body.intro {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: center;
    background-image: url('../Images/intro.jpg');
}

body.e1 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e1.jpg'); }
body.e2 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e2.jpg'); }
body.e3 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e3.jpg'); }
body.e4 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e4.jpg'); }
body.e5 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e5.jpg'); }
body.e6 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e6.jpg'); }
body.e7 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e7.jpg'); }
body.e8 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e8.jpg'); }
body.e9 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e9.jpg'); }
body.e10 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e10.jpg'); }
body.e11 { background-size: cover; background-repeat: no-repeat; background-attachment: fixed; background-position: center; background-image: url('../Images/e11.jpg'); }
body.ex {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: center;
    background-image: url('../Images/ex.jpg');
}

p.fs3 {
    font-family: "zdy3";
    color: #000;
    margin: 1em 0em 1em 0em;
    font-size: 1.0em;
    font-weight: bold;
}

div.zwone {
    margin: 0em 0em 0em 0em;
    text-align: left;
    text-indent: 0em;
    duokan-text-indent: 0em;
}

img.zwone {
    width: 70%;
}

div.neirong {
    text-align: left;
    text-indent: 0em;
    margin: 0em 0em 0em 0em;
    duokan-text-indent: 0em;
}

img.neirong {
    width: 55%;
}

.fs2 {
    font-family: "zdy2";
    font-weight: bold;
}

.txtu {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy5";
    color: #1E90FF;
}

.txtu2 {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy5";
    color: #B22222;
    font-size: 0.95em;
}

p.fs7 {
    font-family: "Maintext";
    color: #000;
    font-size: 0.9em;
    text-align: right;
    margin: 1em 1em 2em 0em;
}

div.roundsolid2 {
    margin: 1em 0em;
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy4";
    color: #02439B;
    font-size: 0.9em;
}

.bu {
    display: block;
    font-size: .9em;
}

/*图片*/
.duokan-image-single {
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
    margin: 1.5em 0;
    text-align: center;
}

.DKimg-left {
    float: left;
    clear: both;
    width: 50%;
    margin: 0 0.5em 0.2em 0;
}

.DKimg-right {
    float: right;
    clear: both;
    width: 50%;
    margin: 0 0 0em 0.5em;
}

.txtu2 {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy6";
    color: #B22222;
    font-size: 0.95em;
}

.txtu {
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "zdy6";
    color: #1E90FF;
    font-size: 0.95em;
}"#;

// --- 数据结构 ---

#[derive(Serialize, Clone, Copy)]
struct MatchLocation {
    line: usize,
    start_char: usize,
    end_char: usize,
}

#[derive(Serialize)]
struct SearchResult {
    found: bool,
    count: usize,
    matches: Vec<MatchLocation>,
}

#[derive(Serialize)]
struct HistoryMeta {
    filename: String,
    path: String,
    timestamp: u64,
    size: u64,
    date_str: String,
}

#[derive(Deserialize, Debug, Clone)]
struct AssetInfo {
    name: String,
    path: String,
    category: String, // "fonts", "images", "others"
}

#[derive(Deserialize, Debug)]
struct EpubMetadata {
    title: String,
    creator: String,
    publisher: String,
    cover_path: String,
    uuid: String,
    md5: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    main_css: String,
    #[serde(default)]
    font_css: String,
    #[serde(default)]
    assets: Vec<AssetInfo>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Serialize, Debug, Clone)]
struct EpubFileNode {
    name: String,
    path: String,
    file_type: String, // folder, html, css, xml, image, font, other
    size: Option<u64>,
    title: Option<String>,      // For HTML files
    resolution: Option<String>, // For Image files (e.g., "1920x1080")
    children: Option<Vec<EpubFileNode>>,
}

#[derive(Serialize, Debug)]
struct FontGlyphAnalyzeResult {
    internal_names: Vec<String>,
    missing_chars: Vec<String>,
    unsupported_reason: Option<String>,
}

// --- 辅助函数 ---

fn escape_xml(input: &str) -> String {
    input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

fn format_vertical_volume(text: &str) -> String {
    text.chars()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join("<br />\n  ")
}

fn split_title(full_title: &str) -> (String, String) {
    let strict_re =
        Regex::new(r"^\s*(第[0-9零一二三四五六七八九十百千万]+[卷章回]|Chapter\s*\d+)\s*(.*)$")
            .unwrap();
    if let Ok(Some(caps)) = strict_re.captures(full_title) {
        let num = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
        let name = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
        if !num.is_empty() {
            return (num, name);
        }
    }
    let loose_re = Regex::new(r"^(.*?)\s+(.*)$").unwrap();
    if let Ok(Some(caps)) = loose_re.captures(full_title) {
        let num = caps.get(1).map_or("", |m| m.as_str()).to_string();
        let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
        return (num, name);
    }
    (full_title.to_string(), "".to_string())
}

// --- 换行符规范化 ---
// 将所有换行符（包括 Mac 旧时代的 \r 以及影响底层布局框架的特殊 Unicode 行分割符 U+2028）
// 统一为正统的 \n，确保后端行号计算与 CodeMirror 编辑器的严格分行计算完全一致。
fn normalize_line_endings(s: String) -> String {
    s.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{2028}', "\n")
        .replace('\u{2029}', "\n")
}

fn percent_decode(input: &str) -> String {
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

fn normalize_local_file_path(path: &str) -> String {
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

fn is_zip_archive_bytes(data: &[u8]) -> bool {
    let cursor = std::io::Cursor::new(data);
    zip::ZipArchive::new(cursor).is_ok()
}

fn is_zip_archive_path(path: &Path) -> bool {
    if let Ok(file) = fs::File::open(path) {
        zip::ZipArchive::new(file).is_ok()
    } else {
        false
    }
}

fn clear_zip_encryption_flags(data: &mut [u8]) -> bool {
    let mut changed = false;
    let mut i = 0usize;
    while i + 10 < data.len() {
        // Local file header: PK\x03\x04, flag at +6
        if data[i] == 0x50 && data[i + 1] == 0x4B && data[i + 2] == 0x03 && data[i + 3] == 0x04
        {
            let flag = u16::from_le_bytes([data[i + 6], data[i + 7]]);
            if flag & 0x0001 != 0 {
                let new_flag = (flag & !0x0001).to_le_bytes();
                data[i + 6] = new_flag[0];
                data[i + 7] = new_flag[1];
                changed = true;
            }
        }
        // Central directory header: PK\x01\x02, flag at +8
        if data[i] == 0x50 && data[i + 1] == 0x4B && data[i + 2] == 0x01 && data[i + 3] == 0x02
        {
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

fn xor_bytes(data: &[u8], key: u8) -> Vec<u8> {
    data.iter().map(|b| b ^ key).collect()
}

fn build_processed_epub_path(original: &Path, suffix: &str) -> PathBuf {
    let parent = original.parent().unwrap_or(Path::new("."));
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");
    let ext = original.extension().and_then(|s| s.to_str()).unwrap_or("epub");

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

fn sanitize_windows_component(part: &str) -> String {
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
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
        "LPT9",
    ];
    if reserved.contains(&upper.as_str()) {
        out.push('_');
    }
    out
}

fn sanitize_zip_entry_name(name: &str) -> String {
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
    let base = Path::new(raw_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
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

fn friendly_name_for_path(path: &str, index: usize) -> String {
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

fn zip_join(base_dir: &str, rel: &str) -> String {
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
    if id_stem.to_ascii_lowercase().contains("slim") || href_name.to_ascii_lowercase().contains("slim")
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

fn collect_opf_id_hints(bytes: &[u8]) -> HashMap<String, String> {
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
        let item_re = match Regex::new(
            r#"(?is)<item\b[^>]*\bid\s*=\s*["']([^"']+)["'][^>]*\bhref\s*=\s*["']([^"']+)["'][^>]*>"#,
        ) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let opf_dir = zip_parent(&opf_path);
        for caps in item_re.captures_iter(&opf_text).flatten() {
            let item_id = caps.get(1).map(|m| m.as_str()).unwrap_or("").trim();
            let href_raw = caps.get(2).map(|m| m.as_str()).unwrap_or("").trim();
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

fn ensure_unique_zip_name(name: &str, used: &mut HashSet<String>) -> String {
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

fn zip_parent(path: &str) -> String {
    if let Some(idx) = path.rfind('/') {
        path[..idx].to_string()
    } else {
        "".to_string()
    }
}

fn zip_relative_path(from_file: &str, to_file: &str) -> String {
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

fn is_text_like_entry(path: &str) -> bool {
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

fn rewrite_text_links(
    text: String,
    current_old_path: &str,
    current_new_path: &str,
    path_map: &HashMap<String, String>,
) -> String {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for (old_abs, new_abs) in path_map {
        if old_abs.ends_with('/') || new_abs.ends_with('/') {
            continue;
        }
        let old_rel = zip_relative_path(current_old_path, old_abs);
        let new_rel = zip_relative_path(current_new_path, new_abs);
        if old_rel == "." || new_rel == "." {
            continue;
        }
        pairs.push((old_rel.clone(), new_rel.clone()));
        pairs.push((old_rel.replace(' ', "%20"), new_rel.replace(' ', "%20")));
        if !old_rel.starts_with("./") {
            pairs.push((format!("./{}", old_rel), format!("./{}", new_rel)));
            pairs.push((
                format!("./{}", old_rel.replace(' ', "%20")),
                format!("./{}", new_rel.replace(' ', "%20")),
            ));
        }
    }

    pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    let mut out = text;
    for (from, to) in pairs {
        if from != to {
            out = out.replace(&from, &to);
        }
    }
    out
}

fn rebuild_epub_with_sanitized_names_from_bytes(
    source: &Path,
    bytes: &[u8],
    suffix: &str,
) -> Result<Option<PathBuf>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    let mut path_map: HashMap<String, String> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    let opf_name_hints = collect_opf_id_hints(bytes);
    let mut friendly_counter: usize = 1;
    let mut changed = false;

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

            if old_obfuscated {
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
                } else {
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
        return Ok(None);
    }

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

    for i in 0..archive2.len() {
        let mut file = archive2
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 条目失败: {}", e))?;
        let old_name = file.name().replace('\\', "/");
        let new_name = path_map
            .get(&old_name)
            .cloned()
            .unwrap_or_else(|| old_name.clone());

        let options = FileOptions::default().compression_method(file.compression());

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

    writer.finish().map_err(|e| format!("完成写入失败: {}", e))?;
    Ok(Some(out_path))
}

#[derive(Serialize)]
struct EpubPrepareResult {
    source_path: String,
    processed_path: String,
    changed: bool,
    action: String,
}

fn get_history_base_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join(".history");
        }
    }
    PathBuf::from(".history")
}

fn history_key_for_path(original_path: &str) -> String {
    // Keep names deterministic and avoid collisions for files that share the same stem.
    let digest = format!("{:x}", md5::compute(original_path.as_bytes()));
    digest[..8].to_string()
}

// --- 指令区域 ---

#[tauri::command]
fn exit_app() {
    process::exit(0);
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    let mut file = fs::File::open(&path).map_err(|e| format!("无法打开: {}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取失败: {}", e))?;

    // 策略：尝试多种编码，选取"乱码"（替换字符 ）最少的一个
    let candidates = vec![
        ("utf-8", encoding_rs::UTF_8),
        ("gb18030", encoding_rs::GB18030),
        ("utf-16le", encoding_rs::UTF_16LE),
        ("utf-16be", encoding_rs::UTF_16BE),
        ("big5", encoding_rs::BIG5),
    ];

    // 1. 优先尝试 UTF-8 (严格)
    if let Ok(s) = String::from_utf8(buffer.clone()) {
        return Ok(normalize_line_endings(s));
    }

    // 2. Chardetng 检测作为基准
    let mut detector = EncodingDetector::new();
    detector.feed(&buffer, true);
    let detected_encoding = detector.guess(Some(b"cn"), true);

    let (cow_detected, _, malformed_detected) = detected_encoding.decode(&buffer);
    let errors_detected = cow_detected.chars().filter(|&c| c == '\u{FFFD}').count();

    let mut best_content = cow_detected.into_owned();
    let mut min_errors = if malformed_detected {
        errors_detected
    } else {
        0
    };
    let mut best_encoding = detected_encoding.name();

    // 如果检测结果完美且不是 windows-1252 (容易误判)，直接返回
    if min_errors == 0 && best_encoding != "windows-1252" && best_encoding != "ISO-8859-1" {
        return Ok(normalize_line_endings(best_content));
    }

    // 3. 遍历候选编码打擂台
    for (name, enc) in candidates {
        let (cow, _, _) = enc.decode(&buffer);
        let content = cow.into_owned();
        let errors = content.chars().filter(|&c| c == '\u{FFFD}').count();

        // 优选错误更少的。
        // 特判：如果 best 是 windows-1252 (常见误判)，只要 candidates 里有 reasonably low error (<10%) 的中文编码，就替换它
        let is_current_bad_guess = best_encoding == "windows-1252" || best_encoding == "ISO-8859-1";

        if errors < min_errors || (is_current_bad_guess && errors < buffer.len() / 20) {
            min_errors = errors;
            best_content = content;
            best_encoding = name;
        }
    }

    // println!("Selected encoding: {} (errors: {})", best_encoding, min_errors);
    Ok(normalize_line_endings(best_content))
}

#[tauri::command]
fn prepare_epub_for_open(epub_path: String) -> Result<EpubPrepareResult, String> {
    let source = PathBuf::from(&epub_path);
    if !source.exists() {
        return Err(format!("文件不存在: {}", epub_path));
    }

    // Case 1: valid ZIP/EPUB, only repair pseudo-encryption bit if needed.
    if is_zip_archive_path(&source) {
        let mut bytes = fs::read(&source).map_err(|e| format!("读取 EPUB 失败: {}", e))?;
        let fixed_encryption_flag = clear_zip_encryption_flags(&mut bytes);

        if let Some(out_path) =
            rebuild_epub_with_sanitized_names_from_bytes(&source, &bytes, "_deobf")?
        {
            return Ok(EpubPrepareResult {
                source_path: source.to_string_lossy().to_string(),
                processed_path: out_path.to_string_lossy().to_string(),
                changed: true,
                action: if fixed_encryption_flag {
                    "已自动解除伪加密并修复文件名混淆".to_string()
                } else {
                    "已自动修复文件名混淆".to_string()
                },
            });
        }

        if fixed_encryption_flag {
            let out_path = build_processed_epub_path(&source, "_decrypted");
            fs::write(&out_path, &bytes).map_err(|e| format!("写入修复文件失败: {}", e))?;
            return Ok(EpubPrepareResult {
                source_path: source.to_string_lossy().to_string(),
                processed_path: out_path.to_string_lossy().to_string(),
                changed: true,
                action: "已自动解除伪加密".to_string(),
            });
        }
        return Ok(EpubPrepareResult {
            source_path: source.to_string_lossy().to_string(),
            processed_path: source.to_string_lossy().to_string(),
            changed: false,
            action: "无需处理".to_string(),
        });
    }

    // Case 2: try simple XOR obfuscation recovery.
    let bytes = fs::read(&source).map_err(|e| format!("读取 EPUB 失败: {}", e))?;
    let mut candidate_keys: Vec<u8> = vec![0xFF, 0xA5, 0x5A];
    if bytes.len() >= 4 {
        let k0 = bytes[0] ^ 0x50;
        if (bytes[1] ^ 0x4B) == k0 && (bytes[2] ^ 0x03) == k0 && (bytes[3] ^ 0x04) == k0 {
            if !candidate_keys.contains(&k0) {
                candidate_keys.insert(0, k0);
            }
        }
    }

    for key in candidate_keys {
        let mut decoded = xor_bytes(&bytes, key);
        let fixed_encryption_flag = clear_zip_encryption_flags(&mut decoded);
        if is_zip_archive_bytes(&decoded) {
            let suffix = if fixed_encryption_flag {
                "_deobf_decrypted"
            } else {
                "_deobf"
            };

            if let Some(out_path) =
                rebuild_epub_with_sanitized_names_from_bytes(&source, &decoded, suffix)?
            {
                return Ok(EpubPrepareResult {
                    source_path: source.to_string_lossy().to_string(),
                    processed_path: out_path.to_string_lossy().to_string(),
                    changed: true,
                    action: "已自动解混淆并修复可读格式".to_string(),
                });
            }

            let out_path = build_processed_epub_path(&source, suffix);
            fs::write(&out_path, &decoded).map_err(|e| format!("写入解混淆文件失败: {}", e))?;
            return Ok(EpubPrepareResult {
                source_path: source.to_string_lossy().to_string(),
                processed_path: out_path.to_string_lossy().to_string(),
                changed: true,
                action: "已自动解混淆并修复可读格式".to_string(),
            });
        }
    }

    Err("该 EPUB 可能经过非通用加密/混淆，当前版本暂无法自动处理".to_string())
}

#[tauri::command]
async fn save_text_file(path: String, content: String) -> Result<(), String> {
    let mut file = fs::File::create(&path).map_err(|e| format!("无法创建: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn read_binary_file(path: String) -> Result<Vec<u8>, String> {
    fs::read(&path).map_err(|e| format!("读取失败: {}", e))
}

#[tauri::command]
fn calculate_md5(content: String) -> String {
    format!("{:x}", md5::compute(content.as_bytes()))
}

#[tauri::command]
async fn save_history(original_path: String, content: String) -> Result<(), String> {
    let path = Path::new(&original_path);
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let history_dir = get_history_base_dir();
    if !history_dir.exists() {
        fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?;
    }
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let path_key = history_key_for_path(&original_path);
    let file_prefix = format!("{}-{}", file_stem, path_key);
    let backup_name = format!("{}.{}.bak", file_prefix, timestamp);
    let backup_path = history_dir.join(backup_name);
    let mut file = fs::File::create(&backup_path).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    if let Ok(entries) = fs::read_dir(&history_dir) {
        let mut backups: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                // Read hashed names first; keep compatibility with old naming.
                name.starts_with(&file_prefix)
                    || (name.starts_with(&format!("{}.", file_stem)) && name.ends_with(".bak"))
            })
            .collect();
        backups.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        if backups.len() > 10 {
            for entry in backups.iter().take(backups.len() - 10) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_history_list(original_path: String) -> Vec<HistoryMeta> {
    let path = Path::new(&original_path);
    let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let history_dir = get_history_base_dir();
    let file_prefix = format!("{}-{}", file_stem, history_key_for_path(&original_path));
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(history_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_string();
            if (fname.starts_with(&file_prefix)
                || fname.starts_with(&format!("{}.", file_stem)))
                && fname.ends_with(".bak")
            {
                if let Ok(meta) = entry.metadata() {
                    list.push(HistoryMeta {
                        filename: fname,
                        path: entry.path().to_string_lossy().to_string(),
                        timestamp: meta
                            .modified()
                            .unwrap_or(SystemTime::now())
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        size: meta.len(),
                        date_str: "".to_string(),
                    });
                }
            }
        }
    }
    list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    list
}

#[derive(serde::Deserialize, Clone)]
pub struct RegexRule {
    pub level: u8,
    pub pattern: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ChapterInfo {
    pub title: String,
    pub line_number: usize,
    pub level: u8,
    pub is_meta: bool,
    pub word_count: usize,
}

#[tauri::command]
async fn scan_chapters(
    content: String,
    rules: Vec<RegexRule>,
) -> Vec<ChapterInfo> {
    // Normalize line endings to ensure consistency with CodeMirror's line counting
    // CodeMirror treats \r, \n, and \r\n all as line separators
    // Rust's .lines() only recognizes \n and \r\n
    let content = content
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{2028}', "\n")
        .replace('\u{2029}', "\n");

    let mut chapters = Vec::new();
    
    // Compile regex rules safely
    let compiled_rules: Vec<(u8, Regex)> = rules
        .into_iter()
        .filter_map(|r| Regex::new(&r.pattern).ok().map(|re| (r.level, re)))
        .collect();

    let mut current_chapter: Option<ChapterInfo> = None;
    for (index, line) in content.lines().enumerate() {
        let line_trim = line.trim();
        let char_count = line_trim.chars().count();
        let is_empty = line_trim.is_empty();
        
        let mut match_level = None;
        if !is_empty {
            for (level, re) in &compiled_rules {
                if re.is_match(line).unwrap_or(false) {
                    match_level = Some(*level);
                    break;
                }
            }
        }

        if let Some(lvl) = match_level {
            if let Some(prev) = current_chapter.take() {
                chapters.push(prev);
            }
            // Auto detect meta to prevent folding chapters into introductions
            // But ensure Volumes (Level 1 containing 卷/部) are NOT treated as meta
            // And only auto-detect meta for Level 1 items (Chapters at Level 3 should not be meta)
            let is_vol_keyword = line_trim.contains("卷") || line_trim.contains("部");
            let is_meta = (lvl == 1) && !is_vol_keyword && (
                          line_trim.contains("简介") 
                       || line_trim.contains("前言") 
                       || line_trim.contains("序") 
                       || line_trim.contains("楔子") 
                       || line_trim.contains("后记") 
                       || line_trim.contains("感言")
                       || line_trim.contains("内容"));
            
            current_chapter = Some(ChapterInfo {
                title: line_trim.to_string(),
                line_number: index + 1,
                level: lvl,
                is_meta,
                word_count: 0,
            });
        } else {
            if let Some(ref mut chapter) = current_chapter {
                if !is_empty {
                    chapter.word_count += char_count;
                }
            }
        }
    }
    if let Some(last) = current_chapter {
        chapters.push(last);
    }
    chapters
}

#[tauri::command]
async fn advanced_search(content: String, pattern: String, is_regex: bool) -> SearchResult {
    if pattern.is_empty() {
        return SearchResult {
            found: false,
            count: 0,
            matches: vec![],
        };
    }
    let mut matches_vec = Vec::new();
    if is_regex {
        if let Ok(re) = Regex::new(&pattern) {
            for (i, line) in content.lines().enumerate() {
                for m in re.find_iter(line) {
                    if let Ok(match_obj) = m {
                        matches_vec.push(MatchLocation {
                            line: i + 1,
                            start_char: line[..match_obj.start()].chars().count(),
                            end_char: line[..match_obj.start()].chars().count()
                                + line[match_obj.start()..match_obj.end()].chars().count(),
                        });
                    }
                }
            }
        }
    } else {
        for (i, line) in content.lines().enumerate() {
            for (byte_idx, part) in line.match_indices(&pattern) {
                matches_vec.push(MatchLocation {
                    line: i + 1,
                    start_char: line[..byte_idx].chars().count(),
                    end_char: line[..byte_idx].chars().count() + part.chars().count(),
                });
            }
        }
    }
    let count = matches_vec.len();
    SearchResult {
        found: count > 0,
        count,
        matches: matches_vec,
    }
}

#[tauri::command]
async fn advanced_replace(
    content: String,
    pattern: String,
    replacement: String,
    is_regex: bool,
) -> Result<String, String> {
    if is_regex {
        let re = Regex::new(&pattern).map_err(|e| format!("Regex Error: {}", e))?;
        Ok(re.replace_all(&content, &replacement).to_string())
    } else {
        Ok(content.replace(&pattern, &replacement))
    }
}

// --- EPUB 导出 ---

#[tauri::command]
async fn export_epub(
    save_path: String,
    content: String,
    chapters: Vec<ChapterInfo>,
    metadata: EpubMetadata,
) -> Result<(), String> {
    let path = Path::new(&save_path);
    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let options_store = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("mimetype", options_store)
        .map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip")
        .map_err(|e| e.to_string())?;

    zip.start_file("META-INF/container.xml", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(
        r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
   <rootfiles>
      <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
   </rootfiles>
</container>"#
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;

    zip.start_file("OEBPS/Styles/font.css", options)
        .map_err(|e| e.to_string())?;
    let font_css = if metadata.font_css.trim().is_empty() {
        CSS_FONT
    } else {
        &metadata.font_css
    };
    zip.write_all(font_css.as_bytes())
        .map_err(|e| e.to_string())?;

    zip.start_file("OEBPS/Styles/main.css", options)
        .map_err(|e| e.to_string())?;
    let main_css = if metadata.main_css.trim().is_empty() {
        CSS_MAIN
    } else {
        &metadata.main_css
    };
    zip.write_all(main_css.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut has_cover = false;
    let mut cover_ext = "jpg".to_string();
    let normalized_cover_path = normalize_local_file_path(&metadata.cover_path);
    if !normalized_cover_path.trim().is_empty() {
        let img_bytes = fs::read(&normalized_cover_path)
            .map_err(|e| format!("读取封面失败: {} ({})", normalized_cover_path, e))?;
        cover_ext = Path::new(&normalized_cover_path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg")
            .to_lowercase();
        let cover_filename = format!("OEBPS/Images/cover.{}", cover_ext);
        zip.start_file(&cover_filename, options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&img_bytes).map_err(|e| e.to_string())?;
        has_cover = true;
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut manifest_items = String::new();
    let mut spine_refs = String::new();
    let mut ncx_navpoints = String::new();
    let mut play_order = 1;

    if has_cover {
        let mime = match cover_ext.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "jpg" | "jpeg" => "image/jpeg",
            _ => "image/jpeg",
        };
        manifest_items.push_str(&format!(r#"<item id="cover-image" href="Images/cover.{}" media-type="{}" properties="cover-image"/>"#, cover_ext, mime));
    }

    // 写入资产文件
    for (i, asset) in metadata.assets.iter().enumerate() {
        if let Ok(asset_bytes) = fs::read(&asset.path) {
            let sub_dir = match asset.category.as_str() {
                "fonts" => "Fonts",
                "images" => "Images",
                _ => "Other",
            };
            let asset_filename = format!("OEBPS/{}/{}", sub_dir, asset.name);
            zip.start_file(&asset_filename, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&asset_bytes).map_err(|e| e.to_string())?;

            let href = format!("{}/{}", sub_dir, asset.name);
            let mime = match Path::new(&asset.name).extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase().as_str() {
                "ttf" => "font/ttf",
                "otf" => "font/otf",
                "woff" => "font/woff",
                "woff2" => "font/woff2",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "gif" => "image/gif",
                "svg" => "image/svg+xml",
                "css" => "text/css",
                "js" => "text/javascript",
                _ => "application/octet-stream",
            };
            manifest_items.push_str(&format!(
                r#"<item id="asset_{}" href="{}" media-type="{}"/>"#,
                i, href, mime
            ));
        }
    }
    manifest_items
        .push_str(r#"<item id="font.css" href="Styles/font.css" media-type="text/css"/>"#);
    manifest_items
        .push_str(r#"<item id="main.css" href="Styles/main.css" media-type="text/css"/>"#);

    for (i, chapter) in chapters.iter().enumerate() {
        let file_name_in_zip = format!("OEBPS/Text/chapter{}.xhtml", i);
        let href_in_opf = format!("Text/chapter{}.xhtml", i);
        let id = format!("chapter{}", i);

        let start_line = chapter.line_number;
        let end_line = if i + 1 < chapters.len() {
            chapters[i + 1].line_number - 2 // line_number 指向标题的下一行，所以要减2
        } else {
            lines.len() - 1
        };
        let safe_end = end_line.min(lines.len() - 1);
        let safe_start = start_line.min(safe_end);
        let body_lines = if safe_start <= safe_end {
            // `line_number` already points to the first body line (next line after title),
            // so do not skip one more line here.
            &lines[safe_start..=safe_end] // 使用 ..= 包含 safe_end
        } else {
            &[]
        };

        let mut html_body = String::new();
        let mut class_attr = "";

        let (chap_num_raw, chap_name_raw) = split_title(&chapter.title);
        let safe_display_title = if !chap_num_raw.is_empty() && !chap_name_raw.is_empty() {
            format!(
                "{} {}",
                escape_xml(&chap_num_raw),
                escape_xml(&chap_name_raw)
            )
        } else {
            escape_xml(&chapter.title)
        };

        if chapter.is_meta {
            html_body.push_str(&format!(
                "  <h1 class=\"nrjj-title\">{}</h1>\n",
                safe_display_title
            ));
            for line in body_lines {
                let trim = line.trim();
                if !trim.is_empty() {
                    html_body.push_str(&format!("  <p>{}</p>\n", escape_xml(trim)));
                }
            }
        } else {
            match chapter.level {
                1 => {
                    class_attr = "Preface1";
                    let safe_vol_num = escape_xml(&chap_num_raw);
                    let safe_vol_name = escape_xml(&chap_name_raw);
                    
                    // We only use the vertical number styling if there's actually a volume number parsed
                    let vertical_num = if !safe_vol_num.is_empty() {
                        format_vertical_volume(&safe_vol_num)
                    } else {
                        String::new()
                    };
                    
                    let formatted_name = if !safe_vol_name.is_empty() {
                        safe_vol_name
                            .chars()
                            .map(|c| format!("{} ", c))
                            .collect::<String>()
                    } else {
                        safe_display_title.clone()
                            .chars()
                            .map(|c| format!("{} ", c))
                            .collect::<String>()
                    };

                    html_body.push_str(&format!(
                        "  <h1 class=\"PrefacehA1\" title=\"{}\"><br /><br />\n  {}</h1>\n  <p class=\"PrefacepA1\">{}</p>\n", 
                        safe_display_title, vertical_num, formatted_name.trim()
                    ));
                    
                    // Add body content for Volume if they exist, to prevent loss of potential inner-body text
                    for line in body_lines {
                        let trim = line.trim();
                        if !trim.is_empty() {
                            html_body.push_str(&format!("  <p>{}</p>\n", escape_xml(trim)));
                        }
                    }
                }
                3 => {
                    let safe_chap_num = escape_xml(&chap_num_raw);
                    let safe_chap_name = escape_xml(&chap_name_raw);
                    
                    if !safe_chap_num.is_empty() {
                        html_body.push_str(&format!(
                            "  <h3 class=\"head\"><span class=\"num\">{}</span><br/><b>{}</b></h3>\n",
                            safe_chap_num, safe_chap_name
                        ));
                    } else {
                        html_body.push_str(&format!(
                            "  <h3 class=\"head\">{}</h3>\n",
                            safe_display_title
                        ));
                    }
                    
                    for line in body_lines {
                        let trim = line.trim();
                        if !trim.is_empty() {
                            html_body.push_str(&format!("  <p>{}</p>\n", escape_xml(trim)));
                        }
                    }
                }
                _ => {
                    html_body.push_str(&format!(
                        "  <h{} class=\"head\">{}</h{}>\n",
                        chapter.level, safe_display_title, chapter.level
                    ));
                    for line in body_lines {
                        let trim = line.trim();
                        if !trim.is_empty() {
                            html_body.push_str(&format!("  <p>{}</p>\n", escape_xml(trim)));
                        }
                    }
                }
            }
        }

        let full_html = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>{}</title>
  <link href="../Styles/font.css" type="text/css" rel="stylesheet"/>
  <link href="../Styles/main.css" type="text/css" rel="stylesheet"/>
</head>
<body{}>
{}
</body>
</html>"#,
            safe_display_title,
            if class_attr.is_empty() {
                String::new()
            } else {
                format!(" class=\"{}\"", class_attr)
            },
            html_body
        );

        zip.start_file(&file_name_in_zip, options)
            .map_err(|e| e.to_string())?;
        zip.write_all(full_html.as_bytes())
            .map_err(|e| e.to_string())?;

        manifest_items.push_str(&format!(
            r#"<item id="{}" href="{}" media-type="application/xhtml+xml"/>"#,
            id, href_in_opf
        ));
        spine_refs.push_str(&format!(r#"<itemref idref="{}"/>"#, id));
    }

    let mut nav_stack_levels: Vec<u8> = Vec::new();

    for (i, chapter) in chapters.iter().enumerate() {
        let href_in_opf = format!("Text/chapter{}.xhtml", i);
        let current_level = chapter.level;
        let (chap_num_raw, chap_name_raw) = split_title(&chapter.title);
        let safe_display_title = if !chap_num_raw.is_empty() && !chap_name_raw.is_empty() {
            format!("{} {}", escape_xml(&chap_num_raw), escape_xml(&chap_name_raw))
        } else {
            escape_xml(&chapter.title)
        };

        while let Some(&top_level) = nav_stack_levels.last() {
            if top_level >= current_level || chapter.is_meta {
                ncx_navpoints.push_str("</navPoint>\n");
                nav_stack_levels.pop();
            } else {
                break;
            }
        }

        ncx_navpoints.push_str(&format!(
            r#"<navPoint id="navPoint-{}" playOrder="{}"><navLabel><text>{}</text></navLabel><content src="{}"/>"#,
            play_order, play_order, safe_display_title, href_in_opf
        ));
        ncx_navpoints.push('\n');

        if !chapter.is_meta {
            nav_stack_levels.push(current_level);
        } else {
            ncx_navpoints.push_str("</navPoint>\n");
        }
        play_order += 1;
    }

    while nav_stack_levels.pop().is_some() {
        ncx_navpoints.push_str("</navPoint>\n");
    }

    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let full_uuid = if metadata.uuid.starts_with("urn:uuid:") {
        metadata.uuid.clone()
    } else {
        format!("urn:uuid:{}", metadata.uuid)
    };

    let mut extra_metadata = String::new();
    for (k, v) in &metadata.extra {
        extra_metadata.push_str(&format!(
            "    <dc:{} pub-type=\"zdy\">{}</dc:{}>\n",
            escape_xml(k),
            escape_xml(v),
            escape_xml(k)
        ));
    }

    let opf_content = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title id="t1">{}</dc:title>
    <dc:creator id="creator">{}</dc:creator>
    <dc:date>{}</dc:date>
    <dc:publisher>{}</dc:publisher>
    <dc:identifier opf:scheme="UUID" id="BookId">{}</dc:identifier>
    <dc:description>{}</dc:description>
    <meta name="cover" content="cover-image" />
    <meta property="reamicro:md5" content="{}" />
{}  </metadata>
  <manifest>
    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
    {}
  </manifest>
  <spine toc="ncx">
    {}
  </spine>
</package>"#,
        escape_xml(&metadata.title),
        escape_xml(&metadata.creator),
        date_str,
        escape_xml(&metadata.publisher),
        full_uuid,
        escape_xml(&metadata.description),
        metadata.md5,
        extra_metadata,
        manifest_items,
        spine_refs
    );

    zip.start_file("OEBPS/content.opf", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(opf_content.as_bytes())
        .map_err(|e| e.to_string())?;

    let ncx_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE ncx PUBLIC "-//NISO//DTD ncx 2005-1//EN" "http://www.daisy.org/z3986/2005/ncx-2005-1.dtd">
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="{}"/>
    <meta name="dtb:depth" content="2"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>{}</text></docTitle>
  <navMap>
    {}
  </navMap>
</ncx>"#,
        full_uuid,
        escape_xml(&metadata.title),
        ncx_navpoints
    );

    zip.start_file("OEBPS/toc.ncx", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(ncx_content.as_bytes())
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

// --- EPUB 编辑器相关命令 ---

#[tauri::command]
async fn extract_epub(epub_path: String) -> Result<Vec<EpubFileNode>, String> {
    // 1. 检查是否已经有缓存且临时内容未关闭
    let existing_temp_path = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    let temp_path_buf: PathBuf;
    let mut _temp_dir_keep: Option<TempDir> = None;

    if let Some(path) = existing_temp_path {
        temp_path_buf = path;
    } else {
        // 2. 创建临时目录并解压
        let temp_dir = TempDir::new().map_err(|e| format!("无法创建临时目录: {}", e))?;
        temp_path_buf = temp_dir.path().to_path_buf();

        {
            let file = fs::File::open(&epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| format!("无效的 EPUB 文件: {}", e))?;
            archive
                .extract(&temp_path_buf)
                .map_err(|e| format!("解压失败: {}", e))?;
        }
        _temp_dir_keep = Some(temp_dir);
    }

    // 3. 遍历目录构建文件列表
    let mut all_files = Vec::new();

    for entry in WalkDir::new(&temp_path_buf)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let full_path = entry.path();
        let relative_path = full_path.strip_prefix(&temp_path_buf).unwrap();
        let path_str = relative_path.to_string_lossy().replace("\\", "/");

        // Hiding system files/folders as requested
        if path_str.starts_with("META-INF") || path_str == "mimetype" {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let file_name = relative_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // 确定文件类型
        let file_type = if file_name.ends_with(".html") || file_name.ends_with(".xhtml") {
            "html"
        } else if file_name.ends_with(".css") {
            "css"
        } else if file_name.ends_with(".xml")
            || file_name.ends_with(".opf")
            || file_name.ends_with(".ncx")
        {
            "xml"
        } else if file_name.ends_with(".jpg")
            || file_name.ends_with(".jpeg")
            || file_name.ends_with(".png")
        {
            "image"
        } else if file_name.ends_with(".ttf") || file_name.ends_with(".otf") {
            "font"
        } else {
            "other"
        }
        .to_string();

        // 提取标题 (如果是 HTML)
        let title = None;
        let mut resolution = None;

        if file_type == "image" {
            // 尝试获取图片分辨率
            if let Ok((width, height)) = image::image_dimensions(full_path) {
                resolution = Some(format!("{}x{}", width, height));
            }
        }

        all_files.push((path_str, file_name, file_type, size, title, resolution));
    }

    // 构建嵌套文件树
    fn build_tree(
        files: &[(String, String, String, u64, Option<String>, Option<String>)],
    ) -> Vec<EpubFileNode> {
        let mut root_map: HashMap<String, Vec<EpubFileNode>> = HashMap::new();

        for (full_path, file_name, file_type, size, title, res) in files {
            let parts: Vec<&str> = full_path.split('/').collect();

            if parts.len() == 1 {
                // 根目录文件
                root_map
                    .entry("__root__".to_string())
                    .or_insert_with(Vec::new)
                    .push(EpubFileNode {
                        name: file_name.clone(),
                        path: full_path.clone(),
                        file_type: file_type.clone(),
                        size: Some(*size),
                        title: title.clone(),
                        resolution: res.clone(),
                        children: None,
                    });
            } else {
                // 有目录的文件
                let dir_key = parts[0].to_string();
                root_map
                    .entry(dir_key)
                    .or_insert_with(Vec::new)
                    .push(EpubFileNode {
                        name: file_name.clone(),
                        path: full_path.clone(),
                        file_type: file_type.clone(),
                        size: Some(*size),
                        title: title.clone(),
                        resolution: res.clone(),
                        children: None,
                    });
            }
        }

        let mut result = Vec::new();

        // 处理根目录文件
        if let Some(root_files) = root_map.remove("__root__") {
            result.extend(root_files);
        }

        // 处理文件夹
        let mut sorted_dirs: Vec<_> = root_map.into_iter().collect();
        sorted_dirs.sort_by(|a, b| a.0.cmp(&b.0));

        for (dir_name, files) in sorted_dirs {
            // 按路径深度分组子文件夹
            let mut subdir_map: HashMap<String, Vec<EpubFileNode>> = HashMap::new();
            let mut dir_files = Vec::new();

            for file in files {
                let path_parts: Vec<&str> = file.path.split('/').collect();
                if path_parts.len() == 2 {
                    // 直接在当前目录下的文件
                    dir_files.push(file);
                } else if path_parts.len() > 2 {
                    // 子目录中的文件
                    let subdir = path_parts[1].to_string();
                    subdir_map.entry(subdir).or_insert_with(Vec::new).push(file);
                }
            }

            // Sort direct files
            dir_files.sort_by(|a, b| a.name.cmp(&b.name));

            // 创建子文件夹节点
            let mut children = dir_files;

            // Sort keys to ensure folders order
            let mut subdir_names: Vec<_> = subdir_map.keys().cloned().collect();
            subdir_names.sort();

            for subdir_name in subdir_names {
                if let Some(mut subdir_files) = subdir_map.remove(&subdir_name) {
                    // Sort files inside subfolder
                    subdir_files.sort_by(|a, b| a.name.cmp(&b.name));

                    children.push(EpubFileNode {
                        name: subdir_name.clone(),
                        path: format!("{}/{}", dir_name, subdir_name),
                        file_type: "folder".to_string(),
                        size: None,
                        title: None,
                        resolution: None,
                        children: Some(subdir_files),
                    });
                }
            }

            result.push(EpubFileNode {
                name: dir_name.clone(),
                path: dir_name,
                file_type: "folder".to_string(),
                size: None,
                title: None,
                resolution: None,
                children: Some(children),
            });
        }

        result
    }

    // 初始化全局缓存 (仅在新建时)
    if _temp_dir_keep.is_some() {
        let mut cache_guard = EPUB_CACHE.lock().unwrap();
        let mut cache = EpubCache::new(epub_path);
        cache.temp_dir = _temp_dir_keep;
        *cache_guard = Some(cache);
    }

    Ok(build_tree(&all_files))
}

#[tauri::command]
async fn read_epub_file_content(epub_path: String, file_path: String) -> Result<String, String> {
    // 1. 获取临时目录路径
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    // 2. 从临时文件读取
    let target_path = temp_path.join(&file_path);
    std::fs::read_to_string(target_path).map_err(|e| format!("读取文件失败: {}", e))
}

#[tauri::command]
async fn read_epub_file_binary(epub_path: String, file_path: String) -> Result<Vec<u8>, String> {
    // 1. 获取临时目录路径
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    // 2. 从临时文件读取
    let target_path = temp_path.join(&file_path);
    std::fs::read(target_path).map_err(|e| format!("读取文件失败: {}", e))
}

// --- 批量读取 API ---

#[tauri::command]
async fn read_epub_files_batch(
    epub_path: String,
    file_paths: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    use std::io::Read;
    use zip::ZipArchive;

    let mut results: HashMap<String, String> = HashMap::new();
    let mut to_read: Vec<String> = Vec::new();

    // 1. 检查缓存，收集需要读取的文件
    {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                for path in &file_paths {
                    if let Some(content) = cache.text_cache.get(path) {
                        results.insert(path.clone(), content.clone());
                    } else {
                        to_read.push(path.clone());
                    }
                }
            } else {
                to_read = file_paths.clone();
            }
        } else {
            to_read = file_paths.clone();
        }
    }

    // 2. 批量读取未缓存的文件
    if !to_read.is_empty() {
        let file = fs::File::open(&epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
        let mut archive = ZipArchive::new(file).map_err(|e| format!("无效的 EPUB 文件: {}", e))?;

        let mut new_contents: Vec<(String, String)> = Vec::new();

        for path in to_read {
            if let Ok(mut zip_file) = archive.by_name(&path) {
                let mut content = String::new();
                if zip_file.read_to_string(&mut content).is_ok() {
                    results.insert(path.clone(), content.clone());
                    new_contents.push((path, content));
                }
            }
        }

        // 3. 存入缓存
        {
            let mut cache_guard = EPUB_CACHE.lock().unwrap();
            if let Some(ref mut cache) = *cache_guard {
                if cache.epub_path == epub_path {
                    for (path, content) in new_contents {
                        cache.text_cache.insert(path, content);
                    }
                }
            }
        }
    }

    Ok(results)
}

#[tauri::command]
async fn read_epub_binary_batch(
    epub_path: String,
    file_paths: Vec<String>,
) -> Result<HashMap<String, Vec<u8>>, String> {
    use std::io::Read;
    use zip::ZipArchive;

    let mut results: HashMap<String, Vec<u8>> = HashMap::new();
    let mut to_read: Vec<String> = Vec::new();

    // 1. 检查缓存
    {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                for path in &file_paths {
                    if let Some(data) = cache.binary_cache.get(path) {
                        results.insert(path.clone(), data.clone());
                    } else {
                        to_read.push(path.clone());
                    }
                }
            } else {
                to_read = file_paths.clone();
            }
        } else {
            to_read = file_paths.clone();
        }
    }

    // 2. 批量读取
    if !to_read.is_empty() {
        let file = fs::File::open(&epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
        let mut archive = ZipArchive::new(file).map_err(|e| format!("无效的 EPUB 文件: {}", e))?;

        let mut new_data: Vec<(String, Vec<u8>)> = Vec::new();

        for path in to_read {
            if let Ok(mut zip_file) = archive.by_name(&path) {
                let mut buffer = Vec::new();
                if zip_file.read_to_end(&mut buffer).is_ok() {
                    results.insert(path.clone(), buffer.clone());
                    new_data.push((path, buffer));
                }
            }
        }

        // 3. 存入缓存
        {
            let mut cache_guard = EPUB_CACHE.lock().unwrap();
            if let Some(ref mut cache) = *cache_guard {
                if cache.epub_path == epub_path {
                    for (path, data) in new_data {
                        cache.binary_cache.insert(path, data);
                    }
                }
            }
        }
    }

    Ok(results)
}

// --- EPUB 文件保存命令 ---

#[tauri::command]
async fn save_epub_file_content(
    epub_path: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    // 1. 获取临时目录路径
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or_else(|| "EPUB 未加载或缓存失效".to_string())?;

    // 2. 写入临时文件
    let target_path = temp_path.join(&file_path);
    std::fs::write(target_path, &content).map_err(|e| format!("写入文件失败: {}", e))?;

    // 3. 更新缓存 (Text Cache)
    {
        let mut cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref mut cache) = *cache_guard {
            if cache.epub_path == epub_path {
                cache.text_cache.insert(file_path, content);
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn save_epub_file_binary(
    epub_path: String,
    file_path: String,
    content: Vec<u8>,
) -> Result<(), String> {
    // 1. 获取临时目录路径
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or_else(|| "EPUB 未加载或缓存失效".to_string())?;

    // 2. 写入二进制文件
    let target_path = temp_path.join(&file_path);
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建目录: {}", e))?;
    }
    std::fs::write(target_path, content).map_err(|e| format!("写入文件失败: {}", e))?;

    // 3. 更新二进制缓存 (Binary Cache)
    {
        let mut cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref mut cache) = *cache_guard {
            if cache.epub_path == epub_path {
                cache.binary_cache.insert(file_path, Default::default());
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn save_epub_files_batch(
    epub_path: String,
    files: HashMap<String, Vec<u8>>,
) -> Result<(), String> {
    // 1. Get Temp Path
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or_else(|| "EPUB 未加载或缓存失效".to_string())?;

    // 2. Iterate and Write
    for (file_path, content) in files {
        let target_path = temp_path.join(&file_path);
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("无法创建目录: {}", e))?;
        }
        std::fs::write(target_path, &content)
            .map_err(|e| format!("写入文件失败 {}: {}", file_path, e))?;

        // Update Cache (Binary or Text doesn't matter for storage, but for cache structure)
        // Since we don't know if it's text, we might clear both entries or just skip cache update for now?
        // Actually, we should probably update cache if it exists.
        // For simplicity and speed in batch mode, let's just invalidate the specific cache entries if they exist.
        {
            let mut cache_guard = EPUB_CACHE.lock().unwrap();
            if let Some(ref mut cache) = *cache_guard {
                if cache.epub_path == epub_path {
                    // If it was text, update with valid utf8?
                    // Risk of decoding binary as text.
                    // Safer to remove from text_cache and let it re-read from disk on next access.
                    cache.text_cache.remove(&file_path);
                    cache.binary_cache.remove(&file_path);
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn save_epub_to_disk(epub_path: String) -> Result<(), String> {
    use zip::write::FileOptions;

    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    // 2. 创建临时 ZIP 文件
    let zip_file_path = format!("{}.zip.tmp", epub_path);
    let zip_file = fs::File::create(&zip_file_path).map_err(|e| format!("创建ZIP失败: {}", e))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);

    let options_deflated =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let options_stored = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // 3. 遍历临时目录并写入 ZIP
    for entry in WalkDir::new(&temp_path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let full_path = entry.path();
        let relative_path = full_path.strip_prefix(&temp_path).unwrap();
        let path_str = relative_path.to_string_lossy().replace("\\", "/");

        let options = if path_str == "mimetype" {
            options_stored
        } else {
            options_deflated
        };

        zip_writer
            .start_file(&path_str, options)
            .map_err(|e| format!("写入文件失败: {}", e))?;

        // Read file content
        let content = fs::read(full_path).map_err(|e| format!("读取文件失败: {}", e))?;
        zip_writer
            .write_all(&content)
            .map_err(|e| format!("写入内容失败: {}", e))?;
    }

    zip_writer
        .finish()
        .map_err(|e| format!("完成 ZIP 失败: {}", e))?;

    // 4. replace original file
    fs::rename(&zip_file_path, &epub_path).map_err(|e| format!("替换文件失败: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn search_in_files(
    epub_path: String,
    files: Vec<String>,
    pattern: String,
    is_regex: bool,
) -> Result<usize, String> {
    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    if files.is_empty() {
        return Ok(0);
    }

    let mut count = 0;

    // Pre-compile regex
    let re = if is_regex {
        Some(Regex::new(&pattern).map_err(|e| format!("正则表达式错误: {}", e))?)
    } else {
        None
    };

    for path in files {
        let target_path = temp_path.join(path);
        if let Ok(content) = fs::read_to_string(target_path) {
            if let Some(ref regex) = re {
                count += regex.find_iter(&content).count();
            } else {
                count += content.matches(&pattern).count();
            }
        }
    }

    Ok(count)
}

#[tauri::command]
async fn add_epub_file(
    epub_path: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    add_epub_file_binary(epub_path, file_path, content.into_bytes()).await
}

#[tauri::command]
async fn add_epub_file_binary(
    epub_path: String,
    file_path: String,
    content: Vec<u8>,
) -> Result<(), String> {
    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    // 2. 写入文件
    let target_path = temp_path.join(&file_path);
    // 确保父目录存在
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    std::fs::write(target_path, content).map_err(|e| format!("写入文件失败: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn delete_epub_file(epub_path: String, file_path: String) -> Result<(), String> {
    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    // 2. 删除文件
    let target_path = temp_path.join(&file_path);
    if target_path.exists() {
        if target_path.is_dir() {
            std::fs::remove_dir_all(target_path).map_err(|e| format!("删除目录失败: {}", e))?;
        } else {
            std::fs::remove_file(target_path).map_err(|e| format!("删除文件失败: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
async fn rename_epub_file(
    epub_path: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    .ok_or("EPUB 未加载或缓存失效".to_string())?;

    // 2. 重命名
    let old_target = temp_path.join(&old_path);
    let new_target = temp_path.join(&new_path);

    // 确保新路径的父目录存在
    if let Some(parent) = new_target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }

    std::fs::rename(old_target, new_target).map_err(|e| format!("重命名失败: {}", e))?;

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
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(cache) = cache_guard.as_ref() {
            if cache.epub_path == epub_path {
                if let Some(data) = cache.binary_cache.get(file_path) {
                    return Ok(data.clone());
                }
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
        let mut cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(cache) = cache_guard.as_mut() {
            if cache.epub_path == epub_path {
                cache
                    .binary_cache
                    .insert(file_path.to_string(), buffer.clone());
            }
        }
    }

    Ok(buffer)
}

#[tauri::command]
async fn analyze_epub_font_glyphs(
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

#[tauri::command]
fn get_launch_args() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    // Index 0 is the executable path
    // Index 1 is usually the file path for file associations on Windows/Linux
    if args.len() > 1 {
        // Filter out common Tauri debug flags if necessary, though simple direct file opening usually puts file at index 1
        return Some(args[1].clone());
    }
    None
}

// ============================================================
// ===== Library (书库) - Phase 1: 数据结构 / load / save =====
// ============================================================

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct LibraryConfig {
    #[serde(default)]
    storage_mode: String,
    #[serde(default)]
    custom_work_dir: String,
    /// 编辑元数据保存时是否把 epub 内的 dcterms:modified 改成"现在"。
    /// 默认 false：保存元数据不改修改日期。
    #[serde(default)]
    update_modified_on_edit: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct BookEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    file_path: String,
    #[serde(default)]
    file_type: String,
    #[serde(default)]
    cover_path: String,
    #[serde(default)]
    added_at: u64,
    #[serde(default)]
    file_size: u64,
    #[serde(default)]
    subtitle: String,
    #[serde(default)]
    filename: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    modified_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default)]
    epub_uuid: String,
    #[serde(default)]
    maker: String,
    #[serde(default)]
    series: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct LibraryData {
    #[serde(default)]
    config: LibraryConfig,
    #[serde(default)]
    books: Vec<BookEntry>,
}

// 默认根：%APPDATA%/<bundle-identifier>（即 Tauri app_data_dir）。
fn library_app_data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .map_err(|e| format!("无法获取 app_data_dir: {}", e))
}

// 指针文件：永远住在 app_data_dir，记录当前书库的真实根目录。
fn library_pointer_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_app_data_root(app)?.join("library.pointer"))
}

// 读指针：有则按指针走（便携版指向 exe_dir 等），无则回退 app_data_dir。
fn library_root_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let pointer = library_pointer_path(app)?;
    if pointer.exists() {
        if let Ok(s) = fs::read_to_string(&pointer) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                let p = PathBuf::from(trimmed);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    library_app_data_root(app)
}

fn library_json_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("library.json"))
}

fn library_covers_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("covers"))
}

fn library_files_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("files"))
}

fn ensure_dir(p: &Path) -> Result<(), String> {
    if !p.exists() {
        fs::create_dir_all(p).map_err(|e| format!("创建目录失败 {}: {}", p.display(), e))?;
    }
    Ok(())
}

fn read_library_data(app: &tauri::AppHandle) -> Result<LibraryData, String> {
    let path = library_json_path(app)?;
    if !path.exists() {
        return Ok(LibraryData::default());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取 library.json 失败: {}", e))?;
    let data: LibraryData =
        serde_json::from_slice(&bytes).map_err(|e| format!("解析 library.json 失败: {}", e))?;
    Ok(data)
}

// 写入策略：customWorkDir 非空则写到那里，否则写到 app_data_dir；
// 同时把真实根目录写进 app_data_dir/library.pointer 给下次冷启动用。
fn write_library_data_atomic(app: &tauri::AppHandle, data: &LibraryData) -> Result<(), String> {
    let target_root = if !data.config.custom_work_dir.trim().is_empty() {
        PathBuf::from(data.config.custom_work_dir.trim())
    } else {
        library_app_data_root(app)?
    };
    ensure_dir(&target_root)?;

    let target = target_root.join("library.json");
    let tmp = target.with_extension("json.tmp");
    let json =
        serde_json::to_vec_pretty(data).map_err(|e| format!("序列化 library.json 失败: {}", e))?;
    {
        let mut f =
            fs::File::create(&tmp).map_err(|e| format!("创建临时文件失败: {}", e))?;
        f.write_all(&json)
            .map_err(|e| format!("写入临时文件失败: {}", e))?;
        f.sync_all().ok();
    }
    // Windows 下 rename 会原子替换已有文件
    if target.exists() {
        let _ = fs::remove_file(&target);
    }
    fs::rename(&tmp, &target).map_err(|e| format!("替换 library.json 失败: {}", e))?;

    // 更新指针
    let app_root = library_app_data_root(app)?;
    ensure_dir(&app_root)?;
    let pointer = app_root.join("library.pointer");
    let _ = fs::write(&pointer, target_root.to_string_lossy().as_bytes());

    Ok(())
}

#[tauri::command]
async fn load_library(app: tauri::AppHandle) -> Result<LibraryData, String> {
    read_library_data(&app)
}

#[tauri::command]
async fn save_library(app: tauri::AppHandle, data: LibraryData) -> Result<(), String> {
    write_library_data_atomic(&app, &data)
}

// ============================================================
// ===== Library - Phase 2: add / remove =====
// ============================================================

struct EpubParsedMeta {
    title: String,
    author: String,
    publisher: Option<String>,
    description: Option<String>,
    epub_uuid: String,
    cover_bytes: Option<Vec<u8>>,
    cover_ext: String, // "jpg" / "png"
    pub_date: Option<u64>,      // <dc:date> 解析后的 unix 秒
    modified_date: Option<u64>, // <meta property="dcterms:modified"> 解析后的 unix 秒
    // 我们写入 OPF 时用的扩展字段，导入时也要读回来
    subtitle: Option<String>,   // <meta name="calibre:subtitle" content="..."/>
    series: Option<String>,     // <meta name="calibre:series" content="..."/>
    maker: Option<String>,      // <meta name="maker" content="..."/>
    tags: Vec<String>,          // 所有 <dc:subject>
}

// fancy_regex 没有 escape 工具，自己写一个最简版
fn re_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '.' | '+' | '*' | '?' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

// 标准化 zip 内部路径（去掉 ./ 与 ../，统一正斜杠）
fn normalize_zip_path(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

// 从 OPF XML 里抽出第一个 <{tag}>...</{tag}> 的内容（容忍属性、容忍换行）
fn extract_first_tag(xml: &str, tag: &str) -> Option<String> {
    let pat = format!(
        r#"<{0}(?:\s[^>]*)?>([\s\S]*?)</{0}>"#,
        re_escape(tag)
    );
    let re = Regex::new(&pat).ok()?;
    match re.captures(xml) {
        Ok(Some(c)) => c.get(1).map(|m| xml_unescape(m.as_str().trim())),
        _ => None,
    }
}

// 抽取所有 <{tag}>...</{tag}> 的内容，返回 Vec（用于多个 dc:subject 这类）
fn extract_all_tags(xml: &str, tag: &str) -> Vec<String> {
    let pat = format!(
        r#"<{0}(?:\s[^>]*)?>([\s\S]*?)</{0}>"#,
        re_escape(tag)
    );
    let re = match Regex::new(&pat) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for c in re.captures_iter(xml).flatten() {
        if let Some(m) = c.get(1) {
            let s = xml_unescape(m.as_str().trim());
            if !s.is_empty() {
                out.push(s);
            }
        }
    }
    out
}

// 抽取 <meta name="X" content="Y"/> 里的 Y（属性顺序无关，先定位 tag 再抽 content）
fn extract_meta_by_name(xml: &str, name: &str) -> Option<String> {
    let outer_pat = format!(
        r#"<meta\s+[^>]*name="{}"[^>]*/?>"#,
        re_escape(name)
    );
    let outer_re = Regex::new(&outer_pat).ok()?;
    let m = match outer_re.find(xml) {
        Ok(Some(m)) => m,
        _ => return None,
    };
    let inner = m.as_str();
    let content_re = Regex::new(r#"content="([^"]*)""#).ok()?;
    match content_re.captures(inner) {
        Ok(Some(c)) => c
            .get(1)
            .map(|m| xml_unescape(m.as_str().trim()))
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

// XML 反转义（对应 xml_escape）
fn xml_unescape(s: &str) -> String {
    s.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

// 从路径名猜图片扩展（用于落盘和前端 MIME 推断）
fn ext_from_path(p: &str) -> &'static str {
    let lower = p.to_lowercase();
    if lower.ends_with(".png") {
        "png"
    } else if lower.ends_with(".webp") {
        "webp"
    } else if lower.ends_with(".gif") {
        "gif"
    } else if lower.ends_with(".jpeg") || lower.ends_with(".jpg") {
        "jpg"
    } else {
        "jpg"
    }
}

// 在 OPF 中定位封面 href，三层兜底
fn find_cover_href(opf_xml: &str) -> Option<String> {
    let href_re = Regex::new(r#"href="([^"]+)""#).ok()?;

    // 1. EPUB 3：<item ... properties="cover-image" ...>
    if let Ok(prop_re) = Regex::new(r#"<item\s+[^>]*properties="cover-image"[^>]*/?>"#) {
        if let Ok(Some(m)) = prop_re.find(opf_xml) {
            if let Ok(Some(c)) = href_re.captures(m.as_str()) {
                if let Some(href) = c.get(1) {
                    return Some(href.as_str().to_string());
                }
            }
        }
    }

    // 2. EPUB 2：<meta name="cover" content="ID"/>（任意属性顺序），manifest 里 id="ID" 的 item
    if let Some(cover_id) = extract_meta_by_name(opf_xml, "cover") {
        if let Ok(id_re) = Regex::new(&format!(
            r#"<item\s+[^>]*id="{}"[^>]*/?>"#,
            re_escape(&cover_id)
        )) {
            if let Ok(Some(m)) = id_re.find(opf_xml) {
                if let Ok(Some(c)) = href_re.captures(m.as_str()) {
                    if let Some(href) = c.get(1) {
                        return Some(href.as_str().to_string());
                    }
                }
            }
        }
    }

    // 3. 兜底：扫 manifest 找像封面的 image 项（href 含 "cover." 且 media-type 是 image/*）
    if let Ok(item_re) = Regex::new(r#"<item\s+[^>]*/?>"#) {
        for m in item_re.find_iter(opf_xml).flatten() {
            let s = m.as_str();
            let lower = s.to_lowercase();
            if !lower.contains("media-type=\"image/") {
                continue;
            }
            if !(lower.contains(r#"id="cover""#) || lower.contains("cover.")) {
                continue;
            }
            if let Ok(Some(c)) = href_re.captures(s) {
                if let Some(href) = c.get(1) {
                    return Some(href.as_str().to_string());
                }
            }
        }
    }

    None
}

// 从 zip 里读一个 entry 的字节；找不到精确名时尝试 percent-decode 与忽略大小写
fn try_read_zip_entry(
    archive: &mut zip::ZipArchive<fs::File>,
    name: &str,
) -> Option<Vec<u8>> {
    fn try_one(a: &mut zip::ZipArchive<fs::File>, n: &str) -> Option<Vec<u8>> {
        let mut zf = a.by_name(n).ok()?;
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    if let Some(b) = try_one(archive, name) {
        return Some(b);
    }
    let decoded = percent_decode(name);
    if decoded != name {
        if let Some(b) = try_one(archive, &decoded) {
            return Some(b);
        }
    }
    // 大小写不敏感扫描
    let target = decoded.to_lowercase();
    let all_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();
    for n in all_names {
        if n.to_lowercase() == target {
            if let Some(b) = try_one(archive, &n) {
                return Some(b);
            }
        }
    }
    None
}

// EPUB 中 dc:date 与 dcterms:modified 的字符串通常是 ISO 8601。容忍年/年月/年月日/带时区。
fn parse_epub_date(s: &str) -> Option<u64> {
    use chrono::NaiveDate;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // 完整 RFC3339（带时区）
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        let ts = dt.timestamp();
        return if ts >= 0 { Some(ts as u64) } else { None };
    }
    // 不带 Z 的本地日期时间（按 UTC 解释）
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        let ts = dt.and_utc().timestamp();
        return if ts >= 0 { Some(ts as u64) } else { None };
    }
    // 仅日期 YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc().timestamp())
            .filter(|ts| *ts >= 0)
            .map(|ts| ts as u64);
    }
    // YYYY-MM
    if s.len() == 7 {
        if let Ok(d) = NaiveDate::parse_from_str(&format!("{}-01", s), "%Y-%m-%d") {
            return d
                .and_hms_opt(0, 0, 0)
                .map(|dt| dt.and_utc().timestamp())
                .filter(|ts| *ts >= 0)
                .map(|ts| ts as u64);
        }
    }
    // YYYY
    if s.len() == 4 {
        if let Ok(d) = NaiveDate::parse_from_str(&format!("{}-01-01", s), "%Y-%m-%d") {
            return d
                .and_hms_opt(0, 0, 0)
                .map(|dt| dt.and_utc().timestamp())
                .filter(|ts| *ts >= 0)
                .map(|ts| ts as u64);
        }
    }
    None
}

fn parse_epub_metadata(epub_path: &Path) -> Result<EpubParsedMeta, String> {
    let file = fs::File::open(epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("无效的 EPUB: {}", e))?;

    // 1. container.xml -> OPF 路径
    let container_xml = {
        let mut f = archive
            .by_name("META-INF/container.xml")
            .map_err(|_| "缺少 META-INF/container.xml".to_string())?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };
    let opf_re = Regex::new(r#"full-path="([^"]+)""#).map_err(|e| e.to_string())?;
    let opf_path = match opf_re.captures(&container_xml) {
        Ok(Some(c)) => c.get(1).map(|m| m.as_str().to_string()),
        _ => None,
    }
    .ok_or_else(|| "container.xml 缺少 full-path".to_string())?;

    // 2. 读 OPF
    let opf_xml = {
        let mut f = archive
            .by_name(&opf_path)
            .map_err(|_| format!("缺少 OPF 文件: {}", opf_path))?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };

    let title = extract_first_tag(&opf_xml, "dc:title").unwrap_or_default();
    let author = extract_first_tag(&opf_xml, "dc:creator").unwrap_or_default();
    let publisher = extract_first_tag(&opf_xml, "dc:publisher");
    let description = extract_first_tag(&opf_xml, "dc:description");
    let epub_uuid = extract_first_tag(&opf_xml, "dc:identifier").unwrap_or_default();

    // 我们的扩展字段（与 write_opf_metadata 写入约定一一对应）
    let subtitle = extract_meta_by_name(&opf_xml, "calibre:subtitle");
    let series = extract_meta_by_name(&opf_xml, "calibre:series");
    let maker = extract_meta_by_name(&opf_xml, "maker");
    let tags = extract_all_tags(&opf_xml, "dc:subject");

    // dc:date 作为"制作时间"（出版/创建日期）
    let pub_date =
        extract_first_tag(&opf_xml, "dc:date").and_then(|s| parse_epub_date(&s));

    // EPUB 3：<meta property="dcterms:modified">YYYY-MM-DDTHH:MM:SSZ</meta>
    let modified_date = match Regex::new(
        r#"<meta\s+[^>]*property="dcterms:modified"[^>]*>([\s\S]*?)</meta>"#,
    ) {
        Ok(re) => match re.captures(&opf_xml) {
            Ok(Some(c)) => c.get(1).and_then(|m| parse_epub_date(m.as_str().trim())),
            _ => None,
        },
        Err(_) => None,
    };

    // 3. 找封面 href（多层兜底）
    let cover_href = find_cover_href(&opf_xml);

    // 4. 抽封面字节（容忍 URL 编码 / 大小写差异）
    let mut cover_bytes: Option<Vec<u8>> = None;
    let mut cover_ext = String::from("jpg");
    if let Some(href) = cover_href {
        // href 相对 OPF 所在目录
        let opf_dir = Path::new(&opf_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let cover_full = if opf_dir.is_empty() {
            href.clone()
        } else {
            format!("{}/{}", opf_dir, href)
        };
        let cover_full = normalize_zip_path(&cover_full);
        if let Some(buf) = try_read_zip_entry(&mut archive, &cover_full) {
            // 优先按路径名定扩展（用于前端 MIME），用 magic bytes 兜底
            cover_ext = match ext_from_path(&cover_full) {
                "jpg" => detect_image_ext(&buf).to_string(),
                other => other.to_string(),
            };
            cover_bytes = Some(buf);
        }
    }

    Ok(EpubParsedMeta {
        title,
        author,
        publisher,
        description,
        epub_uuid,
        cover_bytes,
        cover_ext,
        pub_date,
        modified_date,
        subtitle,
        series,
        maker,
        tags,
    })
}

fn detect_book_file_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .as_deref()
    {
        Some("epub") => "epub",
        Some("txt") => "txt",
        _ => "",
    }
}

fn system_time_to_secs(t: SystemTime) -> Option<u64> {
    t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

#[tauri::command]
async fn add_book_to_library(
    app: tauri::AppHandle,
    file_path: String,
    config: LibraryConfig,
) -> Result<BookEntry, String> {
    let src = PathBuf::from(&file_path);
    if !src.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    let file_type = detect_book_file_type(&src);
    if file_type.is_empty() {
        return Err("仅支持 .epub / .txt 文件".to_string());
    }

    // 读已有 library，把传入的 config 作为权威值同步进去
    let mut data = read_library_data(&app)?;
    data.config = config.clone();

    // 校验自定义模式
    let storage_mode = config.storage_mode.as_str();
    if storage_mode == "copy_custom" && config.custom_work_dir.trim().is_empty() {
        return Err("自定义存储模式下必须先在设置中指定工作目录".to_string());
    }
    let should_copy = matches!(storage_mode, "copy_portable" | "copy_custom");

    let id = uuid::Uuid::new_v4().to_string();
    let now = system_time_to_secs(SystemTime::now()).unwrap_or(0);
    let metadata = fs::metadata(&src).map_err(|e| format!("读取文件元数据失败: {}", e))?;
    let file_size = metadata.len();
    let created_at = metadata.created().ok().and_then(system_time_to_secs);
    let modified_at = metadata.modified().ok().and_then(system_time_to_secs);
    let filename = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 决定文件最终位置
    let final_file_path = if should_copy {
        let files_dir = library_files_dir(&app)?;
        ensure_dir(&files_dir)?;
        let dest = files_dir.join(format!("{}.{}", id, file_type));
        fs::copy(&src, &dest).map_err(|e| format!("复制文件失败: {}", e))?;
        dest.to_string_lossy().to_string()
    } else {
        // ref_only：保持原绝对路径
        src.to_string_lossy().to_string()
    };

    let mut entry = BookEntry {
        id: id.clone(),
        file_path: final_file_path,
        file_type: file_type.to_string(),
        added_at: now,
        file_size,
        filename,
        created_at,
        modified_at,
        ..Default::default()
    };

    if file_type == "epub" {
        match parse_epub_metadata(&src) {
            Ok(meta) => {
                entry.title = if meta.title.is_empty() {
                    src.file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                } else {
                    meta.title
                };
                entry.author = meta.author;
                entry.publisher = meta.publisher;
                entry.description = meta.description;
                entry.epub_uuid = meta.epub_uuid;

                // 扩展字段：subtitle/series/maker/tags 与写入约定对称读回
                entry.subtitle = meta.subtitle.unwrap_or_default();
                entry.series = meta.series.unwrap_or_default();
                entry.maker = meta.maker.unwrap_or_default();
                entry.tags = if meta.tags.is_empty() {
                    None
                } else {
                    Some(meta.tags)
                };

                // 制作时间 / 修改时间 联合规则：
                //  - OPF 两者都有：各自独立
                //  - OPF 仅一个：两边都用该值（用户期望两个时间同显示这一个）
                //  - OPF 都没：保留前面从 fs 读到的 ctime / mtime（不再覆盖）
                match (meta.pub_date, meta.modified_date) {
                    (Some(p), Some(m)) => {
                        entry.created_at = Some(p);
                        entry.modified_at = Some(m);
                    }
                    (Some(p), None) => {
                        entry.created_at = Some(p);
                        entry.modified_at = Some(p);
                    }
                    (None, Some(m)) => {
                        entry.created_at = Some(m);
                        entry.modified_at = Some(m);
                    }
                    (None, None) => {
                        // 保留 fs 兜底
                    }
                }

                if let Some(bytes) = meta.cover_bytes {
                    let covers_dir = library_covers_dir(&app)?;
                    ensure_dir(&covers_dir)?;
                    let cover_path = covers_dir.join(format!("{}.{}", id, meta.cover_ext));
                    fs::write(&cover_path, &bytes)
                        .map_err(|e| format!("写入封面失败: {}", e))?;
                    entry.cover_path = cover_path.to_string_lossy().to_string();
                }
            }
            Err(e) => {
                // 解析失败不阻断入库，回退到文件名作为标题
                entry.title = src
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                eprintln!("[library] 解析 epub 元数据失败: {}", e);
            }
        }
    } else {
        // txt
        entry.title = src
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
    }

    data.books.push(entry.clone());
    write_library_data_atomic(&app, &data)?;
    Ok(entry)
}

#[tauri::command]
async fn remove_book_from_library(
    app: tauri::AppHandle,
    book_id: String,
) -> Result<(), String> {
    let mut data = read_library_data(&app)?;
    let idx = data
        .books
        .iter()
        .position(|b| b.id == book_id)
        .ok_or_else(|| format!("未找到图书: {}", book_id))?;
    let book = data.books[idx].clone();

    // 仅清理位于 files_dir 内的副本（ref_only 模式下原文件不动）
    let files_dir = library_files_dir(&app)?;
    if !book.file_path.is_empty() {
        let book_path = PathBuf::from(&book.file_path);
        if book_path.starts_with(&files_dir) && book_path.exists() {
            let _ = fs::remove_file(&book_path);
        }
    }

    // 封面无论哪种模式都是我们生成的，删除
    if !book.cover_path.is_empty() {
        let cover_path = PathBuf::from(&book.cover_path);
        if cover_path.exists() {
            let _ = fs::remove_file(&cover_path);
        }
    }

    data.books.remove(idx);
    write_library_data_atomic(&app, &data)?;
    Ok(())
}

// ============================================================
// ===== Library - Phase 3: cover data / refresh metadata =====
// ============================================================

#[tauri::command]
fn get_library_cover_data(cover_path: String) -> Result<Vec<u8>, String> {
    if cover_path.trim().is_empty() {
        return Err("封面路径为空".to_string());
    }
    fs::read(&cover_path).map_err(|e| format!("读取封面失败: {}", e))
}

#[tauri::command]
async fn refresh_book_metadata(
    app: tauri::AppHandle,
    book_id: String,
) -> Result<BookEntry, String> {
    let mut data = read_library_data(&app)?;
    let idx = data
        .books
        .iter()
        .position(|b| b.id == book_id)
        .ok_or_else(|| format!("未找到图书: {}", book_id))?;

    // 克隆当前条目，只覆盖元数据 + 封面
    let mut entry = data.books[idx].clone();
    let src = PathBuf::from(&entry.file_path);
    if !src.exists() {
        return Err(format!("源文件不存在: {}", entry.file_path));
    }

    // 同步源文件 size；时间字段统一交给后面的 OPF 联合规则处理（详见 epub 块）。
    // txt 没有 OPF，沿用 fs mtime。
    if let Ok(meta) = fs::metadata(&src) {
        entry.file_size = meta.len();
        if entry.file_type == "txt" {
            entry.modified_at = meta.modified().ok().and_then(system_time_to_secs);
        }
        // created_at 不再更新（保持入库时记录的最早值）
    }

    if entry.file_type == "epub" {
        match parse_epub_metadata(&src) {
            Ok(meta) => {
                if !meta.title.is_empty() {
                    entry.title = meta.title;
                }
                entry.author = meta.author;
                entry.publisher = meta.publisher;
                entry.description = meta.description;
                if !meta.epub_uuid.is_empty() {
                    entry.epub_uuid = meta.epub_uuid;
                }

                // 扩展字段：刷新意味着把库内字段同步成源文件中的值（包括清空）
                entry.subtitle = meta.subtitle.unwrap_or_default();
                entry.series = meta.series.unwrap_or_default();
                entry.maker = meta.maker.unwrap_or_default();
                entry.tags = if meta.tags.is_empty() {
                    None
                } else {
                    Some(meta.tags)
                };

                // 制作时间 / 修改时间 联合规则（与 add_book_to_library 一致）：
                //  - OPF 两者都有：各自独立
                //  - OPF 仅一个：两边都用该值
                //  - OPF 都没：保留库内旧值（不动）
                match (meta.pub_date, meta.modified_date) {
                    (Some(p), Some(m)) => {
                        entry.created_at = Some(p);
                        entry.modified_at = Some(m);
                    }
                    (Some(p), None) => {
                        entry.created_at = Some(p);
                        entry.modified_at = Some(p);
                    }
                    (None, Some(m)) => {
                        entry.created_at = Some(m);
                        entry.modified_at = Some(m);
                    }
                    (None, None) => {
                        // 保留库内旧值
                    }
                }

                // 重新写封面：先删旧封面（哪怕扩展名不同），再写新的
                if let Some(bytes) = meta.cover_bytes {
                    if !entry.cover_path.is_empty() {
                        let old = PathBuf::from(&entry.cover_path);
                        if old.exists() {
                            let _ = fs::remove_file(&old);
                        }
                    }
                    let covers_dir = library_covers_dir(&app)?;
                    ensure_dir(&covers_dir)?;
                    let cover_path =
                        covers_dir.join(format!("{}.{}", entry.id, meta.cover_ext));
                    fs::write(&cover_path, &bytes)
                        .map_err(|e| format!("写入封面失败: {}", e))?;
                    entry.cover_path = cover_path.to_string_lossy().to_string();
                }
            }
            Err(e) => {
                return Err(format!("解析 epub 元数据失败: {}", e));
            }
        }
    } else {
        // txt：仅刷新文件名/标题（如果用户在外部改名了）
        if let Some(stem) = src.file_stem() {
            entry.title = stem.to_string_lossy().to_string();
        }
        if let Some(name) = src.file_name() {
            entry.filename = name.to_string_lossy().to_string();
        }
    }

    data.books[idx] = entry.clone();
    write_library_data_atomic(&app, &data)?;
    Ok(entry)
}

// ============================================================
// ===== Library - Phase 4: update metadata / update cover =====
// ============================================================

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// 检测图片类型，仅依据 magic bytes，未识别则按 jpg 处理
fn detect_image_ext(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        "jpg"
    } else if bytes.len() >= 8
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        "png"
    } else if bytes.len() >= 12
        && &bytes[0..4] == b"RIFF"
        && &bytes[8..12] == b"WEBP"
    {
        "webp"
    } else if bytes.len() >= 6
        && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a")
    {
        "gif"
    } else {
        "jpg"
    }
}

// 替换 OPF 中 <{tag}>...</{tag}> 的内容；不存在则在 </metadata> 前插入
fn replace_or_insert_dc(opf: &str, tag: &str, value: &str) -> String {
    let pat = format!(r#"<{0}((?:\s[^>]*)?)>([\s\S]*?)</{0}>"#, re_escape(tag));
    if let Ok(re) = Regex::new(&pat) {
        if let Ok(Some(c)) = re.captures(opf) {
            let attrs = c.get(1).map(|m| m.as_str()).unwrap_or("");
            let full = c.get(0).unwrap();
            let replacement = format!("<{0}{1}>{2}</{0}>", tag, attrs, xml_escape(value));
            let mut out = String::with_capacity(opf.len() + replacement.len());
            out.push_str(&opf[..full.start()]);
            out.push_str(&replacement);
            out.push_str(&opf[full.end()..]);
            return out;
        }
    }
    // 回退：在 </metadata> 前插入
    let inject = format!("    <{0}>{1}</{0}>\n  ", tag, xml_escape(value));
    insert_before_metadata_close(opf, &inject)
}

// 在 </metadata> 前插入一段（已含缩进/换行的）内容
fn insert_before_metadata_close(opf: &str, content: &str) -> String {
    if let Some(pos) = opf.find("</metadata>") {
        let mut out = String::with_capacity(opf.len() + content.len());
        out.push_str(&opf[..pos]);
        out.push_str(content);
        out.push_str(&opf[pos..]);
        return out;
    }
    opf.to_string()
}

// 删除 OPF 中所有匹配某 pattern 的片段（用于多个 dc:subject、旧 calibre meta 等清理）
fn remove_all_matches(opf: &str, pattern: &str) -> String {
    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return opf.to_string(),
    };
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for m in re.find_iter(opf).flatten() {
        ranges.push((m.start(), m.end()));
    }
    if ranges.is_empty() {
        return opf.to_string();
    }
    let mut out = String::with_capacity(opf.len());
    let mut cursor = 0;
    for (s, e) in &ranges {
        out.push_str(&opf[cursor..*s]);
        cursor = *e;
    }
    out.push_str(&opf[cursor..]);
    out
}

// 通用：把"<meta name=NAME content=VALUE/>"风格的字段先全部删掉、再按需插入
fn replace_meta_by_name(opf: &str, name: &str, value: &str) -> String {
    let rm_pat = format!(
        r#"\s*<meta\s+(?:[^>]*\s)?name="{}"[^>]*/?>"#,
        re_escape(name)
    );
    let mut s = remove_all_matches(opf, &rm_pat);
    if !value.trim().is_empty() {
        let inject = format!(
            "    <meta name=\"{}\" content=\"{}\"/>\n  ",
            name,
            xml_escape(value)
        );
        s = insert_before_metadata_close(&s, &inject);
    }
    s
}

// EPUB 3 修改时间：<meta property="dcterms:modified">YYYY-MM-DDTHH:MM:SSZ</meta>。
// 先删掉所有现有的，再插入一个新的。iso 应是 "2024-01-15T12:00:00Z" 格式。
fn set_dcterms_modified(opf: &str, iso: &str) -> String {
    let rm_pat = r#"\s*<meta\s+(?:[^>]*\s)?property="dcterms:modified"[^>]*>[\s\S]*?</meta>"#;
    let s = remove_all_matches(opf, rm_pat);
    let inject = format!(
        "    <meta property=\"dcterms:modified\">{}</meta>\n  ",
        xml_escape(iso)
    );
    insert_before_metadata_close(&s, &inject)
}

fn write_opf_metadata(
    opf: &str,
    title: &str,
    author: &str,
    description: &str,
    epub_uuid: &str,
    publisher: Option<&str>,
    subtitle: &str,
    maker: &str,
    series: &str,
    tags: &[String],
) -> String {
    let mut s = opf.to_string();

    // 标准 dc:* 字段
    if !title.is_empty() {
        s = replace_or_insert_dc(&s, "dc:title", title);
    }
    if !author.is_empty() {
        s = replace_or_insert_dc(&s, "dc:creator", author);
    }
    s = replace_or_insert_dc(&s, "dc:description", description);
    if !epub_uuid.is_empty() {
        s = replace_or_insert_dc(&s, "dc:identifier", epub_uuid);
    }
    if let Some(p) = publisher {
        s = replace_or_insert_dc(&s, "dc:publisher", p);
    }

    // tags → 多个 dc:subject（EPUB 标准）。先删后插。
    s = remove_all_matches(
        &s,
        r#"\s*<dc:subject(?:\s[^>]*)?>[\s\S]*?</dc:subject>"#,
    );
    if !tags.is_empty() {
        let mut block = String::new();
        for t in tags {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                continue;
            }
            block.push_str(&format!(
                "    <dc:subject>{}</dc:subject>\n  ",
                xml_escape(trimmed)
            ));
        }
        if !block.is_empty() {
            s = insert_before_metadata_close(&s, &block);
        }
    }

    // calibre 风格 meta：subtitle / series；自定义 maker
    s = replace_meta_by_name(&s, "calibre:subtitle", subtitle);
    s = replace_meta_by_name(&s, "calibre:series", series);
    s = replace_meta_by_name(&s, "maker", maker);

    s
}

// 读 epub 内 OPF 文本及其在 zip 内的路径（解析 container.xml）
fn read_opf_from_epub(epub_path: &Path) -> Result<(String, String), String> {
    let file = fs::File::open(epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("无效 EPUB: {}", e))?;

    let opf_path = {
        let mut f = archive
            .by_name("META-INF/container.xml")
            .map_err(|_| "缺少 META-INF/container.xml".to_string())?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        let re = Regex::new(r#"full-path="([^"]+)""#).map_err(|e| e.to_string())?;
        match re.captures(&s) {
            Ok(Some(c)) => c.get(1).unwrap().as_str().to_string(),
            _ => return Err("container.xml 缺少 full-path".to_string()),
        }
    };
    let opf_xml = {
        let mut f = archive
            .by_name(&opf_path)
            .map_err(|_| format!("缺少 OPF 文件: {}", opf_path))?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };
    Ok((opf_path, opf_xml))
}

// 在 OPF 文本中定位封面在 zip 内的完整路径
fn locate_cover_in_opf(opf_path: &str, opf_xml: &str) -> Option<String> {
    let mut cover_href: Option<String> = None;
    let href_re = Regex::new(r#"href="([^"]+)""#).ok()?;

    if let Ok(prop_re) = Regex::new(r#"<item\s+[^>]*properties="cover-image"[^>]*>"#) {
        if let Ok(Some(m)) = prop_re.find(opf_xml) {
            if let Ok(Some(c)) = href_re.captures(m.as_str()) {
                cover_href = c.get(1).map(|x| x.as_str().to_string());
            }
        }
    }
    if cover_href.is_none() {
        if let Ok(meta_re) = Regex::new(r#"<meta\s+[^>]*name="cover"\s+content="([^"]+)""#) {
            if let Ok(Some(c)) = meta_re.captures(opf_xml) {
                if let Some(id_m) = c.get(1) {
                    let cover_id = id_m.as_str();
                    if let Ok(id_re) = Regex::new(&format!(
                        r#"<item\s+[^>]*id="{}"[^>]*>"#,
                        re_escape(cover_id)
                    )) {
                        if let Ok(Some(m)) = id_re.find(opf_xml) {
                            if let Ok(Some(c)) = href_re.captures(m.as_str()) {
                                cover_href = c.get(1).map(|x| x.as_str().to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    cover_href.map(|href| {
        let opf_dir = Path::new(opf_path)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let full = if opf_dir.is_empty() {
            href
        } else {
            format!("{}/{}", opf_dir, href)
        };
        normalize_zip_path(&full)
    })
}

struct EpubRewrite {
    opf_replacement: Option<String>,
    cover_replacement: Option<Vec<u8>>,
}

// 流式重打包：拷贝原 zip 所有项，遇到 OPF / 封面则替换
fn rewrite_epub(epub_path: &Path, changes: &EpubRewrite) -> Result<(), String> {
    use zip::write::FileOptions;

    let (opf_path, opf_xml) = read_opf_from_epub(epub_path)?;
    let cover_internal = if changes.cover_replacement.is_some() {
        locate_cover_in_opf(&opf_path, &opf_xml)
    } else {
        None
    };

    let src = fs::File::open(epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(src).map_err(|e| format!("无效 EPUB: {}", e))?;

    let tmp_path = format!("{}.tmp", epub_path.to_string_lossy());
    let tmp_file =
        fs::File::create(&tmp_path).map_err(|e| format!("创建临时 zip 失败: {}", e))?;
    let mut zw = zip::ZipWriter::new(tmp_file);
    let opts_deflated =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let opts_stored =
        FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    let count = archive.len();
    for i in 0..count {
        let mut zf = archive
            .by_index(i)
            .map_err(|e| format!("读取 zip 项失败: {}", e))?;
        if !zf.is_file() {
            continue;
        }
        let name = zf.name().to_string();
        let opts = if name == "mimetype" {
            opts_stored
        } else {
            opts_deflated
        };
        zw.start_file(&name, opts)
            .map_err(|e| format!("写入 zip 项失败: {}", e))?;

        let mut handled = false;
        if name == opf_path {
            if let Some(ref new_opf) = changes.opf_replacement {
                zw.write_all(new_opf.as_bytes())
                    .map_err(|e| e.to_string())?;
                handled = true;
            }
        }
        if !handled {
            if let (Some(ci), Some(bytes)) = (
                cover_internal.as_ref(),
                changes.cover_replacement.as_ref(),
            ) {
                if &name == ci {
                    zw.write_all(bytes).map_err(|e| e.to_string())?;
                    handled = true;
                }
            }
        }
        if !handled {
            let mut buf = Vec::new();
            zf.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            zw.write_all(&buf).map_err(|e| e.to_string())?;
        }
    }

    zw.finish().map_err(|e| format!("完成 zip 失败: {}", e))?;
    drop(archive); // Windows 上必须释放原 zip 句柄再 rename

    fs::rename(&tmp_path, epub_path)
        .map_err(|e| format!("替换 EPUB 失败: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn update_book_metadata(
    app: tauri::AppHandle,
    book_id: String,
    title: String,
    author: String,
    subtitle: String,
    description: String,
    maker: String,
    series: String,
    tags: Vec<String>,
    epub_uuid: String,
) -> Result<BookEntry, String> {
    let mut data = read_library_data(&app)?;
    let idx = data
        .books
        .iter()
        .position(|b| b.id == book_id)
        .ok_or_else(|| format!("未找到图书: {}", book_id))?;

    let mut entry = data.books[idx].clone();
    entry.title = title.clone();
    entry.author = author.clone();
    entry.subtitle = subtitle;
    entry.description = if description.is_empty() {
        None
    } else {
        Some(description.clone())
    };
    entry.maker = maker;
    entry.series = series;
    entry.tags = if tags.is_empty() { None } else { Some(tags) };
    entry.epub_uuid = epub_uuid.clone();

    // 仅 epub 写回 OPF；txt 只更新库
    if entry.file_type == "epub" {
        let src = PathBuf::from(&entry.file_path);
        if src.exists() {
            let (_opf_path, opf_xml) = read_opf_from_epub(&src)?;
            let mut new_opf = write_opf_metadata(
                &opf_xml,
                &title,
                &author,
                &description,
                &epub_uuid,
                entry.publisher.as_deref(),
                &entry.subtitle,
                &entry.maker,
                &entry.series,
                entry.tags.as_deref().unwrap_or(&[]),
            );

            // 默认不动 dcterms:modified；仅当 config.updateModifiedOnEdit=true 才覆盖为现在
            if data.config.update_modified_on_edit {
                let now = chrono::Utc::now();
                let now_iso = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
                new_opf = set_dcterms_modified(&new_opf, &now_iso);
                let ts = now.timestamp();
                if ts >= 0 {
                    entry.modified_at = Some(ts as u64);
                }
            }

            // zip 重打包是同步阻塞 IO，丢到 blocking pool 不卡 async runtime
            let src_clone = src.clone();
            tauri::async_runtime::spawn_blocking(move || {
                rewrite_epub(
                    &src_clone,
                    &EpubRewrite {
                        opf_replacement: Some(new_opf),
                        cover_replacement: None,
                    },
                )
            })
            .await
            .map_err(|e| format!("写回 EPUB 任务失败: {}", e))??;
        }
    }

    data.books[idx] = entry.clone();
    write_library_data_atomic(&app, &data)?;
    Ok(entry)
}

#[tauri::command]
async fn update_book_cover(
    app: tauri::AppHandle,
    book_id: String,
    cover_data: Vec<u8>,
) -> Result<String, String> {
    if cover_data.is_empty() {
        return Err("封面数据为空".to_string());
    }
    let mut data = read_library_data(&app)?;
    let idx = data
        .books
        .iter()
        .position(|b| b.id == book_id)
        .ok_or_else(|| format!("未找到图书: {}", book_id))?;
    let mut entry = data.books[idx].clone();

    // 删旧封面文件
    if !entry.cover_path.is_empty() {
        let old = PathBuf::from(&entry.cover_path);
        if old.exists() {
            let _ = fs::remove_file(&old);
        }
    }

    // 写新封面到 covers 目录
    let ext = detect_image_ext(&cover_data);
    let covers_dir = library_covers_dir(&app)?;
    ensure_dir(&covers_dir)?;
    let cover_path = covers_dir.join(format!("{}.{}", entry.id, ext));
    fs::write(&cover_path, &cover_data)
        .map_err(|e| format!("写入封面失败: {}", e))?;
    let cover_path_str = cover_path.to_string_lossy().to_string();
    entry.cover_path = cover_path_str.clone();

    // epub：替换内部封面（失败仅日志，不阻断库内封面更新）
    if entry.file_type == "epub" {
        let src = PathBuf::from(&entry.file_path);
        if src.exists() {
            let src_clone = src.clone();
            let bytes_for_zip = cover_data.clone();
            let join = tauri::async_runtime::spawn_blocking(move || {
                rewrite_epub(
                    &src_clone,
                    &EpubRewrite {
                        opf_replacement: None,
                        cover_replacement: Some(bytes_for_zip),
                    },
                )
            })
            .await;
            match join {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    eprintln!("[library] 写回 epub 内部封面失败（库内封面已更新）: {}", e);
                }
                Err(e) => {
                    eprintln!("[library] spawn_blocking 异常（库内封面已更新）: {}", e);
                }
            }
        }
    }

    data.books[idx] = entry;
    write_library_data_atomic(&app, &data)?;
    Ok(cover_path_str)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            read_text_file,
            save_text_file,
            read_binary_file,
            save_history,
            get_history_list,
            calculate_md5,
            scan_chapters,
            advanced_search,
            advanced_replace,
            export_epub,
            extract_epub,
            read_epub_file_content,
            read_epub_file_binary,
            read_epub_files_batch,
            read_epub_binary_batch,
            analyze_epub_font_glyphs,
            save_epub_file_content,
            save_epub_to_disk,
            search_in_files,
            add_epub_file,
            add_epub_file_binary,
            save_epub_file_binary,
            save_epub_files_batch,
            delete_epub_file,
            rename_epub_file,
            get_launch_args, // Register new command
            prepare_epub_for_open,
            exit_app,
            // ===== Library Phase 1 =====
            load_library,
            save_library,
            // ===== Library Phase 2 =====
            add_book_to_library,
            remove_book_from_library,
            // ===== Library Phase 3 =====
            get_library_cover_data,
            refresh_book_metadata,
            // ===== Library Phase 4 =====
            update_book_metadata,
            update_book_cover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
