use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::fs_utils::{
    app_data_root, ensure_dir, normalize_local_file_path, sanitize_filename_part,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct EpubTemplateRepository {
    id: String,
    name: String,
    url: String,
    #[serde(default = "default_template_branch")]
    branch: String,
    #[serde(default)]
    last_synced: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct EpubRemoteTemplate {
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
pub(crate) struct EpubTemplateRepositoryIndex {
    #[serde(default = "default_template_schema")]
    schema: u32,
    #[serde(default)]
    templates: Vec<EpubRemoteTemplate>,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct EpubTemplateInstallResult {
    template_id: String,
    local_path: String,
    file_count: usize,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct StyleTemplateInfo {
    id: String,
    name: String,
    file_name: String,
    path: String,
    is_builtin: bool,
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct StyleTemplateContent {
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
    let parsed = reqwest::Url::parse(trimmed).map_err(|e| format!("模板仓库 URL 无效: {}", e))?;
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

fn read_template_repositories(
    app: &tauri::AppHandle,
) -> Result<Vec<EpubTemplateRepository>, String> {
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
    let bytes =
        serde_json::to_vec_pretty(repos).map_err(|e| format!("序列化模板仓库配置失败: {}", e))?;
    fs::write(path, bytes).map_err(|e| format!("保存模板仓库配置失败: {}", e))
}

#[tauri::command]
pub fn list_epub_template_repositories(
    app: tauri::AppHandle,
) -> Result<Vec<EpubTemplateRepository>, String> {
    read_template_repositories(&app)
}

#[tauri::command]
pub fn add_epub_template_repository(
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
pub async fn sync_epub_template_repository(
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
pub async fn install_remote_epub_template(
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
pub fn list_style_templates(app: tauri::AppHandle) -> Result<Vec<StyleTemplateInfo>, String> {
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
pub fn read_style_template(
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
        id: if is_builtin {
            "builtin".to_string()
        } else {
            id
        },
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
pub fn import_style_template(
    app: tauri::AppHandle,
    path: String,
) -> Result<StyleTemplateInfo, String> {
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
pub fn save_style_template(
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
pub fn restore_builtin_style_template(app: tauri::AppHandle) -> Result<(), String> {
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

    normalized.contains("TEpub template schema: 1") && !normalized.contains(".te-volume-subtitle")
}
