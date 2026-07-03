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

fn hidden_process_command(program: &str) -> process::Command {
    let mut command = process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

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

@import url("font.css");

/* 全书基础页：正文页 / 章节页 / 卷页的默认排版 */

body.te-book-body,
body.te-chapter-page,
body.te-volume-page {
    padding: 0%;
    margin-top: 0%;
    margin-bottom: 0%;
    margin-left: 1%;
    margin-right: 1%;
    line-height: 130%;
    text-align: justify;
    font-family: "Maintext", "DK-SONGTI", "st", "宋体", "zw", sans-serif;
}

/* 正文段落 */
p.te-paragraph {
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    line-height: 130%;
    margin-right: 1%;
    margin-left: 1%;
    font-family: "DK-SONGTI", "st", "宋体", "zw", sans-serif;
}

/* 通用块元素：给封面、简介、头图、分割图等容器打基础样式 */
div {
    margin: 0;
    padding: 0;
    line-height: 130%;
    text-align: center;
    font-family: "zw";
}

/*————————————————————制作说明————————————————————*/
.te-production-card {
    margin: 10% 7.25% 2.75% 7.25%;
    padding: 5.25% 5.25%;
    border: 1.5px solid #6C322D;
    background: url(../Images/production-card-bg.jpg) no-repeat top left;
    background-size: 35% auto;
    background-color: rgba(255, 255, 255, 0.7);
    border-radius: 5px;
}

.te-production-title {
    font-family: "哥特式字体";
    font-size: 110%;
    font-weight: normal;
    color: black;
    margin: 1em 0 0.5em 0;
    text-align: center;
}

body.te-production-page {
    background: #fff no-repeat center;
    background-size: cover;
    background-attachment: fixed;
    background-repeat: no-repeat;
    background-position: bottom center;
    background-image: url(../Images/back.jpg);
    transform: scale(1.0) translate(0px, 0px);
}

.te-production-text {
    font-family: "哥特式字体";
    font-size: 80%;
    color: #220;
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    margin: 0 0 2.5% 0;
}

.te-production-note {
    font-family: "cc", "kt", sans-serif;
    font-size: 65%;
    color: #000;
    text-align: justify;
    text-indent: 2em;
    duokan-text-indent: 2em;
    margin: 2.5% 0 0 0;
}

.te-production-logo {
    margin: 0 20% 0 20%;
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
}

.te-production-logo-img {
    width: 70%;
}

/*————————————————————内容简介————————————————————*/

body.te-intro-page {
    background-color: transparent;
    border-color: rgba(83, 83, 83, 0.5);
    border-width: 0.4em;
}

.te-cover-wrap {
    margin: 3em 0 1em 0;
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0;
    width: 100%;
}

.te-cover-image {
    width: 40%;
    box-shadow: 3px 3px 3px #535353;
    margin: 0 0 0.5em 0;
}

.te-intro-title {
    font-family: "哥特宋";
    font-size: 125%;
    color: #00008B;
    margin: 0.3em 0 0.5em 0;
    text-align: left;
    text-indent: 0;
}

span.book-name {
    font-family: "楷体", sans-serif;
    color: #DC143C;
}

span.author {
    font-family: "小标宋", sans-serif;
}

.te-intro-title span,
.te-intro-heading span {
    background-color: transparent;
    padding: 0.4em 2em 0.2em 0.4em;
}

.te-intro-heading {
    margin: 0.3em 0 0.5em 0;
    text-align: left;
    text-indent: 0;
    duokan-text-indent: 0;
    font-size: 110%;
    color: #00008B;
    font-family: "哥特式字体";
}

div.book-introduction p {
    font-family: "DK-XIHEITI", "黑体", sans-serif;
}

/* 卷标题 / 卷副标题 */
.te-volume-title {
    font-family: "哥特宋", serif;
    font-size: 1.2em;
    color: #59bde6;
    font-weight: 600;
    margin: 2em 0 1em 0;
    text-align: center;
    text-indent: 0em;
    line-height: 130%;
}

.te-volume-subtitle {
    font-family: "哥特宋", serif;
    font-size: 1.2em;
    color: #59bde6;
    margin: 0em 0em 1em 0em;
    text-indent: 0em;
    text-align: center;
    line-height: 110%;
}

/* Header Image */

.te-volume-head-image {
    margin: 0.5em;
    text-align: center;
    text-indent: 0em;
    duokan-text-indent: 0em;
    duokan-bleed: lefttopright;
}

.te-volume-head-img {
    width: 70%;
    max-width: 100%;
}

body.te-volume-page {
    background-size: cover;
    background-repeat: no-repeat;
    background-attachment: fixed;
    background-position: bottom center;
    padding: 1% 1% 5%;
}

body.te-volume-page.te-volume-page--no-image .te-volume-title {
    margin-top: 12.2em;
}

body.te-volume-page.te-volume-page--no-image .te-volume-subtitle {
    margin-bottom: 1.8em;
}

/* 章节标题 / 章节头图 */
.te-chapter-title {
    font-family: "黑体", sans-serif;
    text-align: center;
    margin: 2em 0em 3em 0em;
    font-size: 1.2em;
    font-weight: 900;
    color: #c2181e;
}

body.te-chapter-page.te-chapter-page--no-image .te-chapter-title {
    margin-top: 6.5em;
}

.te-chapter-head-image {
    margin: 0em 0em 0em 0em;
    text-align: left;
    text-indent: 0em;
    duokan-text-indent: 0em;
    duokan-bleed: lefttopright;
}

.te-chapter-head-img {
    width: 100%;
}

.te-chapter-number {
    font-family: "黑体", sans-serif;
    font-weight: 900;
    font-size: 0.8em;
    color: #413245;
    line-height: 130%;
    padding: 0;
    text-align: center;
    background-color: transparent;
}

/* 分割线 */
p.te-divider-line {
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0em;
    margin: 1em 0;
}

/* 分割图：当模板提供 dividerImage 时，用图片替换孤立省略号 */
.te-divider-image {
    text-align: center;
    text-indent: 0;
    duokan-text-indent: 0em;
    margin: 1em 0;
    padding: 0;
    line-height: 130%;
}

.te-divider-img {
    width: 200px;
    max-width: 100%;
    border: none;
    vertical-align: middle;
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
.te-image-single {
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProofLogInfo {
    file_name: String,
    path: String,
    timestamp: u64,
    size: u64,
}

#[derive(Deserialize, Debug, Clone)]
struct AssetInfo {
    name: String,
    path: String,
    category: String, // "fonts", "images", "others"
    #[serde(default)]
    role: String,
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
    tags: Vec<String>,
    #[serde(default)]
    main_css: String,
    #[serde(default)]
    font_css: String,
    #[serde(default)]
    subset_fonts: bool,
    #[serde(default)]
    assets: Vec<AssetInfo>,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EpubTemplateRepository {
    id: String,
    name: String,
    url: String,
    #[serde(default = "default_template_branch")]
    branch: String,
    #[serde(default)]
    last_synced: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EpubRemoteTemplate {
    id: String,
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    path: String,
    #[serde(default)]
    preview: String,
    #[serde(default)]
    category: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EpubTemplateRepositoryIndex {
    #[serde(default = "default_template_schema")]
    schema: u32,
    #[serde(default)]
    templates: Vec<EpubRemoteTemplate>,
}

#[derive(Serialize, Debug, Clone)]
struct EpubTemplateInstallResult {
    template_id: String,
    local_path: String,
    file_count: usize,
}

#[derive(Serialize, Debug, Clone)]
struct LibraryFontInfo {
    family: String,
    css_value: String,
    file_name: String,
    path: String,
}

type LibraryFontAliasMap = HashMap<String, String>;

#[derive(Serialize, Debug, Clone)]
struct StyleTemplateInfo {
    id: String,
    name: String,
    file_name: String,
    path: String,
    is_builtin: bool,
}

#[derive(Serialize, Debug, Clone)]
struct StyleTemplateContent {
    id: String,
    name: String,
    main_css: String,
    is_builtin: bool,
}

fn default_template_branch() -> String {
    "main".to_string()
}

fn default_template_schema() -> u32 {
    1
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

fn decode_basic_html_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn strip_html_tags(input: &str) -> String {
    Regex::new(r"(?is)<[^>]+>")
        .map(|re| re.replace_all(input, "").to_string())
        .unwrap_or_else(|_| input.to_string())
}

fn extract_html_heading_title(content: &str) -> Option<String> {
    let patterns = [
        r"(?is)<h[1-6][^>]*>(.*?)</h[1-6]>",
        r"(?is)<title[^>]*>(.*?)</title>",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Ok(Some(caps)) = re.captures(content) {
                if let Some(raw) = caps.get(1) {
                    let text = decode_basic_html_entities(&strip_html_tags(raw.as_str()))
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
        }
    }

    None
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

fn image_mime_from_ext(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "jpg" | "jpeg" => "image/jpeg",
        _ => "image/jpeg",
    }
}

fn asset_manifest_mime(name: &str) -> &'static str {
    match Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "css" => "text/css",
        "js" => "text/javascript",
        _ => "application/octet-stream",
    }
}

fn build_font_subset_text(content: &str, metadata: &EpubMetadata) -> String {
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

fn ensure_fonttools_available() -> Result<(), String> {
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

fn try_subset_font_bytes(bytes: &[u8], ext: &str, subset_text: &str) -> Result<Vec<u8>, String> {
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

fn asset_slot_file_stem(role: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in role.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx > 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "image-slot".to_string()
    } else {
        trimmed
    }
}

fn asset_slot_class_stem(role: &str) -> String {
    asset_slot_file_stem(role)
}

fn image_slot_html(role: &str, href: &str) -> String {
    let stem = asset_slot_class_stem(role);
    format!(
        "  <div class=\"te-{}-image\"><img class=\"te-{}-img\" src=\"../{}\" alt=\"\" /></div>\n",
        stem,
        stem,
        escape_xml(href)
    )
}

fn divider_image_html(role: &str, href: &str) -> String {
    let stem = asset_slot_class_stem(role);
    format!(
        "  <div class=\"te-divider-image\"><img class=\"te-{}-img te-divider-img\" src=\"../{}\" alt=\"分隔符\" /></div>\n",
        stem,
        escape_xml(href)
    )
}

fn parse_asset_slot_placements(css: &str) -> HashMap<String, String> {
    let mut placements = HashMap::new();
    let Ok(slot_re) = Regex::new(r#"@tepub-asset-slot\s+([A-Za-z][\w-]*)([^*]*)"#) else {
        return placements;
    };
    let Ok(placement_re) = Regex::new(r#"placement\s*=\s*["']([^"']+)["']"#) else {
        return placements;
    };
    for caps_result in slot_re.captures_iter(css) {
        let Ok(caps) = caps_result else {
            continue;
        };
        let Some(role) = caps.get(1).map(|m| m.as_str().to_string()) else {
            continue;
        };
        let attrs = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        if let Ok(Some(place_caps)) = placement_re.captures(attrs) {
            if let Some(place) = place_caps.get(1).map(|m| m.as_str().to_string()) {
                placements.insert(role, place);
            }
        }
    }
    placements
}

fn first_image_slot_for_placement<'a>(
    hrefs: &'a HashMap<String, String>,
    placements: &'a HashMap<String, String>,
    placement: &str,
    fallback_role: &'a str,
) -> Option<(&'a str, &'a str)> {
    for (role, place) in placements {
        if place == placement {
            if let Some(href) = hrefs.get(role) {
                return Some((role.as_str(), href.as_str()));
            }
        }
    }
    hrefs
        .get(fallback_role)
        .map(|href| (fallback_role, href.as_str()))
}

fn is_ellipsis_paragraph(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return false;
    }
    compact.chars().all(|c| c == '…' || c == '.')
        && (compact.matches('…').count() >= 1 || compact.matches('.').count() >= 3)
}

fn append_text_body_lines(
    html_body: &mut String,
    body_lines: &[&str],
    enable_dividers: bool,
    divider_image: Option<(&str, &str)>,
) {
    let non_empty: Vec<(usize, &str)> = body_lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let trim = line.trim();
            if trim.is_empty() {
                None
            } else {
                Some((index, trim))
            }
        })
        .collect();

    let last_non_empty_index = non_empty.last().map(|(index, _)| *index);

    for (position, (line_index, trim)) in non_empty.iter().enumerate() {
        let is_ellipsis = is_ellipsis_paragraph(trim);
        let prev_is_ellipsis = position > 0 && is_ellipsis_paragraph(non_empty[position - 1].1);
        let next_is_ellipsis =
            position + 1 < non_empty.len() && is_ellipsis_paragraph(non_empty[position + 1].1);
        let is_last_non_empty = Some(*line_index) == last_non_empty_index;

        if enable_dividers && is_ellipsis && !prev_is_ellipsis && !next_is_ellipsis && !is_last_non_empty {
            if let Some((role, href)) = divider_image {
                html_body.push_str(&divider_image_html(role, href));
            } else {
                html_body.push_str("  <p class=\"te-divider-line\">※※※</p>\n");
            }
        } else {
            html_body.push_str(&format!(
                "  <p class=\"te-paragraph\">{}</p>\n",
                escape_xml(trim)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::append_text_body_lines;

    #[test]
    fn isolated_ellipsis_becomes_divider() {
        let mut html = String::new();
        let lines = ["第一段", "……", "第二段"];
        append_text_body_lines(&mut html, &lines, true, None);

        assert!(html.contains(r#"<p class="te-divider-line">※※※</p>"#));
        assert!(html.contains(r#"<p class="te-paragraph">第一段</p>"#));
        assert!(html.contains(r#"<p class="te-paragraph">第二段</p>"#));
    }

    #[test]
    fn consecutive_ellipsis_stay_as_normal_paragraphs() {
        let mut html = String::new();
        let lines = ["第一段", "……", "……", "第二段"];
        append_text_body_lines(&mut html, &lines, true, None);

        assert!(!html.contains(r#"<p class="te-divider-line">※※※</p>"#));
        assert_eq!(html.matches(r#"<p class="te-paragraph">……</p>"#).count(), 2);
    }

    #[test]
    fn last_ellipsis_stays_as_normal_paragraph() {
        let mut html = String::new();
        let lines = ["第一段", "……"];
        append_text_body_lines(&mut html, &lines, true, None);

        assert!(!html.contains(r#"<p class="te-divider-line">※※※</p>"#));
        assert!(html.contains(r#"<p class="te-paragraph">……</p>"#));
    }

    #[test]
    fn blank_lines_do_not_break_isolated_ellipsis_detection() {
        let mut html = String::new();
        let lines = ["第一段", "", "   ", "……", "", "第二段"];
        append_text_body_lines(&mut html, &lines, true, None);

        assert!(html.contains(r#"<p class="te-divider-line">※※※</p>"#));
        assert!(!html.contains(r#"<p class="te-paragraph"></p>"#));
    }

    #[test]
    fn divider_image_replaces_isolated_ellipsis_when_available() {
        let mut html = String::new();
        let lines = ["第一段", "……", "第二段"];
        append_text_body_lines(
            &mut html,
            &lines,
            true,
            Some(("dividerImage", "Images/divider-image.png")),
        );

        assert!(html.contains(r#"<div class="te-divider-image"><img class="te-divider-image-img te-divider-img" src="../Images/divider-image.png" alt="分隔符" /></div>"#));
        assert!(!html.contains("※※※"));
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

fn percent_encode_path_ref(path: &str, upper_hex: bool) -> String {
    let mut out = String::new();
    for b in path.as_bytes() {
        match *b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b'/' => out.push(*b as char),
            _ if upper_hex => out.push_str(&format!("%{:02X}", b)),
            _ => out.push_str(&format!("%{:02x}", b)),
        }
    }
    out
}

fn push_path_rewrite_variants(pairs: &mut Vec<(String, String)>, old_ref: &str, new_ref: &str) {
    pairs.push((old_ref.to_string(), new_ref.to_string()));
    pairs.push((
        old_ref.replace(' ', "%20"),
        new_ref.replace(' ', "%20"),
    ));

    let old_encoded = percent_encode_path_ref(old_ref, true);
    let new_encoded = percent_encode_path_ref(new_ref, true);
    pairs.push((old_encoded.clone(), new_encoded.clone()));

    let old_encoded_lower = percent_encode_path_ref(old_ref, false);
    if old_encoded_lower != old_encoded {
        pairs.push((old_encoded_lower, new_encoded));
    }
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
        push_path_rewrite_variants(&mut pairs, &old_rel, &new_rel);
        if !old_rel.starts_with("./") {
            push_path_rewrite_variants(
                &mut pairs,
                &format!("./{}", old_rel),
                &format!("./{}", new_rel),
            );
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolboxEpubToolResult {
    source_path: String,
    output_path: String,
    changed: bool,
    action: String,
    message: String,
}

#[derive(Clone, Debug)]
struct ToolboxManifestItem {
    id: String,
    href: String,
    media_type: String,
    abs_path: String,
}

fn parse_xmlish_attrs(text: &str) -> HashMap<String, String> {
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

fn find_opf_path_in_epub_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    if let Ok(mut container) = archive.by_name("META-INF/container.xml") {
        let mut data = Vec::new();
        container
            .read_to_end(&mut data)
            .map_err(|e| format!("读取 container.xml 失败: {}", e))?;
        let text = String::from_utf8_lossy(&data).to_string();
        if let Ok(re) = Regex::new(r#"<rootfile[^>]*full-path\s*=\s*(['"])(?i:(.*?\.opf))\1"#)
        {
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

fn parse_toolbox_manifest_items(bytes: &[u8], opf_path: &str) -> Result<Vec<ToolboxManifestItem>, String> {
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

fn toolbox_file_category(item: &ToolboxManifestItem) -> &'static str {
    let media = item.media_type.to_ascii_lowercase();
    let href = item.href.to_ascii_lowercase();
    if media == "application/xhtml+xml" || href.ends_with(".xhtml") || href.ends_with(".html") {
        "text"
    } else if media == "text/css" || href.ends_with(".css") {
        "css"
    } else if media.starts_with("image/") {
        "image"
    } else if media.starts_with("font/") || href.ends_with(".ttf") || href.ends_with(".otf") || href.ends_with(".woff") || href.ends_with(".woff2") {
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
    let out_file = fs::File::create(output_path).map_err(|e| format!("创建输出 EPUB 失败: {}", e))?;
    let mut writer = zip::ZipWriter::new(out_file);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec()))
        .map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    let mut opf_old_to_new: HashMap<String, String> = HashMap::new();
    for (old, newp) in path_map {
        if old.to_ascii_lowercase().ends_with(".opf") && newp.to_ascii_lowercase().ends_with(".opf") {
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
            text = rewrite_text_links(text, &old_name, &new_name, path_map);
            data = text.into_bytes();
        } else if is_text_like_entry(&old_name) {
            let text = String::from_utf8_lossy(&data).to_string();
            data = rewrite_text_links(text, &old_name, &new_name, path_map).into_bytes();
        }

        writer
            .start_file(&new_name, options)
            .map_err(|e| format!("写入文件失败: {}", e))?;
        writer
            .write_all(&data)
            .map_err(|e| format!("写入文件内容失败: {}", e))?;
    }

    writer.finish().map_err(|e| format!("完成写入失败: {}", e))?;
    Ok(())
}

fn toolbox_file_encrypt_impl(source: &Path) -> Result<ToolboxEpubToolResult, String> {
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    let bytes = fs::read(source).map_err(|e| format!("读取 EPUB 失败: {}", e))?;
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
    if !source.exists() {
        return Err(format!("文件不存在: {}", source.to_string_lossy()));
    }
    let bytes = fs::read(source).map_err(|e| format!("读取 EPUB 失败: {}", e))?;
    let processed = rebuild_epub_with_sanitized_names_from_bytes(source, &bytes, "_decrypt")?;
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

def collect_supported_chars(font_items):
    from fontTools.ttLib import TTFont

    supported = set()
    for _name, font_data in font_items:
        try:
            font = TTFont(BytesIO(font_data))
            best = font.getBestCmap() or {}
            for code in best.keys():
                ch = chr(code)
                if should_map_char(ch):
                    supported.add(ch)
        except Exception:
            continue
    return supported

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
        supported_chars = collect_supported_chars((name, contents[name]) for name in font_names)
        if not supported_chars:
            raise RuntimeError("内嵌字体未发现可加密汉字")
        chars = []
        seen = set()
        existing_private_chars = set()
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
            contents[name] = transform_text(decode_text(contents[name]), mapping).encode("utf-8")
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
        chars, fonts = encrypt_epub(input_path, output_path)
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

fn toolbox_font_decrypt_impl(source: &Path, txt_path: Option<&Path>) -> Result<ToolboxEpubToolResult, String> {
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
fn toolbox_file_encrypt(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_file_encrypt_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
fn toolbox_file_decrypt(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_file_decrypt_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
fn toolbox_font_encrypt(epub_path: String) -> Result<ToolboxEpubToolResult, String> {
    toolbox_font_encrypt_impl(&PathBuf::from(epub_path))
}

#[tauri::command]
fn toolbox_font_decrypt(epub_path: String, txt_path: Option<String>) -> Result<ToolboxEpubToolResult, String> {
    let txt_path_buf = txt_path.map(PathBuf::from);
    toolbox_font_decrypt_impl(
        &PathBuf::from(epub_path),
        txt_path_buf.as_deref(),
    )
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

fn portable_marker_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("portable.txt"))
}

fn is_portable() -> bool {
    portable_marker_path()
        .map(|p| p.exists())
        .unwrap_or(false)
}

fn portable_data_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("data"))
}

fn app_data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
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

fn epub_templates_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_root(app)?.join("epub-templates"))
}

fn epub_template_repositories_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(epub_templates_root(app)?.join("repositories.json"))
}

fn style_templates_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_root(app)?.join("style-templates"))
}

fn builtin_style_template_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(style_templates_root(app)?.join("builtin.css"))
}

fn style_template_path(app: &tauri::AppHandle, id: &str) -> Result<PathBuf, String> {
    let safe_id = sanitize_filename_part(id);
    if safe_id.is_empty() {
        return Err("模板 ID 无效".to_string());
    }
    Ok(style_templates_root(app)?.join(format!("{}.css", safe_id)))
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
    let bytes = serde_json::to_vec_pretty(aliases).map_err(|e| format!("序列化字体别名失败: {}", e))?;
    fs::write(&path, bytes).map_err(|e| format!("保存字体别名失败: {}", e))
}

fn style_template_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace('_', " ").replace('-', " "))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "样式模板".to_string())
}

fn build_style_template_info(path: &Path, is_builtin: bool) -> Result<StyleTemplateInfo, String> {
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "模板文件名无效".to_string())?
        .to_string();
    let id = if is_builtin {
        "builtin".to_string()
    } else {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "模板 ID 无效".to_string())?
    };
    Ok(StyleTemplateInfo {
        id,
        name: if is_builtin {
            "内置模板".to_string()
        } else {
            style_template_name_from_path(path)
        },
        file_name,
        path: path.to_string_lossy().to_string(),
        is_builtin,
    })
}

fn github_owner_repo(url: &str) -> Result<(String, String), String> {
    let trimmed = url.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(trimmed)
        .map_err(|e| format!("模板仓库 URL 无效: {}", e))?;
    let host = parsed.host_str().unwrap_or_default().to_lowercase();
    if host != "github.com" {
        return Err("当前模板仓库仅支持 github.com".to_string());
    }
    let parts: Vec<&str> = parsed
        .path_segments()
        .map(|segments| segments.collect())
        .unwrap_or_else(Vec::new);
    if parts.len() < 2 {
        return Err("GitHub 仓库 URL 需要包含 owner/repo".to_string());
    }
    let owner = parts[0].to_string();
    let repo = parts[1].trim_end_matches(".git").to_string();
    if owner.is_empty() || repo.is_empty() {
        return Err("GitHub 仓库 URL 需要包含 owner/repo".to_string());
    }
    Ok((owner, repo))
}

fn raw_github_url(repo: &EpubTemplateRepository, path: &str) -> Result<String, String> {
    let (owner, name) = github_owner_repo(&repo.url)?;
    let clean_path = path.trim_start_matches('/');
    Ok(format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        owner, name, repo.branch, clean_path
    ))
}

fn github_tree_url(repo: &EpubTemplateRepository, dir: &str) -> Result<String, String> {
    let (owner, name) = github_owner_repo(&repo.url)?;
    let clean_dir = dir.trim_matches('/');
    Ok(format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}:{}?recursive=1",
        owner, name, repo.branch, clean_dir
    ))
}

fn read_template_repositories(app: &tauri::AppHandle) -> Result<Vec<EpubTemplateRepository>, String> {
    let path = epub_template_repositories_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取模板仓库配置失败: {}", e))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("解析模板仓库配置失败: {}", e))
}

fn write_template_repositories(
    app: &tauri::AppHandle,
    repos: &[EpubTemplateRepository],
) -> Result<(), String> {
    let root = epub_templates_root(app)?;
    ensure_dir(&root)?;
    let path = epub_template_repositories_path(app)?;
    let bytes = serde_json::to_vec_pretty(repos).map_err(|e| format!("序列化模板仓库配置失败: {}", e))?;
    fs::write(path, bytes).map_err(|e| format!("保存模板仓库配置失败: {}", e))
}

fn get_history_base_dir(app: Option<&tauri::AppHandle>) -> PathBuf {
    // 优先使用书库工作目录下的 _data/history（书库已配置时）
    if let Some(app) = app {
        if let Ok(p) = library_history_dir(app) {
            return p;
        }
    }
    // 回退：绿色版 exe_dir/data/history
    if is_portable() {
        if let Some(p) = portable_data_root() {
            return p.join("history");
        }
    }
    // 回退：安装版 app_data_root/history
    if let Some(app) = app {
        if let Ok(p) = app_data_root(app) {
            return p.join("history");
        }
    }
    // 终极兜底
    PathBuf::from(".history")
}

fn history_key_for_path(original_path: &str) -> String {
    // Keep names deterministic and avoid collisions for files that share the same stem.
    let digest = format!("{:x}", md5::compute(original_path.as_bytes()));
    digest[..8].to_string()
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

fn build_library_font_info(app: &tauri::AppHandle, font_path: &Path) -> Result<LibraryFontInfo, String> {
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
fn list_epub_template_repositories(
    app: tauri::AppHandle,
) -> Result<Vec<EpubTemplateRepository>, String> {
    read_template_repositories(&app)
}

#[tauri::command]
fn add_epub_template_repository(
    app: tauri::AppHandle,
    name: String,
    url: String,
    branch: Option<String>,
) -> Result<Vec<EpubTemplateRepository>, String> {
    let (owner, repo_name) = github_owner_repo(&url)?;
    let fallback_name = format!("{}-{}", owner, repo_name);
    let display_name = if name.trim().is_empty() {
        fallback_name.clone()
    } else {
        name.trim().to_string()
    };
    let mut repo_id = sanitize_filename_part(&display_name);
    if repo_id.trim().is_empty() {
        repo_id = sanitize_filename_part(&fallback_name);
    }
    let mut branch_name = branch.unwrap_or_else(default_template_branch);
    if branch_name.trim().is_empty() {
        branch_name = default_template_branch();
    }

    let mut repos = read_template_repositories(&app)?;
    let repo = EpubTemplateRepository {
        id: repo_id,
        name: display_name,
        url: url.trim().to_string(),
        branch: branch_name.trim().to_string(),
        last_synced: String::new(),
    };
    repos.retain(|item| item.id != repo.id && item.url != repo.url);
    repos.push(repo);
    write_template_repositories(&app, &repos)?;
    Ok(repos)
}

#[tauri::command]
async fn sync_epub_template_repository(
    app: tauri::AppHandle,
    repository_id: String,
) -> Result<EpubTemplateRepositoryIndex, String> {
    let mut repos = read_template_repositories(&app)?;
    let repo = repos
        .iter()
        .find(|item| item.id == repository_id)
        .cloned()
        .ok_or_else(|| "模板仓库不存在".to_string())?;
    let index_url = raw_github_url(&repo, "index.json")?;
    let client = reqwest::Client::builder()
        .user_agent("TEpub-Editor")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let text = client
        .get(index_url)
        .send()
        .await
        .map_err(|e| format!("拉取模板索引失败: {}", e))?
        .error_for_status()
        .map_err(|e| format!("模板索引响应异常: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取模板索引失败: {}", e))?;
    let index: EpubTemplateRepositoryIndex =
        serde_json::from_str(&text).map_err(|e| format!("解析模板索引失败: {}", e))?;
    for item in &mut repos {
        if item.id == repository_id {
            item.last_synced = chrono::Local::now().to_rfc3339();
        }
    }
    write_template_repositories(&app, &repos)?;
    Ok(index)
}

#[tauri::command]
async fn install_remote_epub_template(
    app: tauri::AppHandle,
    repository_id: String,
    template: EpubRemoteTemplate,
) -> Result<EpubTemplateInstallResult, String> {
    let repos = read_template_repositories(&app)?;
    let repo = repos
        .iter()
        .find(|item| item.id == repository_id)
        .cloned()
        .ok_or_else(|| "模板仓库不存在".to_string())?;
    let template_dir = Path::new(&template.path)
        .parent()
        .and_then(|p| p.to_str())
        .ok_or_else(|| "模板 path 需要指向 template.json".to_string())?
        .replace('\\', "/");
    let tree_url = github_tree_url(&repo, &template_dir)?;
    let client = reqwest::Client::builder()
        .user_agent("TEpub-Editor")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let tree_text = client
        .get(tree_url)
        .send()
        .await
        .map_err(|e| format!("拉取模板文件树失败: {}", e))?
        .error_for_status()
        .map_err(|e| format!("模板文件树响应异常: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取模板文件树失败: {}", e))?;
    let tree_json: serde_json::Value =
        serde_json::from_str(&tree_text).map_err(|e| format!("解析模板文件树失败: {}", e))?;

    let install_root = epub_templates_root(&app)?
        .join("github")
        .join(sanitize_filename_part(&repository_id))
        .join(sanitize_filename_part(&template.id));
    ensure_dir(&install_root)?;
    let entries = tree_json
        .get("tree")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "GitHub tree 响应缺少 tree 字段".to_string())?;

    let mut file_count = 0usize;
    for entry in entries {
        if entry.get("type").and_then(|v| v.as_str()) != Some("blob") {
            continue;
        }
        let Some(path) = entry.get("path").and_then(|v| v.as_str()) else {
            continue;
        };
        let relative = path
            .trim_start_matches(&template_dir)
            .trim_start_matches('/')
            .replace('\\', "/");
        if relative.is_empty() || relative.contains("..") {
            continue;
        }
        let raw_url = raw_github_url(&repo, path)?;
        let bytes = client
            .get(raw_url)
            .send()
            .await
            .map_err(|e| format!("下载模板文件失败 {}: {}", relative, e))?
            .error_for_status()
            .map_err(|e| format!("模板文件响应异常 {}: {}", relative, e))?
            .bytes()
            .await
            .map_err(|e| format!("读取模板文件失败 {}: {}", relative, e))?;
        let target = install_root.join(relative);
        if let Some(parent) = target.parent() {
            ensure_dir(parent)?;
        }
        fs::write(&target, &bytes).map_err(|e| format!("写入模板文件失败: {}", e))?;
        file_count += 1;
    }

    Ok(EpubTemplateInstallResult {
        template_id: template.id,
        local_path: install_root.to_string_lossy().to_string(),
        file_count,
    })
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

// 入库前自动检测并修复 EPUB 加密 / 混淆 / 错误文件名。
// 处理结果落到系统临时目录，调用方负责通过 IngestTempFile 回收。
// 返回值：
//   Ok(None)  ：源文件已是合法 EPUB，无需处理
//   Ok(Some)  ：(临时清洗文件 path, 动作描述)
//   Err       ：检测到非 ZIP 且无法用通用密钥解混淆
fn try_prepare_epub_for_ingest(source: &Path) -> Result<Option<(PathBuf, String)>, String> {
    let bytes = fs::read(source).map_err(|e| format!("读取 EPUB 失败: {}", e))?;

    // 在系统临时目录构造一个"虚拟 source"，使 build_processed_epub_path /
    // rebuild_epub_with_sanitized_names_from_bytes 把输出落在临时目录而非源目录。
    let temp_dir = std::env::temp_dir();
    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("book");
    let virtual_source = temp_dir.join(format!(
        "tepub_ingest_{}_{}.epub",
        uuid::Uuid::new_v4(),
        stem
    ));

    // Case 1：源已是 ZIP，检查伪加密位 + 文件名混淆
    if is_zip_archive_bytes(&bytes) {
        let mut bytes_mut = bytes;
        let fixed_encryption_flag = clear_zip_encryption_flags(&mut bytes_mut);

        if let Some(out_path) =
            rebuild_epub_with_sanitized_names_from_bytes(&virtual_source, &bytes_mut, "")?
        {
            let action = if fixed_encryption_flag {
                "已自动解除伪加密并修复文件名混淆"
            } else {
                "已自动修复文件名混淆"
            };
            return Ok(Some((out_path, action.to_string())));
        }

        if fixed_encryption_flag {
            let out_path = build_processed_epub_path(&virtual_source, "");
            fs::write(&out_path, &bytes_mut)
                .map_err(|e| format!("写入修复文件失败: {}", e))?;
            return Ok(Some((out_path, "已自动解除伪加密".to_string())));
        }
        return Ok(None);
    }

    // Case 2：源非 ZIP，尝试 XOR 解混淆
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
        let _ = clear_zip_encryption_flags(&mut decoded);
        if is_zip_archive_bytes(&decoded) {
            if let Some(out_path) =
                rebuild_epub_with_sanitized_names_from_bytes(&virtual_source, &decoded, "")?
            {
                return Ok(Some((out_path, "已自动解混淆并修复可读格式".to_string())));
            }
            let out_path = build_processed_epub_path(&virtual_source, "");
            fs::write(&out_path, &decoded)
                .map_err(|e| format!("写入解混淆文件失败: {}", e))?;
            return Ok(Some((out_path, "已自动解混淆并修复可读格式".to_string())));
        }
    }

    Err("该 EPUB 可能经过非通用加密/混淆，当前版本暂无法自动处理".to_string())
}

// RAII：函数结束时自动删除入库流程产生的临时清洗文件
struct IngestTempFile(Option<PathBuf>);
impl Drop for IngestTempFile {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            let _ = fs::remove_file(p);
        }
    }
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

#[derive(Serialize, Clone)]
struct CoverSearchResult {
    id: String,
    title: String,
    author: String,
    image_url: String,
    page_url: String,
    source: String,
    preferred: bool,
}

fn percent_encode_component(input: &str) -> String {
    let mut out = String::new();
    for b in input.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn normalize_cover_url(raw: &str) -> String {
    let mut url = raw
        .replace("\\/", "/")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .trim()
        .to_string();
    if url.starts_with("//") {
        url = format!("https:{}", url);
    }
    url
}

fn html_unescape_basic(raw: &str) -> String {
    raw.replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#x22;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn compact_match_key(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace() && *c != '《' && *c != '》')
        .collect::<String>()
        .to_lowercase()
}

fn is_preferred_cover_source(image_url: &str, page_url: &str, source: &str) -> bool {
    let image = image_url.to_lowercase();
    let page = page_url.to_lowercase();
    let source = source.to_lowercase();
    image.contains("bookcover.yuewen.com")
        || image.contains("icode.qq.com")
        || image.contains("p9-novel-sign.byteimg.com")
        || image.contains("fanqienovel.com")
        || page.contains("m.qidian.com")
        || page.contains("icode.qq.com")
        || page.contains("bookcover.yuewen.com")
        || page.contains("fanqienovel.com")
        || page.contains("p9-novel-sign.byteimg.com")
        || source.contains("m.qidian.com")
        || source.contains("icode.qq.com")
        || source.contains("bookcover.yuewen.com")
        || source.contains("fanqienovel.com")
        || source.contains("p9-novel-sign.byteimg.com")
}

fn cover_download_referer(url: &str) -> &'static str {
    let lower = url.to_lowercase();
    if lower.contains("byteimg.com") || lower.contains("fanqienovel.com") {
        "https://fanqienovel.com/"
    } else {
        "https://m.qidian.com/"
    }
}

fn host_from_url(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
}

fn json_num(value: &serde_json::Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(number) = value.get(*key).and_then(|v| v.as_f64()) {
            return Some(number);
        }
        if let Some(text) = value.get(*key).and_then(|v| v.as_str()) {
            if let Ok(number) = text.parse::<f64>() {
                return Some(number);
            }
        }
    }
    None
}

#[tauri::command]
async fn search_book_covers(title: String, author: String) -> Result<Vec<CoverSearchResult>, String> {
    let clean_title = title.trim();
    let clean_author = author.trim();
    if clean_title.is_empty() || clean_title == "书名" || clean_title == "涔﹀悕" {
        return Err("请先填写书名".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) TEpub-Editor/0.5")
        .build()
        .map_err(|e| format!("初始化封面搜索失败: {}", e))?;
    let image_meta_re = Regex::new(r#"m="([^"]+)""#)
        .map_err(|e| format!("封面结果解析失败: {}", e))?;
    let mut results: Vec<(i32, CoverSearchResult)> = Vec::new();
    let mut seen = HashSet::new();
    let title_key = compact_match_key(clean_title);
    let author_key = compact_match_key(clean_author);

    let mut query_texts = vec![clean_title.to_string()];
    if !clean_author.is_empty() && clean_author != "作者" && clean_author != "浣滆€?" {
        query_texts.push(format!("{} {}", clean_title, clean_author));
    }
    query_texts.push(format!("{} 小说封面", clean_title));

    for (query_idx, query_text) in query_texts.iter().enumerate() {
        let query = percent_encode_component(query_text);
        let search_url = format!(
            "https://cn.bing.com/images/search?q={}&form=HDRSC2&first=1",
            query
        );
        let html = client
            .get(&search_url)
            .header("Referer", "https://cn.bing.com/images")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .send()
            .await
            .map_err(|e| format!("搜索封面失败: {}", e))?
            .text()
            .await
            .map_err(|e| format!("读取封面搜索结果失败: {}", e))?;

        for (idx, item) in image_meta_re.captures_iter(&html).enumerate() {
            let captures = match item {
                Ok(c) => c,
                Err(_) => continue,
            };
            let raw_meta = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let meta_text = html_unescape_basic(raw_meta);
            let meta = match serde_json::from_str::<serde_json::Value>(&meta_text) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let image_url = match meta
                .get("murl")
                .and_then(|value| value.as_str())
                .or_else(|| meta.get("turl").and_then(|value| value.as_str()))
            {
                Some(url) => normalize_cover_url(url),
                None => continue,
            };
            if image_url.is_empty()
                || !(image_url.starts_with("https://") || image_url.starts_with("http://"))
                || !seen.insert(image_url.clone())
            {
                continue;
            }

            let page_url = meta
                .get("purl")
                .and_then(|value| value.as_str())
                .map(normalize_cover_url)
                .unwrap_or_default();
            let result_title = meta
                .get("t")
                .and_then(|value| value.as_str())
                .map(html_unescape_basic)
                .unwrap_or_else(|| clean_title.to_string());
            let id = meta
                .get("md5")
                .and_then(|value| value.as_str())
                .or_else(|| meta.get("cid").and_then(|value| value.as_str()))
                .map(|value| value.to_string())
                .unwrap_or_else(|| format!("{}-{}", query_idx, idx));
            let source = {
                let page_host = host_from_url(&page_url);
                if page_host.is_empty() {
                    host_from_url(&image_url)
                } else {
                    page_host
                }
            };
            let preferred = is_preferred_cover_source(&image_url, &page_url, &source);

            let result_title_key = compact_match_key(&result_title);
            let page_key = compact_match_key(&page_url);
            let source_key = compact_match_key(&source);
            let image_key = compact_match_key(&image_url);
            let combined_key = format!("{}{}{}{}", result_title_key, page_key, source_key, image_key);
            let has_title_match = !title_key.is_empty() && combined_key.contains(&title_key);

            let mut score = if preferred { 180 } else { 0 };
            score += match query_idx {
                0 => 70,
                1 => 30,
                _ => 10,
            };
            if !title_key.is_empty() && result_title_key == title_key {
                score += 150;
            } else if has_title_match {
                score += 100;
            } else if !title_key.is_empty() && result_title_key.contains(&title_key) {
                score += 70;
            } else {
                score -= 80;
            }
            if !author_key.is_empty()
                && (result_title_key.contains(&author_key) || page_key.contains(&author_key))
            {
                score += 20;
            }
            if source.contains("icode.qq.com") {
                score += 90;
            }
            if let (Some(width), Some(height)) = (
                json_num(&meta, &["w", "width", "ow", "imgw"]),
                json_num(&meta, &["h", "height", "oh", "imgh"]),
            ) {
                if width > 0.0 && height > 0.0 {
                    let ratio = width / height;
                    if (0.62..=0.82).contains(&ratio) {
                        score += 35;
                    } else if ratio >= 1.0 {
                        score -= 45;
                    }
                }
            }
            score -= (query_idx as i32 * 20) + idx as i32;

            results.push((
                score,
                CoverSearchResult {
                    id,
                    title: if result_title.trim().is_empty() {
                        clean_title.to_string()
                    } else {
                        result_title
                    },
                    author: source.clone(),
                    image_url,
                    page_url,
                    source,
                    preferred,
                },
            ));
        }
    }

    results.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(results.into_iter().take(12).map(|(_, result)| result).collect())
}
#[tauri::command]
async fn download_cover_to_temp(image_url: String, title: String) -> Result<String, String> {
    let url = normalize_cover_url(&image_url);
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("封面地址无效".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) TEpub-Editor/0.5")
        .build()
        .map_err(|e| format!("初始化封面下载失败: {}", e))?;
    let bytes = client
        .get(&url)
        .header("Referer", cover_download_referer(&url))
        .send()
        .await
        .map_err(|e| format!("下载封面失败: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("读取封面数据失败: {}", e))?;
    if bytes.is_empty() {
        return Err("下载到的封面为空".to_string());
    }
    if bytes.len() > 12 * 1024 * 1024 {
        return Err("封面图片过大".to_string());
    }

    let ext = detect_image_ext(&bytes);
    let base = sanitize_filename_part(&title);
    let stem = if base.is_empty() { "cover".to_string() } else { base };
    let dir = std::env::temp_dir().join("TEpub-Editor").join("covers");
    fs::create_dir_all(&dir).map_err(|e| format!("创建封面缓存目录失败: {}", e))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("生成封面缓存文件名失败: {}", e))?
        .as_millis();
    let path = dir.join(format!("{}-{}.{}", stem, now, ext));
    fs::write(&path, &bytes).map_err(|e| format!("保存封面失败: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn calculate_md5(content: String) -> String {
    format!("{:x}", md5::compute(content.as_bytes()))
}

#[tauri::command]
async fn save_history(
    app: tauri::AppHandle,
    original_path: String,
    content: String,
) -> Result<(), String> {
    let path = Path::new(&original_path);
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let history_dir = get_history_base_dir(Some(&app));
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
async fn get_history_list(
    app: tauri::AppHandle,
    original_path: String,
) -> Vec<HistoryMeta> {
    let path = Path::new(&original_path);
    let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let history_dir = get_history_base_dir(Some(&app));
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

#[tauri::command]
fn list_library_fonts(app: tauri::AppHandle) -> Result<Vec<LibraryFontInfo>, String> {
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
fn import_library_font(app: tauri::AppHandle, path: String) -> Result<LibraryFontInfo, String> {
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
fn rename_library_font(
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
fn delete_library_font(app: tauri::AppHandle, file_name: String) -> Result<(), String> {
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

#[tauri::command]
fn list_style_templates(app: tauri::AppHandle) -> Result<Vec<StyleTemplateInfo>, String> {
    let root = style_templates_root(&app)?;
    ensure_dir(&root)?;

    let builtin_path = builtin_style_template_path(&app)?;
    let mut templates = vec![build_style_template_info(&builtin_path, true)?];

    let entries = fs::read_dir(&root).map_err(|e| format!("读取样式模板目录失败: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("读取样式模板目录项失败: {}", e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext != "css" {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("builtin.css") {
            continue;
        }
        templates.push(build_style_template_info(&path, false)?);
    }

    let mut imported = templates.split_off(1);
    imported.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    templates.extend(imported);
    Ok(templates)
}

#[tauri::command]
fn read_style_template(
    app: tauri::AppHandle,
    id: String,
) -> Result<StyleTemplateContent, String> {
    let is_builtin = id == "builtin";
    let path = if is_builtin {
        builtin_style_template_path(&app)?
    } else {
        style_template_path(&app, &id)?
    };
    let main_css = if path.exists() {
        let css = fs::read_to_string(&path).map_err(|e| format!("读取样式模板失败: {}", e))?;
        if is_builtin && is_stale_builtin_style_template_css(&css) {
            fs::remove_file(&path).map_err(|e| format!("升级内置样式模板失败: {}", e))?;
            String::new()
        } else {
            css
        }
    } else {
        String::new()
    };
    Ok(StyleTemplateContent {
        id: if is_builtin { "builtin".to_string() } else { id },
        name: if is_builtin {
            "内置模板".to_string()
        } else {
            style_template_name_from_path(&path)
        },
        main_css,
        is_builtin,
    })
}

#[tauri::command]
fn import_style_template(app: tauri::AppHandle, path: String) -> Result<StyleTemplateInfo, String> {
    let normalized = normalize_local_file_path(&path);
    let source = PathBuf::from(&normalized);
    if !source.exists() || !source.is_file() {
        return Err("样式模板文件不存在".to_string());
    }

    let ext = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "css" {
        return Err("仅支持导入 CSS 样式模板".to_string());
    }

    let css = fs::read_to_string(&source).map_err(|e| format!("读取样式模板失败: {}", e))?;
    let root = style_templates_root(&app)?;
    ensure_dir(&root)?;

    let source_stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("style-template");
    let safe_stem = {
        let candidate = sanitize_filename_part(source_stem);
        if candidate.is_empty() {
            "style-template".to_string()
        } else {
            candidate
        }
    };

    let mut target = root.join(format!("{}.css", safe_stem));
    let mut seq = 2usize;
    while target.exists() {
        let existing = fs::read_to_string(&target).unwrap_or_default();
        if existing == css {
            return build_style_template_info(&target, false);
        }
        target = root.join(format!("{}-{}.css", safe_stem, seq));
        seq += 1;
    }

    fs::write(&target, css).map_err(|e| format!("保存样式模板失败: {}", e))?;
    build_style_template_info(&target, false)
}

#[tauri::command]
fn save_style_template(
    app: tauri::AppHandle,
    id: String,
    main_css: String,
) -> Result<StyleTemplateInfo, String> {
    let path = if id == "builtin" {
        builtin_style_template_path(&app)?
    } else {
        style_template_path(&app, &id)?
    };
    let root = style_templates_root(&app)?;
    ensure_dir(&root)?;
    fs::write(&path, main_css).map_err(|e| format!("保存样式模板失败: {}", e))?;
    build_style_template_info(&path, id == "builtin")
}

#[tauri::command]
fn restore_builtin_style_template(app: tauri::AppHandle) -> Result<(), String> {
    let path = builtin_style_template_path(&app)?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("恢复内置样式失败: {}", e))?;
    }
    Ok(())
}

fn is_stale_builtin_style_template_css(css: &str) -> bool {
    let normalized = css.trim();
    if normalized.is_empty() {
        return false;
    }

    normalized.contains("TEpub template schema: 1")
        && !normalized.contains(".te-volume-subtitle")
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
            let is_first_heading = chapters.is_empty();
            let is_meta = !is_vol_keyword
                && mobile_is_meta_title(line_trim)
                && (lvl == 1 || is_first_heading);
            
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
    let asset_slot_placements = parse_asset_slot_placements(main_css);
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
        let mime = image_mime_from_ext(&cover_ext);
        manifest_items.push_str(&format!(r#"<item id="cover-image" href="Images/cover.{}" media-type="{}" properties="cover-image"/>"#, cover_ext, mime));
    }

    // 写入资产文件
    let mut image_slot_hrefs: HashMap<String, String> = HashMap::new();
    let font_subset_text = if metadata.subset_fonts {
        Some(build_font_subset_text(&content, &metadata))
    } else {
        None
    };
    for (i, asset) in metadata.assets.iter().enumerate() {
        let normalized_asset_path = normalize_local_file_path(&asset.path);
        if let Ok(mut asset_bytes) = fs::read(&normalized_asset_path) {
            let sub_dir = match asset.category.as_str() {
                "fonts" => "Fonts",
                "images" => "Images",
                _ => "Other",
            };
            let ext = Path::new(&asset.name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let safe_name = if !asset.role.trim().is_empty() && asset.category == "images" {
                let stem = asset_slot_file_stem(&asset.role);
                if ext.is_empty() {
                    format!("{}.jpg", stem)
                } else {
                    format!("{}.{}", stem, ext)
                }
            } else {
                let sanitized = sanitize_filename_part(&asset.name);
                if sanitized.is_empty() {
                    format!("asset_{}", i)
                } else {
                    sanitized
                }
            };

            if asset.category == "fonts" {
                if let Some(subset_text) = font_subset_text.as_deref() {
                    match try_subset_font_bytes(&asset_bytes, &ext, subset_text) {
                        Ok(subsetted) => asset_bytes = subsetted,
                        Err(err) => eprintln!("字体子集化已回退原文件: {} ({})", asset.name, err),
                    }
                }
            }

            let asset_filename = format!("OEBPS/{}/{}", sub_dir, safe_name);
            zip.start_file(&asset_filename, options)
                .map_err(|e| e.to_string())?;
            zip.write_all(&asset_bytes).map_err(|e| e.to_string())?;

            let href = format!("{}/{}", sub_dir, safe_name);
            let mime = asset_manifest_mime(&safe_name);
            if !asset.role.trim().is_empty() && asset.category == "images" {
                image_slot_hrefs.insert(asset.role.clone(), href.clone());
            }
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
        let mut class_attr = "te-book-body te-chapter-page";

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
            class_attr = "te-book-body te-intro-page";
            if i == 0 && has_cover {
                html_body.push_str(&format!(
                    "  <div class=\"te-cover-wrap\"><img class=\"te-cover-image\" src=\"../Images/cover.{}\" alt=\"封面\" /><br/>\n    <span class=\"book-name\">{}</span>\n  </div>\n\n",
                    cover_ext,
                    escape_xml(&metadata.title)
                ));
            }
            html_body.push_str(&format!(
                "  <h1 class=\"te-intro-title\" title=\"{}\"><span><b>{}</b></span></h1>\n",
                safe_display_title,
                safe_display_title
            ));
            append_text_body_lines(
                &mut html_body,
                body_lines,
                true,
                first_image_slot_for_placement(
                    &image_slot_hrefs,
                    &asset_slot_placements,
                    "replace-ellipsis",
                    "dividerImage",
                ),
            );
        } else {
            match chapter.level {
                1 => {
                    class_attr = "te-book-body te-volume-page";
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
                    } else {
                        safe_display_title.clone()
                    };

                    let inserted_volume_head = if let Some((role, href)) = first_image_slot_for_placement(
                        &image_slot_hrefs,
                        &asset_slot_placements,
                        "volume-before-title",
                        "volumeHead",
                    ) {
                        html_body.push_str(&image_slot_html(role, href));
                        true
                    } else {
                        false
                    };

                    if !inserted_volume_head {
                        class_attr = "te-book-body te-volume-page te-volume-page--no-image";
                    }

                    html_body.push_str(&format!(
                        "  <h1 class=\"te-volume-title\" title=\"{}\"><br /><br />\n  {}</h1>\n  <p class=\"te-volume-subtitle\">{}</p>\n",
                        safe_display_title, vertical_num, formatted_name.trim()
                    ));

                    // Add body content for Volume if they exist, to prevent loss of potential inner-body text
                    append_text_body_lines(
                        &mut html_body,
                        body_lines,
                        true,
                        first_image_slot_for_placement(
                            &image_slot_hrefs,
                            &asset_slot_placements,
                            "replace-ellipsis",
                            "dividerImage",
                        ),
                    );
                }
                3 => {
                    let safe_chap_num = escape_xml(&chap_num_raw);
                    let safe_chap_name = escape_xml(&chap_name_raw);
                    
                    let inserted_chapter_head = if let Some((role, href)) = first_image_slot_for_placement(
                        &image_slot_hrefs,
                        &asset_slot_placements,
                        "chapter-before-title",
                        "chapterHead",
                    ) {
                        html_body.push_str(&image_slot_html(role, href));
                        true
                    } else {
                        false
                    };

                    if !inserted_chapter_head {
                        class_attr = "te-book-body te-chapter-page te-chapter-page--no-image";
                    }

                    if !safe_chap_num.is_empty() {
                        html_body.push_str(&format!(
                            "  <h3 class=\"te-chapter-title\"><span class=\"te-chapter-number\">{}</span><br/><b class=\"te-chapter-name\">{}</b></h3>\n",
                            safe_chap_num, safe_chap_name
                        ));
                    } else {
                        html_body.push_str(&format!(
                            "  <h3 class=\"te-chapter-title\">{}</h3>\n",
                            safe_display_title
                        ));
                    }
                    
                    append_text_body_lines(
                        &mut html_body,
                        body_lines,
                        true,
                        first_image_slot_for_placement(
                            &image_slot_hrefs,
                            &asset_slot_placements,
                            "replace-ellipsis",
                            "dividerImage",
                        ),
                    );
                }
                _ => {
                    html_body.push_str(&format!(
                        "  <h{} class=\"te-chapter-title\">{}</h{}>\n",
                        chapter.level, safe_display_title, chapter.level
                    ));
                    append_text_body_lines(
                        &mut html_body,
                        body_lines,
                        true,
                        first_image_slot_for_placement(
                            &image_slot_hrefs,
                            &asset_slot_placements,
                            "replace-ellipsis",
                            "dividerImage",
                        ),
                    );
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
            format!(" class=\"{}\"", class_attr),
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
    for tag in &metadata.tags {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            continue;
        }
        extra_metadata.push_str(&format!(
            "    <dc:subject>{}</dc:subject>\n",
            escape_xml(trimmed)
        ));
    }
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
async fn extract_epub(
    app: tauri::AppHandle,
    epub_path: String,
) -> Result<Vec<EpubFileNode>, String> {
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
        // 2. 在书库 _data/extract/ 下创建临时子目录并解压
        let extract_root = library_extract_dir(&app)
            .unwrap_or_else(|_| std::env::temp_dir().join("tepub-extract"));
        ensure_dir(&extract_root).ok();
        let temp_dir = tempfile::Builder::new()
            .prefix("epub_")
            .tempdir_in(&extract_root)
            .or_else(|_| TempDir::new())
            .map_err(|e| format!("无法创建临时目录: {}", e))?;
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
        let lower_file_name = file_name.to_ascii_lowercase();
        let file_type = if lower_file_name.ends_with(".html")
            || lower_file_name.ends_with(".htm")
            || lower_file_name.ends_with(".xhtml")
        {
            "html"
        } else if lower_file_name.ends_with(".css") {
            "css"
        } else if lower_file_name.ends_with(".xml")
            || lower_file_name.ends_with(".opf")
            || lower_file_name.ends_with(".ncx")
        {
            "xml"
        } else if lower_file_name.ends_with(".jpg")
            || lower_file_name.ends_with(".jpeg")
            || lower_file_name.ends_with(".png")
            || lower_file_name.ends_with(".gif")
            || lower_file_name.ends_with(".webp")
            || lower_file_name.ends_with(".bmp")
            || lower_file_name.ends_with(".svg")
        {
            "image"
        } else if lower_file_name.ends_with(".ttf")
            || lower_file_name.ends_with(".otf")
            || lower_file_name.ends_with(".woff")
            || lower_file_name.ends_with(".woff2")
        {
            "font"
        } else {
            "other"
        }
        .to_string();

        // 提取标题 (如果是 HTML)
        let mut title = None;
        let mut resolution = None;

        if file_type == "html" {
            if let Ok(content) = fs::read_to_string(full_path) {
                title = extract_html_heading_title(&content);
            }
        }

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
async fn get_epub_temp_dir_path(epub_path: String) -> Result<String, String> {
    let cache_guard = EPUB_CACHE.lock().unwrap();
    if let Some(ref cache) = *cache_guard {
        if cache.epub_path == epub_path {
            if let Some(ref temp) = cache.temp_dir {
                return Ok(temp.path().to_string_lossy().to_string());
            }
        }
    }
    Err("EPUB 未加载或缓存失效".to_string())
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
    tauri::async_runtime::spawn_blocking(move || {
        use zip::write::FileOptions;

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

        let zip_file_path = format!("{}.zip.tmp", epub_path);
        let backup_file_path = format!("{}.bak.tmp", epub_path);

        if Path::new(&zip_file_path).exists() {
            let _ = fs::remove_file(&zip_file_path);
        }
        if Path::new(&backup_file_path).exists() {
            let _ = fs::remove_file(&backup_file_path);
        }

        let zip_file =
            fs::File::create(&zip_file_path).map_err(|e| format!("创建 ZIP 失败: {}", e))?;
        let mut zip_writer = zip::ZipWriter::new(zip_file);

        let options_deflated =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let options_stored =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for entry in WalkDir::new(&temp_path).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let full_path = entry.path();
            let relative_path = full_path
                .strip_prefix(&temp_path)
                .map_err(|e| format!("计算相对路径失败: {}", e))?;
            let path_str = relative_path.to_string_lossy().replace("\\", "/");

            let options = if path_str == "mimetype" {
                options_stored
            } else {
                options_deflated
            };

            zip_writer
                .start_file(&path_str, options)
                .map_err(|e| format!("写入文件失败: {}", e))?;

            let content = fs::read(full_path).map_err(|e| format!("读取文件失败: {}", e))?;
            zip_writer
                .write_all(&content)
                .map_err(|e| format!("写入内容失败: {}", e))?;
        }

        zip_writer
            .finish()
            .map_err(|e| format!("完成 ZIP 失败: {}", e))?;

        if Path::new(&epub_path).exists() {
            fs::rename(&epub_path, &backup_file_path)
                .map_err(|e| format!("备份原 EPUB 失败: {}", e))?;
        }

        if let Err(e) = fs::rename(&zip_file_path, &epub_path) {
            if Path::new(&backup_file_path).exists() {
                let _ = fs::rename(&backup_file_path, &epub_path);
            }
            return Err(format!("替换 EPUB 失败: {}", e));
        }

        if Path::new(&backup_file_path).exists() {
            let _ = fs::remove_file(&backup_file_path);
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("保存 EPUB 任务失败: {}", e))?
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
        // 跳过我们注入的 --action= 之类的 flag
        for a in args.iter().skip(1) {
            if a.starts_with("--") {
                continue;
            }
            return Some(a.clone());
        }
    }
    None
}

// 启动信息：file_path + 可选 action (来自 --action=X 标志，决定路由到 reader/editor/epub-editor)
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct LaunchInfo {
    file_path: Option<String>,
    file_paths: Vec<String>,
    action: Option<String>,
}

#[tauri::command]
fn get_launch_info() -> LaunchInfo {
    let args: Vec<String> = std::env::args().collect();
    let mut files: Vec<String> = Vec::new();
    let mut action: Option<String> = None;
    for a in args.iter().skip(1) {
        if let Some(stripped) = a.strip_prefix("--action=") {
            action = Some(stripped.to_string());
        } else if a.starts_with("--") {
            // ignore unknown flags
        } else {
            files.push(a.clone());
        }
    }
    LaunchInfo {
        file_path: files.first().cloned(),
        file_paths: files,
        action,
    }
}

// ============================================================
// ===== Windows 文件关联（注册右键菜单 verb） =====
// ============================================================
//
// 三个 verb：
//   epub-read  → .epub 上的"EPUB 阅读"，启动时带 --action=reader
//   epub-edit  → .epub 上的"EPUB 编辑"，启动时带 --action=epub-editor
//   txt-make-epub → .txt 上的"制作 EPUB"，启动时带 --action=make-epub
//
// 写到 HKCU\Software\Classes（用户级，无需管理员），与安装版的 HKLM 注册并存且优先生效。
// 用 reg.exe 命令实现，避免引入 winreg crate 依赖。

#[cfg(target_os = "windows")]
fn run_reg_command(args: &[&str]) -> Result<(), String> {
    use std::process::Command;
    let out = Command::new("reg")
        .args(args)
        .output()
        .map_err(|e| format!("调用 reg.exe 失败: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // reg.exe DELETE 在 key 不存在时也返回非零，对我们而言不算真正失败
        if stderr.contains("找不到") || stderr.to_lowercase().contains("unable to find") {
            return Ok(());
        }
        Err(format!("reg 操作失败: {}", stderr))
    }
}

#[cfg(target_os = "windows")]
fn current_exe_quoted() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("获取 exe 路径失败: {}", e))?;
    Ok(exe.to_string_lossy().to_string())
}

#[cfg(target_os = "windows")]
fn install_verb(
    ext: &str,
    verb: &str,
    display: &str,
    action_flag: &str,
) -> Result<(), String> {
    let exe = current_exe_quoted()?;
    // 走 SystemFileAssociations：不依赖文件类型默认 ProgID，
    // 用户即便把 .epub 默认设给 Calibre/SumatraPDF，我们的右键菜单依然会出现。
    let verb_key = format!(
        r"HKCU\Software\Classes\SystemFileAssociations\{}\shell\{}",
        ext, verb
    );
    run_reg_command(&["ADD", &verb_key, "/v", "MUIVerb", "/d", display, "/f"])?;
    let icon_value = format!(r#""{}",0"#, exe);
    run_reg_command(&["ADD", &verb_key, "/v", "Icon", "/d", &icon_value, "/f"])?;
    let cmd_key = format!("{}\\command", verb_key);
    let cmd_value = format!(r#""{}" {} "%1""#, exe, action_flag);
    run_reg_command(&["ADD", &cmd_key, "/ve", "/d", &cmd_value, "/f"])?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_verb(ext: &str, verb: &str, legacy_progid: &str) -> Result<(), String> {
    // 新位置：SystemFileAssociations
    let verb_key_new = format!(
        r"HKCU\Software\Classes\SystemFileAssociations\{}\shell\{}",
        ext, verb
    );
    let _ = run_reg_command(&["DELETE", &verb_key_new, "/f"]);
    // 兼容旧版本：如果 0.4.6 之前曾把 verb 挂在自定义 ProgID 下，一并清掉
    let verb_key_old = format!(
        r"HKCU\Software\Classes\{}\shell\{}",
        legacy_progid, verb
    );
    let _ = run_reg_command(&["DELETE", &verb_key_old, "/f"]);
    Ok(())
}

#[tauri::command]
async fn set_file_assoc(verb: String, enabled: bool) -> Result<(), String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (verb, enabled);
        return Err("文件关联仅支持 Windows".to_string());
    }
    #[cfg(target_os = "windows")]
    {
        // 把 reg.exe 调用搬到 spawn_blocking，避免阻塞 Tauri IPC 线程导致 UI 卡顿
        let res: Result<(), String> = tauri::async_runtime::spawn_blocking(move || {
            // verb -> (扩展名, 旧 ProgID, shell verb 名, 显示文字, --action= 值)
            let (ext, legacy_progid, shell_verb, display, action) = match verb.as_str() {
                "epub-read" => (
                    ".epub",
                    "TEpubEditor.epub",
                    "TEpubEditorRead",
                    "用 TEpub-Editor 阅读",
                    "--action=reader",
                ),
                "epub-edit" => (
                    ".epub",
                    "TEpubEditor.epub",
                    "TEpubEditorEdit",
                    "用 TEpub-Editor 编辑",
                    "--action=epub-editor",
                ),
                "txt-make-epub" => (
                    ".txt",
                    "TEpubEditor.txt",
                    "TEpubEditorMakeEpub",
                    "用 TEpub-Editor 制作 EPUB",
                    "--action=make-epub",
                ),
                _ => return Err(format!("未知的关联 verb: {}", verb)),
            };

            if enabled {
                install_verb(ext, shell_verb, display, action)
            } else {
                uninstall_verb(ext, shell_verb, legacy_progid)
            }
        })
        .await
        .map_err(|e| format!("注册任务失败: {}", e))?;
        res
    }
}

// 在系统文件管理器里打开并选中指定文件（Windows: explorer.exe /select,"path"）
#[tauri::command]
async fn reveal_in_explorer(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    let res: Result<(), String> = tauri::async_runtime::spawn_blocking(move || {
        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            // /select 后跟逗号 + 引号包路径
            let arg = format!("/select,{}", path);
            Command::new("explorer.exe")
                .arg(&arg)
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
            Ok(())
        }
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("open")
                .args(["-R", &path])
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
            Ok(())
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            use std::process::Command;
            // Linux: 打开父目录
            let parent = std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string());
            Command::new("xdg-open")
                .arg(&parent)
                .spawn()
                .map_err(|e| format!("打开文件管理器失败: {}", e))?;
            Ok(())
        }
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?;
    res
}

// 重命名书库内的本地文件，并同步 library.json 的 file_path / filename。
// - 仅在「书在 books_dir」或「ref_only 文件可访问」时生效
// - 同名冲突返回 BOOK_FILE_COLLISION:{suggested_filename}
// - 自动按文件类型补回扩展名（用户输入"新名"或"新名.epub"都行）
#[tauri::command]
async fn rename_book_file(
    app: tauri::AppHandle,
    book_id: String,
    new_filename: String,
) -> Result<BookEntry, String> {
    let mut data = read_library_data(&app)?;
    let idx = data
        .books
        .iter()
        .position(|b| b.id == book_id)
        .ok_or_else(|| format!("未找到图书: {}", book_id))?;

    let book = data.books[idx].clone();
    let cur_path = PathBuf::from(&book.file_path);
    if !cur_path.exists() {
        return Err(format!("源文件不存在: {}", book.file_path));
    }
    let parent = cur_path
        .parent()
        .ok_or_else(|| "源文件无父目录".to_string())?
        .to_path_buf();
    let ext = book.file_type.clone();

    // 清洗用户输入：去扩展名 + sanitize + 补扩展名
    let raw = new_filename.trim();
    if raw.is_empty() {
        return Err("文件名不能为空".to_string());
    }
    let stripped = raw
        .strip_suffix(&format!(".{}", ext))
        .or_else(|| raw.strip_suffix(&format!(".{}", ext.to_uppercase())))
        .unwrap_or(raw);
    let stem = sanitize_filename_part(stripped);
    if stem.is_empty() {
        return Err("文件名清洗后为空".to_string());
    }
    let target_name = format!("{}.{}", stem, ext);
    let target_path = parent.join(&target_name);

    if target_path == cur_path {
        // 没变更，直接返回当前条目
        return Ok(book);
    }
    if target_path.exists() {
        return Err(format!("BOOK_FILE_COLLISION:{}", target_name));
    }

    fs::rename(&cur_path, &target_path).map_err(|e| format!("重命名失败: {}", e))?;

    let entry = &mut data.books[idx];
    entry.file_path = target_path.to_string_lossy().to_string();
    entry.filename = target_name;
    let updated = entry.clone();
    write_library_data_atomic(&app, &data)?;
    Ok(updated)
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct RebuildSummary {
    renamed: u32,
    skipped: u32,
    failed: u32,
    failures: Vec<String>,
}

// 按当前 LibraryConfig 的 naming_template 把 books_dir 内所有书重命名一遍。
// - 仅处理位于 books_dir 内的副本（ref_only 不动用户原文件）
// - 同名冲突自动加 " (2)"、" (3)"… 后缀
// - 命名模式 = "source" 时跳过（保持当前文件名）
// - 处理过程中如果某本失败，记录到 failures 但不中断
#[tauri::command]
async fn rebuild_book_filenames(
    app: tauri::AppHandle,
    config: LibraryConfig,
) -> Result<RebuildSummary, String> {
    let mut data = read_library_data(&app)?;
    // 把传入的（可能比 disk 上更新的）config 同步进去，确保使用最新模板
    data.config = config.clone();
    let books_dir = library_books_dir(&app)?;

    let mut summary = RebuildSummary::default();
    if config.naming_mode.trim() == "source" {
        // 源文件名模式：不重命名任何已入库的书
        summary.skipped = data.books.len() as u32;
        write_library_data_atomic(&app, &data)?;
        return Ok(summary);
    }
    let template = effective_naming_template(&config);

    // 第一遍：决定每本书的目标文件名（含 (2)/(3) 防冲突）
    // 用 in-memory set 跟踪本批次会出现的新名，避免 a→b 和 c→b 撞车
    let mut planned: HashSet<String> = HashSet::new();
    // 把当前所有非本批次目标的文件名先收进去，避免目标 与 别的没动到的文件 撞车
    if let Ok(entries) = fs::read_dir(&books_dir) {
        for e in entries.flatten() {
            if let Some(name) = e.file_name().to_str() {
                planned.insert(name.to_string());
            }
        }
    }

    // 收集需要做的重命名：(idx, from_path, target_name)
    let mut plan: Vec<(usize, PathBuf, String)> = Vec::new();
    for (i, book) in data.books.iter().enumerate() {
        let cur_path = PathBuf::from(&book.file_path);
        if !cur_path.starts_with(&books_dir) || !cur_path.exists() {
            summary.skipped += 1;
            continue;
        }
        let ext = book.file_type.clone();
        let stem = render_filename_template(&template, book);
        let stem = if stem.is_empty() {
            sanitize_filename_part(&format!("{}-{}", book.title, book.author))
        } else {
            stem
        };
        let stem = if stem.is_empty() { "未命名".to_string() } else { stem };
        let mut target = format!("{}.{}", stem, ext);
        // 如果目标 == 当前名，不需要改
        if let Some(cur_name) = cur_path.file_name().and_then(|s| s.to_str()) {
            if cur_name == target {
                summary.skipped += 1;
                continue;
            }
            // 把当前名先从 planned 里抠掉，否则下面 (2) 后缀会从 2 起步
            planned.remove(cur_name);
        }
        // 解决与 planned 中其它名字的冲突
        if planned.contains(&target) {
            let mut n = 2;
            loop {
                let candidate = format!("{} ({}).{}", stem, n, ext);
                if !planned.contains(&candidate) {
                    target = candidate;
                    break;
                }
                n += 1;
                if n > 999 {
                    break;
                }
            }
        }
        planned.insert(target.clone());
        plan.push((i, cur_path, target));
    }

    // 第二遍：执行重命名并更新 entry
    // 为避免 A→B 同时 B→C 时 A→B 先执行覆盖了 B，先把所有被改动的文件改成临时名，再改成最终名
    let mut staged: Vec<(usize, PathBuf, PathBuf)> = Vec::new(); // (idx, tmp_path, final_path)
    for (idx, from, target_name) in plan {
        let tmp_name = format!(
            ".__rename_tmp_{}_{}",
            idx,
            uuid::Uuid::new_v4().simple()
        );
        let tmp_path = books_dir.join(&tmp_name);
        let final_path = books_dir.join(&target_name);
        if let Err(e) = fs::rename(&from, &tmp_path) {
            summary.failed += 1;
            summary.failures.push(format!(
                "{}: {}",
                from.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
                e
            ));
            continue;
        }
        staged.push((idx, tmp_path, final_path));
    }
    for (idx, tmp_path, final_path) in staged {
        match fs::rename(&tmp_path, &final_path) {
            Ok(_) => {
                let entry = &mut data.books[idx];
                entry.file_path = final_path.to_string_lossy().to_string();
                entry.filename = final_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                summary.renamed += 1;
            }
            Err(e) => {
                summary.failed += 1;
                summary.failures.push(format!(
                    "{} → {}: {}",
                    tmp_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(""),
                    final_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(""),
                    e
                ));
                // 尝试还原，失败的话只能在 failures 里留个记录
                let _ = fs::rename(&tmp_path, &final_path);
            }
        }
    }

    write_library_data_atomic(&app, &data)?;
    Ok(summary)
}

// ============================================================
// ===== Library (书库) - Phase 1: 数据结构 / load / save =====
// ============================================================

fn default_true() -> bool {
    true
}

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
    /// 入库时新书文件的命名方式：
    ///   "source"   = 沿用源文件名
    ///   "template" = 按 naming_template 渲染（默认）
    /// 空字符串等同于 "template"。
    #[serde(default)]
    naming_mode: String,
    /// 命名模板，支持占位符：
    ///   {title} {author} {subtitle} {series} {maker} {publisher}
    ///   {tags}  自动展开为 [标签1][标签2]…，无 tag 时为空串
    /// 空字符串时回退到 "{title}-{author}"。
    #[serde(default)]
    naming_template: String,
    #[serde(default = "default_true")]
    close_library_on_txt_open: bool,
    #[serde(default = "default_true")]
    close_library_on_epub_open: bool,
    #[serde(default = "default_true")]
    close_library_on_toolbox_open: bool,
    #[serde(default)]
    txt_editor_close_action: String,
    #[serde(default)]
    ai_proofing: AiProofingConfig,
    #[serde(default)]
    ai_providers: Vec<AiProviderConfig>,
    #[serde(default)]
    txt_ai_proofing: TxtAiProofingConfig,
    #[serde(default)]
    library_ai_match: LibraryAiMatchConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct AiProofingConfig {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_ai_base_url")]
    base_url: String,
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_ai_model")]
    model: String,
    #[serde(default = "default_ai_temperature")]
    temperature: f32,
    #[serde(default = "default_ai_max_chapter_chars")]
    max_chapter_chars: usize,
    #[serde(default = "default_ai_response_timeout_sec")]
    response_timeout_sec: u64,
    #[serde(default)]
    auto_approve: bool,
    #[serde(default)]
    extra_prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct AiProviderConfig {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default = "default_ai_base_url")]
    base_url: String,
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_ai_model")]
    model: String,
    #[serde(default = "default_ai_temperature")]
    temperature: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct TxtAiProofingConfig {
    #[serde(default)]
    provider_id: String,
    #[serde(default)]
    approval_provider_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct LibraryAiMatchConfig {
    #[serde(default)]
    provider_id: String,
    #[serde(default)]
    extra_prompt: String,
}

impl Default for AiProofingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: default_ai_base_url(),
            api_key: String::new(),
            model: default_ai_model(),
            temperature: default_ai_temperature(),
            max_chapter_chars: default_ai_max_chapter_chars(),
            response_timeout_sec: default_ai_response_timeout_sec(),
            auto_approve: false,
            extra_prompt: String::new(),
        }
    }
}

fn default_ai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_ai_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_ai_temperature() -> f32 {
    0.1
}

fn default_ai_max_chapter_chars() -> usize {
    12000
}

fn default_ai_response_timeout_sec() -> u64 {
    300
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

// library 模块的"app 数据根"：直接复用顶层 app_data_root（绿色版/安装版自动）
fn library_app_data_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_data_root(app)
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
    Ok(library_data_dir(app)?.join("library.json"))
}

// _data 子目录，存放所有内部数据（json、covers、history、extract）
fn library_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("_data"))
}

// 图书 子目录：所有书文件按「书名-作者.ext」命名后存这里
fn library_books_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("图书"))
}

fn library_fonts_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("字体"))
}

fn library_covers_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("covers"))
}

fn library_history_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("history"))
}

fn library_proof_logs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("proof_logs"))
}

fn library_extract_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("extract"))
}

// 兼容旧路径：迁移时用来定位旧位置
fn legacy_library_json_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("library.json"))
}

fn legacy_library_files_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("files"))
}

fn legacy_library_covers_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("covers"))
}

// 文件名清洗：去掉 Windows 非法字符 + 控制字符 + 末尾点/空格，限制长度
fn sanitize_filename_part(s: &str) -> String {
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

// 拼装「书名-作者.ext」基础文件名（不含目录）。
fn build_book_filename(title: &str, author: &str, ext: &str) -> String {
    let t = sanitize_filename_part(title);
    let a = sanitize_filename_part(author);
    let stem = match (t.is_empty(), a.is_empty()) {
        (true, true) => "未命名".to_string(),
        (false, true) => t,
        (true, false) => a,
        (false, false) => format!("{}-{}", t, a),
    };
    format!("{}.{}", stem, ext)
}

// 默认命名模板
const DEFAULT_NAMING_TEMPLATE: &str = "{title}-{author}";

// 把模板字符串里的占位符替换成 entry 对应字段；
// {tags} 展开为 "[标签1][标签2]…"；{title}/{author} 永远会被替换。
// 渲染结果再走 sanitize_filename_part。
fn render_filename_template(template: &str, entry: &BookEntry) -> String {
    let tags_brackets: String = entry
        .tags
        .as_ref()
        .map(|ts| ts.iter().map(|t| format!("[{}]", t)).collect::<String>())
        .unwrap_or_default();
    let publisher = entry.publisher.as_deref().unwrap_or("");
    let result = template
        .replace("{title}", &entry.title)
        .replace("{author}", &entry.author)
        .replace("{subtitle}", &entry.subtitle)
        .replace("{series}", &entry.series)
        .replace("{maker}", &entry.maker)
        .replace("{publisher}", publisher)
        .replace("{tags}", &tags_brackets);
    sanitize_filename_part(&result)
}

// 选用配置生效的模板字符串（空 → 默认）
fn effective_naming_template(cfg: &LibraryConfig) -> String {
    let t = cfg.naming_template.trim();
    if t.is_empty() {
        DEFAULT_NAMING_TEMPLATE.to_string()
    } else {
        t.to_string()
    }
}

// 用解析出的元数据构造一个临时 BookEntry，专供 render_filename_template 使用。
// EPUB 元数据缺失时按文件名 stem 兜底；非 EPUB（txt）只填 title=stem。
fn template_entry_from_parsed(
    file_type: &str,
    file_stem: &str,
    parsed: Option<&EpubParsedMeta>,
) -> BookEntry {
    let mut e = BookEntry::default();
    e.file_type = file_type.to_string();
    if let Some(meta) = parsed {
        e.title = if meta.title.is_empty() {
            file_stem.to_string()
        } else {
            meta.title.clone()
        };
        e.author = meta.author.clone();
        e.publisher = meta.publisher.clone();
        e.subtitle = meta.subtitle.clone().unwrap_or_default();
        e.series = meta.series.clone().unwrap_or_default();
        e.maker = meta.maker.clone().unwrap_or_default();
        if !meta.tags.is_empty() {
            e.tags = Some(meta.tags.clone());
        }
    } else {
        e.title = file_stem.to_string();
    }
    e
}

// 选定入库目标文件名（不含路径）：override 优先 → naming_mode 决定 → 兜底
fn pick_book_filename(
    cfg: &LibraryConfig,
    file_type: &str,
    src_stem: &str,
    parsed: Option<&EpubParsedMeta>,
    override_filename: Option<&str>,
) -> String {
    if let Some(n) = override_filename {
        let n = n.trim();
        if !n.is_empty() {
            let stripped = n
                .strip_suffix(&format!(".{}", file_type))
                .or_else(|| n.strip_suffix(&format!(".{}", file_type.to_uppercase())))
                .unwrap_or(n);
            let stem = sanitize_filename_part(stripped);
            if !stem.is_empty() {
                return format!("{}.{}", stem, file_type);
            }
        }
    }
    let mode = cfg.naming_mode.trim();
    if mode == "source" {
        let stem = sanitize_filename_part(src_stem);
        if !stem.is_empty() {
            return format!("{}.{}", stem, file_type);
        }
    }
    // 默认/template
    let template = effective_naming_template(cfg);
    let entry = template_entry_from_parsed(file_type, src_stem, parsed);
    let stem = render_filename_template(&template, &entry);
    if stem.is_empty() {
        return build_book_filename(&entry.title, &entry.author, file_type);
    }
    format!("{}.{}", stem, file_type)
}


fn ensure_dir(p: &Path) -> Result<(), String> {
    if !p.exists() {
        fs::create_dir_all(p).map_err(|e| format!("创建目录失败 {}: {}", p.display(), e))?;
    }
    Ok(())
}

fn read_library_data(app: &tauri::AppHandle) -> Result<LibraryData, String> {
    // 先做一次旧布局迁移（幂等，无旧文件时直接返回）
    let _ = migrate_legacy_library_layout(app);

    let path = library_json_path(app)?;
    if !path.exists() {
        return Ok(LibraryData::default());
    }
    let bytes = fs::read(&path).map_err(|e| format!("读取 library.json 失败: {}", e))?;
    let data: LibraryData =
        serde_json::from_slice(&bytes).map_err(|e| format!("解析 library.json 失败: {}", e))?;
    Ok(data)
}

// 把旧布局（library.json/files/covers 直接放在 root + history 在 app_data）
// 平滑迁移到新布局（_data/* + 图书/书名-作者.ext）。
//
// - 新 _data/library.json 已存在 → 已迁移过，直接返回
// - 旧 root/library.json 不存在 → 全新库，无需迁移
fn migrate_legacy_library_layout(app: &tauri::AppHandle) -> Result<(), String> {
    let new_json = library_json_path(app)?;
    if new_json.exists() {
        return Ok(());
    }
    let old_json = legacy_library_json_path(app)?;
    if !old_json.exists() {
        return Ok(());
    }

    // 读旧 library.json
    let bytes = fs::read(&old_json)
        .map_err(|e| format!("迁移：读旧 library.json 失败: {}", e))?;
    let mut data: LibraryData = serde_json::from_slice(&bytes)
        .map_err(|e| format!("迁移：解析旧 library.json 失败: {}", e))?;

    let data_dir = library_data_dir(app)?;
    ensure_dir(&data_dir)?;
    let books_dir = library_books_dir(app)?;
    let new_covers_dir = library_covers_dir(app)?;
    let old_files_dir = legacy_library_files_dir(app)?;
    let old_covers_dir = legacy_library_covers_dir(app)?;

    // 1) 迁移每本书
    let mut used_filenames: HashSet<String> = HashSet::new();
    for book in data.books.iter_mut() {
        // 1.1 书文件
        if !book.file_path.is_empty() {
            let cur = PathBuf::from(&book.file_path);
            // 仅迁移落在旧 files_dir 内的副本（ref_only 不动用户原文件）
            if cur.starts_with(&old_files_dir) && cur.exists() {
                ensure_dir(&books_dir).ok();
                let ext = book.file_type.clone();
                let mut name = build_book_filename(&book.title, &book.author, &ext);
                // 同名冲突 → 加 (n)
                let mut n = 2;
                while used_filenames.contains(&name) || books_dir.join(&name).exists() {
                    let stem = build_book_filename(&book.title, &book.author, &ext)
                        .trim_end_matches(&format!(".{}", ext))
                        .to_string();
                    name = format!("{} ({}).{}", stem, n, ext);
                    n += 1;
                }
                let dest = books_dir.join(&name);
                if fs::rename(&cur, &dest).is_ok() {
                    book.file_path = dest.to_string_lossy().to_string();
                    book.filename = name.clone();
                    used_filenames.insert(name);
                }
            }
        }
        // 1.2 封面
        if !book.cover_path.is_empty() {
            let cur = PathBuf::from(&book.cover_path);
            if cur.starts_with(&old_covers_dir) && cur.exists() {
                ensure_dir(&new_covers_dir).ok();
                if let Some(fname) = cur.file_name() {
                    let dest = new_covers_dir.join(fname);
                    if fs::rename(&cur, &dest).is_ok() {
                        book.cover_path = dest.to_string_lossy().to_string();
                    }
                }
            }
        }
    }

    // 2) 迁移 history（从 app_data_root/history → _data/history）
    let new_history = library_history_dir(app)?;
    let old_history_app = app_data_root(app).ok().map(|p| p.join("history"));
    if let Some(old) = old_history_app {
        if old.exists() && old != new_history {
            ensure_dir(&new_history).ok();
            if let Ok(entries) = fs::read_dir(&old) {
                for entry in entries.flatten() {
                    let from = entry.path();
                    if let Some(name) = from.file_name() {
                        let to = new_history.join(name);
                        let _ = fs::rename(&from, &to);
                    }
                }
            }
            let _ = fs::remove_dir(&old);
        }
    }

    // 3) 写新 library.json，删旧
    write_library_data_atomic(app, &data)?;
    let _ = fs::remove_file(&old_json);

    // 4) 收尾：旧 files/、旧 covers/ 目录若空则清理
    let _ = fs::remove_dir(&old_files_dir);
    let _ = fs::remove_dir(&old_covers_dir);

    Ok(())
}

// 写入策略：customWorkDir 非空则写到那里，否则写到 app_data_dir；
// library.json 实际落点是 {target_root}/_data/library.json。
// 同时把真实根目录写进 app_data_dir/library.pointer 给下次冷启动用。
fn write_library_data_atomic(app: &tauri::AppHandle, data: &LibraryData) -> Result<(), String> {
    let target_root = if !data.config.custom_work_dir.trim().is_empty() {
        PathBuf::from(data.config.custom_work_dir.trim())
    } else {
        library_app_data_root(app)?
    };
    ensure_dir(&target_root)?;
    let target_data_dir = target_root.join("_data");
    ensure_dir(&target_data_dir)?;

    let target = target_data_dir.join("library.json");
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

// 启动时给前端的"应用模式 + 是否首次启动"信息
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct AppModeInfo {
    /// 是否绿色版（exe 同级目录有 portable.txt 标记文件即为 true）
    is_portable: bool,
    /// 给首次启动弹窗预填的建议路径：portable→<exe>/EPUB；installed→空字符串
    suggested_library_dir: String,
    /// 是否已经配置过书库（library.pointer 存在且指向的目录存在）
    is_library_configured: bool,
    /// portable.txt 应在的位置（用于在 UI 里告诉用户怎么切到绿色版）
    portable_marker_path: String,
    /// 当前应用数据根目录（绿色版=<exe>/data，安装版=app_data_dir），UI 展示用
    app_data_dir: String,
}

#[tauri::command]
fn get_app_mode_info(app: tauri::AppHandle) -> Result<AppModeInfo, String> {
    let portable = is_portable();

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.to_path_buf()));

    let suggested = if portable {
        exe_dir
            .as_ref()
            .map(|d| d.join("EPUB").to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let marker = exe_dir
        .as_ref()
        .map(|d| d.join("portable.txt").to_string_lossy().to_string())
        .unwrap_or_default();

    let data_root = app_data_root(&app)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // 是否已配置：library.pointer 存在 且其指向的目录还存在
    let pointer = library_pointer_path(&app).ok();
    let configured = match pointer {
        Some(p) if p.exists() => match fs::read_to_string(&p) {
            Ok(s) => {
                let target = PathBuf::from(s.trim());
                !s.trim().is_empty() && target.exists()
            }
            Err(_) => false,
        },
        _ => false,
    };

    Ok(AppModeInfo {
        is_portable: portable,
        suggested_library_dir: suggested,
        is_library_configured: configured,
        portable_marker_path: marker,
        app_data_dir: data_root,
    })
}

#[tauri::command]
async fn save_library(app: tauri::AppHandle, data: LibraryData) -> Result<(), String> {
    write_library_data_atomic(&app, &data)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AiProofingRequest {
    config: AiProofingConfig,
    system_prompt: String,
    user_prompt: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AiProofingResponse {
    content: String,
}

fn preview_ai_response_body(text: &str) -> String {
    const LIMIT: usize = 1200;
    let compact = text.replace('\r', "\\r").replace('\n', "\\n");
    if compact.chars().count() > LIMIT {
        format!("{}...", compact.chars().take(LIMIT).collect::<String>())
    } else {
        compact
    }
}

#[tauri::command]
async fn run_ai_proofing(request: AiProofingRequest) -> Result<AiProofingResponse, String> {
    let base_url = request.config.base_url.trim().trim_end_matches('/').to_string();
    if base_url.is_empty() {
        return Err("请先填写 API 地址".to_string());
    }
    if request.config.api_key.trim().is_empty() {
        return Err("请先填写 API Key".to_string());
    }
    if request.config.model.trim().is_empty() {
        return Err("请先填写模型名".to_string());
    }

    let endpoint = if base_url.ends_with("/chat/completions") {
        base_url
    } else {
        format!("{}/chat/completions", base_url)
    };
    let timeout_secs = request.config.response_timeout_sec.clamp(30, 1800);
    let model_name = request.config.model.trim();
    let body = serde_json::json!({
        "model": request.config.model,
        "temperature": request.config.temperature.clamp(0.0, 1.0),
        "max_tokens": 8192,
        "response_format": { "type": "json_object" },
        "thinking": { "type": "disabled" },
        "stream": false,
        "messages": [
            { "role": "system", "content": request.system_prompt },
            { "role": "user", "content": request.user_prompt }
        ]
    });
    let fallback_body = serde_json::json!({
        "model": request.config.model,
        "temperature": request.config.temperature.clamp(0.0, 1.0),
        "max_tokens": 8192,
        "thinking": { "type": "disabled" },
        "stream": false,
        "messages": [
            { "role": "system", "content": request.system_prompt },
            { "role": "user", "content": request.user_prompt }
        ]
    });

    let client = reqwest::Client::builder()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;
    let send_ai_request = |body: &serde_json::Value| {
        client
            .post(&endpoint)
            .bearer_auth(request.config.api_key.trim())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Accept-Encoding", "identity")
            .body(body.to_string())
    };
    let resp = send_ai_request(&body)
        .send()
        .await
        .map_err(|e| format!("请求智能校对接口失败: {}", e))?;
    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                format!(
                    "读取智能校对响应超时：模型超过 {} 秒仍未返回完整结果，请调小单章上限、调大响应时间或换更快模型",
                    timeout_secs
                )
            } else {
                format!("读取智能校对响应失败: {}", e)
            }
        })?;
    let mut text = String::from_utf8_lossy(&bytes).to_string();

    if !status.is_success()
        && text.contains("response_format")
        && model_name.to_ascii_lowercase().contains("deepseek")
    {
        let retry_resp = send_ai_request(&fallback_body)
            .send()
            .await
            .map_err(|e| format!("重试智能校对接口失败: {}", e))?;
        let retry_status = retry_resp.status();
        let retry_bytes = retry_resp
            .bytes()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    format!(
                        "读取智能校对重试响应超时：模型超过 {} 秒仍未返回完整结果，请调小单章上限、调大响应时间或换更快模型",
                        timeout_secs
                    )
                } else {
                    format!("读取智能校对重试响应失败: {}", e)
                }
            })?;
        text = String::from_utf8_lossy(&retry_bytes).to_string();
        if !retry_status.is_success() {
            return Err(format!(
                "智能校对接口返回 {}: {}",
                retry_status,
                preview_ai_response_body(&text)
            ));
        }
    } else if !status.is_success() {
        return Err(format!(
            "智能校对接口返回 {}: {}",
            status,
            preview_ai_response_body(&text)
        ));
    }

    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| {
            format!(
                "解析智能校对响应失败: {}；响应片段: {}",
                e,
                preview_ai_response_body(&text)
            )
        })?;
    let content = value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .or_else(|| {
            value
                .get("choices")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|choice| choice.get("text"))
                .and_then(|content| content.as_str())
        })
        .ok_or_else(|| {
            format!(
                "智能校对响应缺少 choices[0].message.content；响应片段: {}",
                preview_ai_response_body(&text)
            )
        })?
        .to_string();

    Ok(AiProofingResponse { content })
}

#[tauri::command]
async fn save_ai_proofing_log(
    app: tauri::AppHandle,
    txt_path: String,
    model: String,
    content: String,
) -> Result<String, String> {
    let dir = library_proof_logs_dir(&app)?;
    ensure_dir(&dir)?;

    let txt_stem = Path::new(&txt_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未命名");
    let stem = {
        let safe = sanitize_filename_part(txt_stem);
        if safe.is_empty() { "未命名".to_string() } else { safe }
    };
    let model_name = {
        let safe = sanitize_filename_part(model.trim());
        if safe.is_empty() { "unknown-model".to_string() } else { safe }
    };
    let time_text = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let mut file_name = format!("{}-{}-{}.log", stem, time_text, model_name);
    let mut path = dir.join(&file_name);
    let mut suffix = 2;
    while path.exists() {
        file_name = format!("{}-{}-{}-{}.log", stem, time_text, model_name, suffix);
        path = dir.join(&file_name);
        suffix += 1;
    }
    fs::write(&path, content).map_err(|e| format!("写入校对日志失败: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
async fn list_ai_proofing_logs(app: tauri::AppHandle) -> Result<Vec<ProofLogInfo>, String> {
    let dir = library_proof_logs_dir(&app)?;
    ensure_dir(&dir)?;
    let mut list = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("读取校对日志目录失败: {}", e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("log") && !ext.eq_ignore_ascii_case("txt") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            list.push(ProofLogInfo {
                file_name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                timestamp: meta
                    .modified()
                    .unwrap_or(SystemTime::now())
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                size: meta.len(),
            });
        }
    }
    list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(list)
}

#[tauri::command]
async fn read_ai_proofing_log(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("读取校对日志失败: {}", e))
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct MobileEpubMetadata {
    title: String,
    author: String,
    publisher: String,
    description: String,
    epub_uuid: String,
    subtitle: String,
    series: String,
    maker: String,
    tags: Vec<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
struct MobileEpubCover {
    bytes: Vec<u8>,
}

#[derive(Serialize, Clone, Debug)]
struct MobileExportResult {
    output_path: String,
    public_output: bool,
    message: String,
}

#[derive(Serialize, Clone, Debug)]
struct MobileMakeEpubResult {
    output_path: String,
    title: String,
    chapter_count: usize,
    word_count: usize,
}

impl From<EpubParsedMeta> for MobileEpubMetadata {
    fn from(meta: EpubParsedMeta) -> Self {
        Self {
            title: meta.title,
            author: meta.author,
            publisher: meta.publisher.unwrap_or_default(),
            description: meta.description.unwrap_or_default(),
            epub_uuid: meta.epub_uuid,
            subtitle: meta.subtitle.unwrap_or_default(),
            series: meta.series.unwrap_or_default(),
            maker: meta.maker.unwrap_or_default(),
            tags: meta.tags,
        }
    }
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

fn read_opf_from_dir(root: &Path) -> Result<(String, String), String> {
    let container_path = root.join("META-INF").join("container.xml");
    let container_xml =
        fs::read_to_string(&container_path).map_err(|e| format!("读取 container.xml 失败: {}", e))?;
    let opf_re = Regex::new(r#"full-path="([^"]+)""#).map_err(|e| e.to_string())?;
    let opf_path = match opf_re.captures(&container_xml) {
        Ok(Some(c)) => c.get(1).map(|m| m.as_str().to_string()),
        _ => None,
    }
    .ok_or_else(|| "container.xml 缺少 full-path".to_string())?;
    let opf_xml = fs::read_to_string(root.join(&opf_path))
        .map_err(|e| format!("读取 OPF 文件失败 {}: {}", opf_path, e))?;
    Ok((opf_path, opf_xml))
}

fn parse_epub_metadata_from_parts(
    opf_path: &str,
    opf_xml: &str,
    cover_bytes: Option<Vec<u8>>,
    cover_ext: String,
) -> Result<EpubParsedMeta, String> {
    let title = extract_first_tag(opf_xml, "dc:title").unwrap_or_default();
    let author = extract_first_tag(opf_xml, "dc:creator").unwrap_or_default();
    let publisher = extract_first_tag(opf_xml, "dc:publisher");
    let description = extract_first_tag(opf_xml, "dc:description");
    let epub_uuid = extract_first_tag(opf_xml, "dc:identifier").unwrap_or_default();
    let subtitle = extract_meta_by_name(opf_xml, "calibre:subtitle");
    let series = extract_meta_by_name(opf_xml, "calibre:series");
    let maker = extract_meta_by_name(opf_xml, "maker");
    let tags = extract_all_tags(opf_xml, "dc:subject");
    let pub_date = extract_first_tag(opf_xml, "dc:date").and_then(|s| parse_epub_date(&s));
    let modified_date = match Regex::new(
        r#"<meta\s+[^>]*property="dcterms:modified"[^>]*>([\s\S]*?)</meta>"#,
    ) {
        Ok(re) => match re.captures(opf_xml) {
            Ok(Some(c)) => c.get(1).and_then(|m| parse_epub_date(m.as_str().trim())),
            _ => None,
        },
        Err(_) => None,
    };

    let normalized_cover_ext = if cover_ext.is_empty() {
        find_cover_href(opf_xml)
            .map(|href| {
                let opf_dir = Path::new(opf_path)
                    .parent()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                let cover_full = if opf_dir.is_empty() {
                    href
                } else {
                    format!("{}/{}", opf_dir, href)
                };
                ext_from_path(&normalize_zip_path(&cover_full)).to_string()
            })
            .unwrap_or_else(|| "jpg".to_string())
    } else {
        cover_ext
    };

    Ok(EpubParsedMeta {
        title,
        author,
        publisher,
        description,
        epub_uuid,
        cover_bytes,
        cover_ext: normalized_cover_ext,
        pub_date,
        modified_date,
        subtitle,
        series,
        maker,
        tags,
    })
}

fn parse_epub_metadata_live(epub_path: &Path) -> Result<EpubParsedMeta, String> {
    let temp_path = {
        let cache_guard = EPUB_CACHE.lock().unwrap();
        if let Some(ref cache) = *cache_guard {
            if cache.epub_path == epub_path.to_string_lossy() {
                cache.temp_dir.as_ref().map(|temp| temp.path().to_path_buf())
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(root) = temp_path {
        let (opf_path, opf_xml) = read_opf_from_dir(&root)?;
        let mut cover_bytes: Option<Vec<u8>> = None;
        let mut cover_ext = String::from("jpg");
        if let Some(href) = find_cover_href(&opf_xml) {
            let opf_dir = Path::new(&opf_path)
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            let cover_full = if opf_dir.is_empty() {
                href
            } else {
                format!("{}/{}", opf_dir, href)
            };
            let normalized = normalize_zip_path(&cover_full);
            let disk_path = normalized.split('/').fold(root.clone(), |acc, part| acc.join(part));
            if let Ok(buf) = fs::read(&disk_path) {
                cover_ext = match ext_from_path(&normalized) {
                    "jpg" => detect_image_ext(&buf).to_string(),
                    other => other.to_string(),
                };
                cover_bytes = Some(buf);
            }
        }
        return parse_epub_metadata_from_parts(&opf_path, &opf_xml, cover_bytes, cover_ext);
    }

    parse_epub_metadata(epub_path)
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
    override_filename: Option<String>,
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
    let original_filename = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 提前解析一次 EPUB 元数据，用来：
    // 1) 渲染命名模板（{title}/{author}/{tags}…）
    // 2) 后续填充 BookEntry，省一次 IO
    //
    // 入库前自动尝试解除伪加密 / XOR 解混淆 / 修复文件名混淆。
    // 若处理成功，effective_src 指向临时干净副本，后续 parse 与 copy 都用它；
    // 函数返回时由 _epub_cleanup_guard 自动清理临时文件。
    let epub_prep_result: Option<(PathBuf, String)> = if file_type == "epub" {
        match try_prepare_epub_for_ingest(&src) {
            Ok(Some(r)) => Some(r),
            Ok(None) => None,
            Err(e) => {
                eprintln!(
                    "[library] EPUB 入库前处理失败 ({})，按原文件继续: {}",
                    file_path, e
                );
                None
            }
        }
    } else {
        None
    };
    if let Some((_, action)) = &epub_prep_result {
        eprintln!("[library] EPUB 入库自动处理: {} ({})", action, file_path);
    }
    let _epub_cleanup_guard =
        IngestTempFile(epub_prep_result.as_ref().map(|(p, _)| p.clone()));
    let effective_src: PathBuf = epub_prep_result
        .as_ref()
        .map(|(p, _)| p.clone())
        .unwrap_or_else(|| src.clone());

    let parsed_meta: Option<EpubParsedMeta> = if file_type == "epub" {
        parse_epub_metadata(&effective_src).ok()
    } else {
        None
    };
    let src_stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // 决定文件最终位置
    // 若 EPUB 已被入库前处理过，即使配置是 ref_only 也强制复制 —— 否则引用的还是加密源，没意义。
    let force_copy_due_to_prep = epub_prep_result.is_some();
    let final_file_path = if should_copy || force_copy_due_to_prep {
        let books_dir = library_books_dir(&app)?;
        ensure_dir(&books_dir)?;

        // 按 override → naming_mode → 默认 模板顺序选名
        let target_name = pick_book_filename(
            &config,
            file_type,
            &src_stem,
            parsed_meta.as_ref(),
            override_filename.as_deref(),
        );

        let dest = books_dir.join(&target_name);
        // 冲突 → 返回结构化错误
        if dest.exists() {
            return Err(format!("BOOK_FILE_COLLISION:{}", target_name));
        }
        fs::copy(&effective_src, &dest).map_err(|e| format!("复制文件失败: {}", e))?;
        dest.to_string_lossy().to_string()
    } else {
        // ref_only：保持原绝对路径
        src.to_string_lossy().to_string()
    };

    let saved_filename = if should_copy || force_copy_due_to_prep {
        PathBuf::from(&final_file_path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        original_filename
    };

    // 入库前若做过处理（解密/解混淆），实际落盘大小与源不同；以最终文件为准
    let file_size = fs::metadata(&final_file_path)
        .map(|m| m.len())
        .unwrap_or(file_size);

    let mut entry = BookEntry {
        id: id.clone(),
        file_path: final_file_path,
        file_type: file_type.to_string(),
        added_at: now,
        file_size,
        filename: saved_filename,
        created_at,
        modified_at,
        ..Default::default()
    };

    if file_type == "epub" {
        match parsed_meta {
            Some(meta) => {
                entry.title = if meta.title.is_empty() {
                    src_stem.clone()
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
            None => {
                // 解析失败不阻断入库，回退到文件名作为标题
                entry.title = src_stem.clone();
                eprintln!("[library] EPUB 元数据解析失败，回退到文件名: {}", file_path);
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

    // 仅清理位于 books_dir 内的副本（ref_only 模式下原文件不动）；
    // 同时兼容老布局下还在 files/ 里的残留
    let books_dir = library_books_dir(&app)?;
    let legacy_files_dir = legacy_library_files_dir(&app)?;
    if !book.file_path.is_empty() {
        let book_path = PathBuf::from(&book.file_path);
        let inside_managed = book_path.starts_with(&books_dir)
            || book_path.starts_with(&legacy_files_dir);
        if inside_managed && book_path.exists() {
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

    // tags → 多个 dc:subject（EPUB 标准）。先删后插，**所有 tag 拼到同一行**。
    s = remove_all_matches(
        &s,
        r#"\s*<dc:subject(?:\s[^>]*)?>[\s\S]*?</dc:subject>"#,
    );
    if !tags.is_empty() {
        let mut chunks: Vec<String> = Vec::new();
        for t in tags {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                continue;
            }
            chunks.push(format!("<dc:subject>{}</dc:subject>", xml_escape(trimmed)));
        }
        if !chunks.is_empty() {
            let block = format!("    {}\n  ", chunks.join(""));
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
async fn mobile_cache_input_file(
    app: tauri::AppHandle,
    source_name: String,
    data: Vec<u8>,
    fallback_ext: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if data.is_empty() {
            return Err("文件内容为空".to_string());
        }

        let dir = app_data_root(&app)?.join("mobile-imports");
        ensure_dir(&dir)?;

        let mut name = sanitize_filename_part(&source_name);
        if name.is_empty() {
            name = "selected".to_string();
        }

        let ext = fallback_ext.trim().trim_start_matches('.').to_string();
        if Path::new(&name).extension().is_none() && !ext.is_empty() {
            name = format!("{}.{}", name, ext);
        }

        let path = dir.join(format!("{}_{}", uuid::Uuid::new_v4().simple(), name));
        fs::write(&path, data).map_err(|e| format!("写入移动端缓存文件失败: {}", e))?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("缓存移动端文件任务失败: {}", e))?
}

fn mobile_default_chapter_rules() -> Vec<RegexRule> {
    vec![
        RegexRule {
            level: 1,
            pattern: "^\\s*(?:内容简介|本书相关|完本感言)\\s*(?:[:：].*)?$".to_string(),
        },
        RegexRule {
            level: 1,
            pattern: "^\\s*(?:第\\s*[零〇一二两三四五六七八九十百千万0-9]+\\s*卷|卷\\s*[零〇一二两三四五六七八九十百千万0-9]+)(?:\\s+|[:：、.．\\-—]+)\\S+.*".to_string(),
        },
        RegexRule {
            level: 3,
            pattern: "^\\s*(?:简介|序(?:章|言)?|前言|楔子|后记|尾声)\\s*(?:[:：].*)?$".to_string(),
        },
        RegexRule {
            level: 3,
            pattern: "^\\s*(?:第\\s*[一二两三四五六七八九十零〇百千万0-9]+\\s*(?:[章节]|回(?:[^合]|$))|Chapter\\s*\\d+|终章(?:\\s+|[:：、.．\\-—])\\S+|(?:新增\\s*)?(?:番外|后日谈)(?:\\s+|[:：、.．\\-—])\\S+|【\\s*(?:番外|后日谈)\\s*】\\s*\\S+).*".to_string(),
        },
    ]
}

fn mobile_is_meta_title(title: &str) -> bool {
    let trimmed = title.trim();
    Regex::new(r"^(?:内容简介|简介|序(?:章|言)?|前言|楔子|后记|尾声|完本感言|本书相关)(?:\s|[:：、.．\-—]|$)")
        .map(|re| re.is_match(trimmed).unwrap_or(false))
        .unwrap_or(false)
}

fn mobile_is_likely_toc_title(title: &str, level: u8) -> bool {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return false;
    }

    if level == 1 {
        if let Ok(re) = Regex::new(r"^序(?!\s*(?:章|言)?\s*(?:[:：]|$))") {
            if re.is_match(trimmed).unwrap_or(false) {
                return false;
            }
        }
    }

    if let Ok(prose_sequence) =
        Regex::new(r"^序列\s*[0-9零〇一二两三四五六七八九十百千万]+\p{Han}")
    {
        if prose_sequence.is_match(trimmed).unwrap_or(false) {
            if let Ok(heading_sequence) =
                Regex::new(r"^序列\s*[0-9零〇一二两三四五六七八九十百千万]+(?:\s|[:：、.．\-—]|$)")
            {
                if !heading_sequence.is_match(trimmed).unwrap_or(false) {
                    return false;
                }
            }
        }
    }

    if let Ok(re) = Regex::new(r"^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(章|节|回)(\S?)") {
        if let Ok(Some(caps)) = re.captures(trimmed) {
            let keyword = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let next_char = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            if keyword == "节" && !next_char.is_empty() {
                if let Ok(chinese) = Regex::new(r"^\p{Han}") {
                    if chinese.is_match(next_char).unwrap_or(false) {
                        return false;
                    }
                }
            }
            if !next_char.is_empty()
                && !["：", ":", "、", ".", "．", "-", "—"].contains(&next_char)
            {
                if "的了时侯候后前中里内外上下来去得地着过将把被与和及都也才只能已会在是有为对从用以课程数次目期"
                    .contains(next_char)
                {
                    return false;
                }
            }
        }
    }

    true
}

#[tauri::command]
async fn mobile_scan_chapters(content: String, rules: Vec<RegexRule>) -> Vec<ChapterInfo> {
    scan_chapters(content, rules)
        .await
        .into_iter()
        .filter(|chapter| mobile_is_likely_toc_title(&chapter.title, chapter.level))
        .map(|mut chapter| {
            if mobile_is_meta_title(&chapter.title) {
                chapter.is_meta = true;
            }
            chapter
        })
        .collect()
}

#[tauri::command]
async fn mobile_make_epub(
    app: tauri::AppHandle,
    source_path: String,
    title: String,
    author: String,
    cover_path: String,
    uuid: String,
    rules: Vec<RegexRule>,
) -> Result<MobileMakeEpubResult, String> {
    let content = read_text_file(source_path.clone())?;
    if content.trim().is_empty() {
        return Err("文本内容为空，无法制作 EPUB".to_string());
    }

    let source_name = Path::new(&source_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("未命名书籍")
        .to_string();
    let book_title = if title.trim().is_empty() {
        source_name
    } else {
        title.trim().to_string()
    };
    let effective_rules = if rules.is_empty() {
        mobile_default_chapter_rules()
    } else {
        rules
    };
    let mut chapters = mobile_scan_chapters(content.clone(), effective_rules).await;
    if chapters.is_empty() {
        chapters.push(ChapterInfo {
            title: "正文".to_string(),
            line_number: 0,
            level: 3,
            is_meta: false,
            word_count: content.chars().filter(|c| !c.is_whitespace()).count(),
        });
    }

    let out_dir = app_data_root(&app)?.join("mobile-exports");
    ensure_dir(&out_dir)?;
    let out_path = build_processed_epub_path(&out_dir.join(format!("{}.epub", sanitize_filename_part(&book_title))), "");
    let word_count = content.chars().filter(|c| !c.is_whitespace()).count();
    let chapter_count = chapters.len();
    let epub_uuid = if uuid.trim().is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        uuid.trim().trim_start_matches("urn:uuid:").to_string()
    };

    export_epub(
        out_path.to_string_lossy().to_string(),
        content.clone(),
        chapters,
        EpubMetadata {
            title: book_title.clone(),
            creator: author.trim().to_string(),
            publisher: String::new(),
            cover_path: cover_path.trim().to_string(),
            uuid: epub_uuid,
            md5: format!("{:x}", md5::compute(content.as_bytes())),
            description: String::new(),
            tags: Vec::new(),
            main_css: String::new(),
            font_css: String::new(),
            subset_fonts: false,
            assets: Vec::new(),
            extra: HashMap::new(),
        },
    )
    .await?;

    Ok(MobileMakeEpubResult {
        output_path: out_path.to_string_lossy().to_string(),
        title: book_title,
        chapter_count,
        word_count,
    })
}

#[tauri::command]
async fn mobile_read_epub_metadata(epub_path: String) -> Result<MobileEpubMetadata, String> {
    let path = PathBuf::from(epub_path);
    tauri::async_runtime::spawn_blocking(move || {
        if !path.exists() {
            return Err("EPUB 文件不存在".to_string());
        }
        parse_epub_metadata_live(&path).map(MobileEpubMetadata::from)
    })
    .await
    .map_err(|e| format!("读取 EPUB 元数据任务失败: {}", e))?
}

#[tauri::command]
async fn mobile_read_epub_cover(epub_path: String) -> Result<MobileEpubCover, String> {
    let path = PathBuf::from(epub_path);
    tauri::async_runtime::spawn_blocking(move || {
        if !path.exists() {
            return Err("EPUB 文件不存在".to_string());
        }
        let parsed = parse_epub_metadata_live(&path)?;
        Ok(MobileEpubCover {
            bytes: parsed.cover_bytes.unwrap_or_default(),
        })
    })
    .await
    .map_err(|e| format!("读取 EPUB 封面任务失败: {}", e))?
}

#[tauri::command]
async fn mobile_update_epub_metadata(
    epub_path: String,
    metadata: MobileEpubMetadata,
) -> Result<MobileEpubMetadata, String> {
    let path = PathBuf::from(&epub_path);
    tauri::async_runtime::spawn_blocking(move || {
        if !path.exists() {
            return Err("EPUB 文件不存在".to_string());
        }

        let (_opf_path, opf_xml) = read_opf_from_epub(&path)?;
        let publisher = metadata.publisher.trim();
        let publisher = if publisher.is_empty() {
            None
        } else {
            Some(publisher)
        };
        let tags: Vec<String> = metadata
            .tags
            .iter()
            .map(|tag| tag.trim().to_string())
            .filter(|tag| !tag.is_empty())
            .collect();

        let new_opf = write_opf_metadata(
            &opf_xml,
            metadata.title.trim(),
            metadata.author.trim(),
            metadata.description.trim(),
            metadata.epub_uuid.trim(),
            publisher,
            metadata.subtitle.trim(),
            metadata.maker.trim(),
            metadata.series.trim(),
            &tags,
        );

        let temp_root = {
            let cache_guard = EPUB_CACHE.lock().unwrap();
            if let Some(ref cache) = *cache_guard {
                if cache.epub_path == epub_path {
                    cache.temp_dir.as_ref().map(|temp| temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(root) = temp_root {
            let (temp_opf_path, _) = read_opf_from_dir(&root)?;
            let temp_opf_disk_path = temp_opf_path
                .split('/')
                .fold(root.clone(), |acc, part| acc.join(part));
            if let Some(parent) = temp_opf_disk_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建 OPF 目录失败: {}", e))?;
            }
            fs::write(&temp_opf_disk_path, &new_opf).map_err(|e| format!("写入缓存 OPF 失败: {}", e))?;
        }

        rewrite_epub(
            &path,
            &EpubRewrite {
                opf_replacement: Some(new_opf),
                cover_replacement: None,
            },
        )?;
        parse_epub_metadata(&path).map(MobileEpubMetadata::from)
    })
    .await
    .map_err(|e| format!("写回 EPUB 元数据任务失败: {}", e))?
}

#[tauri::command]
async fn mobile_update_epub_cover(
    epub_path: String,
    cover_data: Vec<u8>,
) -> Result<MobileEpubCover, String> {
    if cover_data.is_empty() {
        return Err("封面数据为空".to_string());
    }

    let path = PathBuf::from(&epub_path);
    tauri::async_runtime::spawn_blocking(move || {
        if !path.exists() {
            return Err("EPUB 文件不存在".to_string());
        }

        let temp_root = {
            let cache_guard = EPUB_CACHE.lock().unwrap();
            if let Some(ref cache) = *cache_guard {
                if cache.epub_path == epub_path {
                    cache.temp_dir.as_ref().map(|temp| temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(root) = temp_root {
            let (opf_path, opf_xml) = read_opf_from_dir(&root)?;
            if let Some(cover_internal) = locate_cover_in_opf(&opf_path, &opf_xml) {
                let cover_disk_path = cover_internal
                    .split('/')
                    .fold(root.clone(), |acc, part| acc.join(part));
                if let Some(parent) = cover_disk_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("创建封面目录失败: {}", e))?;
                }
                fs::write(&cover_disk_path, &cover_data)
                    .map_err(|e| format!("写入缓存封面失败: {}", e))?;
            }
        }

        rewrite_epub(
            &path,
            &EpubRewrite {
                opf_replacement: None,
                cover_replacement: Some(cover_data.clone()),
            },
        )?;

        let parsed = parse_epub_metadata_live(&path)?;
        Ok(MobileEpubCover {
            bytes: parsed.cover_bytes.unwrap_or(cover_data),
        })
    })
    .await
    .map_err(|e| format!("写回 EPUB 封面任务失败: {}", e))?
}

#[tauri::command]
async fn mobile_export_epub(
    app: tauri::AppHandle,
    epub_path: String,
    file_name: String,
) -> Result<MobileExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let src = PathBuf::from(epub_path);
        if !src.exists() {
            return Err("待导出的 EPUB 文件不存在".to_string());
        }

        let mut safe_name = sanitize_filename_part(&file_name);
        if safe_name.is_empty() {
            safe_name = src
                .file_name()
                .and_then(|s| s.to_str())
                .map(sanitize_filename_part)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "book.epub".to_string());
        }
        if Path::new(&safe_name).extension().is_none() {
            safe_name.push_str(".epub");
        }

        let mut export_errors: Vec<String> = Vec::new();
        for public_download in [
            PathBuf::from("/storage/emulated/0/Download"),
            PathBuf::from("/sdcard/Download"),
        ] {
            if !public_download.exists() {
                continue;
            }
            let out_dir = public_download.join("TEpub-Editor");
            match ensure_dir(&out_dir) {
                Ok(()) => {
                    let target = build_processed_epub_path(&out_dir.join(&safe_name), "");
                    match fs::copy(&src, &target) {
                        Ok(_) => {
                            let output_path = target.to_string_lossy().to_string();
                            return Ok(MobileExportResult {
                                output_path: output_path.clone(),
                                public_output: true,
                                message: format!("已导出到下载目录: {}", output_path),
                            });
                        }
                        Err(e) => export_errors.push(format!(
                            "写入公共下载目录 {} 失败: {}",
                            out_dir.to_string_lossy(),
                            e
                        )),
                    }
                }
                Err(e) => export_errors.push(e),
            }
        }

        {
            use tauri::Manager;
            match app.path().download_dir() {
                Ok(download_dir) => {
                    let out_dir = download_dir.join("TEpub-Editor");
                    match ensure_dir(&out_dir) {
                        Ok(()) => {
                            let target = build_processed_epub_path(&out_dir.join(&safe_name), "");
                            match fs::copy(&src, &target) {
                                Ok(_) => {
                                    let output_path = target.to_string_lossy().to_string();
                                    return Ok(MobileExportResult {
                                        output_path: output_path.clone(),
                                        public_output: true,
                                        message: format!("已导出到下载目录: {}", output_path),
                                    });
                                }
                                Err(e) => {
                                    export_errors.push(format!("写入下载目录失败: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            export_errors.push(e);
                        }
                    }
                }
                Err(e) => {
                    export_errors.push(format!("无法获取下载目录: {}", e));
                }
            }
        }

        let out_dir = app_data_root(&app)?.join("mobile-exports");
        ensure_dir(&out_dir)?;
        let target = build_processed_epub_path(&out_dir.join(&safe_name), "");
        fs::copy(&src, &target).map_err(|e| format!("导出 EPUB 失败: {}", e))?;
        let output_path = target.to_string_lossy().to_string();
        Ok(MobileExportResult {
            output_path: output_path.clone(),
            public_output: false,
            message: format!(
                "已导出到应用目录: {}{}",
                output_path,
                if export_errors.is_empty() {
                    String::new()
                } else {
                    format!("（下载目录不可写: {}）", export_errors.join("; "))
                }
            ),
        })
    })
    .await
    .map_err(|e| format!("导出 EPUB 任务失败: {}", e))?
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

#[cfg(test)]
mod toolbox_tests {
    use super::*;

    fn write_zip_entry(
        writer: &mut zip::ZipWriter<fs::File>,
        name: &str,
        content: &[u8],
    ) -> Result<(), String> {
        writer
            .start_file(name, FileOptions::default())
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
        write_zip_entry(&mut writer, "OPS/Styles/|_*main.css", b"body { color: #111; }")?;
        writer.finish().map_err(|e| e.to_string())?;
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
        assert!(encrypted_names.iter().any(|name| name.contains('*') || name.contains(':')));

        let decrypted = toolbox_file_decrypt_impl(&encrypted_path)?;
        assert!(decrypted.changed);
        let decrypted_path = PathBuf::from(&decrypted.output_path);
        let decrypted_names = epub_names(&decrypted_path)?;
        assert!(decrypted_names.iter().any(|name| name == "OPS/Text/chapter.xhtml"));
        assert!(decrypted_names.iter().any(|name| name == "OPS/Styles/style.css"));

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
            decrypted_names.iter().any(|name| name == "OPS/Text/gyzw0001.xhtml"),
            "{:?}",
            decrypted_names
        );
        assert!(
            decrypted_names.iter().any(|name| name == "OPS/Styles/main.css"),
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
            search_book_covers,
            download_cover_to_temp,
            save_history,
            get_history_list,
            calculate_md5,
            list_library_fonts,
            import_library_font,
            rename_library_font,
            delete_library_font,
            list_style_templates,
            read_style_template,
            import_style_template,
            save_style_template,
            restore_builtin_style_template,
            list_epub_template_repositories,
            add_epub_template_repository,
            sync_epub_template_repository,
            install_remote_epub_template,
            scan_chapters,
            advanced_search,
            advanced_replace,
            export_epub,
            extract_epub,
            read_epub_file_content,
            read_epub_file_binary,
            get_epub_temp_dir_path,
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
            get_launch_info,
            set_file_assoc,
            reveal_in_explorer,
            rename_book_file,
            rebuild_book_filenames,
            prepare_epub_for_open,
            toolbox_file_encrypt,
            toolbox_file_decrypt,
            toolbox_font_encrypt,
            toolbox_font_decrypt,
            exit_app,
            // ===== Library Phase 1 =====
            load_library,
            save_library,
            run_ai_proofing,
            save_ai_proofing_log,
            list_ai_proofing_logs,
            read_ai_proofing_log,
            get_app_mode_info,
            // ===== Library Phase 2 =====
            add_book_to_library,
            remove_book_from_library,
            // ===== Library Phase 3 =====
            get_library_cover_data,
            refresh_book_metadata,
            mobile_cache_input_file,
            mobile_scan_chapters,
            mobile_make_epub,
            mobile_read_epub_metadata,
            mobile_read_epub_cover,
            mobile_update_epub_cover,
            mobile_update_epub_metadata,
            mobile_export_epub,
            // ===== Library Phase 4 =====
            update_book_metadata,
            update_book_cover
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
