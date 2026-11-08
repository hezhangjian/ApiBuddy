# 开发与验证

## 2026-11-08：原生与 WebView 双桌面入口

原生版 `apibuddy-app` 与 WebView 版 `apibuddy-webview` 同时保留，共用 Rust UI/状态及 `apibuddy-core::host::Host`。Web 页面用项目固定 NSH revision 的工具打包，wasm-bindgen 固定为 0.2.130；WebView 使用 NSH 自带控件和消息通道，HTTP/存储/剪贴板由本地执行。

验证记录：

- `cargo check --locked`、`cargo fmt --all --check`、`git diff --check` 通过。共享 UI 的 `wasm32-unknown-unknown` 编译检查通过；原生 CLI 可选 feature 与保留 CLI 的编译检查通过。
- `cargo test --locked -p apibuddy-core -p apibuddy-app -p apibuddy-webview`：18 项 Rust 测试通过。`node --test web/host.test.mjs`：3 项桥接测试通过。覆盖共享文件/修订冲突、跨请求事件保留、关闭后释放并发槽、响应上限/UTF-8/取消/超时、资源路径与 Host 限制、消息匹配与超时。
- 两个 release 二进制构建通过；同一 Rust UI 的正式 WASM 打包成功，页面资源约 10.5 MB（含预压缩文件）。两个 macOS `.app` 分别生成，WebView 包内自带页面资源。
- 使用系统临时数据目录和本机随机端口回显服务，真实原生窗口发送 GET 得到 200 并保存。WebView 窗口读取该接口，发送得到 200；慢请求取消显示「已取消」，中文 SSE 三段全部显示。
- WebView 导入含中文 JSON、自定义请求头和查询参数的 cURL，POST 回显均正确；复制响应后粘贴验证本地剪贴板通道。WebView 保存修订 2 后，原生刷新可见；原生旧草稿保存被修订冲突拒绝。
- 最终两个 release 应用包重启后均读取到已保存接口，macOS WebView 显示正确的 ⌘ 快捷键提示，实际 ⌘Enter 已验证。界面截图检查通过。

未验证 Windows/Linux 的编译及实际交互；真实拼音 IME、屏幕阅读器和跨显示器行为仍需专项验证。测试没有访问日常数据。没有提交、推送或合并 main。


## 2026-11-07：Windows 构建交接

按用户要求，将当前 Rust/NSH 实现通过 `feat/nsh-postman-core` 功能分支交接到另一台电脑，保持 CLI 默认关闭。README 补充 Windows MSVC 前置条件、PowerShell 克隆与 release 构建命令、exe 路径和隔离数据目录设置。

推送前修复共享核心中无条件引用 Unix 文件 API 的编译问题，权限设置仅在 Unix 启用；原子写入在重命名前关闭文件。用户目录改由标准库跨平台解析，Windows 使用 USERPROFILE/系统用户配置目录。路径测试改用系统临时目录；Windows 界面使用 Ctrl 快捷键提示及 Consolas 等宽字体。

本机 macOS 的 `cargo check --locked`、核心 10 项测试、工作台 4 项测试及 release 构建通过。Windows 编译和原生交互尚未实测，需在目标电脑执行 README 中的步骤。此前 macOS 桌面交互验收记录见下文。

## 2026-11-07：独立桌面 release 包，CLI 暂缓

按用户最新要求，交付本机 macOS `.app`，CLI 保留为后续构想。桌面「CLI 与 AI」入口及创建服务时的命令预览已隐藏，相关接入代码通过默认关闭的 `cli-integration` feature 保留；默认构建只选择桌面，打包脚本只复制桌面程序并默认使用 release。

- 原 debug 包共 85,013,310 字节（85.01 MB），其中桌面 73,744,704 字节、CLI 11,267,896 字节。新的 `dist/ApiBuddy.app` 共 31,490,550 字节（31.49 MB / 30.03 MiB），减少约 63%。使用现有 `opt-level = "s"`、Thin LTO、单代码生成单元及符号剥离，未删除桌面请求功能。
- `cargo check`、核心 10 项测试、NSH 工作台 4 项测试、`cargo build --release -p apibuddy-app`、格式检查和 `git diff --check` 通过。
- 使用最终包内同一 release 二进制及临时数据目录进行原生交互验证：顶部无 CLI 入口，新建服务无命令预览；实际发送本机回显请求收到 200，查询参数及中文响应正确。
- 本轮交付仍为本地 macOS arm64 构建，未做 Developer ID 签名、公证或正式安装。以下 CLI 验证记录属于暂缓前的历史状态。

## 2026-11-07：NSH 原生 Postman 核心工作台

在 `feat/nsh-postman-core` 分支完成，本轮从 `dev` 切出并保留工作区已有的未提交 Rust 迁移。当前实现使用 Longbridge Native Shell v0.4.0，固定 Git revision；纯 Rust 模式由 AppKit 渲染。按用户授权，将本机 stable 工具链升级至 Rust 1.99.1，项目同时以 `rust-toolchain.toml` 固定该版本。

- `cargo check`、`cargo test -p apibuddy-core`、`cargo test -p apibuddy-app --lib`、`cargo build -p apibuddy-app -p apibuddy-cli`、格式检查及 `git diff --check` 通过。共 14 项测试：6 项核心单元测试、4 项存储/真实 HTTP 集成测试、4 项 NSH 工作台交互测试。
- 核心覆盖 cURL 转义与拒绝执行 shell/读取文件、环境合并与修订冲突、分块中文/emoji 解码、非 2xx 响应、取消、超时和 8 MiB 响应截断。工作台覆盖独立标签、保存重开、未保存关闭提示、对话框关闭后的状态存活、剪贴板导出、服务与环境，以及输入框获得焦点时的发送快捷键。
- macOS arm64 原生窗口使用系统临时目录隔离数据与本机 42427 回显服务。实际创建服务、导入中文 JSON cURL、保存并重启后打开接口、添加中文查询参数、选择环境、Bearer 变量替换、POST 发送、慢请求取消均通过；回显确认方法、请求头、正文和参数。
- 实际复制响应并粘贴回输入框，中文与 emoji 保持完整。桌面生成的独立 CLI 在仓库外执行同一保存接口，返回与桌面一致的环境解析结果；Agent Markdown 包含选择的环境，未展开环境值。
- 最终打包二进制在原生窗口复核：URL 输入框保持焦点时按 ⌘Enter，收到 200 和预期查询参数。修复了 NSH 默认快捷键策略在文本输入框中不触发发送的问题，并加入回归测试。
- `python3 scripts/package-macos.py` 生成 `dist/ApiBuddy.app`；桌面入口名为 `apibuddy-app`，CLI 名为 `apibuddy`，避免 macOS 大小写不敏感文件系统上相互覆盖。使用包内相同二进制完成隔离原生验证，未做正式安装、签名、公证或跨平台验收。

保留隔离体验窗口及本机回显服务。源码未提交、推送或合并到 main。标签草稿与历史目前仅会话保留；完整边界见 [ENGINEERING](ENGINEERING.md)。以下记录属于旧版历史，不代表当前 NSH 分支的能力或测试结果。

## 2026-10-26：dev 版本提交

按用户确认，本轮提交将多标签与草稿恢复、桌面/CLI 共用环境、cURL 导入导出、紧凑可调工作台和长响应阅读器一并纳入 dev。提交沿用下方已通过的 43 项自动测试、类型检查、stable 构建和原生交互验收结果；本次整理仅补充交付记录，没有再次修改运行代码。main 保持原基线，下方“未提交”等表述属于各轮开发时的历史状态。

## 2026-10-26：长响应阅读器优化

本轮在紧凑布局基础上新增响应搜索导航、JSON 语法着色、换行开关、请求/响应单独放大及流式跟随。成功复制使用现有底栏短提示，成功保存使用现有状态栏，不再增加提示行。

- 类型检查、`git diff --check`、43 项自动测试及最终 stable 安装包构建通过。新增 4 项覆盖 Unicode 原文偏移、字面量搜索、匹配上限与导航回绕、含转义字符串的 JSON 分段及跨 token 匹配、较大响应回退时文本不丢失。
- 原生 dev 使用隔离数据和本机 42428 测试接口，GET `/large-json` 返回 200 / 49.8 KiB / 160 项。语法着色可见；搜索 ready 显示 160 处，Enter 跳至 3/160，Shift+Enter 回到 2/160，当前命中自动滚入视野。
- 请求与响应分别放大通过；只看请求时 ⌘F 恢复响应并聚焦搜索；搜索框 Esc 先关闭查找，下一次 Esc 恢复分栏。⌘B 收放侧栏正常。
- `/stream-many` 真实输出 100 行；上滚后按钮变为“继续跟随”，点击后恢复“跟随最新”。204 `/empty` 显示“响应正文为空”。
- 长行不换行显示正常，复制成功只在现有底栏显示“已复制响应”；保存长响应测试请求后显示修订 1，没有成功提示行挤占正文。最终窗口保留已保存的长响应请求及放大阅读模式，Esc 可恢复分栏。

最终 dev 临时构建为 `apibuddy-electrobun-IaaCT4`，stable 为 `apibuddy-electrobun-OPmTT3`。原生交互验收在 dev 进行，stable 只验证打包；未声明发布、安装或跨平台验收。原有 42427 回显与新增 42428 长响应服务保留供隔离窗口使用，测试数据未进入日常库。

## 2026-10-26：紧凑工作台与可调分区

按窗口空间反馈重排主界面：移除装饰标题、常驻说明和卡片外边距；URL/发送/保存/环境固定在顶部，请求与响应占用剩余高度。低频操作、说明、存储路径、响应查找和字段路径按需展开。

- `hutch run check`、`git diff --check` 通过；39 项自动测试全部通过；最终源码的 macOS arm64 stable 安装包构建通过。
- 使用原有隔离 QA 目录和本机回显服务，在真实 dev 窗口中编辑并格式化中文 JSON、发送 POST，得到 200；编辑中文说明并保存，新模板出现在侧栏。
- 拖动左右分隔线，将请求区由 45% 调至约 31%；右方向键增加到 33%，回车均分。切换上下布局正常，收起侧栏后 ⌘K 自动恢复侧栏并聚焦搜索。
- 窗口缩到约 800×650 时自动上下排列，URL、发送、保存和响应保持可见，无整页滚动；窄窗的布局切换按钮明确显示自动模式。
- 按需展开响应查找，中文“手机”匹配 1 处，Escape 收起并清空搜索；字段 `/message` 提取成功，复制后粘贴核对“本地回显成功”；JSON 树显示并可定位字段。更多操作菜单和说明展开/收起均正常。
- 最终 dev 进程重启后恢复四个标签、已保存中文说明、侧栏收起状态和 50% 分区比例；重新发送返回 200。WebView 存储仅保存布局偏好，接口与环境数据格式不变。

安装包输出在系统临时目录 `apibuddy-electrobun-svqDER`，最终 dev 窗口为 `apibuddy-electrobun-aaRI58`。安装包只验证构建，本轮交互验收使用相同源码的 dev 窗口；未重新验证正式安装、签名或跨平台。保留最终隔离测试窗口和本机回显服务供体验，源码仍是 dev 工作区改动，未提交或合并 main。

## 2026-10-26：多标签、环境、cURL 与响应定位

改动位于 `~/code/apibuddy/ApiBuddy-dev` 的 `dev`，保留前一轮 worktree 文档修改；不合并 main，不提交或推送。

- 类型检查与最终 macOS stable 安装包构建通过；自动测试扩展至 39 项并全部通过。新增覆盖 cURL 引号/中文/重复请求头/不支持语法拒绝、环境优先级与服务隔离、配置修订冲突/损坏文件/符号链接、未完成草稿恢复、JSON Pointer，以及生成的独立 CLI 实际 HTTP 请求和环境参数覆盖。
- 原生 macOS arm64 使用临时数据目录和本机回显服务：创建“订单测试”服务与 local 环境，把模板默认地址设为不可用端口，选择 local 后发送仍返回 200；证明环境覆盖生效。
- 原生粘贴含中文 JSON 的多行 cURL，导入为 POST 草稿，快捷键发送后回显正文正确。JSON Pointer 提取 `/message`，复制并在搜索框粘贴核对“本地回显成功”；JSON 树实际显示字段。
- 发起慢请求后切到 POST 标签，原 POST 响应保留；切回慢请求看到其独立的 10 秒完成结果。新布局的发送/停止按钮、取消终态、关闭未保存标签的取消保护均已操作验证。
- 关闭测试窗口后，以相同临时目录启动新版：四个标签、选中标签、中文未保存描述和环境选择恢复；响应不恢复，也不自动发送。
- 解锁后完成最终 CLI/AI 原生复核：选择 local 的“查询订单”标签，接入面板及实际剪贴板 Markdown 均包含 `--environment 'local'`；命名接口和原始路径示例都继承该环境。仓库外执行桌面刚生成的 `apibuddy orders 查询订单 --environment local`，返回 200、`/orders?id=42` 和“本地回显成功”，桌面发送结果一致。`environments` 只列变量名。
- 最终原生验收使用与 stable 安装包相同源码的 dev 构建。stable 打包成功，但其启动器的自动化窗口读取返回 `AXError.cannotComplete`，未把该结果当作安装包启动交互验收通过。未进行完整 IME、屏幕阅读器、Windows/Linux 或生产签名验证。

构建与 QA 产物放在系统临时目录；旧测试进程及过期构建已清理；解锁后的最终 dev 测试窗口和本机回显服务保留供体验，使用独立临时数据目录。不能把临时产物存在视为正式安装或长期分发。产品仍缺少 OpenAPI 导入、持久请求历史、脚本断言、multipart 和秘密存储，不能据此宣称已全面超过 Postman。

## 环境

2026-10-22 本机验证：macOS arm64，Hutch 0.27.1，Electrobun 2.0.1，构建 pragma Cottontail 0.6.0，SDK 随包运行时 Cottontail 0.5.0，Node.js v23.9.0，Vite 6.4.3。框架与依赖沿用 hutch.lock，未升级。

```sh
hutch run install
hutch electrobun prepare
hutch run check
hutch run test
APIBUDDY_DIR="/path/to/data" hutch run dev
hutch run build
```

`check` 检查 src 和 tests，类型库为 ES2021。`test` 使用现有 Vite 工具链中的 esbuild 在临时目录编译测试，再运行 Node test runner；临时文件自动清理。HTTP 测试仅监听 127.0.0.1，需要允许本地端口。

`dev` 和 `build` 调用 scripts/desktop.mjs，将当前源码复制到系统临时目录 apibuddy-electrobun-*。构建不改变工作区中的旧产物；命令输出当前临时路径。dev 每次构建快照后启动，无 watch/HMR。macOS 通过 open -n -W 启动 .app，并显式传递 APIBUDDY_DIR；Finder 启动不会继承终端环境。

## 2026-10-23：ApiBuddy / dev2 验证

目标仓库为 ~/code/apibuddy，在原有 main 历史上创建 dev2。已带入原 ApiX-dev 当前的 50 份源码/配置/文档（含未提交的新建接口修复），独立安装依赖；未复制缓存、旧 .git 或用户模板。hutch.lock 仅修改 workspace 包名，pnpm-lock.yaml 保持原样。

- 冻结安装、TypeScript 检查与 31 项测试通过。环境兼容覆盖 APIBUDDY_DIR > APIABC_DIR > APIX_DIR、显式空值拒绝、旧默认目录原地重开与新目录优先。
- stable 构建通过，产物位于系统临时目录 apibuddy-electrobun-utTco3；应用为 build/stable-macos-arm64/ApiBuddy.app，安装镜像为 artifacts/macos-arm64-ApiBuddy.dmg。Bundle ID 为 io.github.hezhangjian.apibuddy。
- 原生桌面在 /private/tmp/apibuddy-dev2-qa-wep9skin 创建「订单服务」，服务 ID orders；新建 GET /orders/{{id}}，变量自动补齐，保存 id=123 后桌面发送返回 200，回显 /orders/123。
- 原生 Agent 面板生成 bin/apibuddy、cli/apibuddy.cjs 和服务 AGENT.md；窗口、命令、环境变量及说明均采用 ApiBuddy。点击复制显示成功，面板 Markdown 内容与路径可查看。
- 在仓库外执行桌面生成的 CLI：get-order --id 456 返回 200 和 /orders/456；request POST /orders 发送中文 JSON 正文并返回 200。测试仅使用本机回显服务。
- 从新仓库直接执行 hutch run dev 成功构建并打开 ApiBuddy-dev；原生窗口确认当前沿用 /Users/akka/.apix/templates，测试数据留在独立临时目录，回显服务已停止。dev 启动命令保持运行以供用户使用。
- 沙箱中的本机监听及 hdiutil 操作受限，授权后重跑通过；未更改测试来规避限制。构建与桌面验证不覆盖正式签名、公证、Windows 或 Linux。

本次代码保留为 dev2 工作区改动，未提交、推送或 npm 发布。下方为旧仓库的历史记录，提交哈希与路径保留当时的值。

## 历史检查记录（原 ApiX-dev 仓库）

| 检查 | 2026-10-22 结果 |
| --- | --- |
| hutch run install | 冻结安装通过，依赖无变化 |
| hutch run check | 通过，含应用与测试 TypeScript |
| hutch run test | 31 项通过 |
| hutch run dev | 通过，真实 Electrobun 系统 WebView 窗口 |
| hutch run build | 通过，生成本地 macOS arm64 .app 和 DMG |
| git diff --check | 通过 |

原有 16 项测试包含：环境路径解析；模板重新读取、修改、复制、删除与权限；损坏/未知版本/符号链接保留；写入冲突和锁释放；变量替换、查询编码与重复参数；缺失变量、重复变量和非法输入；GET 忽略正文；POST、UTF-8 跨块、HTTP 非 2xx、取消、任务释放、超时、断流和 8 MiB 上限。

新增 9 项测试覆盖：服务创建/重开/唯一标识、跨服务读写拒绝、损坏元数据与链接目录、CLI 文件权限、隐藏变量默认值、变量覆盖与 dry-run、真实 HTTP、保存后直接生效、错误/超时/SIGINT 退出码，以及数据根目录覆盖。测试启动实际生成的独立 CLI 子进程，而非只模拟命令解析。

测试运行时是 Node；桌面行为另在应用内 Cottontail 验证，不能把 Node 测试单独当作运行时兼容证明。

## 桌面交互记录

测试数据根目录为 /private/tmp/apix-template-qa-20261022，未向真实模板库写入测试请求。可控服务器由以下命令启动：

```sh
node scripts/fixture-server.mjs
```

服务只监听 127.0.0.1:42427，提供 /echo、/slow、/stream，不需要外部 API 或凭据。

已实际验证：

- 指定 APIX_DIR 后，窗口左下角显示正确的临时存储目录。
- 中文模板名、变量值和 JSON 正文可粘贴编辑；⌘A 正确全选；复制响应后可粘贴回编辑器。
- 保存模板后列表出现记录；更新产生修订 2；另存副本产生新 UUID。
- POST 正文中 {{base_url}} 被替换，真实 RPC/Fetch 返回中文 JSON 回显。
- 新应用进程重新读取两份模板，恢复名称、方法、URL、变量和正文。
- 未保存修改时新建模板出现保存/放弃/取消对话框。
- 延迟请求可取消，显示明确的已取消终态。
- 数字输入粘贴 1 后，请求在约 1.01 秒以已超时结束。
- /stream 第一段在请求进行中即可见，随后收到第二段中文及表情。

未验证：真实中文拼音 IME 合成、撤销/重做、跨平台、多显示器缩放、屏幕阅读器、生产签名和公证。模板删除及损坏文件场景有自动测试，但未宣称每条都做过桌面点击验收。

## 服务与 CLI 桌面验证

第二轮使用独立目录 `/private/tmp/apix-service-qa-20261022`：

- 从桌面分别创建「订单服务」与「用户服务」，默认地址带入新模板。
- 各自保存请求后切换，列表、编辑草稿和实际目录均按服务隔离；未保存内容会提示保存/放弃/取消。
- 订单模板重新加载后通过桌面 RPC 请求本地 /echo，返回 200 和 order=1001。
- 点击「接入 AI Agent」后生成 apix-orders.cjs 与 AGENT.md；复制说明后用 ⌘V 粘贴，确认 Markdown 内容完整进入系统剪贴板。
- 在仓库外执行桌面生成的 CLI，list 只返回订单模板，run 覆盖 order_id=2002，真实返回 200 与 /echo?order=2002。
- 最终 stable 安装包使用同一临时数据重新启动后，可重新读取两个服务和已保存模板，并成功生成包含随包运行时的 CLI。
- 自动化临时粘贴工具曾在一个字段读到旧剪贴板；通过原生 AX 输入确认中文名称可保存，正常 ⌘V 与产品复制按钮已验证。未将该工具行为当作完整输入法验证。

## 简化 CLI 验证

2026-10-22 按用户反馈统一改为 APIX_DIR，公开命令为 apix-服务名 接口名 --参数 值。新增两项集成测试分别覆盖一个 CLI 调用全部已保存接口，以及 request 调用没有保存模板的路径；均覆盖七种 HTTP 方法。还验证 PATH 启动、ESM 项目内的 CJS 运行时、中文/空格/引号路径、help 参数清单、保存新接口后立即发现和原文请求体。

本轮示例数据在 /private/tmp/apix-cli-simple-qa-20261022，订单服务包含 list-orders、get-order、create-order 三个接口，地址指向本机回显服务器；不是远端业务数据。最终安装包已实际展示三个接口的命令清单与 APIX_DIR；桌面生成的 apix-orders 在仓库外逐一调用三个接口，并通过 request GET /orders/999 调用未保存路径，四次均返回 200。

## 统一入口验证

最新命令形式为 apix 服务 接口。新增集成测试验证一个可执行文件发现 orders、users、default 和后续创建的 billing 服务；同一个入口按服务路由且拒绝跨服务模板 ID；生成另一个服务的说明不改变原服务调用；APIX_DIR 切换时使用新目录服务清单，未知服务明确失败。28 项测试通过。

最终 stable 应用使用上述临时示例数据启动，原生界面已显示「订单服务 → apix orders」和三个接口的新命令。通过桌面生成的共享 bin/apix，在仓库外执行 apix --help、apix orders --help；get-order --id 123 与 request GET /orders/999 均向本机回显服务返回 200。界面、随包运行时与实际生成的命令一致。

提交前再次通过类型检查、28 项测试及 stable 构建。在最新原生窗口中打开「新建服务」，向「服务 ID」字段输入 orders，确认调用预览实时变为 apix orders <接口名>，格式与唯一性提示可见；验证后取消，未新增测试服务。

## 新建接口流程验证

原按钮在空草稿上点击没有可见变化，已改为创建弹窗。新增两项测试覆盖路径前缀与变量、完整 URL、服务隔离和非法输入；全部 30 项测试通过，类型检查与 stable 构建通过。

桌面使用独立临时目录 /private/tmp/apix-interface-qa-fbjf2k_s：点击按钮立即出现名称、方法、路径表单；输入 get-order 和 /orders/{{id}} 后预览正确地址。通过测试用写锁模拟保存失败，错误在弹窗显示且输入保留；解除该测试锁后重试，列表立即出现接口、自动打开并显示修订 1，id 变量已补齐。取消下一次创建保留原接口及列表数量；修改变量后再新建会提示保存/放弃/取消，「保存并继续」成功保存后打开创建框，Escape 可取消。桌面生成的 CLI 调用 apix orders get-order --id 123，对本机回显服务返回 200 和 /orders/123。

## 打包记录

本轮最终构建位于系统临时目录 apix-electrobun-udHmaK；应用位于 build/stable-macos-arm64/ApiX.app，安装镜像位于 artifacts/macos-arm64-ApiX.dmg。正式图标尚缺；未配置 release.baseUrl，跳过差量更新。上一版已按用户要求提交为本地 dev 的 b16de52；本次新建接口修复保留为工作区改动，未上传、发布或推送远端。

## ApiABC 更名验证

2026-10-22 产品、包名、CLI、环境变量和构建产物已统一采用新名称。类型检查与 31 项测试通过；新增验证已有 ~/.apix 模板原地重开、新默认目录优先、APIABC_DIR 优先于旧 APIX_DIR，以及显式空值拒绝。CLI 集成测试通过 PATH 执行 apiabc 并覆盖新旧变量对服务目录的选择。

stable 构建位于系统临时目录 apiabc-electrobun-f7AyiI，应用为 build/stable-macos-arm64/ApiABC.app，安装镜像为 artifacts/macos-arm64-ApiABC.dmg。Info.plist 的 CFBundleName 为 ApiABC，CFBundleIdentifier 为 io.github.hezhangjian.apiabc。

原生桌面使用独立测试目录 /private/tmp/apiabc-rename-qa-mejlh85y，确认窗口标题、品牌、存储提示及 Agent 面板均使用新名称，并成功读取更名前保存的三个订单接口。桌面生成 bin/apiabc 与新的 Markdown；在仓库外运行帮助和 get-order --id 123，后者将 base_url 临时覆盖为本机随机端口的回显服务，返回 200 和 /orders/123，未改写已保存参数。

沙箱内测试监听及 DMG 创建最初受系统限制，使用获准的本机权限重跑后通过。桌面自动化初次定位遇到引导进程残留，结束本次启动的无子进程引导程序后，真实窗口与交互验证通过。上述更名前的验证记录保留当时的名称和路径。当前改动仍在 dev 工作区，未提交、推送或发布 npm。

Dioxus WebView 试验已保存在迁移前的 Git stash，名称见 DECISIONS。需要恢复时先保护当前开发改动，再切回对应试验分支；不要直接将 stash 应用到当前 Electrobun 工作区。
