use chardetng::EncodingDetector;
use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;
use std::sync::Mutex;

use crate::fs_utils::{app_data_root, is_portable};
use crate::library::library_pointer_path;

// --- 换行符规范化 ---
// 将所有换行符（包括 Mac 旧时代的 \r 以及影响底层布局框架的特殊 Unicode 行分割符 U+2028）
// 统一为正统的 \n，确保后端行号计算与 CodeMirror 编辑器的严格分行计算完全一致。
fn normalize_line_endings(s: String) -> String {
    s.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{2028}', "\n")
        .replace('\u{2029}', "\n")
}

#[tauri::command]
pub fn exit_app() {
    process::exit(0);
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
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
pub async fn save_text_file(path: String, content: String) -> Result<(), String> {
    let mut file = fs::File::create(&path).map_err(|e| format!("无法创建: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn read_binary_file(path: String) -> Result<Vec<u8>, String> {
    fs::read(&path).map_err(|e| format!("读取失败: {}", e))
}

#[tauri::command]
pub fn get_launch_args() -> Option<String> {
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
pub(crate) struct LaunchInfo {
    file_path: Option<String>,
    file_paths: Vec<String>,
    action: Option<String>,
}

impl LaunchInfo {
    fn from_args(args: &[String], cwd: &std::path::Path) -> Self {
        let mut files = Vec::new();
        let mut action = None;
        for arg in args.iter().skip(1) {
            if let Some(value) = arg.strip_prefix("--action=") {
                action = Some(value.to_string());
            } else if !arg.starts_with("--") && !arg.is_empty() {
                let path = PathBuf::from(arg);
                let path = if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                };
                if matches!(
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(str::to_ascii_lowercase)
                        .as_deref(),
                    Some("epub" | "txt")
                ) {
                    files.push(path.to_string_lossy().into_owned());
                }
            }
        }
        Self {
            file_path: files.first().cloned(),
            file_paths: files,
            action,
        }
    }
}

// Keep requests until the main webview has installed its listener, including cold starts.
pub(crate) struct LaunchRequests(Mutex<Vec<LaunchInfo>>);

impl Default for LaunchRequests {
    fn default() -> Self {
        let launch = get_launch_info();
        Self(Mutex::new(if launch.file_paths.is_empty() {
            Vec::new()
        } else {
            vec![launch]
        }))
    }
}

impl LaunchRequests {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    pub(crate) fn enqueue(&self, args: &[String], cwd: &str) -> bool {
        let launch = LaunchInfo::from_args(args, std::path::Path::new(cwd));
        if launch.file_paths.is_empty() {
            return false;
        }
        self.0.lock().unwrap().push(launch);
        true
    }

    fn take(&self) -> Vec<LaunchInfo> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

#[tauri::command]
pub(crate) fn take_launch_requests(requests: tauri::State<'_, LaunchRequests>) -> Vec<LaunchInfo> {
    requests.take()
}

#[tauri::command]
pub fn get_launch_info() -> LaunchInfo {
    let args: Vec<String> = std::env::args().collect();
    LaunchInfo::from_args(&args, &std::env::current_dir().unwrap_or_default())
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn launch_paths_use_the_senders_working_directory() {
        let cwd = tempfile::tempdir().unwrap();
        let launch = LaunchInfo::from_args(
            &args(&[
                "app",
                "--action=reader",
                "first book.EPUB",
                "notes.txt",
                "--unknown",
                "image.png",
            ]),
            cwd.path(),
        );
        assert_eq!(launch.action.as_deref(), Some("reader"));
        assert_eq!(
            launch.file_paths,
            vec![
                cwd.path()
                    .join("first book.EPUB")
                    .to_string_lossy()
                    .into_owned(),
                cwd.path().join("notes.txt").to_string_lossy().into_owned(),
            ]
        );
        assert_eq!(launch.file_path.as_ref(), launch.file_paths.first());
    }

    #[test]
    fn absolute_launch_paths_are_not_rebased() {
        let cwd = tempfile::tempdir().unwrap();
        let path = cwd.path().join("book.epub").to_string_lossy().into_owned();
        let launch = LaunchInfo::from_args(&args(&["app", &path]), std::path::Path::new("other"));
        assert_eq!(launch.file_paths, vec![path]);
    }

    #[test]
    fn queued_launches_are_drained_once_and_repeated_requests_are_preserved() {
        let requests = LaunchRequests(Mutex::new(Vec::new()));
        let cwd = tempfile::tempdir().unwrap();
        let cwd = cwd.path().to_string_lossy().into_owned();
        let argv = args(&["app", "one.epub", "two.epub"]);
        assert!(requests.enqueue(&argv, &cwd));
        assert!(requests.enqueue(&argv, &cwd));
        let launches = requests.take();
        assert_eq!(launches.len(), 2);
        assert_eq!(launches[0].file_paths.len(), 2);
        assert_eq!(launches[0].file_paths, launches[1].file_paths);
        assert!(requests.take().is_empty());
        assert!(!requests.enqueue(&args(&["app"]), &cwd));
        assert!(!requests.enqueue(&args(&["app", "unsupported.png"]), &cwd));
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
fn install_verb(ext: &str, verb: &str, display: &str, action_flag: &str) -> Result<(), String> {
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
    let verb_key_old = format!(r"HKCU\Software\Classes\{}\shell\{}", legacy_progid, verb);
    let _ = run_reg_command(&["DELETE", &verb_key_old, "/f"]);
    Ok(())
}

#[tauri::command]
pub async fn set_file_assoc(verb: String, enabled: bool) -> Result<(), String> {
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
pub async fn reveal_in_explorer(path: String) -> Result<(), String> {
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

// 启动时给前端的"应用模式 + 是否首次启动"信息
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppModeInfo {
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
pub fn get_app_mode_info(app: tauri::AppHandle) -> Result<AppModeInfo, String> {
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
