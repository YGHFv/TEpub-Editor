[简体中文](../README.md) | **繁體中文** | [English](README.en.md)

# TEpub Editor

![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android-blue)
![Version](https://img.shields.io/badge/Version-1.4.0-orange)
![Stack](https://img.shields.io/badge/Stack-Tauri%202%20%2B%20Svelte%205%20%2B%20Rust-00a7d0)
![Languages](https://img.shields.io/badge/Languages-%E7%AE%80%E4%BD%93%E4%B8%AD%E6%96%87%20%7C%20%E7%B9%81%E9%AB%94%E4%B8%AD%E6%96%87%20%7C%20English-green)

TEpub Editor 是一個圍繞 TXT 與 EPUB 的工作台，把書庫管理、長文本編輯、EPUB 製作、EPUB 內部編輯、閱讀和一批 EPUB 處理工具放在同一個應用裡，同時提供桌面客戶端和瀏覽器 Web 版。介面支援簡體中文、繁體中文與 English。

## 功能

應用以「工具箱」為入口，包含以下工具：

**常用入口**

- **書庫**：管理匯入的圖書、封面與後設資料
- **TXT 製作 EPUB**：從 TXT 生成 EPUB，進入目錄預覽與製作流程
- **EPUB 樣式庫**：預覽頭圖與標題樣式
- **TXT 編輯器**：匯入、校對、查詢替換
- **EPUB 編輯器**：編輯 EPUB 內部檔案
- **EPUB 閱讀器**：閱讀 EPUB

桌面版採用單一實例程式、多視窗模式：再次啟動會喚起已有視窗，雙擊或右鍵開啟 TXT / EPUB 時會把檔案交給已有程式，每個檔案仍可在獨立編輯器或閱讀器視窗中開啟，不會關閉或替換其他圖書視窗。

**EPUB 處理**（生成新檔案，支援單個或批次）

- **字型加密 / 字型解密**
- **檔案加密 / 檔案解密**
- **EPUB 重構**：整理目錄與引用
- **圖片轉換**：轉換 EPUB 內的 WebP 圖片
- **圖片處理**：製作全螢幕封面與閱微橫幅
- **EPUB 診斷**：檢查 OPF、manifest 和內部引用
- **EPUB 轉 TXT、EPUB 2.0 / 3.0 互轉**
- **整書簡繁轉換、廣告清理、拼音標註、批註與腳註轉換**
- **圖片壓縮、圖片隱形水印、PNG/JPEG 與 WebP 雙向轉換**
- **EPUB 合併、按章節拆分、字型子集化**
- **Send to Kindle**：開啟 Amazon 官方傳書頁面

上述新增轉換與整理工具使用共享瀏覽器核心，桌面安裝版和 Web 版執行相同處理邏輯。完整對照見 [`epub-toolkit-feature-audit.md`](epub-toolkit-feature-audit.md)。

### 編輯與校對

- 基於 CodeMirror 6 的文本編輯器
- 查詢、替換、正則搜尋
- 簡繁轉換（基於 opencc-js）
- 可配置的目錄識別正則（分卷 / 分章兩級）
- 可選的 AI 校對與生圖：在設定裡配置文字模型或生圖模型（Base URL / API Key / 模型名）

### 閱讀器

- 分頁與滾動兩種模式
- 字號、行距、段距、頁邊距調整
- 主題、背景、字型設定
- 目錄導航、書籤、進度儲存

## 多語言介面

- 支援簡體中文、繁體中文和 English，簡體中文為來源語言
- 首次啟動預設跟隨裝置或瀏覽器語言，也可在工具箱或手機端設定中手動切換
- 語言偏好會持久儲存，並在桌面多視窗、Web 版與行動端之間同步生效
- 繁體中文介面由 OpenCC 自動轉換，覆蓋所有主要頁面、工具、設定與執行時提示

## 技術棧

- **前端**：Svelte 5 + SvelteKit + Vite
- **桌面殼**：Tauri 2
- **後端**：Rust
- **編輯器**：CodeMirror 6
- **簡繁轉換**：opencc-js
- **介面國際化**：Svelte 5 執行時翻譯 + OpenCC

## 下載與安裝

前往 [Releases](https://github.com/YGHFv/TEpub-Editor/releases) 下載：

- **Windows**：`.msi` / `.exe`
- **macOS**：`.dmg`
- **Linux**：`.deb` / `.AppImage`
- **Android**：`.apk`

## Web 版

除桌面客戶端外，專案還能建置成瀏覽器版本。Web 版由靜態前端加一個可選的輕量 Node 服務組成；服務端提供帳號註冊、登入、帳號級設定儲存和 AI 請求代理，帳號資料寫入 `/data`。出於安全考量，AI 校對、AI 生圖和遠端圖片下載等服務端代理功能需要先登入帳號才能使用。

Web 工具箱已提供全部 14 個入口：書庫、TXT 製作 EPUB、EPUB 樣式庫、TXT 編輯器、EPUB 編輯器 / 閱讀器、字型加密 / 解密、檔案加密 / 解密、EPUB 重構、圖片轉換、圖片處理和 EPUB 診斷。Web 書庫使用 IndexedDB 儲存圖書與歷史版本，可請求瀏覽器持久儲存；TXT 和 EPUB 可從書庫直接進入閱讀、編輯並寫回。

> 檔案關聯、任意本機路徑覆蓋寫入、原生多視窗和系統檔案管理員等操作依賴桌面系統能力，瀏覽器中採用檔案選擇、下載和頁面導航等 Web 原生方式，不會請求不安全的本機路徑權限。

### 線上演示

線上演示地址：

- Cloudflare Pages：<https://tepub.ygvlive.com/>
- GitHub Pages：<https://yghfv.github.io/TEpub-Editor/>

> 演示站是純靜態部署，不含 Node 服務，因此帳號註冊 / 登入、設定同步等依賴 `/api` 的功能不可用。需要完整功能請用下面的 Docker 或 Node 方式自行託管。

### 用 Docker 執行

倉庫帶有 `docker-compose.yml`，在倉庫目錄下執行：

```bash
docker compose up -d
```

啟動後訪問 `http://<伺服器 IP>:5233`。帳號與同步設定儲存在 `tepub-web-data` 卷中，容器重建不會丟失；圖書檔案儲存在訪問裝置的瀏覽器 IndexedDB 中，不會上傳到伺服器。

換埠：

```bash
TEPUB_WEB_PORT=8090 docker compose up -d
```

### 不用 Docker

需要 Node.js：

```bash
pnpm install
pnpm build:web    # 建置靜態前端
pnpm serve:web    # 啟動內建 Node 服務，預設 http://127.0.0.1:5233
```

## 本地開發

環境要求：Node.js 18+、Rust Stable、pnpm。

```bash
pnpm install
pnpm tauri dev      # 桌面開發
pnpm tauri build    # 建置桌面安裝包
pnpm dev:web        # Web 版開發伺服器
```

## 授權

本專案不聲明任何開源授權，僅公示原始碼。保留所有權利。
