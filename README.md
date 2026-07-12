# TEpub Editor

![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![Version](https://img.shields.io/badge/Version-0.6.2-orange)
![Stack](https://img.shields.io/badge/Stack-Tauri%20%2B%20Svelte%205%20%2B%20Rust-00a7d0)
![License](https://img.shields.io/badge/License-MIT-green)

**TEpub Editor** 是一套面向小说创作、TXT 整稿、EPUB 制作、EPUB 修复与 EPUB 深度编辑的桌面工作台。

它不是单一的“TXT 转 EPUB”工具，而是把 **书库管理、长文本编辑、章节校对、封面与元数据处理、样式模板、内置阅读器、EPUB 文件级编辑、移动端流程** 放进了同一个项目里。

## 项目定位

TEpub Editor 目前主要覆盖 5 条核心工作流：

1. **长篇 TXT / MD / HTML 稿件编辑与校对**
2. **从源文本制作标准 EPUB**
3. **以书库方式管理 TXT 与 EPUB**
4. **直接打开并编辑 EPUB 内部文件结构**
5. **阅读、检查、修复并导出 EPUB**

如果你的日常工作流里经常出现这些场景：

- 长篇 TXT 需要自动分章、重排目录、检查断序
- 做电子书时要反复改封面、作者、简介、标签、UUID
- 想把 EPUB 内部的 `html / css / xml / 图片 / 字体` 直接拆开修改
- 遇到伪加密、资源路径异常、字体缺失的 EPUB 需要修复
- 希望把编辑、校对、制作、阅读、书库管理放在一个应用里完成

那么这个项目就是为这类需求设计的。

## 功能全景

### 1. 书库管理

- 支持把 `TXT` 和 `EPUB` 导入同一套本地书库
- 支持复制入库、引用模式、自定义书库目录、便携模式
- 首次启动可配置书库位置
- 绿色版可通过 `portable.txt` 让数据跟随程序目录
- 书架支持九宫格、带封面列表、普通列表等多种视图
- 支持搜索、排序、标签筛选、封面预览
- 支持按命名模板批量重建书库文件名
- 支持编辑标题、作者、副标题、系列、制作方、简介、标签、UUID
- 支持刷新 EPUB 元数据、替换封面、在资源管理器中定位文件
- 支持文件关联开关：`EPUB -> 阅读`、`EPUB -> 编辑`、`TXT -> 制作 EPUB`

### 2. TXT 长文本编辑

- 基于 CodeMirror 6 的长文编辑器，适合大体量小说文本
- 自动扫描卷章结构，生成多级目录
- 目录与正文联动定位，支持快速跳转
- 支持查找、替换、正则搜索、目录联动搜索
- 自动保存与历史快照，支持查看和回退历史版本
- 支持字体、字号、显示样式等编辑器设置
- 内置封面搜索与下载辅助流程

### 3. 校对与文本处理

- 断序检查：识别章节编号跳跃
- 标题检查：发现异常标题和目录结构问题
- 字数检查：筛出过短或过长章节
- 目录重排：支持按卷/章规则重写标题编号
- 内置文本规则检查，支持预览后选择性应用
- 简繁转换：支持繁转简、简转繁，并提供差异预览
- AI 智能校对：支持多 Provider 配置、分范围校对、审批流与日志保存

### 4. EPUB 制作

- 从 `TXT / MD / HTML / XHTML` 制作 EPUB
- 支持自定义章节识别正则
- 可先预览目录，再导出 EPUB
- 支持写入书名、作者、封面、UUID 等元信息
- 输出标准 EPUB，适合后续阅读、归档或发布
- 内置样式模板系统，可控制封面页、卷页、章节页、分割图等结构
- 支持模板资源槽位注入
- 支持字体资源导入与嵌入
- 支持字体子集化，减少导出体积

### 5. EPUB 元数据与模板系统

- 独立元数据窗口，支持编辑 publisher、UUID、MD5 与自定义 metadata
- 支持管理模板所需的字体、图片和其它资源文件
- 支持本地样式模板导入、保存、恢复内置模板
- 支持 GitHub 模板仓库
- 可添加模板仓库、同步索引、远程安装模板

### 6. EPUB 深度编辑

- 可直接解包并浏览 EPUB 内部文件树
- 支持编辑 `html / xhtml / css / xml`
- 支持多标签页编辑与未保存状态提示
- 支持图片预览、字体预览、字体内部名读取
- 支持字体缺字分析，查看当前 EPUB 用字与缺失字形
- 支持当前文件或多文件范围的查找替换
- 支持新增文件、重命名文件、删除文件、保存回工作副本
- 支持将修改后的 EPUB 保存并导出到磁盘

### 7. 内置 EPUB 阅读器

- 内置阅读页，支持分页模式与滚动模式
- 支持单栏 / 双栏布局
- 支持字号、行距、段距、页边距调整
- 支持主题、背景图、字体、正文字重设置
- 支持目录导航、书签、进度保存、全屏阅读
- 针对嵌入字体、资源路径、章节背景图做了专门兼容处理

### 8. EPUB 修复与预处理

- 打开前自动预处理异常 EPUB
- 可处理伪加密、混淆命名、非法路径、资源链接错乱等问题
- 会生成可读工作副本，不直接破坏原文件
- 提供“解密 EPUB / 导出清理结果”的独立流程

### 9. 移动端流程

项目内还包含一套移动端页面，用于更轻量的 EPUB 工作流：

- 移动端制作 EPUB
- 移动端解密 / 修复 EPUB
- 移动端编辑 EPUB 元数据与封面
- 移动端浏览并编辑 EPUB 内部文件
- 移动端批量查找替换

## 适合谁用

- 网文作者、同人作者、长篇小说作者
- 习惯先写 TXT、后做 EPUB 的个人用户
- 电子书排版与模板爱好者
- 喜欢直接编辑 EPUB 内部结构的重度用户
- 经常需要修复问题 EPUB、重做封面与元数据的人

## 技术栈

- **前端**: Svelte 5 + SvelteKit + Vite
- **桌面壳**: Tauri v2
- **后端**: Rust
- **编辑器**: CodeMirror 6
- **其它**: OpenCC、Tauri Dialog / FS / Opener 插件

项目整体分工比较清晰：

- Svelte 负责界面、编辑器交互、预览与多窗口流程
- Rust 负责文件系统、EPUB 解析重写、书库持久化和高性能处理
- Tauri 负责桌面集成、文件关联和原生能力接入

## 下载与安装

请前往 [Releases](https://github.com/YGHFv/TEpub-Editor/releases) 下载最新版本。

- **Windows**: `.msi` 或 `.exe`
- **macOS**: `.dmg`
- **Linux**: `.deb` 或 `.AppImage`

## Web 版与 Docker 部署

除了桌面客户端，TEpub Editor 还提供一套 **Web 版**，可以自托管到服务器上，通过浏览器访问。Web 版由一个静态前端加一个轻量 Node 服务组成，服务端负责账号登录、设置同步和 AI 生图代理，用户数据持久化在 `/data` 目录。

### 方式一：docker run（最快上手）

镜像发布在 Docker Hub，可直接拉取运行：

```bash
docker run -d \
  --name tepub-editor-web \
  -p 5233:5233 \
  -v tepub-web-data:/data \
  --restart unless-stopped \
  yghf/tepub-editor-web:latest
```

启动后访问 `http://<服务器IP>:5233` 即可。用户账号和设置会保存在 `tepub-web-data` 这个命名卷里，容器重建也不会丢。

### 方式二：docker compose（推荐长期部署）

仓库根目录已经带了 `docker-compose.yml`。克隆仓库或只下载这个文件后，执行：

```bash
docker compose up -d
```

默认会从 Docker Hub 拉取 `yghf/tepub-editor-web:latest` 镜像并在 `5233` 端口启动，无需本地编译。可以通过环境变量调整：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `TEPUB_WEB_PORT` | `5233` | 对外映射的端口 |
| `PUBLIC_TEPUB_API_BASE` | `/api` | 前端请求 API 的基础路径 |

如果想改成从本地源码现场构建，把 `docker-compose.yml` 里的 `image:` 段换成 `build: { context: . }` 即可。

### 方式三：本地构建镜像

```bash
docker build -t tepub-editor-web .
docker run -d -p 5233:5233 -v tepub-web-data:/data tepub-editor-web
```

### 数据与升级

- **数据位置**：所有用户数据落在容器内 `/data`（`users.json`），务必挂载卷持久化。
- **健康检查**：`GET /healthz` 返回 `ok`，可用于探针。
- **升级**：拉取新镜像后重建容器即可，数据卷保持不变。

```bash
docker compose pull && docker compose up -d
# 或 docker run 方式：
docker pull yghf/tepub-editor-web:latest
docker rm -f tepub-editor-web && docker run -d ...（同上）
```

## 本地开发

### 环境要求

- Node.js 18+
- Rust Stable
- pnpm

### 启动开发环境

```bash
pnpm install
pnpm tauri dev
```

### 构建发布包

```bash
pnpm tauri build
```

### Web 版开发与构建

```bash
pnpm dev:web          # 启动 Web 开发服务器（127.0.0.1:5233）
pnpm build:web        # 构建静态前端到 build/
pnpm serve:web        # 用内置 Node 服务托管 build/（含 API）
```

## 持续集成与自动发布

仓库内置了几条 GitHub Actions 流水线：

- **Release**（`release.yml`）：推送 `v*` tag 时自动构建 Windows / macOS / Linux 安装包与 Android APK，并发布到 Releases。
- **Deploy Web To GitHub Pages**（`pages.yml`）：推送到 `main` 时把 Web 版部署到 GitHub Pages（纯静态，API 不可用）。
- **Publish Docker Image**（`docker-publish.yml`）：推送到 `main` 或 `v*` tag 时，自动构建多架构镜像并推送到 Docker Hub。

### 配置 Docker Hub 自动上传

要让 `docker-publish.yml` 能推送镜像，需要在 Docker Hub 和 GitHub 各做一步配置：

1. **在 Docker Hub 创建访问令牌（Access Token）**
   - 登录 [hub.docker.com](https://hub.docker.com) → 右上角头像 → **Account settings** → **Personal access tokens**。
   - 点击 **Generate new token**，权限选择 **Read & Write**，复制生成的令牌（只显示一次）。
   - 用令牌而不是登录密码更安全，也不会因改密码而失效。

2. **在 GitHub 仓库添加 Secrets**
   - 进入仓库 → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**。
   - 添加两个：
     - `DOCKERHUB_USERNAME`：你的 Docker Hub 用户名。
     - `DOCKERHUB_TOKEN`：上一步生成的访问令牌。

3. **触发发布**
   - 推送到 `main` 分支会打上 `latest` 标签。
   - 推送 `v1.2.3` 这样的 tag 会额外打上 `1.2.3`、`1.2` 语义化版本标签。
   - 也可以在 Actions 页面手动 **Run workflow**。

镜像会自动构建 `linux/amd64` 和 `linux/arm64` 两种架构，并使用 GitHub Actions 缓存加速后续构建。首次成功推送后，就能在 Docker Hub 的仓库页面看到镜像，之后每次推送代码都会自动更新。

> 提示：Docker Hub 仓库默认是公开的。如果想要私有仓库，在 Docker Hub 网站上把对应仓库改为 Private 即可，workflow 无需改动。

## 这个项目的特点

和常见“只做转换”的工具相比，TEpub Editor 更像一张完整的电子书工作台：

- 不只做导出，也做编辑、校对、阅读和修复
- 不只处理 TXT，也能直接修改 EPUB 内部结构
- 不只照顾普通用户，也覆盖模板、字体、资源槽位、元数据等重度需求
- 不只提供桌面主流程，也准备了移动端轻量入口

## 后续方向

- 继续增强模板生态与样式复用能力
- 继续优化大体量文本与大型 EPUB 的性能体验
- 继续补强移动端工作流
- 继续完善 AI 校对与元数据辅助能力
