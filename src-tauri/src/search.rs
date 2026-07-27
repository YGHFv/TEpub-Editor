use fancy_regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use crate::fs_utils::{lock_epub_cache};

#[derive(Serialize, Clone, Copy)]
struct MatchLocation {
    line: usize,
    start_char: usize,
    end_char: usize,
}

#[derive(Serialize)]
pub(crate) struct SearchResult {
    found: bool,
    count: usize,
    matches: Vec<MatchLocation>,
}

#[tauri::command]
pub async fn advanced_search(content: String, pattern: String, is_regex: bool) -> SearchResult {
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
pub async fn advanced_replace(
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
pub async fn search_in_files(
    epub_path: String,
    files: Vec<String>,
    pattern: String,
    is_regex: bool,
) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 1. 获取临时目录
        let temp_path: PathBuf = {
            let cache_guard = lock_epub_cache()?;
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
    })
    .await
    .map_err(|e| format!("任务失败: {}", e))?
}

