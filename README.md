# TEpub Editor 📚

![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)
![Version](https://img.shields.io/badge/Version-0.1.0-orange)

**TEpub Editor** 是一款基于 **Tauri + Svelte 5 + Rust** 构建的高性能小说编辑器。
专为网文作者和电子书制作者设计，它完美解决了长篇 TXT 小说的分章节管理、内容质量检查以及一键生成标准 EPUB 电子书的需求。

---

## ✨ 核心功能

### 📖 智能目录与阅读
- **自动分章**：内置强大的正则引擎，自动识别“卷”与“章”结构。
- **极速响应**：基于 CodeMirror 6，轻松处理百万字超长文本。
- **实时同步**：点击目录跳转正文，滚动正文自动定位目录。

### 🛡️ 创作安全
- **自动保存**：编辑过程中实时保存，无需担心断电丢失。
- **历史快照**：每次保存自动生成快照，支持一键回溯历史版本。

### 🔍 质量控制 (Quality Check)
- **断序检测**：自动发现章节序号跳跃（如第10章后接第12章）。
- **异常扫描**：一键找出空标题章节、字数超标章节。
- **正则查找**：支持高级正则替换，轻松批量修文。

### 📤 EPUB 导出
- **标准格式**：生成符合 EPUB 3.0 标准的电子书。
- **元数据**：支持自定义封面、作者、简介，自动生成 UUID。

---

## 📥 下载安装

请前往 [Releases 页面](https://github.com/YGHFv/TEpub-Editor/releases) 下载最新版本：

- **Windows**: 下载 `.msi` 安装包或 `.exe` 绿色版。
- **macOS**: 下载 `.dmg` 镜像（注意：首次运行需右键打开以绕过安全警告）。
- **Linux**: 提供 `.deb` 和 `.AppImage`。

---

## 🛠️ 本地开发指南

如果您想参与开发或自己编译：

### 1. 环境准备
- **Node.js**: v18+
- **Rust**: 最新 Stable 版本
- **pnpm**: `npm install -g pnpm`

### 2. 启动开发环境
```bash
# 安装依赖
pnpm install

# 启动开发预览 (支持热更新)
pnpm tauri dev
```

### 3. 打包发布
```bash
# 自动构建当前系统的安装包
pnpm tauri build
```

---

## 🚀 路线图 (Roadmap)

- [x] 基础编辑与目录解析
- [x] 历史版本回溯
- [x] 内容质量检查面板
- [x] EPUB 导出
- [x] GitHub Actions 自动构建
- [ ] **主题系统** (暗黑模式/羊皮纸)
- [ ] **排版自动化** (一键缩进/去空行)
- [ ] **插件系统**

