# 技术决策

## 2026-11-08：同时支持原生与 WebView 桌面

用户明确要求保留原生版，同时增加具有完整本地能力的 WebView 版。本次不切换 GUI 框架；`apibuddy-app` 的 NSH Rust UI 编译为原生或 WASM，原生直接调用共享 Host，WebView 通过消息通道调用同一个 Host。reqwest/Tokio、文件与环境操作留在本地，页面只呈现和编辑。`apibuddy-webview` 使用 NSH 自己的 WebView 控件，少量 JS 负责传递结构化消息；不引入 React 或 Tauri。

两版共用数据目录及文件格式，分别打包，main 保持原样。增加异步操作边界，防止迟到的保存覆盖新草稿、过期集合加载覆盖当前服务、请求开始确认前的轮询丢事件，以及关闭标签后遗留网络操作。

## 2026-11-07：先交付独立桌面应用，CLI 暂缓

按用户最新要求，先提供本机可直接运行的 macOS `.app`。CLI 与 Agent Markdown 保留为后续构想，桌面入口默认关闭，创建服务时不再展示命令，安装包不携带 CLI。相关源码通过默认关闭的 `cli-integration` feature 保留，未删除。默认构建只选择桌面；交付打包默认使用 release，避免把含调试符号的开发包当作最终体积。

## 2026-11-07：NSH 原生 Postman 核心工作台

按用户要求从 dev 切出 `feat/nsh-postman-core`，保留工作区已有未提交的 Rust 重写。桌面改用 Longbridge Native Shell（NSH）v0.4.0，固定提交 `e626a8dd8f8d8b277b9507cfd5ac74d0e67020f3`；纯 Rust 模式关闭平台默认 js feature，macOS 使用 AppKit。此决定覆盖下方 GPUI 选型。固定 Rust 1.99.1 满足 NSH 使用的标准库 API，用户另授权更新本机 Rust 工具链。

复用 apibuddy-core 的数据格式、存储、环境合并、请求执行及 CLI，不迁移用户库。主要范围为多标签、服务集合、参数/头/正文、Bearer 认证、环境编辑、响应查看、cURL 与会话历史；标签与历史暂不落盘，不宣称完整复刻 Postman。

## 2026-10-30：重写为 Rust + GPUI

用户明确要求将 ApiBuddy-dev 从 Electrobun/React/TypeScript 重写为 Rust + GPUI。沿用 `~/code/apix-gpui` 验证过的 GPUI Kit 0.7.1、reqwest、Tokio 组合；桌面无 WebView。共享逻辑放入 `apibuddy-core`，桌面与 CLI 分别为 `apibuddy-app` / `apibuddy-cli`。数据目录与 JSON 模板/服务格式保持兼容，避免迁移用户库。首版对齐核心闭环（服务、接口、发送、CLI/Agent），多标签、环境编辑 UI、cURL、JSON 树等后续在 Rust 栈上补齐。

## 2026-10-26：优先分配窗口给请求与响应

用户反馈无用区域过多、核心编辑空间不足。工作台改为固定工具栏与可调分区，移除大标题、装饰文案和卡片外边距；窗口不再整页滚动。低频操作和说明按需展开，发送、保存、环境切换始终可见。布局偏好只使用本机 WebView 存储，不混入接口或环境文件；不可用时退回默认布局，不阻止请求调试。保留原有功能与数据格式，不新增 UI 依赖。

## 2026-10-26：先提升个人连续调试与 Agent 复用

本轮围绕“cURL 导入 → 选择环境 → 多标签发送 → 响应定位 → CLI/AI 复用”实现完整路径。保留 Electrobun/Cottontail/React 技术栈，新增功能不引入运行依赖。

工作区与环境保存在普通 JSON 文件，沿用独立修订号、写锁、原子写入和大小限制；恢复允许保留还没输入完的变量名，不削弱发送和保存接口时的校验。响应仅留在运行中的标签内，不扩展为持久请求历史。环境值覆盖模板默认值，CLI 显式参数再覆盖环境；所选环境随 CLI 示例传递。

cURL 仅作为单条 POSIX 命令文本解析，不执行 shell、不读取引用文件，未支持的行为明确拒绝。JSON 响应按文本呈现，可展开和定位，不执行远端内容。用户希望超过 Postman 是后续迭代方向，本次交付以具体能力与验证证据衡量。

## 2026-10-26：main、dev 双 worktree

按用户要求，将本地原 dev 重命名为 dev-old、原 dev2 重命名为 dev，保留各自的提交历史。主仓库位于 ~/code/apibuddy/ApiBuddy-main，检出 main；开发 worktree 位于 ~/code/apibuddy/ApiBuddy-dev，检出 dev，沿用已有依赖。dev-old 仅保留为历史分支。

main 继续保留初始化版本，当前实现留在 dev；本次没有合并代码、创建提交或推送远端。日常开发在 dev，验证通过并经用户要求后再合并到 main。下方记录中的 dev2 和原仓库路径属于当时的历史状态。

## 2026-10-23：ApiBuddy / dev2，先交付可用的本地工具

用户确定把代码和产品思考整理到 ~/code/apibuddy，新建 dev2，要求快速可用。以目标仓库原有 Git 历史为基础，复制旧 ApiX-dev 的当前源文件，包含未提交的新建接口修复；不导入依赖目录、构建产物、用户模板或旧仓库 .git。下方历史提交和 stash 均属于旧仓库，保留原处。

产品、窗口和安装包统一为 ApiBuddy；包名与命令为 apibuddy，无中划线。服务 ID 由用户输入，调用仍是 apibuddy orders get-order --id 123。只生成一个共享 CLI，实时读取按服务隔离的模板，不为每个服务生成不同命令，不增加常驻 API/MCP 服务。

桌面处理请求构筑、保存和调试；CLI 处理执行和 JSON 输出；Agent 接入以可复制的 Markdown 为起点。服务命名接口加 request METHOD /path 覆盖已保存和未保存的路径；“全部接口”不意味着自动发现远端 API。当前先跑通这条路径，不增加复杂注册协议。

数据变量采用 APIBUDDY_DIR，兼容 APIABC_DIR、APIX_DIR，三者按此顺序优先。默认目录依次沿用已有 ~/.apibuddy、~/.apiabc、~/.apix，否则使用 ~/.apibuddy。不搬动或合并数据；旧 CLI 文件保留，需要重新生成新入口。

npm 名称查询仅作预检，404 或查询工具的 available 不能证明最终发布权限。此前 apibuddy@0.0.1 发布尝试未成功；用户转而要求非发布方式检测。本仓库保持 private: true，当前可用性不依赖 npm；不自动发布、提交或推送。

## 2026-10-22：回到 Electrobun，构建请求模板工作台

用户明确要求继续在 ApiX 的 dev 分支开发，并指定 Electrobun、请求模板构筑/存储，以及通过环境变量自定义存储路径。这覆盖此前 Dioxus 试验方向。

恢复 Electrobun 2.0.1、Cottontail、React、TypeScript 和 Vite，保留原有 hutch.lock。沿用现有依赖，界面用普通 React 组件和 CSS；未引入 Mantine 或其他 UI 依赖。主进程目录改为 src/main，避免以 bun 目录名误导实际运行时。RPC schema 的 bun 键是 SDK 要求。

模板采用单文件 JSON 和 schemaVersion=1，不增加数据库服务。APIX_DIR 为数据根目录；路径只由主进程读取，不通过浏览器存储决定。变量使用 {{name}} 单次文本替换，不增加脚本执行能力。

## 保留的历史

- 原 Electrobun 原型：f8ab18a。
- Dioxus Native 基线：dcd4060，2026-10-13 按用户要求试验。
- Dioxus Desktop/WebView 未提交试验：迁移前完整保存为名为 `ApiX Dioxus WebView experiment before Electrobun templates 2026-10-22` 的 Git stash，原分支 experiment/dioxus-webview 保留。

不维护两套当前运行入口，不修改 ApiX-main，不重写 Git 历史。提交与推送由用户明确指示。

## 2026-10-22：按服务隔离并生成 Agent CLI（初版）

用户追加要求按服务隔离、按服务生成 CLI，并通过可复制 Markdown 接入任意 AI Agent。采用服务 UUID 目录、每服务独立 CLI、JSON 输出和 Markdown 使用约定，避免绑定单一 Agent 厂商。旧模板保留在默认服务，无批量迁移。

CLI 以 Node 单文件运行，直接复用 TypeScript 存储/请求模块。没有额外 API 服务、MCP 服务、全局 PATH 安装或 Agent 配置改写。服务隔离限定模板发现与调用范围，不冒充系统权限沙箱。

## 2026-10-22：缩短配置和命令（中间版本）

用户要求只用 APIX_DIR，且一个服务 CLI 要能调用全部接口，说明必须直观。统一环境变量，生成 APIX_DIR/bin/apix-服务标识；每个已保存接口直接对应命令，变量作为 --参数 值传入。另提供 request METHOD /path 访问未保存模板的接口，不假定已自动发现远端 API。接入面板直接列出接口命令与示例，Markdown 包含当前完整清单，--help 每次读取最新接口。

## 2026-10-22：统一 apix 入口（更名前）

用户明确选择 apix orders 的命令形式。只生成 APIX_DIR/bin/apix 和共享运行时 APIX_DIR/cli/apix.cjs。程序先从当前数据目录按服务标识解析服务，再执行接口；apix --help 列服务，apix orders --help 列接口。服务与接口新增后直接可用，无需再次生成；按服务复制的 Markdown 只是说明，不再意味着一个独立程序。模板目录仍按服务 UUID 隔离。

创建服务时将命令标识明确显示为「服务 ID」，由用户输入并实时预览调用方式；存储字段仍为 slug，内部 UUID 自动生成。用户要求先在本地 dev 分支提交一版，未要求推送远端。

## 2026-10-22：新建接口提供明确的创建流程

用户反馈「新建接口」不好用。原按钮仅重置草稿，在空草稿上没有可见变化；改为名称、请求方法、路径/URL 的创建弹窗。确认后写入当前服务、清除搜索过滤并选中新接口，取消保留原编辑器，失败在弹窗中保留输入。复用现有保存 RPC，不发送 HTTP 请求；路径变量自动补齐到模板变量表。

## 2026-10-22：更名为 ApiABC（中间版本）

用户确定采用 ApiABC。产品、窗口和安装包使用 ApiABC；项目包名和统一命令使用 apiabc；应用标识使用 io.github.hezhangjian.apiabc；配置改为 APIABC_DIR，默认目录为 ~/.apiabc。按服务调用形式保持 apiabc orders get-order --id 123，界面、CLI 帮助和生成给 Agent 的 Markdown 一致。

兼容旧 APIX_DIR，以新变量优先；未显式配置且新默认目录不存在时，原地使用已有 ~/.apix。已有服务和模板无需迁移，重新生成 CLI 即可获得新命令。旧 CLI 不删除，不自动修改用户已复制到外部的 Markdown。仓库远端、本地检出目录与历史记录保留原名；npm 尚未发布，包仍标为 private。
