use fancy_regex::Regex;
use md5;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::app::read_text_file;
use crate::epub::{
    export_epub, locate_cover_in_opf, parse_epub_metadata, parse_epub_metadata_live,
    read_opf_from_dir, read_opf_from_epub, rewrite_epub, scan_chapters, write_opf_metadata,
    ChapterInfo, EpubMetadata, EpubParsedMeta, EpubRewrite, RegexRule,
};
use crate::fs_utils::{
    app_data_root, build_processed_epub_path, ensure_dir, lock_epub_cache, sanitize_filename_part,
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub(crate) struct MobileEpubMetadata {
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
pub(crate) struct MobileEpubCover {
    bytes: Vec<u8>,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct MobileExportResult {
    output_path: String,
    public_output: bool,
    message: String,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct MobileMakeEpubResult {
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

#[tauri::command]
pub async fn mobile_cache_input_file(
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

pub(crate) fn mobile_is_meta_title(title: &str) -> bool {
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

    if let Ok(prose_sequence) = Regex::new(r"^序列\s*[0-9零〇一二两三四五六七八九十百千万]+\p{Han}")
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

    if let Ok(re) = Regex::new(r"^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(章|节|回)(\S?)")
    {
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
            if !next_char.is_empty() && !["：", ":", "、", ".", "．", "-", "—"].contains(&next_char)
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
pub async fn mobile_scan_chapters(content: String, rules: Vec<RegexRule>) -> Vec<ChapterInfo> {
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
pub async fn mobile_make_epub(
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
    let out_path = build_processed_epub_path(
        &out_dir.join(format!("{}.epub", sanitize_filename_part(&book_title))),
        "",
    );
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
pub async fn mobile_read_epub_metadata(epub_path: String) -> Result<MobileEpubMetadata, String> {
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
pub async fn mobile_read_epub_cover(epub_path: String) -> Result<MobileEpubCover, String> {
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
pub async fn mobile_update_epub_metadata(
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
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                cache
                    .temp_dir
                    .as_ref()
                    .map(|temp| temp.path().to_path_buf())
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
            fs::write(&temp_opf_disk_path, &new_opf)
                .map_err(|e| format!("写入缓存 OPF 失败: {}", e))?;
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
pub async fn mobile_update_epub_cover(
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
            let cache_guard = lock_epub_cache()?;
            if let Some(cache) = cache_guard.get(&epub_path) {
                cache
                    .temp_dir
                    .as_ref()
                    .map(|temp| temp.path().to_path_buf())
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
pub async fn mobile_export_epub(
    app: tauri::AppHandle,
    epub_path: String,
    file_name: String,
) -> Result<MobileExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let src = PathBuf::from(epub_path);
        if !src.exists() {
            return Err("待导出的 EPUB 文件不存在".to_string());
        }

        // 导出目标可能落在公共下载目录（Android /storage/emulated/0/Download 等），
        // 位于 fs 插件静态 scope（$APPDATA/**）之外。前端 offerSystemExport 会用
        // plugin-fs 读回该文件交给系统分享面板，因此导出成功后按实际路径补授权。
        let allow_read_back = |path: &Path| {
            use tauri_plugin_fs::FsExt;
            if let Some(scope) = app.try_fs_scope() {
                let _ = scope.allow_file(path);
            }
        };

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
                            allow_read_back(&target);
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
                                    allow_read_back(&target);
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
        allow_read_back(&target);
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
