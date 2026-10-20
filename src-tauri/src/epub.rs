use fancy_regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tempfile::TempDir;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use crate::font::{build_font_subset_text, try_subset_font_bytes};
use crate::fs_utils::{
    build_processed_epub_path, clear_zip_encryption_flags, detect_image_ext, ensure_dir,
    escape_xml, is_zip_archive_bytes, is_zip_archive_path, lock_epub_cache,
    normalize_local_file_path, percent_decode, read_epub_bytes,
    rebuild_epub_with_sanitized_names_from_bytes,
    rebuild_epub_with_sanitized_names_from_bytes_with_progress, sanitize_filename_part, xor_bytes,
    EpubCache,
};
use crate::library::library_extract_dir;
use crate::mobile::mobile_is_meta_title;

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

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct AssetInfo {
    name: String,
    path: String,
    category: String, // "fonts", "images", "others"
    #[serde(default)]
    role: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct EpubMetadata {
    pub(crate) title: String,
    pub(crate) creator: String,
    pub(crate) publisher: String,
    pub(crate) cover_path: String,
    pub(crate) uuid: String,
    pub(crate) md5: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) tags: Vec<String>,
    #[serde(default)]
    pub(crate) main_css: String,
    #[serde(default)]
    pub(crate) font_css: String,
    #[serde(default)]
    pub(crate) subset_fonts: bool,
    #[serde(default)]
    pub(crate) assets: Vec<AssetInfo>,
    #[serde(flatten)]
    pub(crate) extra: HashMap<String, String>,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct EpubFileNode {
    name: String,
    path: String,
    file_type: String, // folder, html, css, xml, image, font, other
    size: Option<u64>,
    title: Option<String>,      // For HTML files
    resolution: Option<String>, // For Image files (e.g., "1920x1080")
    children: Option<Vec<EpubFileNode>>,
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

        if enable_dividers
            && is_ellipsis
            && !prev_is_ellipsis
            && !next_is_ellipsis
            && !is_last_non_empty
        {
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

#[derive(Serialize, Clone)]
pub(crate) struct EpubPrepareResult {
    source_path: String,
    processed_path: String,
    changed: bool,
    action: String,
    /// 源文件的修改时间（unix 毫秒）与大小，随结果一起返回。
    /// 前端拿它给「整本书解析结果」做缓存键：同窗口切标签会让阅读页重新
    /// 挂载，没有这两个值就只能每次重新 fetch 并解析全书（几百章、几秒）。
    source_modified_ms: u64,
    source_size: u64,
}

/// 构造结果时顺便带上源文件的 mtime / 大小（见上面两个字段的用途）
fn epub_prepare_result(
    source: &Path,
    processed: String,
    changed: bool,
    action: impl Into<String>,
) -> EpubPrepareResult {
    let (source_modified_ms, source_size) = prepare_cache_key(source)
        .map(|(_, stamp, len)| (stamp, len))
        .unwrap_or((0, 0));
    EpubPrepareResult {
        source_path: source.to_string_lossy().to_string(),
        processed_path: processed,
        changed,
        action: action.into(),
        source_modified_ms,
        source_size,
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpubPrepareStageEvent {
    epub_path: String,
    message: String,
}

/// 预处理结果缓存：(路径, 修改时间, 大小) → 上次算出的结论。
///
/// prepare 要读整个 EPUB（十几 MB 到上百 MB）并扫一遍 ZIP 目录，耗时几百
/// 毫秒起。同窗口模式切标签会让页面重新挂载、重新调一次 prepare，
/// 每次都重算等于每切一次标签就卡一下。文件没被改过时直接返回上次结论。
/// 修改时间和大小都参与比对：只比时间的话，同一秒内改写的文件会被误判。
type PrepareCacheKey = (String, u64, u64);
static PREPARE_CACHE: once_cell::sync::Lazy<
    std::sync::Mutex<HashMap<PrepareCacheKey, EpubPrepareResult>>,
> = once_cell::sync::Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

fn prepare_cache_key(source: &Path) -> Option<PrepareCacheKey> {
    let meta = fs::metadata(source).ok()?;
    let modified = meta.modified().ok()?;
    let stamp = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis() as u64;
    Some((source.to_string_lossy().to_string(), stamp, meta.len()))
}

fn emit_epub_prepare_stage(app: &tauri::AppHandle, epub_path: &Path, message: impl Into<String>) {
    let _ = app.emit(
        "epub-prepare-stage",
        EpubPrepareStageEvent {
            epub_path: epub_path.to_string_lossy().to_string(),
            message: message.into(),
        },
    );
}

#[tauri::command]
pub async fn prepare_epub_for_open(
    app: tauri::AppHandle,
    epub_path: String,
) -> Result<EpubPrepareResult, String> {
    // 这个命令会读整个 EPUB（几十 MB）并扫全部 ZIP 条目，同步命令会占住
    // 主线程，界面在打开图书时直接冻住。丢到阻塞线程池里跑。
    tauri::async_runtime::spawn_blocking(move || prepare_epub_for_open_impl(app, epub_path))
        .await
        .map_err(|e| format!("准备 EPUB 任务失败: {}", e))?
}

fn prepare_epub_for_open_impl(
    app: tauri::AppHandle,
    epub_path: String,
) -> Result<EpubPrepareResult, String> {
    let source = PathBuf::from(&epub_path);
    if !source.exists() {
        return Err(format!("文件不存在: {}", epub_path));
    }

    // 文件没变过就直接复用上次结论（见 PREPARE_CACHE 注释）
    let cache_key = prepare_cache_key(&source);
    if let Some(key) = cache_key.clone() {
        if let Ok(cache) = PREPARE_CACHE.lock() {
            if let Some(hit) = cache.get(&key) {
                return Ok(hit.clone());
            }
        }
    }

    let result = prepare_epub_for_open_uncached(&app, &source)?;
    if let Some(key) = cache_key {
        if let Ok(mut cache) = PREPARE_CACHE.lock() {
            cache.insert(key, result.clone());
        }
    }
    Ok(result)
}

fn prepare_epub_for_open_uncached(
    app: &tauri::AppHandle,
    source: &Path,
) -> Result<EpubPrepareResult, String> {
    emit_epub_prepare_stage(app, source, "准备 EPUB 文件");

    // Case 1: valid ZIP/EPUB, only repair pseudo-encryption bit if needed.
    if is_zip_archive_path(source) {
        emit_epub_prepare_stage(app, source, "读取 EPUB 文件");
        let mut bytes = read_epub_bytes(source)?;
        emit_epub_prepare_stage(app, source, "检查伪加密标记");
        let fixed_encryption_flag = clear_zip_encryption_flags(&mut bytes);

        let rebuilt = {
            let mut progress = |message: &str| {
                emit_epub_prepare_stage(app, source, message);
            };
            rebuild_epub_with_sanitized_names_from_bytes_with_progress(
                source,
                &bytes,
                "_deobf",
                &mut progress,
            )?
        };

        if let Some(out_path) = rebuilt {
            let action = if fixed_encryption_flag {
                "已自动解除伪加密并修复文件名混淆"
            } else {
                "已自动修复文件名混淆"
            };
            return Ok(epub_prepare_result(
                source,
                out_path.to_string_lossy().to_string(),
                true,
                action,
            ));
        }

        if fixed_encryption_flag {
            emit_epub_prepare_stage(app, source, "写出伪加密修复文件");
            let out_path = build_processed_epub_path(source, "_decrypted");
            fs::write(&out_path, &bytes).map_err(|e| format!("写入修复文件失败: {}", e))?;
            return Ok(epub_prepare_result(
                source,
                out_path.to_string_lossy().to_string(),
                true,
                "已自动解除伪加密".to_string(),
            ));
        }
        emit_epub_prepare_stage(app, source, "EPUB 无需预处理");
        return Ok(epub_prepare_result(
            source,
            source.to_string_lossy().to_string(),
            false,
            "无需处理".to_string(),
        ));
    }

    // Case 2: try simple XOR obfuscation recovery.
    emit_epub_prepare_stage(app, source, "检测非标准混淆格式");
    let bytes = read_epub_bytes(source)?;
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
        emit_epub_prepare_stage(app, source, "尝试 XOR 解混淆");
        let mut decoded = xor_bytes(&bytes, key);
        emit_epub_prepare_stage(app, source, "检查解混淆后的 ZIP 标记");
        let fixed_encryption_flag = clear_zip_encryption_flags(&mut decoded);
        if is_zip_archive_bytes(&decoded) {
            let suffix = if fixed_encryption_flag {
                "_deobf_decrypted"
            } else {
                "_deobf"
            };

            let rebuilt = {
                let mut progress = |message: &str| {
                    emit_epub_prepare_stage(app, source, message);
                };
                rebuild_epub_with_sanitized_names_from_bytes_with_progress(
                    source,
                    &decoded,
                    suffix,
                    &mut progress,
                )?
            };

            if let Some(out_path) = rebuilt {
                return Ok(epub_prepare_result(
                    source,
                    out_path.to_string_lossy().to_string(),
                    true,
                    "已自动解混淆并修复可读格式".to_string(),
                ));
            }

            emit_epub_prepare_stage(app, source, "写出解混淆后的 EPUB");
            let out_path = build_processed_epub_path(&source, suffix);
            fs::write(&out_path, &decoded).map_err(|e| format!("写入解混淆文件失败: {}", e))?;
            return Ok(epub_prepare_result(
                source,
                out_path.to_string_lossy().to_string(),
                true,
                "已自动解混淆并修复可读格式".to_string(),
            ));
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
pub(crate) fn try_prepare_epub_for_ingest(
    source: &Path,
) -> Result<Option<(PathBuf, String)>, String> {
    let bytes = read_epub_bytes(source)?;

    // 在系统临时目录构造一个"虚拟 source"，使 build_processed_epub_path /
    // rebuild_epub_with_sanitized_names_from_bytes 把输出落在临时目录而非源目录。
    let temp_dir = std::env::temp_dir();
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");
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
            fs::write(&out_path, &bytes_mut).map_err(|e| format!("写入修复文件失败: {}", e))?;
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
            fs::write(&out_path, &decoded).map_err(|e| format!("写入解混淆文件失败: {}", e))?;
            return Ok(Some((out_path, "已自动解混淆并修复可读格式".to_string())));
        }
    }

    Err("该 EPUB 可能经过非通用加密/混淆，当前版本暂无法自动处理".to_string())
}

// RAII：函数结束时自动删除入库流程产生的临时清洗文件
pub(crate) struct IngestTempFile(pub(crate) Option<PathBuf>);

impl Drop for IngestTempFile {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            let _ = fs::remove_file(p);
        }
    }
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
pub async fn scan_chapters(content: String, rules: Vec<RegexRule>) -> Vec<ChapterInfo> {
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
pub async fn export_epub(
    save_path: String,
    content: String,
    chapters: Vec<ChapterInfo>,
    metadata: EpubMetadata,
) -> Result<(), String> {
    let path = Path::new(&save_path);
    let file = fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let options_store =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
                safe_display_title, safe_display_title
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

                    let inserted_volume_head = if let Some((role, href)) =
                        first_image_slot_for_placement(
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

                    let inserted_chapter_head = if let Some((role, href)) =
                        first_image_slot_for_placement(
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
            format!(
                "{} {}",
                escape_xml(&chap_num_raw),
                escape_xml(&chap_name_raw)
            )
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
pub async fn extract_epub(
    app: tauri::AppHandle,
    epub_path: String,
) -> Result<Vec<EpubFileNode>, String> {
    tauri::async_runtime::spawn_blocking(move || extract_epub_impl(app, epub_path))
        .await
        .map_err(|e| format!("解压 EPUB 任务失败: {}", e))?
}

fn extract_epub_impl(
    app: tauri::AppHandle,
    epub_path: String,
) -> Result<Vec<EpubFileNode>, String> {
    // 1. 检查是否已经有缓存且临时内容未关闭
    let existing_temp_path = {
        let cache_guard = lock_epub_cache()?;
        if let Some(cache) = cache_guard.get(&epub_path) {
            if let Some(ref temp) = cache.temp_dir {
                Some(temp.path().to_path_buf())
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

    // 2.5 把解压目录授权给 asset 协议。
    // tauri.conf.json 的 assetProtocol.scope 只静态放行 $APPDATA/**，而解压目录
    // 落在书库 _data/extract/ 下（书库根可被用户指向任意目录，便携版还会指向
    // exe 同级 data/），也可能回退到系统临时目录。reader 通过 asset:// 直接
    // fetch 这些文件（比 IPC 快一个数量级），因此必须在运行时按实际路径补授权。
    {
        use tauri::Manager;
        let _ = app
            .asset_protocol_scope()
            .allow_directory(&temp_path_buf, true);
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

        // 提取标题 (如果是 HTML) - 延迟到 load_epub_file_meta 按需补全
        let title = None;
        let mut resolution = None;

        if file_type == "image" {
            // 仅从文件大小预估，不读取图片头（image_dimensions 会全量解码图片头，大图很慢）
            resolution = None;
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
        let mut cache_guard = lock_epub_cache()?;
        let mut cache = EpubCache::new();
        cache.temp_dir = _temp_dir_keep;
        // Concurrent windows opening the same file must not drop an existing extraction.
        cache_guard.entry(epub_path).or_insert(cache);
    }

    Ok(build_tree(&all_files))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EpubFileMeta {
    path: String,
    title: Option<String>,
    resolution: Option<String>,
}

#[tauri::command]
pub async fn load_epub_file_meta(
    epub_path: String,
    file_paths: Vec<String>,
) -> Result<Vec<EpubFileMeta>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
                } else {
                    None
                }
            } else {
                None
            }
        }
        .ok_or("EPUB 未加载或缓存失效".to_string())?;

        let mut results = Vec::with_capacity(file_paths.len());
        for rel_path in file_paths {
            let full_path = temp_path.join(&rel_path);
            let lower = rel_path.to_ascii_lowercase();
            let mut title = None;
            let mut resolution = None;

            if lower.ends_with(".html") || lower.ends_with(".htm") || lower.ends_with(".xhtml") {
                if let Ok(content) = fs::read_to_string(&full_path) {
                    title = extract_html_heading_title(&content);
                }
            } else if lower.ends_with(".jpg")
                || lower.ends_with(".jpeg")
                || lower.ends_with(".png")
                || lower.ends_with(".gif")
                || lower.ends_with(".webp")
                || lower.ends_with(".bmp")
                || lower.ends_with(".svg")
            {
                if let Ok((width, height)) = image::image_dimensions(&full_path) {
                    resolution = Some(format!("{}x{}", width, height));
                }
            }

            results.push(EpubFileMeta {
                path: rel_path,
                title,
                resolution,
            });
        }
        Ok(results)
    })
    .await
    .map_err(|e| format!("加载文件元信息失败: {}", e))?
}

#[tauri::command]
pub async fn get_epub_temp_dir_path(epub_path: String) -> Result<String, String> {
    let cache_guard = lock_epub_cache()?;
    if let Some(cache) = cache_guard.get(&epub_path) {
        if let Some(ref temp) = cache.temp_dir {
            return Ok(temp.path().to_string_lossy().to_string());
        }
    }
    Err("EPUB 未加载或缓存失效".to_string())
}

#[tauri::command]
pub async fn read_epub_file_content(
    epub_path: String,
    file_path: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. 获取临时目录路径
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

#[tauri::command]
pub async fn read_epub_file_binary(
    epub_path: String,
    file_path: String,
) -> Result<Vec<u8>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. 获取临时目录路径
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

// --- 批量读取 API ---

#[tauri::command]
pub async fn read_epub_files_batch(
    epub_path: String,
    file_paths: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        use zip::ZipArchive;

        let mut results: HashMap<String, String> = HashMap::new();
        let mut to_read: Vec<String> = Vec::new();

        // 1. 检查缓存，收集需要读取的文件
        {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
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
        }

        // 2. 批量读取未缓存的文件
        if !to_read.is_empty() {
            let file = fs::File::open(&epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
            let mut archive =
                ZipArchive::new(file).map_err(|e| format!("无效的 EPUB 文件: {}", e))?;

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
                let mut cache_guard = lock_epub_cache()?;
                if let Some(cache) = cache_guard.get_mut(&epub_path) {
                    for (path, content) in new_contents {
                        cache.text_cache.insert(path, content);
                    }
                }
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

#[tauri::command]
pub async fn read_epub_binary_batch(
    epub_path: String,
    file_paths: Vec<String>,
) -> Result<HashMap<String, Vec<u8>>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Read;
        use zip::ZipArchive;

        let mut results: HashMap<String, Vec<u8>> = HashMap::new();
        let mut to_read: Vec<String> = Vec::new();

        // 1. 检查缓存
        {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
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
        }

        // 2. 批量读取
        if !to_read.is_empty() {
            let file = fs::File::open(&epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
            let mut archive =
                ZipArchive::new(file).map_err(|e| format!("无效的 EPUB 文件: {}", e))?;

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
                let mut cache_guard = lock_epub_cache()?;
                if let Some(cache) = cache_guard.get_mut(&epub_path) {
                    for (path, data) in new_data {
                        cache.binary_cache.insert(path, data);
                    }
                }
            }
        }

        Ok(results)
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

// --- EPUB 文件保存命令 ---

#[tauri::command]
pub async fn save_epub_file_content(
    epub_path: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. 获取临时目录路径
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
            let mut cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get_mut(&epub_path) {
                cache.text_cache.insert(file_path, content);
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

#[tauri::command]
pub async fn save_epub_file_binary(
    epub_path: String,
    file_path: String,
    content: Vec<u8>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. 获取临时目录路径
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
            let mut cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get_mut(&epub_path) {
                cache.binary_cache.insert(file_path, Default::default());
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

#[tauri::command]
pub async fn save_epub_files_batch(
    epub_path: String,
    files: HashMap<String, Vec<u8>>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. Get Temp Path
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
                let mut cache_guard = lock_epub_cache()?;
                if let Some(cache) = cache_guard.get_mut(&epub_path) {
                    // If it was text, update with valid utf8?
                    // Risk of decoding binary as text.
                    // Safer to remove from text_cache and let it re-read from disk on next access.
                    cache.text_cache.remove(&file_path);
                    cache.binary_cache.remove(&file_path);
                }
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

#[tauri::command]
pub async fn save_epub_to_disk(epub_path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        use zip::write::SimpleFileOptions;

        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let options_stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
pub async fn add_epub_file(
    epub_path: String,
    file_path: String,
    content: String,
) -> Result<(), String> {
    add_epub_file_binary(epub_path, file_path, content.into_bytes()).await
}

#[tauri::command]
pub async fn add_epub_file_binary(
    epub_path: String,
    file_path: String,
    content: Vec<u8>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. 获取临时目录
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                if let Some(ref temp) = cache.temp_dir {
                    Some(temp.path().to_path_buf())
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
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

#[tauri::command]
pub async fn delete_epub_file(epub_path: String, file_path: String) -> Result<(), String> {
    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = lock_epub_cache()?;
        if let Some(cache) = cache_guard.get(&epub_path) {
            if let Some(ref temp) = cache.temp_dir {
                Some(temp.path().to_path_buf())
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
pub async fn rename_epub_file(
    epub_path: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    // 1. 获取临时目录
    let temp_path: PathBuf = {
        let cache_guard = lock_epub_cache()?;
        if let Some(cache) = cache_guard.get(&epub_path) {
            if let Some(ref temp) = cache.temp_dir {
                Some(temp.path().to_path_buf())
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

#[cfg(test)]
mod multi_book_tests {
    use super::*;

    #[test]
    fn different_books_keep_independent_extractions_and_can_both_be_saved() {
        let root = tempfile::tempdir().unwrap();
        let chapter = "OEBPS/chapter.xhtml".to_string();
        let mut books = Vec::new();
        for (name, text) in [
            ("first.epub", "first original"),
            ("second.epub", "second original"),
        ] {
            let path = root.path().join(name).to_string_lossy().into_owned();
            let temp = tempfile::tempdir().unwrap();
            fs::create_dir_all(temp.path().join("OEBPS")).unwrap();
            fs::write(temp.path().join("mimetype"), "application/epub+zip").unwrap();
            fs::write(temp.path().join(&chapter), text).unwrap();
            let mut cache = EpubCache::new();
            cache.temp_dir = Some(temp);
            lock_epub_cache().unwrap().insert(path.clone(), cache);
            books.push(path);
        }

        tauri::async_runtime::block_on(async {
            assert_ne!(
                get_epub_temp_dir_path(books[0].clone()).await.unwrap(),
                get_epub_temp_dir_path(books[1].clone()).await.unwrap(),
            );
            assert_eq!(
                read_epub_file_content(books[0].clone(), chapter.clone())
                    .await
                    .unwrap(),
                "first original",
            );
            assert_eq!(
                read_epub_file_content(books[1].clone(), chapter.clone())
                    .await
                    .unwrap(),
                "second original",
            );
            for (path, text) in books.iter().zip(["first edited", "second edited"]) {
                save_epub_file_content(path.clone(), chapter.clone(), text.to_string())
                    .await
                    .unwrap();
            }
            for (path, text) in books.iter().zip(["first edited", "second edited"]) {
                assert_eq!(
                    read_epub_file_content(path.clone(), chapter.clone())
                        .await
                        .unwrap(),
                    text,
                );
                assert_eq!(
                    read_epub_files_batch(path.clone(), vec![chapter.clone()])
                        .await
                        .unwrap()[&chapter],
                    text,
                );
                save_epub_to_disk(path.clone()).await.unwrap();
                let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
                let mut saved = String::new();
                archive
                    .by_name(&chapter)
                    .unwrap()
                    .read_to_string(&mut saved)
                    .unwrap();
                assert_eq!(saved, text);
            }
        });
        let mut caches = lock_epub_cache().unwrap();
        for path in books {
            caches.remove(&path);
        }
    }
}

pub(crate) struct EpubParsedMeta {
    pub(crate) title: String,
    pub(crate) author: String,
    pub(crate) publisher: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) epub_uuid: String,
    pub(crate) cover_bytes: Option<Vec<u8>>,
    pub(crate) cover_ext: String,          // "jpg" / "png"
    pub(crate) pub_date: Option<u64>,      // <dc:date> 解析后的 unix 秒
    pub(crate) modified_date: Option<u64>, // <meta property="dcterms:modified"> 解析后的 unix 秒
    // 我们写入 OPF 时用的扩展字段，导入时也要读回来
    pub(crate) subtitle: Option<String>, // <meta name="calibre:subtitle" content="..."/>
    pub(crate) series: Option<String>,   // <meta name="calibre:series" content="..."/>
    pub(crate) maker: Option<String>,    // <meta name="maker" content="..."/>
    pub(crate) tags: Vec<String>,        // 所有 <dc:subject>
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
    let pat = format!(r#"<{0}(?:\s[^>]*)?>([\s\S]*?)</{0}>"#, re_escape(tag));
    let re = Regex::new(&pat).ok()?;
    match re.captures(xml) {
        Ok(Some(c)) => c.get(1).map(|m| xml_unescape(m.as_str().trim())),
        _ => None,
    }
}

// 抽取所有 <{tag}>...</{tag}> 的内容，返回 Vec（用于多个 dc:subject 这类）
fn extract_all_tags(xml: &str, tag: &str) -> Vec<String> {
    let pat = format!(r#"<{0}(?:\s[^>]*)?>([\s\S]*?)</{0}>"#, re_escape(tag));
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
    let outer_pat = format!(r#"<meta\s+[^>]*name="{}"[^>]*/?>"#, re_escape(name));
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
fn try_read_zip_entry(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> Option<Vec<u8>> {
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

pub(crate) fn parse_epub_metadata(epub_path: &Path) -> Result<EpubParsedMeta, String> {
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
    let pub_date = extract_first_tag(&opf_xml, "dc:date").and_then(|s| parse_epub_date(&s));

    // EPUB 3：<meta property="dcterms:modified">YYYY-MM-DDTHH:MM:SSZ</meta>
    let modified_date =
        match Regex::new(r#"<meta\s+[^>]*property="dcterms:modified"[^>]*>([\s\S]*?)</meta>"#) {
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

pub(crate) fn read_opf_from_dir(root: &Path) -> Result<(String, String), String> {
    let container_path = root.join("META-INF").join("container.xml");
    let container_xml = fs::read_to_string(&container_path)
        .map_err(|e| format!("读取 container.xml 失败: {}", e))?;
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
    let modified_date =
        match Regex::new(r#"<meta\s+[^>]*property="dcterms:modified"[^>]*>([\s\S]*?)</meta>"#) {
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

pub(crate) fn parse_epub_metadata_live(epub_path: &Path) -> Result<EpubParsedMeta, String> {
    let temp_path = {
        let cache_guard = lock_epub_cache()?;
        if let Some(cache) = cache_guard.get(epub_path.to_string_lossy().as_ref()) {
            cache
                .temp_dir
                .as_ref()
                .map(|temp| temp.path().to_path_buf())
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
            let disk_path = normalized
                .split('/')
                .fold(root.clone(), |acc, part| acc.join(part));
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

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
pub(crate) fn set_dcterms_modified(opf: &str, iso: &str) -> String {
    let rm_pat = r#"\s*<meta\s+(?:[^>]*\s)?property="dcterms:modified"[^>]*>[\s\S]*?</meta>"#;
    let s = remove_all_matches(opf, rm_pat);
    let inject = format!(
        "    <meta property=\"dcterms:modified\">{}</meta>\n  ",
        xml_escape(iso)
    );
    insert_before_metadata_close(&s, &inject)
}

pub(crate) fn write_opf_metadata(
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
    s = remove_all_matches(&s, r#"\s*<dc:subject(?:\s[^>]*)?>[\s\S]*?</dc:subject>"#);
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
pub(crate) fn read_opf_from_epub(epub_path: &Path) -> Result<(String, String), String> {
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
pub(crate) fn locate_cover_in_opf(opf_path: &str, opf_xml: &str) -> Option<String> {
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

pub(crate) struct EpubRewrite {
    pub(crate) opf_replacement: Option<String>,
    pub(crate) cover_replacement: Option<Vec<u8>>,
}

// 流式重打包：拷贝原 zip 所有项，遇到 OPF / 封面则替换
pub(crate) fn rewrite_epub(epub_path: &Path, changes: &EpubRewrite) -> Result<(), String> {
    use zip::write::SimpleFileOptions;

    let (opf_path, opf_xml) = read_opf_from_epub(epub_path)?;
    let cover_internal = if changes.cover_replacement.is_some() {
        locate_cover_in_opf(&opf_path, &opf_xml)
    } else {
        None
    };

    let src = fs::File::open(epub_path).map_err(|e| format!("无法打开 EPUB: {}", e))?;
    let mut archive = zip::ZipArchive::new(src).map_err(|e| format!("无效 EPUB: {}", e))?;

    let tmp_path = format!("{}.tmp", epub_path.to_string_lossy());
    let tmp_file = fs::File::create(&tmp_path).map_err(|e| format!("创建临时 zip 失败: {}", e))?;
    let mut zw = zip::ZipWriter::new(tmp_file);
    let opts_deflated =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let opts_stored =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
            if let (Some(ci), Some(bytes)) =
                (cover_internal.as_ref(), changes.cover_replacement.as_ref())
            {
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

    fs::rename(&tmp_path, epub_path).map_err(|e| format!("替换 EPUB 失败: {}", e))?;
    Ok(())
}
