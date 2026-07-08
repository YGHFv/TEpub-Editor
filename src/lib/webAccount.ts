export interface WebAccountSession {
  username: string;
  token: string;
  localOnly?: boolean;
}

interface AuthResponse {
  username: string;
  token: string;
  settings?: Record<string, any>;
}

const LEGACY_SETTINGS_KEY = "app-settings";
const GUEST_SETTINGS_KEY = "tepub-web-guest-settings";
const AUTH_KEY = "tepub-web-auth";
const LOCAL_USERS_KEY = "tepub-web-local-users";

function browserReady() {
  return typeof window !== "undefined" && typeof localStorage !== "undefined";
}

export function isTauriRuntime() {
  if (!browserReady()) return false;
  const global = window as any;
  return Boolean(global.__TAURI__ || global.__TAURI_INTERNALS__) || window.location.protocol === "tauri:";
}

export function usesWebScopedSettings() {
  return browserReady() && !isTauriRuntime();
}

export function webApiBaseUrl() {
  const env = import.meta.env.PUBLIC_TEPUB_API_BASE;
  return String(env || "/api").replace(/\/+$/, "");
}

function accountSettingsKey(username: string) {
  return `tepub-web-account-settings:${username.toLowerCase()}`;
}

function readJson<T>(store: Storage, key: string, fallback: T): T {
  try {
    const raw = store.getItem(key);
    return raw ? JSON.parse(raw) as T : fallback;
  } catch {
    return fallback;
  }
}

function writeJson(store: Storage, key: string, value: unknown) {
  store.setItem(key, JSON.stringify(value));
}

export function getWebAccountSession(): WebAccountSession | null {
  if (!browserReady()) return null;
  return readJson<WebAccountSession | null>(localStorage, AUTH_KEY, null);
}

function setWebAccountSession(session: WebAccountSession | null) {
  if (!browserReady()) return;
  if (session) writeJson(localStorage, AUTH_KEY, session);
  else localStorage.removeItem(AUTH_KEY);
}

export function readScopedSettingsRaw(): Record<string, any> {
  if (!browserReady()) return {};
  if (!usesWebScopedSettings()) return readJson(localStorage, LEGACY_SETTINGS_KEY, {});

  const session = getWebAccountSession();
  if (session?.username) {
    return readJson(localStorage, accountSettingsKey(session.username), {});
  }
  return readJson(sessionStorage, GUEST_SETTINGS_KEY, {});
}

export function writeScopedSettingsRaw(settings: Record<string, any>) {
  if (!browserReady()) return;
  if (!usesWebScopedSettings()) {
    writeJson(localStorage, LEGACY_SETTINGS_KEY, settings);
    return;
  }

  const session = getWebAccountSession();
  if (session?.username) {
    writeJson(localStorage, accountSettingsKey(session.username), settings);
    if (!session.localOnly) {
      void saveRemoteSettings(settings).catch((error) => console.warn("同步 Web 设置失败:", error));
    }
  } else {
    writeJson(sessionStorage, GUEST_SETTINGS_KEY, settings);
  }
}

export function notifySettingsChanged() {
  if (!browserReady()) return;
  window.dispatchEvent(new CustomEvent("tepub-settings-updated"));
}

async function requestJson<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${webApiBaseUrl()}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(init.headers || {}),
    },
  });
  const text = await response.text();
  if (!response.ok) {
    let message = text || `HTTP ${response.status}`;
    try {
      message = JSON.parse(text).error || message;
    } catch {
      // keep raw response text
    }
    const error = new Error(message) as Error & { status?: number };
    error.status = response.status;
    throw error;
  }
  return text ? JSON.parse(text) as T : undefined as T;
}

function localUsers() {
  return readJson<Record<string, { password: string; settings: Record<string, any> }>>(localStorage, LOCAL_USERS_KEY, {});
}

function saveLocalUsers(users: Record<string, { password: string; settings: Record<string, any> }>) {
  writeJson(localStorage, LOCAL_USERS_KEY, users);
}

async function localRegister(username: string, password: string): Promise<AuthResponse> {
  const key = username.toLowerCase();
  const users = localUsers();
  if (users[key]) throw new Error("账号已存在。");
  users[key] = { password, settings: {} };
  saveLocalUsers(users);
  return { username, token: `local-${key}`, settings: {} };
}

async function localLogin(username: string, password: string): Promise<AuthResponse> {
  const key = username.toLowerCase();
  const users = localUsers();
  if (!users[key] || users[key].password !== password) throw new Error("账号或密码不正确。");
  return { username, token: `local-${key}`, settings: users[key].settings || {} };
}

export async function registerWebAccount(username: string, password: string) {
  try {
    const result = await requestJson<AuthResponse>("/auth/register", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
    applyAuthResult(result, false);
    return result;
  } catch (error) {
    if ((error as Error & { status?: number }).status) throw error;
    const result = await localRegister(username, password);
    applyAuthResult(result, true);
    return result;
  }
}

export async function loginWebAccount(username: string, password: string) {
  try {
    const result = await requestJson<AuthResponse>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ username, password }),
    });
    applyAuthResult(result, false);
    return result;
  } catch (error) {
    if ((error as Error & { status?: number }).status) throw error;
    const result = await localLogin(username, password);
    applyAuthResult(result, true);
    return result;
  }
}

function applyAuthResult(result: AuthResponse, localOnly: boolean) {
  setWebAccountSession({ username: result.username, token: result.token, localOnly });
  writeJson(localStorage, accountSettingsKey(result.username), result.settings || {});
  notifySettingsChanged();
}

export function logoutWebAccount() {
  setWebAccountSession(null);
  notifySettingsChanged();
}

export async function syncRemoteSettings() {
  const session = getWebAccountSession();
  if (!session || session.localOnly) return null;
  const result = await requestJson<{ settings: Record<string, any> }>("/settings", {
    headers: { authorization: `Bearer ${session.token}` },
  });
  writeJson(localStorage, accountSettingsKey(session.username), result.settings || {});
  notifySettingsChanged();
  return result.settings || {};
}

export async function saveRemoteSettings(settings: Record<string, any>) {
  const session = getWebAccountSession();
  if (!session || session.localOnly) return;
  await requestJson("/settings", {
    method: "PUT",
    headers: { authorization: `Bearer ${session.token}` },
    body: JSON.stringify({ settings }),
  });
}
