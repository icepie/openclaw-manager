# 🦞 OpenClaw Manager

跨平台 AI 助手管理工具，基于 **Tauri 2.0 + React + TypeScript + Rust** 构建。

支持**纯离线安装**，无需 Node.js / npm 等系统依赖，不破坏现有环境。

![Platform](https://img.shields.io/badge/platform-macOS%20|%20Windows%20|%20Linux-blue)
![Tauri](https://img.shields.io/badge/Tauri-2.0-orange)
![Rust](https://img.shields.io/badge/Rust-1.70+-red)

---

## ✨ 核心特性

### 📦 纯离线安装，零系统依赖

- 一键下载离线包（含 Node.js + OpenClaw CLI），解压即用
- **不依赖**系统已安装的 Node.js、npm、git 等工具
- **不污染**系统 PATH，不修改任何全局配置
- 所有文件安装在 `~/.openclaw/`，卸载只需删除该目录
- 支持通过 `OPENCLAW_HOME` 环境变量自定义安装目录

### 🖥️ 可视化管理

- 实时服务状态监控（端口、PID、内存、运行时间）
- 一键启动 / 停止 / 重启 OpenClaw Gateway
- 实时日志查看，自动滚动刷新
- 快捷打开继承离线环境变量的终端

### 🤖 AI 配置

- 支持 14+ AI 提供商（Anthropic、OpenAI、DeepSeek、Moonshot、Gemini 等）
- 自定义 API 端点，兼容 OpenAI 格式第三方服务
- 一键设置主模型，快速切换

### 📱 消息渠道

- Telegram、Discord、Slack、飞书、WhatsApp、iMessage、微信、钉钉
- 可视化配置，无需手动编辑配置文件

### 🧪 系统诊断

- 自动检测运行环境（Node.js、OpenClaw 版本、配置目录）
- AI 连接测试、渠道连通性测试
- 跨平台兼容（macOS / Windows / Linux）

---

## 🚀 快速开始

### 下载安装

前往 [Releases](https://github.com/icepie/openclaw-manager/releases) 下载对应平台的安装包。

| 平台 | 格式 |
|------|------|
| macOS (Intel + Apple Silicon) | `.dmg` |
| Windows | `.msi` / `.exe` |
| Linux | `.deb` / `.AppImage` |

### 首次使用

1. 打开 OpenClaw Manager
2. 进入**设置**页面，点击**离线安装**，自动下载并安装 OpenClaw 离线包
3. 安装完成后，在**概览**页面点击**启动**即可运行 Gateway 服务

> 离线包下载地址走 ghproxy 镜像加速，国内网络可正常访问。

---

## 📁 安装目录结构

所有文件均安装在 `~/.openclaw/`（可通过 `OPENCLAW_HOME` 自定义）：

```
~/.openclaw/
├── node/              # 内置 Node.js（不影响系统 Node）
├── node_modules/      # OpenClaw CLI 及依赖
├── node_modules/.bin/ # 可执行文件（openclaw 命令）
├── logs/              # Gateway 运行日志
├── openclaw.json      # 主配置文件
└── env                # 环境变量（API Key 等）
```

卸载：在设置页面点击**卸载 OpenClaw**，或直接删除 `~/.openclaw/` 目录。

---

## 🍎 macOS 常见问题

### "已损坏，无法打开"

```bash
xattr -cr /Applications/OpenClaw\ Manager.app
```

或在**系统偏好设置 > 隐私与安全性**中点击**仍要打开**。

---

## 🛠️ 开发构建

### 环境要求

- Node.js >= 18
- Rust >= 1.70
- pnpm（推荐）

### macOS

```bash
xcode-select --install
```

### Windows

- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### Linux (Ubuntu/Debian)

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### 运行

```bash
git clone https://github.com/icepie/openclaw-manager.git
cd openclaw-manager
npm install
npm run tauri:dev    # 开发模式
npm run tauri:build  # 构建发布版本
```

---

## 🛠️ 技术栈

| 层级 | 技术 |
|------|------|
| 前端 | React 18 + TypeScript + TailwindCSS |
| 状态 | Zustand |
| 后端 | Rust (Tauri 2.0) |
| 解压 | 纯 Rust（flate2 + tar + zip，无外部命令依赖） |

---

## 📄 许可证

MIT License

---

**Made with ❤️ by OpenClaw Team**
