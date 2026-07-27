use fancy_regex::Regex;
use md5;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::epub::{EpubParsedMeta, EpubRewrite, IngestTempFile, parse_epub_metadata, read_opf_from_epub, rewrite_epub, set_dcterms_modified, try_prepare_epub_for_ingest, write_opf_metadata};
use crate::fs_utils::{app_data_root, detect_image_ext, ensure_dir, is_portable, portable_data_root, sanitize_filename_part};

#[derive(Serialize)]
pub(crate) struct HistoryMeta {
    filename: String,
    path: String,
    timestamp: u64,
    size: u64,
    date_str: String,
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

#[derive(Serialize, Clone)]
pub(crate) struct CoverSearchResult {
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
pub async fn search_book_covers(
    title: String,
    author: String,
) -> Result<Vec<CoverSearchResult>, String> {
    let clean_title = title.trim();
    let clean_author = author.trim();
    if clean_title.is_empty() || clean_title == "书名" || clean_title == "涔﹀悕" {
        return Err("请先填写书名".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) TEpub-Editor/0.5")
        .build()
        .map_err(|e| format!("初始化封面搜索失败: {}", e))?;
    let image_meta_re =
        Regex::new(r#"m="([^"]+)""#).map_err(|e| format!("封面结果解析失败: {}", e))?;
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
            let combined_key = format!(
                "{}{}{}{}",
                result_title_key, page_key, source_key, image_key
            );
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
    Ok(results
        .into_iter()
        .take(12)
        .map(|(_, result)| result)
        .collect())
}

#[tauri::command]
pub async fn download_cover_to_temp(image_url: String, title: String) -> Result<String, String> {
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
    let stem = if base.is_empty() {
        "cover".to_string()
    } else {
        base
    };
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
pub fn calculate_md5(content: String) -> String {
    format!("{:x}", md5::compute(content.as_bytes()))
}

#[tauri::command]
pub async fn save_history(
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
pub async fn get_history_list(app: tauri::AppHandle, original_path: String) -> Vec<HistoryMeta> {
    let path = Path::new(&original_path);
    let file_stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let history_dir = get_history_base_dir(Some(&app));
    let file_prefix = format!("{}-{}", file_stem, history_key_for_path(&original_path));
    let mut list = Vec::new();
    if let Ok(entries) = fs::read_dir(history_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let fname = entry.file_name().to_string_lossy().to_string();
            if (fname.starts_with(&file_prefix) || fname.starts_with(&format!("{}.", file_stem)))
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

// 重命名书库内的本地文件，并同步 library.json 的 file_path / filename。
// - 仅在「书在 books_dir」或「ref_only 文件可访问」时生效
// - 同名冲突返回 BOOK_FILE_COLLISION:{suggested_filename}
// - 自动按文件类型补回扩展名（用户输入"新名"或"新名.epub"都行）
#[tauri::command]
pub async fn rename_book_file(
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
pub(crate) struct RebuildSummary {
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
pub async fn rebuild_book_filenames(
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
        let stem = if stem.is_empty() {
            "未命名".to_string()
        } else {
            stem
        };
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
        let tmp_name = format!(".__rename_tmp_{}_{}", idx, uuid::Uuid::new_v4().simple());
        let tmp_path = books_dir.join(&tmp_name);
        let final_path = books_dir.join(&target_name);
        if let Err(e) = fs::rename(&from, &tmp_path) {
            summary.failed += 1;
            summary.failures.push(format!(
                "{}: {}",
                from.file_name().and_then(|s| s.to_str()).unwrap_or(""),
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
                    tmp_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
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
pub(crate) struct LibraryConfig {
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
pub(crate) struct AiProofingConfig {
    #[serde(default)]
    pub(crate) enabled: bool,
    #[serde(default = "default_ai_base_url")]
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default = "default_ai_model")]
    pub(crate) model: String,
    #[serde(default = "default_ai_temperature")]
    pub(crate) temperature: f32,
    #[serde(default = "default_ai_max_chapter_chars")]
    pub(crate) max_chapter_chars: usize,
    #[serde(default = "default_ai_response_timeout_sec")]
    pub(crate) response_timeout_sec: u64,
    #[serde(default)]
    pub(crate) auto_approve: bool,
    #[serde(default)]
    pub(crate) extra_prompt: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiProviderConfig {
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
pub(crate) struct TxtAiProofingConfig {
    #[serde(default)]
    provider_id: String,
    #[serde(default)]
    approval_provider_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryAiMatchConfig {
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
pub(crate) struct BookEntry {
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
pub(crate) struct LibraryData {
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
pub(crate) fn library_pointer_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
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
pub(crate) fn library_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("_data"))
}

// 图书 子目录：所有书文件按「书名-作者.ext」命名后存这里
fn library_books_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("图书"))
}

pub(crate) fn library_fonts_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_root_dir(app)?.join("字体"))
}

fn library_covers_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("covers"))
}

fn library_history_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("history"))
}

pub(crate) fn library_proof_logs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(library_data_dir(app)?.join("proof_logs"))
}

pub(crate) fn library_extract_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
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
    let bytes = fs::read(&old_json).map_err(|e| format!("迁移：读旧 library.json 失败: {}", e))?;
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
        let mut f = fs::File::create(&tmp).map_err(|e| format!("创建临时文件失败: {}", e))?;
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
pub async fn load_library(app: tauri::AppHandle) -> Result<LibraryData, String> {
    read_library_data(&app)
}

#[tauri::command]
pub async fn save_library(app: tauri::AppHandle, data: LibraryData) -> Result<(), String> {
    write_library_data_atomic(&app, &data)
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
pub async fn add_book_to_library(
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
    let _epub_cleanup_guard = IngestTempFile(epub_prep_result.as_ref().map(|(p, _)| p.clone()));
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
                    fs::write(&cover_path, &bytes).map_err(|e| format!("写入封面失败: {}", e))?;
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
pub async fn remove_book_from_library(app: tauri::AppHandle, book_id: String) -> Result<(), String> {
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
        let inside_managed =
            book_path.starts_with(&books_dir) || book_path.starts_with(&legacy_files_dir);
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
pub fn get_library_cover_data(cover_path: String) -> Result<Vec<u8>, String> {
    if cover_path.trim().is_empty() {
        return Err("封面路径为空".to_string());
    }
    fs::read(&cover_path).map_err(|e| format!("读取封面失败: {}", e))
}

#[tauri::command]
pub async fn refresh_book_metadata(
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
                    let cover_path = covers_dir.join(format!("{}.{}", entry.id, meta.cover_ext));
                    fs::write(&cover_path, &bytes).map_err(|e| format!("写入封面失败: {}", e))?;
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

#[tauri::command]
pub async fn update_book_metadata(
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
pub async fn update_book_cover(
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
    fs::write(&cover_path, &cover_data).map_err(|e| format!("写入封面失败: {}", e))?;
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

