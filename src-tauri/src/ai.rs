use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::fs_utils::{ensure_dir, sanitize_filename_part};
use crate::library::AiProofingConfig;
use crate::library::library_proof_logs_dir;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProofLogInfo {
    file_name: String,
    path: String,
    timestamp: u64,
    size: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiProofingRequest {
    config: AiProofingConfig,
    system_prompt: String,
    user_prompt: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiProofingResponse {
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
pub async fn run_ai_proofing(request: AiProofingRequest) -> Result<AiProofingResponse, String> {
    let base_url = request
        .config
        .base_url
        .trim()
        .trim_end_matches('/')
        .to_string();
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

    let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
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
pub async fn save_ai_proofing_log(
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
        if safe.is_empty() {
            "未命名".to_string()
        } else {
            safe
        }
    };
    let model_name = {
        let safe = sanitize_filename_part(model.trim());
        if safe.is_empty() {
            "unknown-model".to_string()
        } else {
            safe
        }
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
pub async fn list_ai_proofing_logs(app: tauri::AppHandle) -> Result<Vec<ProofLogInfo>, String> {
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
pub async fn read_ai_proofing_log(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("读取校对日志失败: {}", e))
}

// ============================================================
// ===== Library - Phase 2: add / remove =====
// ============================================================

