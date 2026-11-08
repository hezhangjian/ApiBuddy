# ApiBuddy

用 Rust + **Native Shell（NSH）0.4.0** 构建的桌面 API 客户端，在桌面编辑、发送、查看响应并保存请求。

本分支为 `feat/nsh-postman-core`，从 `dev` 切出并保留原有未提交的 Rust 迁移。现同时提供原生桌面与 WebView 桌面两个入口，共用 NSH Rust UI、交互状态和本地 Rust 后端。原生版在 macOS 使用 AppKit；WebView 版把同一 UI 编译成 WASM，由 WKWebView 显示。两个应用均不使用 QuickJS 或 Node 运行应用代码。

## 运行

项目通过 `rust-toolchain.toml` 固定 Rust 1.99.1；首次构建需要下载工具链及 Cargo 依赖。

```sh
cargo run -p apibuddy-app
```

需要独立测试数据时：

```sh
APIBUDDY_DIR="$(mktemp -d -t apibuddy-demo)" cargo run -p apibuddy-app
```

启动本机回显服务，便于不依赖外部 API 验证：

```sh
python3 scripts/fixture-server.py
```

在桌面新建服务：名称「订单服务」、ID `orders`、基础地址 `http://127.0.0.1:42427`。新建请求后填写名称和 URL（例如 `{{base_url}}/echo`），编辑 Params / Headers / Body，再发送并保存。

## 两种桌面版本

| 入口 | 运行方式 | 网络与存储 |
| --- | --- | --- |
| 原生版 `apibuddy-app`（默认） | NSH 原生控件 | 本地 Rust |
| WebView 版 `apibuddy-webview` | 同一 NSH Rust UI → WASM → 系统 WebView | 同一份本地 Rust，通过消息桥调用 |

```sh
# 原生版（沿用原命令）
cargo run -p apibuddy-app

# WebView 版：首次准备固定版本工具、WASM UI 与 release 宿主
python3 scripts/build-webview.py
cargo run --release -p apibuddy-webview

# macOS 两个独立应用包
cargo build --release -p apibuddy-app
python3 scripts/package-macos.py
python3 scripts/package-macos.py release webview
```

输出为 `dist/ApiBuddy.app`、`dist/ApiBuddy WebView.app`。两版默认读写同一个 `~/.apibuddy`，也都接受 `APIBUDDY_DIR`；可以同时运行，外部修改后点击「刷新集合」，已打开草稿保留，保存沿用修订冲突检测。

WebView 版的页面资源随应用打包，离线也能启动。HTTP 请求、环境解析、文件存储和剪贴板仍在本机 Rust 内执行；浏览器不会向目标 API 发请求，因此不要求目标服务配置 CORS。页面资源由随机本机端口提供，不对外暴露后端 API。这不是独立浏览器版，直接访问页面会提示需要桌面宿主。

首次 Web 构建需要联网下载 WASM 目标和编译工具；macOS 需要可编译 WASM 的 LLVM（例如 Homebrew `llvm`）。`NSH_CLI`、`NSH_WASM_BINDGEN` 可指向已有匹配工具；默认安装在 `target/webview-tools`，不覆盖全局工具。`APIBUDDY_WEB_ASSETS` 可覆盖页面资源目录用于开发。详见 [双版本架构](docs/ARCHITECTURE.md)。

## 在 Windows 构建

使用 Windows 10 1809+（建议 Windows 11），安装 Git、[Rust MSVC 工具链](https://www.rust-lang.org/tools/install) 和 Visual Studio Build Tools 的「使用 C++ 的桌面开发」工作负载（含 Windows SDK）。本项目不启用 NSH 的 JavaScript/QuickJS 功能，不需要 Node。

在 PowerShell 中执行：

```powershell
git clone --branch feat/nsh-postman-core https://github.com/hezhangjian/ApiBuddy.git
cd ApiBuddy
cargo build --locked --release -p apibuddy-app
.\target\release\apibuddy-app.exe
```

`rust-toolchain.toml` 会选择 Rust 1.99.1，首次编译需要联网获取工具链和固定版本依赖。生成的桌面程序为 `target\release\apibuddy-app.exe`，无需构建 CLI，也不运行 macOS 打包脚本。默认数据目录为 `%USERPROFILE%\.apibuddy`；隔离测试可先设置 `$env:APIBUDDY_DIR = Join-Path $env:TEMP ('apibuddy-demo-' + [guid]::NewGuid())`。

若 `windows` crate 编译提示栈溢出，可设置 `$env:RUST_MIN_STACK = '268435456'` 后重试。Windows 的编译与实际交互仍待 Windows 机器验证；后端要求参考 [NSH Windows 文档](https://github.com/longbridge/nsh/blob/e626a8dd8f8d8b277b9507cfd5ac74d0e67020f3/docs/dev/windows.md)。

## 桌面能力

- 服务集合、名称/URL/方法搜索、请求保存与另存副本。
- 最多 20 个独立请求标签，切换保留草稿和响应；关闭未保存标签有保存提示。
- 七种 HTTP 方法；可启停的查询参数和请求头；原始 JSON、Text、URL-encoded Form 正文及 JSON 格式化。
- Bearer Token 表单；其他认证可通过请求头配置。模板变量与每服务环境（环境覆盖模板默认值）。
- 真实 HTTP 请求、流式文本、取消、超时、状态码、耗时、字节数；最多 8 个并发请求、8 MiB 响应。
- Pretty / Raw / 响应头，复制完整响应；请求与响应区域可拖动调整。
- 常用 POSIX cURL 文本导入和实际请求导出。解析不会执行 shell，也不会读取 cURL 引用的文件。
- 本次会话最近 50 条请求历史，点击恢复为可编辑草稿。

快捷键：⌘/Ctrl+Enter 发送或取消，⌘/Ctrl+S 保存，⌘/Ctrl+T 新建，⌘/Ctrl+W 关闭标签，⌘/Ctrl+K 搜索，⌘/Ctrl+B 收放侧栏，Esc 取消请求。

标签、未保存草稿、响应和历史目前只保留在会话中；重启后从集合重新打开已保存接口。不含团队协作、脚本断言、OAuth 向导、OpenAPI/Postman Collection 导入、multipart/文件上传、WebSocket、持久历史和秘密管理。实现的是 Postman 风格的 HTTP 调试核心工作流。

## 保留构想

CLI 与 AI Agent 接入暂缓：源码保留，桌面入口默认关闭，安装包不包含 CLI。以后再决定是否启用；[原有设计说明](docs/AGENT-CLI.md) 仅作参考。

## 数据与开发

数据目录为 `APIBUDDY_DIR`，默认 `~/.apibuddy`；兼容 `APIABC_DIR`、`APIX_DIR`。沿用服务、模板、环境 JSON 格式，读取现有数据而不迁移。模板和环境采用版本校验、写锁和原子写入。

| Crate | 职责 |
| --- | --- |
| `apibuddy-core` | 请求/服务/环境、存储、HTTP、cURL、CLI |
| `apibuddy-app` | 两版共用的 NSH Rust 工作台，以及原生入口 |
| `apibuddy-webview` | WebView 桌面宿主、消息桥与本地静态资源服务 |
| `apibuddy-cli` | 保留的 CLI 构想，不默认构建或打包 |

```sh
cargo check
cargo test -p apibuddy-core
cargo test -p apibuddy-app --lib
cargo build --release -p apibuddy-app
python3 scripts/package-macos.py
```

HTTP 测试监听本机随机端口；桌面、存储测试使用系统临时目录。构建产物在 `target/`，本地安装包在 `dist/ApiBuddy.app`。打包默认使用 release 的体积优化、Thin LTO 和符号剥离；仅调试时显式传入 `python3 scripts/package-macos.py debug`。

[设计与工程文档](docs/README.md)
