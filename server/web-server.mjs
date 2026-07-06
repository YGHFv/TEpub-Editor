import { createServer } from "node:http";
import { createReadStream, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { extname, join, normalize, resolve } from "node:path";
import { randomBytes, scryptSync, timingSafeEqual } from "node:crypto";

const port = Number(process.env.PORT || 5233);
const root = resolve(process.env.TEPUB_WEB_ROOT || "build");
const dataDir = resolve(process.env.TEPUB_DATA_DIR || "data");
const usersFile = join(dataDir, "users.json");

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

async function handleApi(req, res, url) {
  try {
    const db = loadDb();

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

