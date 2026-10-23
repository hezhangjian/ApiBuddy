# 开发与验证

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
