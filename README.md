# ApiBuddy

用 Rust + **Native Shell（NSH）0.4.0** 构建的桌面 API 客户端，在桌面编辑、发送、查看响应并保存请求。

当前开发分支为 `dev`，专注 NSH 原生桌面：macOS 使用 AppKit，不使用 WebView、QuickJS 或 Node 运行应用代码。完整双版本基线已存档在 [`archive/nsh-webview-20261108`](https://github.com/hezhangjian/ApiBuddy/tree/archive/nsh-webview-20261108)，当前开发树不再维护 WebView 入口。原 dev 备份 `dev-before-nsh-20261108` 仍仅保留本地。

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

打开应用后直接输入 `http://127.0.0.1:42427/echo` 并发送，无需创建服务。保存时命名并选择集合，也可在保存框里新建集合。左侧 Collections / History 切换集合与本地历史；点击 + 新建独立请求标签。

## macOS 应用包

```sh
cargo build --locked --release -p apibuddy-app
python3 scripts/package-macos.py
```

输出为 `dist/ApiBuddy.app`，默认读写 `~/.apibuddy`，接受 `APIBUDDY_DIR`。集合、环境、历史及原生草稿沿用现有格式；旧 WebView 工作区文件保留原位，不自动迁移或删除。

## 日常开发：NSH CLI 自动重编

原生版日常编辑优先使用官方 `nsh run`。项目的 `nsh.json` 已声明 Rust 根组件；CLI 会监听 UI crate 和本地共享后端，源码变更后自动编译并重启。Rust 模式是重启应用；草稿和标签自动保存在本机，响应正文不恢复。重启前确认底栏显示「草稿已保存在本机」。

安装固定版本 CLI 到项目工具目录，在仓库根目录运行：

```sh
cargo install --locked --git https://github.com/longbridge/nsh --rev e626a8dd8f8d8b277b9507cfd5ac74d0e67020f3 --root target/nsh-tools nsh-cli
export PATH="$PWD/target/nsh-tools/bin:$PATH"
export APIBUDDY_DIR="$HOME/.apibuddy-dev"
nsh run                 # 保存 Rust 代码后自动重编重启，Ctrl-C 停止
```

`nsh check` 检查应用 crate；`nsh package --target macos` 生成原生安装包。`nsh run` 不更新下面固定到 Dock 的独立开发包。

当前 NSH 0.4.0 的监听器只识别依赖清单中的显式 `path`，因此应用对 `apibuddy-core` 使用 `path = "../apibuddy-core"`；仍指向同一个 workspace crate。

## macOS 一键开发包与 Dock 更新

在仓库根目录运行：

```sh
./dev                 # 增量构建并重启原生开发版
./dev --release       # 使用 release 构建
./dev --data-dir /绝对路径
```

命令构建、打包、更新 `~/Applications/ApiBuddy Dev.app`，首次固定到 Dock，随后复用同一图标。默认使用 debug 增量构建；没有文件监听，修改后再次执行命令。开发版默认使用 `~/.apibuddy-dev`，独立 Bundle ID；数据目录随包保存，从 Dock 启动生效。临时打包文件放在系统临时目录。

**重启前确认底栏的草稿保存状态**：编辑后约 0.5 秒自动保存标签、草稿和环境选择；立即强制退出仍可能丢失最后一次改动。编译和打包完成后才退出旧开发版；只重启上述固定安装路径的应用，不强制杀进程。现有 `cargo run` 和独立打包命令仍可使用。

## 在 Windows 构建

使用 Windows 10 1809+（建议 Windows 11），安装 Git、[Rust MSVC 工具链](https://www.rust-lang.org/tools/install) 和 Visual Studio Build Tools 的「使用 C++ 的桌面开发」工作负载（含 Windows SDK）。本项目不启用 NSH 的 JavaScript/QuickJS 功能，不需要 Node。

当前已验证本机 macOS，Windows 尚未验证。Windows 可在 PowerShell 中克隆 `dev` 并构建：

```powershell
git clone --branch dev https://github.com/hezhangjian/ApiBuddy.git
cd ApiBuddy
cargo build --locked --release -p apibuddy-app
.\target\release\apibuddy-app.exe
```

`rust-toolchain.toml` 会选择 Rust 1.99.1，首次编译需要联网获取工具链和固定版本依赖。生成的桌面程序为 `target\release\apibuddy-app.exe`，无需构建 CLI，也不运行 macOS 打包脚本。默认数据目录为 `%USERPROFILE%\.apibuddy`；隔离测试可先设置 `$env:APIBUDDY_DIR = Join-Path $env:TEMP ('apibuddy-demo-' + [guid]::NewGuid())`。

若 `windows` crate 编译提示栈溢出，可设置 `$env:RUST_MIN_STACK = '268435456'` 后重试。Windows 的编译与实际交互仍待 Windows 机器验证；后端要求参考 [NSH Windows 文档](https://github.com/longbridge/nsh/blob/e626a8dd8f8d8b277b9507cfd5ac74d0e67020f3/docs/dev/windows.md)。

## 桌面能力

- 原生工作台采用冷白/石墨灰与靛蓝主题，随系统适配浅深色；集合分段导航、方法颜色、活动请求高亮、可收放侧栏，以及独立的空响应提示。
- 无前置服务步骤的临时请求；Collections / History、名称/URL/方法搜索、保存时选择或新建集合、重命名与另存副本。
- 最多 20 个独立请求标签，支持复制为独立草稿和键盘循环切换；关闭当前标签后激活相邻标签，切换保留草稿和响应；关闭未保存标签有保存提示。草稿自动落盘，重启恢复且不会自动重发。
- 七种 HTTP 方法；可启停的查询参数和请求头；JSON、Text、URL-encoded Form 字段及 JSON 格式化；Form 字段自动编码，支持启停。切换原始正文与 Form 保留各自草稿及字段状态，保存/重启后可继续编辑。
- 原生正文代码编辑器：行号、JSON 着色、缩进、折叠和自动换行；格式化、压缩可在当前编辑视图中撤销重做，错误会定位到对应行列。
- Bearer Token、Basic Auth、API Key（Header / Query）及手动 Headers 认证。模板变量与每服务环境（环境覆盖模板默认值）。
- 真实 HTTP 请求、流式文本、取消、超时、状态码、耗时、字节数；最多 8 个并发请求、8 MiB 响应。
- Pretty / Raw / 响应头；只读代码视图、行号、JSON 着色和自动换行开关；响应文本查找、区分大小写、命中循环导航；JSON Pointer 字段提取与复制。长响应预览上限 256 KiB，字段提取与复制仍使用完整响应；请求与响应区域可拖动调整。
- 常用 POSIX cURL 文本导入和实际请求导出，支持 JSON、URL 编码数据、GET 查询与 Basic 认证；导入前预览方法、URL、正文大小与认证类型。解析不会执行 shell，也不会读取 cURL 引用的文件。
- 本机最近 50 条请求历史（最多 8 MiB，含成功、失败、取消与超时），点击恢复为可编辑草稿；保存发送时的请求与环境选择。

快捷键：Ctrl+Tab / Ctrl+Shift+Tab 切换标签，⌘/Ctrl+1–8 选第 N 个标签、⌘/Ctrl+9 选末尾，⌘/Ctrl+Shift+D 复制请求；⌘/Ctrl+Enter 发送或取消，⌘/Ctrl+S 保存，⌘/Ctrl+T 新建，⌘/Ctrl+W 关闭标签，⌘/Ctrl+K 搜索，⌘/Ctrl+B 收放侧栏，⌘/Ctrl+F 查找响应，Enter 或 ⌘/Ctrl+G 下一处，Shift+⌘/Ctrl+G 上一处；Esc 优先关闭查找，否则取消请求。

响应正文仅保留在会话中；未保存草稿也会明文保存在本机。原生标签独立恢复，集合、环境和历史沿用现有数据。不含团队协作、脚本断言、完整 OAuth、OpenAPI / 外部集合导入、multipart/文件上传、WebSocket 和秘密管理。当前聚焦个人 HTTP 调试核心工作流。

## 保留构想

CLI 与 AI Agent 接入暂缓：源码保留，桌面入口默认关闭，安装包不包含 CLI。以后再决定是否启用；[原有设计说明](docs/AGENT-CLI.md) 仅作参考。

## 数据与开发

数据目录为 `APIBUDDY_DIR`，默认 `~/.apibuddy`；兼容 `APIABC_DIR`、`APIX_DIR`。沿用服务、模板、环境 JSON 格式，读取现有数据而不迁移。模板和环境采用版本校验、写锁和原子写入。

| Crate | 职责 |
| --- | --- |
| `apibuddy-core` | 请求/服务/环境、存储、HTTP、cURL、CLI |
| `apibuddy-app` | NSH 原生工作台、后台任务与桌面入口 |
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
