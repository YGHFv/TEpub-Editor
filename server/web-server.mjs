import { createServer } from "node:http";
import { createReadStream, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { extname, join, normalize, resolve } from "node:path";
import { randomBytes, scryptSync, timingSafeEqual } from "node:crypto";

const port = Number(process.env.PORT || 5233);
const root = resolve(process.env.TEPUB_WEB_ROOT || "build");
const dataDir = resolve(process.env.TEPUB_DATA_DIR || "data");
const usersFile = join(dataDir, "users.json");
const maxAiImageResponseBytes = 64 * 1024 * 1024;
const maxAiImageInputBytes = 20 * 1024 * 1024;
const aiImageUserAgent = "TEpub-Editor/web-image-tools";

const contentTypes = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".webp": "image/webp",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
};

mkdirSync(dataDir, { recursive: true });

function loadDb() {
  try {
    return JSON.parse(readFileSync(usersFile, "utf8"));
  } catch {
    return { users: {} };
  }
}

function saveDb(db) {
  writeFileSync(usersFile, JSON.stringify(db, null, 2));
}

function json(res, status, body) {
  res.writeHead(status, { "content-type": "application/json; charset=utf-8" });
  res.end(JSON.stringify(body));
}

function hashPassword(password, salt = randomBytes(16).toString("hex")) {
  const hash = scryptSync(password, salt, 64).toString("hex");
  return `${salt}:${hash}`;
}

function verifyPassword(password, stored) {
  const [salt, hash] = String(stored || "").split(":");
  if (!salt || !hash) return false;
  const expected = Buffer.from(hash, "hex");
  const actual = Buffer.from(hashPassword(password, salt).split(":")[1], "hex");
  return expected.length === actual.length && timingSafeEqual(expected, actual);
}

async function readBody(req) {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const raw = Buffer.concat(chunks).toString("utf8");
  return raw ? JSON.parse(raw) : {};
}

function normalizeUsername(value) {
  const username = String(value || "").trim();
  if (!/^[A-Za-z0-9_\-.]{3,32}$/.test(username)) {
    throw new Error("账号需为 3-32 位字母、数字、下划线、点或短横线。");
  }
  return username;
}

function normalizePassword(value) {
  const password = String(value || "");
  if (password.length < 6 || password.length > 128) {
    throw new Error("密码长度需为 6-128 位。");
  }
  return password;
}

function publicUser(user) {
  return { username: user.username, settings: user.settings || {} };
}

function authUser(req, db) {
  const header = String(req.headers.authorization || "");
  const token = header.replace(/^Bearer\s+/i, "").trim();
  if (!token) return null;
  return Object.values(db.users).find((user) => user.sessions?.includes(token)) || null;
}

function normalizeAiBaseUrl(value) {
  return String(value || "").trim().replace(/\/+$/, "");
}

function removeSuffixIgnoreCase(value, suffix) {
  return value.toLowerCase().endsWith(suffix.toLowerCase()) ? value.slice(0, -suffix.length) : value;
}

function isVersionedAiBaseUrl(baseUrl) {
  const lower = baseUrl.toLowerCase();
  const match = lower.match(/\/api\/v(\d+)$/);
  return Boolean(match);
}

function aiImageGenerationUrl(baseUrl) {
  const base = normalizeAiBaseUrl(baseUrl);
  if (!base) throw new Error("Base URL 不能为空");
  const lower = base.toLowerCase();
  if (lower.endsWith("/images/generations")) return base;
  if (lower.endsWith("/chat/completions")) return `${removeSuffixIgnoreCase(base, "/chat/completions")}/images/generations`;
  if (lower.endsWith("/v1") || isVersionedAiBaseUrl(base)) return `${base}/images/generations`;
  return `${base}/v1/images/generations`;
}

function aiImageEditsUrl(baseUrl) {
  const base = normalizeAiBaseUrl(baseUrl);
  if (!base) throw new Error("Base URL 不能为空");
  const lower = base.toLowerCase();
  if (lower.endsWith("/images/edits")) return base;
  if (lower.endsWith("/images/generations")) return `${removeSuffixIgnoreCase(base, "/images/generations")}/images/edits`;
  if (lower.endsWith("/chat/completions")) return `${removeSuffixIgnoreCase(base, "/chat/completions")}/images/edits`;
  if (lower.endsWith("/v1") || isVersionedAiBaseUrl(base)) return `${base}/images/edits`;
  return `${base}/v1/images/edits`;
}

function imageMimeFromBytes(bytes) {
  if (bytes.length >= 12) {
    const riff = Buffer.from(bytes.slice(0, 4)).toString("ascii");
    const webp = Buffer.from(bytes.slice(8, 12)).toString("ascii");
    if (riff === "RIFF" && webp === "WEBP") return { extension: "webp", mime: "image/webp" };
  }
  if (bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) return { extension: "png", mime: "image/png" };
  if (bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) return { extension: "gif", mime: "image/gif" };
  if (bytes[0] === 0xff && bytes[1] === 0xd8) return { extension: "jpg", mime: "image/jpeg" };
  return { extension: "jpg", mime: "image/jpeg" };
}

function validateAiImageBytes(bytes, label) {
  if (!bytes?.length) throw new Error(`${label}为空`);
  if (bytes.length > maxAiImageResponseBytes) throw new Error(`${label}过大`);
  const { mime } = imageMimeFromBytes(bytes);
  if (!mime.startsWith("image/")) throw new Error(`无法识别${label}的图片内容`);
}

function decodeAiBase64Image(value) {
  const trimmed = String(value || "").trim();
  const encoded = trimmed.toLowerCase().startsWith("data:image/") ? trimmed.split(",").slice(1).join(",").trim() : trimmed;
  if (encoded.length < 80 || /\s/.test(encoded)) return null;
  try {
    const bytes = Buffer.from(encoded, "base64");
    validateAiImageBytes(bytes, "返回图片");
    return bytes;
  } catch {
    return null;
  }
}

function isRemoteAiUrl(value) {
  return /^https?:\/\//i.test(String(value || "").trim());
}

function aiJsonErrorPreview(text) {
  try {
    const parsed = JSON.parse(text);
    const error = parsed?.error?.message || parsed?.error;
    if (typeof error === "string") return error;
    if (error) return JSON.stringify(error).slice(0, 240);
  } catch {
    // fall through
  }
  return String(text || "").slice(0, 240);
}

function imageSizeCandidates(requested, target) {
  const fallback = target === "banner" ? "1536x768" : target === "standard" ? "1200x1600" : "1024x1792";
  const secondary = target === "banner" ? "1280x640" : target === "standard" ? "1024x1365" : "1400x2400";
  return [...new Set([String(requested || "").trim(), fallback, secondary, ""])];
}

function renderAiImagePrompt(template, titleValue, authorValue) {
  const title = String(titleValue || "").trim() || "当前书籍";
  const rawAuthor = String(authorValue || "").trim();
  let prompt = String(template || "").trim();
  if (!rawAuthor) {
    for (const marker of ["作者{author}的", "作者{{author}}的", "作者 {author} 的", "作者 {{author}} 的"]) {
      prompt = prompt.replaceAll(marker, "");
    }
  }
  const replaced = prompt
    .replaceAll("{title}", title)
    .replaceAll("{{title}}", title)
    .replaceAll("{{text}}", title)
    .replaceAll("{author}", rawAuthor)
    .replaceAll("{{author}}", rawAuthor);
  if (prompt.includes("{title}") || prompt.includes("{{title}}") || prompt.includes("{{text}}") || prompt.includes("{author}") || prompt.includes("{{author}}")) {
    return replaced;
  }
  return `${replaced}\n\n书籍信息：${rawAuthor ? `${title} / ${rawAuthor}` : title}`;
}

function buildAiImageRequestBodies(model, prompt, size, referenceDataUrl) {
  const baseBody = () => {
    const body = { model, prompt, n: 1 };
    if (String(size || "").trim()) body.size = String(size).trim();
    return body;
  };
  const bodies = [];
  if (referenceDataUrl) {
    for (const variant of ["image_array", "image_string", "image_urls_array", "image_array_no_format", "image_string_no_format", "image_urls_array_no_format"]) {
      const body = baseBody();
      if (variant.startsWith("image_array")) body.image = [referenceDataUrl];
      else if (variant.startsWith("image_string")) body.image = referenceDataUrl;
      else body.image_urls = [referenceDataUrl];
      if (!variant.endsWith("_no_format")) body.response_format = "b64_json";
      bodies.push(body);
    }
  }
  bodies.push({ ...baseBody(), response_format: "b64_json" });
  bodies.push(baseBody());
  const seen = new Set();
  return bodies.filter((body) => {
    const key = JSON.stringify(body);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

async function downloadAiImageUrl(url) {
  const response = await fetch(String(url || "").trim(), {
    headers: { accept: "image/*,*/*;q=0.8", "user-agent": aiImageUserAgent },
  });
  const bytes = Buffer.from(await response.arrayBuffer());
  if (!response.ok) throw new Error(`下载生成图片返回 ${response.status}`);
  validateAiImageBytes(bytes, "生成图片");
  return bytes;
}

async function extractAiImageBytes(value, keyHint = "") {
  if (typeof value === "string") {
    const decoded = decodeAiBase64Image(value);
    if (decoded) return decoded;
    const key = String(keyHint || "").toLowerCase();
    if ((key.includes("url") || key === "uri" || key === "href") && isRemoteAiUrl(value)) {
      try {
        return await downloadAiImageUrl(value);
      } catch {
        return null;
      }
    }
    return null;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      const bytes = await extractAiImageBytes(item, keyHint);
      if (bytes) return bytes;
    }
    return null;
  }
  if (value && typeof value === "object") {
    for (const key of ["b64_json", "base64", "image_base64", "data", "content", "url", "image_url", "uri", "href"]) {
      if (key in value) {
        const bytes = await extractAiImageBytes(value[key], key);
        if (bytes) return bytes;
      }
    }
    for (const [key, item] of Object.entries(value)) {
      const bytes = await extractAiImageBytes(item, key);
      if (bytes) return bytes;
    }
  }
  return null;
}

async function requestAiImageGeneration(request, body) {
  const response = await fetch(aiImageGenerationUrl(request.baseUrl), {
    method: "POST",
    headers: {
      accept: "application/json,image/*",
      "content-type": "application/json",
      authorization: `Bearer ${String(request.apiKey || "").trim()}`,
      "user-agent": aiImageUserAgent,
    },
    body: JSON.stringify(body),
  });
  const contentType = response.headers.get("content-type") || "";
  const bytes = Buffer.from(await response.arrayBuffer());
  if (bytes.length > maxAiImageResponseBytes) throw new Error("生图响应过大");
  if (!response.ok) throw new Error(`生图接口返回 ${response.status}: ${aiJsonErrorPreview(bytes.toString("utf8"))}`);
  if (contentType.startsWith("image/")) {
    validateAiImageBytes(bytes, "生成图片");
    return bytes;
  }
  let value;
  try {
    value = JSON.parse(bytes.toString("utf8"));
  } catch (error) {
    throw new Error(`生图接口未返回有效 JSON: ${error.message}; ${bytes.toString("utf8").slice(0, 180)}`);
  }
  const out = await extractAiImageBytes(value);
  if (!out) throw new Error("生图接口返回中没有图片数据");
  return out;
}

async function requestAiImageEdit(request, prompt, size, referenceBytes) {
  const { extension, mime } = imageMimeFromBytes(referenceBytes);
  const form = new FormData();
  form.set("model", String(request.model || "").trim());
  form.set("prompt", prompt);
  form.set("n", "1");
  form.set("response_format", "b64_json");
  if (String(size || "").trim()) form.set("size", String(size).trim());
  form.set("image", new Blob([referenceBytes], { type: mime }), `reference.${extension}`);
  const response = await fetch(aiImageEditsUrl(request.baseUrl), {
    method: "POST",
    headers: {
      accept: "application/json,image/*",
      authorization: `Bearer ${String(request.apiKey || "").trim()}`,
      "user-agent": aiImageUserAgent,
    },
    body: form,
  });
  const contentType = response.headers.get("content-type") || "";
  const bytes = Buffer.from(await response.arrayBuffer());
  if (bytes.length > maxAiImageResponseBytes) throw new Error("图片编辑响应过大");
  if (!response.ok) throw new Error(`图片编辑接口返回 ${response.status}: ${aiJsonErrorPreview(bytes.toString("utf8"))}`);
  if (contentType.startsWith("image/")) {
    validateAiImageBytes(bytes, "生成图片");
    return bytes;
  }
  let value;
  try {
    value = JSON.parse(bytes.toString("utf8"));
  } catch (error) {
    throw new Error(`图片编辑接口未返回有效 JSON: ${error.message}; ${bytes.toString("utf8").slice(0, 180)}`);
  }
  const out = await extractAiImageBytes(value);
  if (!out) throw new Error("图片编辑接口返回中没有图片数据");
  return out;
}

function sanitizeFilenamePart(value) {
  return String(value || "")
    .replace(/[\\/:*?"<>|]/g, "_")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 80);
}

async function generateAiImageCommand(payload) {
  const request = payload?.request || {};
  if (!String(request.baseUrl || "").trim() || !String(request.apiKey || "").trim() || !String(request.model || "").trim()) {
    throw new Error("请先填写 Base URL、API Key 和模型");
  }
  if (!String(request.prompt || "").trim()) throw new Error("提示词不能为空");
  const target = ["duokan", "cover"].includes(String(request.target || "").trim())
    ? "duokan"
    : String(request.target || "").trim() === "standard"
      ? "standard"
      : String(request.target || "").trim() === "banner"
        ? "banner"
        : "duokan";
  const referenceBytes = Array.isArray(request.referenceData) ? Buffer.from(request.referenceData) : null;
  if (referenceBytes?.length) {
    if (referenceBytes.length > maxAiImageInputBytes) throw new Error("参考封面图片过大");
    validateAiImageBytes(referenceBytes, "参考封面");
  }
  const prompt = renderAiImagePrompt(request.prompt, request.title, request.author);
  const referenceDataUrl = referenceBytes?.length ? `data:${imageMimeFromBytes(referenceBytes).mime};base64,${referenceBytes.toString("base64")}` : "";
  const sizes = imageSizeCandidates(request.size, target);
  const isGptImage = String(request.model || "").toLowerCase().includes("gpt-image");
  let lastError = null;

  if (isGptImage && referenceBytes?.length) {
    for (const size of sizes) {
      try {
        const out = await requestAiImageEdit(request, prompt, size, referenceBytes);
        return formatAiImageResult(out, request.title, target, prompt, size);
      } catch (error) {
        lastError = error;
      }
    }
  }

  for (const size of sizes) {
    for (const body of buildAiImageRequestBodies(request.model, prompt, size, referenceDataUrl)) {
      try {
        const out = await requestAiImageGeneration(request, body);
        return formatAiImageResult(out, request.title, target, prompt, size);
      } catch (error) {
        lastError = error;
      }
    }
  }

  throw lastError || new Error("未获取到生图结果");
}

function formatAiImageResult(bytes, title, target, prompt, size) {
  const { extension, mime } = imageMimeFromBytes(bytes);
  const stem = sanitizeFilenamePart(title) || target;
  return {
    bytes: Array.from(bytes),
    mime,
    extension,
    fileName: `${stem}-${target}.${extension}`,
    prompt,
    size,
  };
}

async function runAiProofingCommand(payload) {
  const request = payload?.request || {};
  const config = request.config || {};
  const base = String(config.baseUrl || "").trim().replace(/\/+$/, "");
  const apiKey = String(config.apiKey || "").trim();
  const model = String(config.model || "").trim();
  if (!base || !apiKey || !model) throw new Error("请先填写文字模型 API 地址、API Key 和模型名");
  const endpoint = base.toLowerCase().endsWith("/chat/completions") ? base : `${base}/chat/completions`;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), Math.max(30, Math.min(1800, Number(config.responseTimeoutSec) || 300)) * 1000);
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { accept: "application/json", "content-type": "application/json", authorization: `Bearer ${apiKey}` },
      body: JSON.stringify({
        model,
        temperature: Math.max(0, Math.min(1, Number(config.temperature) || 0.1)),
        max_tokens: 8192,
        stream: false,
        messages: [
          { role: "system", content: String(request.systemPrompt || "") },
          { role: "user", content: String(request.userPrompt || "") },
        ],
      }),
      signal: controller.signal,
    });
    const text = await response.text();
    let value;
    try { value = JSON.parse(text); } catch { throw new Error(`文字模型未返回有效 JSON：${text.slice(0, 180)}`); }
    if (!response.ok) throw new Error(value?.error?.message || value?.error || value?.message || `文字模型返回 ${response.status}`);
    const raw = value?.choices?.[0]?.message?.content;
    const content = Array.isArray(raw)
      ? raw.map((item) => typeof item === "string" ? item : item?.text || item?.content || "").join("")
      : String(raw || "");
    if (!content.trim()) throw new Error("文字模型响应中没有可用内容");
    return { content };
  } finally {
    clearTimeout(timer);
  }
}

async function handleCommand(req, res, command) {
  try {
    const body = await readBody(req);
    if (req.method === "POST" && command === "toolbox_generate_ai_image") {
      return json(res, 200, await generateAiImageCommand(body));
    }
    if (req.method === "POST" && command === "run_ai_proofing") {
      return json(res, 200, await runAiProofingCommand(body));
    }
    return json(res, 404, { error: "Not found" });
  } catch (error) {
    return json(res, 400, { error: String(error?.message || error) });
  }
}

async function handleApi(req, res, url) {
  try {
    const db = loadDb();

    if (url.pathname.startsWith("/api/commands/")) {
      return handleCommand(req, res, decodeURIComponent(url.pathname.slice("/api/commands/".length)));
    }

    if (req.method === "POST" && url.pathname === "/api/auth/register") {
      const body = await readBody(req);
      const username = normalizeUsername(body.username);
      const password = normalizePassword(body.password);
      const key = username.toLowerCase();
      if (db.users[key]) return json(res, 409, { error: "账号已存在。" });
      const token = randomBytes(32).toString("hex");
      db.users[key] = {
        username,
        passwordHash: hashPassword(password),
        sessions: [token],
        settings: {},
        createdAt: new Date().toISOString(),
      };
      saveDb(db);
      return json(res, 200, { token, ...publicUser(db.users[key]) });
    }

    if (req.method === "POST" && url.pathname === "/api/auth/login") {
      const body = await readBody(req);
      const username = normalizeUsername(body.username);
      const password = normalizePassword(body.password);
      const user = db.users[username.toLowerCase()];
      if (!user || !verifyPassword(password, user.passwordHash)) {
        return json(res, 401, { error: "账号或密码不正确。" });
      }
      const token = randomBytes(32).toString("hex");
      user.sessions = [...(user.sessions || []).slice(-4), token];
      saveDb(db);
      return json(res, 200, { token, ...publicUser(user) });
    }

    if (url.pathname === "/api/settings") {
      const user = authUser(req, db);
      if (!user) return json(res, 401, { error: "未登录。" });
      if (req.method === "GET") return json(res, 200, { settings: user.settings || {} });
      if (req.method === "PUT") {
        const body = await readBody(req);
        user.settings = body.settings && typeof body.settings === "object" ? body.settings : {};
        user.updatedAt = new Date().toISOString();
        saveDb(db);
        return json(res, 200, { settings: user.settings });
      }
    }

    return json(res, 404, { error: "Not found" });
  } catch (error) {
    return json(res, 400, { error: String(error?.message || error) });
  }
}

function serveStatic(req, res, url) {
  if (url.pathname === "/mobile/make" || url.pathname === "/mobile/make/") {
    const next = new URL("/toolbox/make-epub", `http://${req.headers.host || "localhost"}`);
    for (const [key, value] of url.searchParams.entries()) next.searchParams.set(key, value);
    next.searchParams.set("view", "desktop");
    res.writeHead(302, { location: `${next.pathname}${next.search}` });
    res.end();
    return;
  }
  if (url.pathname === "/mobile" || url.pathname.startsWith("/mobile/")) {
    res.writeHead(302, { location: "/" });
    res.end();
    return;
  }
  const requestPath = decodeURIComponent(url.pathname);
  const candidate = normalize(join(root, requestPath));
  const file = candidate.startsWith(root) && existsSync(candidate) && !candidate.endsWith("/") ? candidate : join(root, "index.html");
  const type = contentTypes[extname(file).toLowerCase()] || "application/octet-stream";
  res.writeHead(200, { "content-type": type });
  createReadStream(file).pipe(res);
}

createServer((req, res) => {
  const url = new URL(req.url || "/", `http://${req.headers.host || "localhost"}`);
  if (url.pathname === "/healthz") {
    res.writeHead(200, { "content-type": "text/plain; charset=utf-8" });
    res.end("ok\n");
    return;
  }
  if (url.pathname.startsWith("/api/")) {
    void handleApi(req, res, url);
    return;
  }
  serveStatic(req, res, url);
}).listen(port, "0.0.0.0", () => {
  console.log(`TEpub web server listening on ${port}`);
});
