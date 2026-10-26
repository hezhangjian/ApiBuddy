# ApiBuddy

把 API 保存成可复用的请求模板，再交给终端和 AI Agent 调用。桌面使用 Electrobun，代码使用 TypeScript。

```sh
apibuddy orders get-order --id 123
apibuddy orders create-order --name 手机
apibuddy users get-user --id 42
```

一个 `apibuddy` 入口，第一层是服务 ID，第二层是接口。新增、修改接口保存后，CLI 立即读取最新内容。

## 先跑起来

本机需要 Hutch、Node.js 18+ 和 pnpm。当前开发分支为 `dev`，采用以下 Git worktree 布局：

- `~/code/apibuddy/ApiBuddy-main`：`main` 分支，保留已确认的基线；目前仍为初始化版本。
- `~/code/apibuddy/ApiBuddy-dev`：`dev` 分支，日常开发，原名 `dev2`。

旧 `dev` 已重命名为 `dev-old`，保留历史实现，不单独检出。开发改动验证通过后，再按用户要求合并到 `main`。

```sh
cd ~/code/apibuddy/ApiBuddy-dev
hutch run install
hutch run dev
```

启动桌面后：

1. 点击「新建服务」：名称填「订单服务」，服务 ID 填 `orders`，地址填你的 API 地址。
2. 点击「新建接口」：名称填 `get-order`，方法选 GET，路径填 `/orders/{{id}}`。创建后立即保存并打开，`id` 自动成为变量。
3. 在「变量」中填写 `id`，点击「发送」查看响应；⌘/Ctrl+S 保存修改。
4. 点击「CLI 与 AI 接入」，生成 CLI 并复制面板中的初始化命令，然后调用接口。

没有现成 API 时，可在另一个终端运行本机回显服务：

```sh
cd ~/code/apibuddy/ApiBuddy-dev
node scripts/fixture-server.mjs
```

服务地址填 `http://127.0.0.1:42427`。GET `/orders/{{id}}` 和 POST `/orders` 都会回显方法、路径和正文，方便先体验完整流程。按 Ctrl+C 停止回显服务。

## 连续调试工作流

- **紧凑工作台**：URL、发送、保存和环境固定在顶部，正文区域独立滚动。拖动中间分隔线调整比例，右上角切换左右/上下排列；侧栏可收起，⌘/Ctrl+K 展开并搜索。窄窗口自动上下排列。两栏右上角可单独放大，Esc 恢复分栏；⌘/Ctrl+B 收放侧栏。

- **多标签**：同时打开不同服务的请求，后台请求继续执行，响应保留在各自标签中。`⌘/Ctrl+T` 新建草稿、`⌘/Ctrl+W` 关闭标签、`⌘/Ctrl+K` 搜索接口。
- **自动保护草稿**：标签、未保存编辑和当前环境写入本机工作区，重启后恢复；以底部“草稿已保存在本机”为准。关闭有改动的标签时可保存、放弃或取消。
- **环境**：在右上角“环境”添加 local、staging 等配置，环境变量覆盖同名模板默认值。选择环境后，发送、预览、复制 cURL 和 AI 接入命令使用同一环境。
- **cURL**：从左侧导入 POSIX/bash 格式的 cURL，预览后在新标签编辑。支持 URL、方法、请求头、文本/JSON 请求体、Cookie 文本和超时；不执行命令、不读取 `@文件`，不支持的选项会明确报错。顶部「•••」中的「复制 cURL」包含当前环境解析后的实际值，另存副本和删除也在此处。
- **响应定位**：格式化 JSON、按需展开 JSON 树、查找文本、使用 `/data/items/0/id` 形式的字段路径提取并复制结果；Raw 保留原始文本。响应栏的「查找」「字段」按需展开，⌘/Ctrl+F 聚焦响应查找，Enter / Shift+Enter 跳到下一处/上一处。JSON 语法着色、换行开关和流式跟随辅助长响应阅读；接口描述通过顶部「说明」编辑。

环境和恢复草稿与模板一样以本机 JSON 明文保存。响应只保留在当前运行的标签中，不写入恢复文件。

## 终端与 AI Agent

在接入面板生成后，按面板显示的实际目录配置终端：

```sh
export APIBUDDY_DIR="$HOME/.apibuddy"
export PATH="$APIBUDDY_DIR/bin:$PATH"

apibuddy --help                       # 查看全部服务
apibuddy orders --help                # 查看该服务全部已保存接口
apibuddy orders get-order --id 123
apibuddy orders environments                  # 查看该服务环境
apibuddy orders get-order --id 123 --environment local
apibuddy orders get-order --id 123 --dry-run
apibuddy orders request GET /orders   # 调用尚未保存模板的路径
```

示例接口需先在桌面创建。变量直接对应 `--参数 值`，支持 URL、参数、请求头和正文中的 `{{变量}}`；中文接口名也能直接调用。`request METHOD /path` 使用服务地址及路径前缀，不自动发现远端接口或继承其他模板的认证头。

「复制给 AI Agent」会生成含初始化命令、服务全部已保存接口和示例的 Markdown。粘贴到任意 Agent 会读取的文档即可；Agent 需要本机命令执行能力和 Node.js 18+。CLI 独立运行，无需桌面常驻或项目依赖，不自动修改 Agent 或 shell 配置。

[完整 CLI 说明](docs/AGENT-CLI.md)

## 存储

`APIBUDDY_DIR` 默认是 `~/.apibuddy`；可设置绝对路径、相对路径或 `~/`。例如：

```sh
APIBUDDY_DIR="$HOME/ApiBuddy-data" hutch run dev
```

每个服务独立保存 JSON 接口模板。统一入口为 `$APIBUDDY_DIR/bin/apibuddy`，运行时为 `$APIBUDDY_DIR/cli/apibuddy.cjs`。

兼容旧配置 `APIABC_DIR`、`APIX_DIR`，优先级为 `APIBUDDY_DIR > APIABC_DIR > APIX_DIR`。三者均未设置时，依次沿用已有 `~/.apibuddy`、`~/.apiabc`、`~/.apix`，否则使用 `~/.apibuddy`；不会搬动或合并数据。升级后重新生成 CLI，并复制新说明。CLI 未显式配置目录时使用生成时绑定的目录。

变量默认值、请求头和正文以 JSON 明文保存在本机。临时凭据可用 `--env token=MY_API_TOKEN` 读取，不回写模板。

## 开发与范围

```sh
hutch run check      # TypeScript 检查
hutch run test       # 存储、服务隔离、CLI 和真实 HTTP 测试
hutch run build      # 本地安装包，输出路径显示在终端
```

构建和测试产物位于系统临时目录。`dev` 每次构建快照后启动，无 watch/HMR。项目保留 `private: true`；目前 CLI 由桌面生成，不依赖 npm 发布。

当前已实现服务创建、模板新建/保存/复制/删除/搜索、多标签与草稿恢复、服务环境、cURL 导入与复制、JSON 树/字段提取/搜索、请求取消/超时和通用 Agent Markdown。macOS arm64 已验证；尚无服务编辑/删除、秘密存储、OpenAPI 导入、持久请求历史、正式签名或公证。流式响应仅展示文本，单次接收上限 8 MiB。

[设计与工程文档](docs/README.md) · [产品范围](docs/PRODUCT.md) · [决策记录](docs/DECISIONS.md)
