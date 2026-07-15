import { platform } from "$lib/platform";
import type { AiProofingConfig } from "$lib/appSettings";

export type WebAiTextResponse = { content: string };

function completionUrl(baseUrl: string) {
  const base = baseUrl.trim().replace(/\/+$/, "");
  if (!base) throw new Error("请先填写 API 地址");
  return base.toLowerCase().endsWith("/chat/completions") ? base : `${base}/chat/completions`;
}

function extractContent(value: any) {
  const content = value?.choices?.[0]?.message?.content;
  if (typeof content === "string" && content.trim()) return content;
  if (Array.isArray(content)) {
    const text = content.map((item) => typeof item === "string" ? item : item?.text || item?.content || "").join("");
    if (text.trim()) return text;
  }
  throw new Error("文字模型响应中没有可用内容");
}

async function requestDirect(config: AiProofingConfig, systemPrompt: string, userPrompt: string) {
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), Math.max(30, config.responseTimeoutSec) * 1000);
  try {
    const response = await fetch(completionUrl(config.baseUrl), {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        authorization: `Bearer ${config.apiKey.trim()}`,
      },
      body: JSON.stringify({
        model: config.model.trim(),
        temperature: config.temperature,
        max_tokens: 8192,
        stream: false,
        messages: [
          { role: "system", content: systemPrompt },
          { role: "user", content: userPrompt },
        ],
      }),
      signal: controller.signal,
    });
    const text = await response.text();
    let value: any;
    try { value = JSON.parse(text); } catch { throw new Error(`文字模型未返回有效 JSON：${text.slice(0, 180)}`); }
    if (!response.ok) throw new Error(value?.error?.message || value?.error || value?.message || `文字模型返回 ${response.status}`);
    return { content: extractContent(value) } satisfies WebAiTextResponse;
  } finally {
    window.clearTimeout(timer);
  }
}

export async function runWebAiText(config: AiProofingConfig, systemPrompt: string, userPrompt: string) {
  if (!config.apiKey.trim() || !config.model.trim()) throw new Error("请先在工具箱设置中补全文字模型 API Key 和模型名");
  try {
    return await requestDirect(config, systemPrompt, userPrompt);
  } catch (directError) {
    try {
      return await platform.invoke<WebAiTextResponse>("run_ai_proofing", {
        request: { config, systemPrompt, userPrompt },
      });
    } catch (proxyError) {
      throw new Error(`浏览器直连失败：${directError instanceof Error ? directError.message : directError}；后端代理失败：${proxyError instanceof Error ? proxyError.message : proxyError}`);
    }
  }
}
