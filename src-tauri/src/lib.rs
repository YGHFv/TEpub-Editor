use chardetng::EncodingDetector;
use fancy_regex::Regex;
use md5;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    fn clear(&mut self) {
        self.text_cache.clear();
        self.binary_cache.clear();
        self.temp_dir = None;
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
enum TocType {
    Volume,
    Chapter,
    Meta,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChapterInfo {
    title: String,
    line_number: usize,
    toc_type: TocType,
    word_count: usize,
}

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
}

#[derive(Serialize, Debug, Clone)]
struct EpubFileNode {
    name: String,
    path: String,
    file_type: String, // folder, html, css, xml, image, font, other
    size: Option<u64>,
    title: Option<String>, // For HTML files
    children: Option<Vec<EpubFileNode>>,
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
// 将所有换行符统一为 \n，确保后端行号计算与 CodeMirror 编辑器一致
fn normalize_line_endings(s: String) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
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

    let mut min_errors = usize::MAX;
    let mut best_content = String::new();
    let mut best_encoding = "utf-8";

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

    best_content = cow_detected.into_owned();
    min_errors = if malformed_detected {
        errors_detected
    } else {
        0
    };
    best_encoding = detected_encoding.name();

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
    let parent = path.parent().unwrap_or(Path::new("."));
    let file_stem = path.file_stem().unwrap().to_string_lossy();
    let history_dir = parent.join(".history");
    if !history_dir.exists() {
        fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?;
    }
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let backup_name = format!("{}.{}.bak", file_stem, timestamp);
    let backup_path = history_dir.join(backup_name);
    let mut file = fs::File::create(&backup_path).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes())
        .map_err(|e| e.to_string())?;
    if let Ok(entries) = fs::read_dir(&history_dir) {
        let mut backups: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(&*file_stem))
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
    let parent = path.parent().unwrap_or(Path::new("."));
    let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let history_dir = parent.join(".history");
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(history_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with(&*file_stem) && fname.ends_with(".bak") {
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
async fn scan_chapters(
    content: String,
    volreg: String,
    chapreg: String,
    metareg: String,
) -> Vec<ChapterInfo> {
    // Normalize line endings to ensure consistency with CodeMirror's line counting
    // CodeMirror treats \r, \n, and \r\n all as line separators
    // Rust's .lines() only recognizes \n and \r\n
    let content = content.replace("\r\n", "\n").replace('\r', "\n");

    let mut chapters = Vec::new();
    let re_volume = Regex::new(&volreg).unwrap_or_else(|_| {
        Regex::new(r"^\s*第[零一二三四五六七八九十百千万0-9]+[卷部].*").unwrap()
    });
    let re_chapter = Regex::new(&chapreg).unwrap_or_else(|_| {
        Regex::new(r"^\s*(第[一二三四五六七八九十百千万0-9]+[章回]|Chapter\s*\d+).*").unwrap()
    });
    let re_meta = Regex::new(&metareg).unwrap_or_else(|_| {
        Regex::new(r"^\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*").unwrap()
    });
    let mut current_chapter: Option<ChapterInfo> = None;
    for (index, line) in content.lines().enumerate() {
        let line_trim = line.trim();
        let char_count = line_trim.chars().count();
        let is_empty = line_trim.is_empty();
        let toc_type = if !is_empty && char_count <= 60 {
            if re_volume.is_match(line).unwrap_or(false) {
                Some(TocType::Volume)
            } else if re_meta.is_match(line).unwrap_or(false) {
                Some(TocType::Meta)
            } else if re_chapter.is_match(line).unwrap_or(false) {
                Some(TocType::Chapter)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(t) = toc_type {
            if let Some(prev) = current_chapter.take() {
                chapters.push(prev);
            }
            current_chapter = Some(ChapterInfo {
                title: line_trim.to_string(),
                line_number: index + 1,
                toc_type: t,
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
    zip.write_all(CSS_FONT.as_bytes())
        .map_err(|e| e.to_string())?;
    zip.start_file("OEBPS/Styles/main.css", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(CSS_MAIN.as_bytes())
        .map_err(|e| e.to_string())?;

    let mut has_cover = false;
    let mut cover_ext = "jpg".to_string();
    if !metadata.cover_path.is_empty() {
        if let Ok(img_bytes) = fs::read(&metadata.cover_path) {
            cover_ext = Path::new(&metadata.cover_path)
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
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut manifest_items = String::new();
    let mut spine_refs = String::new();
    let mut ncx_navpoints = String::new();
    let mut play_order = 1;
    let mut open_volume = false;

    if has_cover {
        let mime = if cover_ext == "png" {
            "image/png"
        } else {
            "image/jpeg"
        };
        manifest_items.push_str(&format!(r#"<item id="cover-image" href="Images/cover.{}" media-type="{}" properties="cover-image"/>"#, cover_ext, mime));
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
        let body_lines = if safe_start + 1 <= safe_end {
            &lines[safe_start + 1..=safe_end] // 使用 ..= 包含 safe_end
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

        match chapter.toc_type {
            TocType::Volume => {
                class_attr = "Preface1";
                let safe_vol_num = escape_xml(&chap_num_raw);
                let safe_vol_name = escape_xml(&chap_name_raw);
                let vertical_num = format_vertical_volume(&safe_vol_num);
                let formatted_name = safe_vol_name
                    .chars()
                    .map(|c| format!("{} ", c))
                    .collect::<String>();
                html_body.push_str(&format!(
                    "  <h1 class=\"PrefacehA1\" title=\"{}\"><br /><br />\n  {}</h1>\n  <p class=\"PrefacepA1\">{}</p>\n", 
                    safe_display_title, vertical_num, formatted_name.trim()
                ));
            }
            TocType::Chapter => {
                let safe_chap_num = escape_xml(&chap_num_raw);
                let safe_chap_name = escape_xml(&chap_name_raw);
                html_body.push_str(&format!(
                    "  <h3 class=\"head\"><span class=\"num\">{}</span><br/><b>{}</b></h3>\n",
                    safe_chap_num, safe_chap_name
                ));
                for line in body_lines {
                    let trim = line.trim();
                    if !trim.is_empty() {
                        html_body.push_str(&format!("  <p>{}</p>\n", escape_xml(trim)));
                    }
                }
            }
            TocType::Meta => {
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

        if chapter.toc_type == TocType::Volume {
            if open_volume {
                ncx_navpoints.push_str("</navPoint>\n");
            }
            ncx_navpoints.push_str(&format!(
                r#"<navPoint id="navPoint-{}" playOrder="{}"><navLabel><text>{}</text></navLabel><content src="{}"/>"#,
                play_order, play_order, safe_display_title, href_in_opf
            ));
            ncx_navpoints.push('\n');
            open_volume = true;
        } else if chapter.toc_type == TocType::Chapter {
            ncx_navpoints.push_str(&format!(
                r#"<navPoint id="navPoint-{}" playOrder="{}"><navLabel><text>{}</text></navLabel><content src="{}"/></navPoint>"#,
                play_order, play_order, safe_display_title, href_in_opf
            ));
            ncx_navpoints.push('\n');
        } else {
            if open_volume {
                ncx_navpoints.push_str("</navPoint>\n");
                open_volume = false;
            }
            ncx_navpoints.push_str(&format!(
                r#"<navPoint id="navPoint-{}" playOrder="{}"><navLabel><text>{}</text></navLabel><content src="{}"/></navPoint>"#,
                play_order, play_order, safe_display_title, href_in_opf
            ));
            ncx_navpoints.push('\n');
        }
        play_order += 1;
    }

    if open_volume {
        ncx_navpoints.push_str("</navPoint>\n");
    }

    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let full_uuid = if metadata.uuid.starts_with("urn:uuid:") {
        metadata.uuid.clone()
    } else {
        format!("urn:uuid:{}", metadata.uuid)
    };

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
  </metadata>
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
        // if file_type == "html" {
        //     if let Ok(content) = fs::read_to_string(full_path) {
        //         let re = Regex::new(r"<title>(.*?)</title>").unwrap();
        //         if let Ok(Some(caps)) = re.captures(&content) {
        //             title = Some(caps[1].to_string());
        //         }
        //     }
        // }

        all_files.push((path_str, file_name, file_type, size, title));
    }

    // 构建嵌套文件树
    fn build_tree(files: &[(String, String, String, u64, Option<String>)]) -> Vec<EpubFileNode> {
        let mut root_map: HashMap<String, Vec<EpubFileNode>> = HashMap::new();

        for (full_path, file_name, file_type, size, title) in files {
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

        for (dir_name, mut files) in sorted_dirs {
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
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
