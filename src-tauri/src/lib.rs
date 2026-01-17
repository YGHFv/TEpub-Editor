use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::process; // 引入进程控制
use chardetng::EncodingDetector;
use fancy_regex::Regex;
use serde::{Serialize, Deserialize};
use zip::write::FileOptions;
use md5; 

// --- 静态资源: 整理后的 CSS ---

const CSS_FONT: &str = r#"@charset "utf-8";
/*筑紫A丸+sleek+液晶数字 日标*/
@font-face {
    font-family: "Maintext";
    src: url("../Fonts/Maintext.ttf");
}

/*哥特式字体*/
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
enum TocType { Volume, Chapter, Meta }

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ChapterInfo {
    title: String,
    line_number: usize,
    toc_type: TocType,
    word_count: usize,
}

#[derive(Serialize, Clone, Copy)]
struct MatchLocation { line: usize, start_char: usize, end_char: usize }

#[derive(Serialize)]
struct SearchResult { found: bool, count: usize, matches: Vec<MatchLocation> }

#[derive(Serialize)]
struct HistoryMeta { filename: String, path: String, timestamp: u64, size: u64, date_str: String }

#[derive(Deserialize, Debug)]
struct EpubMetadata {
    title: String,
    creator: String,
    publisher: String,
    cover_path: String,
    uuid: String,
    md5: String,
}

// --- 辅助函数 ---

fn escape_xml(input: &str) -> String {
    input.replace("&", "&amp;")
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
    let strict_re = Regex::new(r"^\s*(第[0-9零一二三四五六七八九十百千万]+[卷章回]|Chapter\s*\d+)\s*(.*)$").unwrap();
    if let Ok(Some(caps)) = strict_re.captures(full_title) {
        let num = caps.get(1).map_or("", |m| m.as_str()).trim().to_string();
        let name = caps.get(2).map_or("", |m| m.as_str()).trim().to_string();
        if !num.is_empty() { return (num, name); }
    }
    let loose_re = Regex::new(r"^(.*?)\s+(.*)$").unwrap();
    if let Ok(Some(caps)) = loose_re.captures(full_title) {
        let num = caps.get(1).map_or("", |m| m.as_str()).to_string();
        let name = caps.get(2).map_or("", |m| m.as_str()).to_string();
        return (num, name);
    }
    (full_title.to_string(), "".to_string())
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
    file.read_to_end(&mut buffer).map_err(|e| format!("读取失败: {}", e))?;
    let mut detector = EncodingDetector::new();
    detector.feed(&buffer, true);
    let encoding = detector.guess(None, true);
    let (cow, _, _) = encoding.decode(&buffer);
    Ok(cow.into_owned())
}

#[tauri::command]
async fn save_text_file(path: String, content: String) -> Result<(), String> {
    let mut file = fs::File::create(&path).map_err(|e| format!("无法创建: {}", e))?;
    file.write_all(content.as_bytes()).map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
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
    if !history_dir.exists() { fs::create_dir_all(&history_dir).map_err(|e| e.to_string())?; }
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let backup_name = format!("{}.{}.bak", file_stem, timestamp);
    let backup_path = history_dir.join(backup_name);
    let mut file = fs::File::create(&backup_path).map_err(|e| e.to_string())?;
    file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    if let Ok(entries) = fs::read_dir(&history_dir) {
        let mut backups: Vec<_> = entries.filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with(&*file_stem)).collect();
        backups.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH));
        if backups.len() > 10 { for entry in backups.iter().take(backups.len() - 10) { let _ = fs::remove_file(entry.path()); } }
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
                        timestamp: meta.modified().unwrap_or(SystemTime::now()).duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
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
async fn scan_chapters(content: String, volreg: String, chapreg: String, metareg: String) -> Vec<ChapterInfo> {
    let mut chapters = Vec::new();
    let re_volume = Regex::new(&volreg).unwrap_or_else(|_| Regex::new(r"^\s*第[零一二三四五六七八九十百千万0-9]+[卷部].*").unwrap());
    let re_chapter = Regex::new(&chapreg).unwrap_or_else(|_| Regex::new(r"^\s*(第[一二三四五六七八九十百千万0-9]+[章回]|Chapter\s*\d+).*").unwrap());
    let re_meta = Regex::new(&metareg).unwrap_or_else(|_| Regex::new(r"^\s*(内容)?(简介|序[章言]?|前言|楔子|后记|完本感言).*").unwrap());
    let mut current_chapter: Option<ChapterInfo> = None;
    for (index, line) in content.lines().enumerate() {
        let line_trim = line.trim();
        let char_count = line_trim.chars().count();
        let is_empty = line_trim.is_empty();
        let toc_type = if !is_empty && char_count <= 60 { 
            if re_volume.is_match(line).unwrap_or(false) { Some(TocType::Volume) }
            else if re_meta.is_match(line).unwrap_or(false) { Some(TocType::Meta) }
            else if re_chapter.is_match(line).unwrap_or(false) { Some(TocType::Chapter) }
            else { None }
        } else { None };
        if let Some(t) = toc_type {
            if let Some(prev) = current_chapter.take() { chapters.push(prev); }
            current_chapter = Some(ChapterInfo { title: line_trim.to_string(), line_number: index + 1, toc_type: t, word_count: 0 });
        } else {
            if let Some(ref mut chapter) = current_chapter { if !is_empty { chapter.word_count += char_count; } }
        }
    }
    if let Some(last) = current_chapter { chapters.push(last); }
    chapters
}

#[tauri::command]
async fn advanced_search(content: String, pattern: String, is_regex: bool) -> SearchResult {
    if pattern.is_empty() { return SearchResult { found: false, count: 0, matches: vec![] }; }
    let mut matches_vec = Vec::new();
    if is_regex {
        if let Ok(re) = Regex::new(&pattern) {
            for (i, line) in content.lines().enumerate() {
                for m in re.find_iter(line) {
                    if let Ok(match_obj) = m {
                        matches_vec.push(MatchLocation { line: i + 1, start_char: line[..match_obj.start()].chars().count(), end_char: line[..match_obj.start()].chars().count() + line[match_obj.start()..match_obj.end()].chars().count() });
                    }
                }
            }
        }
    } else {
        for (i, line) in content.lines().enumerate() {
            for (byte_idx, part) in line.match_indices(&pattern) {
                matches_vec.push(MatchLocation { line: i + 1, start_char: line[..byte_idx].chars().count(), end_char: line[..byte_idx].chars().count() + part.chars().count() });
            }
        }
    }
    let count = matches_vec.len();
    SearchResult { found: count > 0, count, matches: matches_vec }
}

#[tauri::command]
async fn advanced_replace(content: String, pattern: String, replacement: String, is_regex: bool) -> Result<String, String> {
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
    metadata: EpubMetadata
) -> Result<(), String> {
    let path = Path::new(&save_path);
    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let options_store = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("mimetype", options_store).map_err(|e| e.to_string())?;
    zip.write_all(b"application/epub+zip").map_err(|e| e.to_string())?;

    zip.start_file("META-INF/container.xml", options).map_err(|e| e.to_string())?;
    zip.write_all(r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
   <rootfiles>
      <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
   </rootfiles>
</container>"#.as_bytes()).map_err(|e| e.to_string())?;

    zip.start_file("OEBPS/Styles/font.css", options).map_err(|e| e.to_string())?;
    zip.write_all(CSS_FONT.as_bytes()).map_err(|e| e.to_string())?;
    zip.start_file("OEBPS/Styles/main.css", options).map_err(|e| e.to_string())?;
    zip.write_all(CSS_MAIN.as_bytes()).map_err(|e| e.to_string())?;

    let mut has_cover = false;
    let mut cover_ext = "jpg".to_string();
    if !metadata.cover_path.is_empty() {
        if let Ok(img_bytes) = fs::read(&metadata.cover_path) {
            cover_ext = Path::new(&metadata.cover_path).extension().and_then(|s| s.to_str()).unwrap_or("jpg").to_lowercase();
            let cover_filename = format!("OEBPS/Images/cover.{}", cover_ext);
            zip.start_file(&cover_filename, options).map_err(|e| e.to_string())?;
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
        let mime = if cover_ext == "png" { "image/png" } else { "image/jpeg" };
        manifest_items.push_str(&format!(r#"<item id="cover-image" href="Images/cover.{}" media-type="{}" properties="cover-image"/>"#, cover_ext, mime));
    }
    manifest_items.push_str(r#"<item id="font.css" href="Styles/font.css" media-type="text/css"/>"#);
    manifest_items.push_str(r#"<item id="main.css" href="Styles/main.css" media-type="text/css"/>"#);

    for (i, chapter) in chapters.iter().enumerate() {
        let file_name_in_zip = format!("OEBPS/Text/chapter{}.xhtml", i);
        let href_in_opf = format!("Text/chapter{}.xhtml", i);
        let id = format!("chapter{}", i);
        
        let start_line = chapter.line_number;
        let end_line = if i + 1 < chapters.len() { chapters[i+1].line_number } else { lines.len() };
        let safe_end = end_line.min(lines.len());
        let safe_start = start_line.min(safe_end);
        let body_lines = if safe_start + 1 < safe_end { &lines[safe_start + 1..safe_end] } else { &[] };

        let mut html_body = String::new();
        let mut class_attr = "";
        
        let (chap_num_raw, chap_name_raw) = split_title(&chapter.title);
        let safe_display_title = if !chap_num_raw.is_empty() && !chap_name_raw.is_empty() {
            format!("{} {}", escape_xml(&chap_num_raw), escape_xml(&chap_name_raw))
        } else {
            escape_xml(&chapter.title)
        };

        match chapter.toc_type {
            TocType::Volume => {
                class_attr = "Preface1";
                let safe_vol_num = escape_xml(&chap_num_raw);
                let safe_vol_name = escape_xml(&chap_name_raw);
                let vertical_num = format_vertical_volume(&safe_vol_num);
                let formatted_name = safe_vol_name.chars().map(|c| format!("{} ", c)).collect::<String>();
                html_body.push_str(&format!(
                    "  <h1 class=\"PrefacehA1\" title=\"{}\"><br /><br />\n  {}</h1>\n  <p class=\"PrefacepA1\">{}</p>\n", 
                    safe_display_title, vertical_num, formatted_name.trim()
                ));
            },
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
            },
            TocType::Meta => {
                html_body.push_str(&format!("  <h1 class=\"nrjj-title\">{}</h1>\n", safe_display_title));
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
            if class_attr.is_empty() { String::new() } else { format!(" class=\"{}\"", class_attr) },
            html_body
        );

        zip.start_file(&file_name_in_zip, options).map_err(|e| e.to_string())?;
        zip.write_all(full_html.as_bytes()).map_err(|e| e.to_string())?;

        manifest_items.push_str(&format!(r#"<item id="{}" href="{}" media-type="application/xhtml+xml"/>"#, id, href_in_opf));
        spine_refs.push_str(&format!(r#"<itemref idref="{}"/>"#, id));
        
        if chapter.toc_type == TocType::Volume {
            if open_volume { ncx_navpoints.push_str("</navPoint>\n"); }
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

    if open_volume { ncx_navpoints.push_str("</navPoint>\n"); }

    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let full_uuid = if metadata.uuid.starts_with("urn:uuid:") { metadata.uuid.clone() } else { format!("urn:uuid:{}", metadata.uuid) };

    let opf_content = format!(
r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" unique-identifier="BookId" version="2.0">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
    <dc:title id="t1">{}</dc:title>
    <dc:creator id="creator">{}</dc:creator>
    <dc:date>{}</dc:date>
    <dc:publisher>{}</dc:publisher>
    <dc:identifier opf:scheme="UUID" id="BookId">{}</dc:identifier>
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
        escape_xml(&metadata.title), escape_xml(&metadata.creator), date_str, escape_xml(&metadata.publisher), 
        full_uuid, metadata.md5, manifest_items, spine_refs
    );

    zip.start_file("OEBPS/content.opf", options).map_err(|e| e.to_string())?;
    zip.write_all(opf_content.as_bytes()).map_err(|e| e.to_string())?;

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
        full_uuid, escape_xml(&metadata.title), ncx_navpoints
    );

    zip.start_file("OEBPS/toc.ncx", options).map_err(|e| e.to_string())?;
    zip.write_all(ncx_content.as_bytes()).map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            read_text_file, save_text_file, save_history, get_history_list, calculate_md5,
            scan_chapters, advanced_search, advanced_replace, export_epub,
            exit_app // 注册新指令
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}