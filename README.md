# TEpub Editor

![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux%20%7C%20Android-blue)
![Version](https://img.shields.io/badge/Version-1.0.0-orange)
![Stack](https://img.shields.io/badge/Stack-Tauri%202%20%2B%20Svelte%205%20%2B%20Rust-00a7d0)

TEpub Editor 是一个围绕 TXT 与 EPUB 的桌面工作台，把书库管理、长文本编辑、EPUB 制作、EPUB 内部编辑、阅读和一批 EPUB 处理工具放在同一个应用里。它同时提供一个功能受限的浏览器 Web 版。

## 功能

应用以「工具箱」为入口，包含以下工具：

**常用入口**

- **书库**：管理导入的图书与元数据（仅桌面版）
- **TXT 制作 EPUB**：从 TXT 生成 EPUB，进入目录预览与制作流程
- **EPUB 样式库**：预览头图与标题样式
- **TXT 编辑器**：导入、校对、查找替换
- **EPUB 编辑器**：编辑 EPUB 内部文件
- **EPUB 阅读器**：阅读 EPUB

**EPUB 处理**（生成新文件，支持单个或批量）

- **字体加密 / 字体解密**
- **文件加密 / 文件解密**
- **EPUB 重构**：整理目录与引用
- **图片转换**：转换 EPUB 内的 WebP 图片
- **图片处理**：制作全屏封面与阅微横幅
- **EPUB 诊断**：检查 OPF、manifest 和内部引用

### 编辑与校对

- 基于 CodeMirror 6 的文本编辑器
- 查找、替换、正则搜索
- 简繁转换（基于 opencc-js）
- 可配置的目录识别正则（分卷 / 分章两级）
- 可选的 AI 校对与生图：在设置里配置文字模型或生图模型（Base URL / API Key / 模型名）

### 阅读器

- 分页与滚动两种模式
- 字号、行距、段距、页边距调整
- 主题、背景、字体设置
- 目录导航、书签、进度保存

## 技术栈

- **前端**：Svelte 5 + SvelteKit + Vite
- **桌面壳**：Tauri 2
- **后端**：Rust
- **编辑器**：CodeMirror 6
- **简繁转换**：opencc-js

## 下载与安装

前往 [Releases](https://github.com/YGHFv/TEpub-Editor/releases) 下载：

- **Windows**：`.msi` / `.exe`
- **macOS**：`.dmg`
- **Linux**：`.deb` / `.AppImage`
- **Android**：`.apk`

## Web 版

除桌面客户端外，项目还能构建成一个浏览器版本。Web 版由静态前端加一个轻量 Node 服务组成，服务端提供账号注册、登录和账号级设置保存，数据写入 `/data`。

> Web 版是桌面版的子集。受浏览器限制，书库、文件关联、本地路径处理等依赖系统能力的功能不可用；可用工具为 TXT 制作 EPUB、EPUB 样式库、TXT 编辑器、EPUB 编辑器 / 阅读器、图片处理。

### 在线演示

在线演示地址：

- Cloudflare Pages：<https://tepub.ygvlive.com/>
- GitHub Pages：<https://yghfv.github.io/TEpub-Editor/>

> 演示站是纯静态部署，不含 Node 服务，因此账号注册 / 登录、设置同步等依赖 `/api` 的功能不可用。需要完整功能请用下面的 Docker 或 Node 方式自托管。

### 用 Docker 运行

仓库带有 `docker-compose.yml`，在仓库目录下执行：

```bash
docker compose up -d
```

启动后访问 `http://<服务器IP>:5233`。用户数据保存在 `tepub-web-data` 卷中，容器重建不会丢失。

换端口：

```bash
TEPUB_WEB_PORT=8090 docker compose up -d
```

### 不用 Docker

需要 Node.js：

```bash
pnpm install
pnpm build:web    # 构建静态前端
pnpm serve:web    # 启动内置 Node 服务，默认 http://127.0.0.1:5233
```

## 本地开发

环境要求：Node.js 18+、Rust Stable、pnpm。

```bash
pnpm install
pnpm tauri dev      # 桌面开发
pnpm tauri build    # 构建桌面安装包
pnpm dev:web        # Web 版开发服务器
```

## 许可

本项目不声明任何开源许可，仅公示源代码。保留所有权利。
