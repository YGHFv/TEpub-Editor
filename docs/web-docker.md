# Web Docker 部署

Web 版本容器包含静态前端和轻量 Node API 服务。API 提供账号注册、登录、账号级设置保存，以及文字模型和生图请求的可选代理；账号数据默认写入容器卷 `/data/users.json`。

## 本地开发

```bash
pnpm dev:web
```

默认地址：

```text
http://127.0.0.1:5233/
```

`pnpm dev:web` 只启动前端开发服务。登录注册会优先请求 `/api`，如果本地没有 API 服务，会退回浏览器本地开发账号。

## 构建并运行 Web 服务

```bash
pnpm build:web
pnpm serve:web
```

默认地址：

```text
http://127.0.0.1:5233/
```

可通过环境变量调整：

```bash
PORT=5233 TEPUB_DATA_DIR=./data pnpm serve:web
```

## Docker 运行

```bash
docker compose up -d --build
```

默认访问：

```text
http://服务器IP:5233/
```

换端口：

```bash
TEPUB_WEB_PORT=8090 docker compose up -d --build
```

## 更新流程

```bash
git checkout main
git pull
docker compose up -d --build
docker image prune -f
```

## 数据说明

- 未登录：设置只保存在当前浏览器会话，关闭浏览器后会恢复默认。
- 已登录：设置按账号保存到服务器 `/data/users.json`，包括主题、API 配置、工具箱设置和目录正则。
- Web 书库图书、封面和 TXT 历史版本保存在当前浏览器的 IndexedDB 中，不会上传到服务端；清理站点数据会删除这些本地内容。
- 桌面端/Tauri 不使用这个 Node API，原有平台功能不受影响。

