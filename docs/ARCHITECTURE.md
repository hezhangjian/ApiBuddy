# 当前架构

```text
React WebView
  ├─ 多标签、服务环境、cURL 导入、模板编辑与草稿恢复
  ├─ 业务 RPC → ServiceStore → 按服务 TemplateStore → 本地 JSON
  ├─ 业务 RPC → CLI 生成器 → 独立 Node CLI + Agent Markdown
  └─ 业务 RPC → HttpClient → Fetch → 目标 HTTP API
                    └─ request-event → React 响应状态

AI Agent → apibuddy <服务标识> <接口名> --参数 值
  ├─ 服务标识 → ServiceStore 查找 ID → TemplateStore → 本地 JSON
  └─ 与桌面共享 resolveRequest / HttpClient → Fetch → JSON 结果
```

桌面界面通过 Electrobun RPC 与 Cottontail 主进程通信。独立 CLI 由 Node 执行共享 TypeScript 的打包结果，直接读文件和请求 HTTP，无需桌面 IPC。

## 模块

| 目录/文件 | 责任 |
| --- | --- |
| src/cli/runtime.ts | 统一 CLI、服务路由、变量覆盖、结构化输出和退出码 |
| src/main/cli-generator.ts | 生成共享 CLI 与按服务编写的可复制说明 |
| src/main/files.ts | 新服务元数据与 CLI 文件的原子写入 |
| src/main/http-client.ts | HTTP 请求、取消、超时、UTF-8 解码、接收上限和终态 |
| src/main/index.ts | 窗口、RPC 注册、系统剪贴板、窗口关闭时取消工作 |
| src/main/service-store.ts | 服务目录、元数据验证、服务标识唯一性与模板隔离 |
| src/main/state-store.ts | 服务环境与恢复工作区的版本校验、限额、写锁和原子保存 |
| src/main/template-store.ts | 环境变量路径、文件验证、写锁、原子保存、修订号冲突 |
| src/mainview/AgentPanel.tsx | CLI 生成结果与 Markdown 复制 |
| src/mainview/App.tsx | 模板库、编辑器、保存与切换流程 |
| src/mainview/PairEditor.tsx | 可开关的查询参数和请求头编辑 |
| src/mainview/ResponsePanel.tsx | 文本响应、JSON 格式化、Headers/Raw 和复制 |
| src/mainview/ServiceDialog.tsx | 创建服务 |
| src/mainview/clipboard.ts | WKWebView 输入框的全选、剪切、复制和粘贴桥接 |
| src/mainview/rpc.ts | 类型化 RPC 和按 requestId 监听 |
| src/mainview/use-request.ts | 每标签独立请求、监听时序、计时与关闭清理 |
| src/mainview/use-workspace.ts | 工作区恢复、串行自动保存及错误保护 |
| src/shared/environments.ts | 环境模型校验与变量优先级，供桌面/CLI 共用 |
| src/shared/workspace.ts | 可保留未完成编辑的草稿和标签模型校验 |
| src/shared/curl.ts | 单条 cURL 文本解析和实际请求导出 |
| src/shared/json-view.ts | JSON Pointer 字段访问 |
| src/shared/cli-commands.ts | 接口命令名称、参数和示例，供界面与 CLI 共用 |
| src/shared/rpc-schema.ts | 两端共享通信契约 |
| src/shared/services.ts | 服务及 CLI 配置模型与校验 |
| src/shared/templates.ts | 模板模型、外部数据验证、变量替换和最终请求解析 |

## 存储

数据根目录取运行时 `APIBUDDY_DIR`，默认 `~/.apibuddy`，默认服务沿用根目录下的 `templates`，其他服务使用 `services/<UUID>/templates`。服务元数据保存在对应服务的 `service.json`。所有模板 RPC 必须显式传入服务 ID，先验证服务存在，再构造该服务的 TemplateStore。WebView 只提交模板与 UUID，不决定文件路径。主进程验证 ID、版本、修订号、字段类型、长度和文件大小，拒绝模板文件符号链接。每份模板最多 2 MiB。

更名兼容：依次读取 APIBUDDY_DIR、APIABC_DIR、APIX_DIR；三者均未设置时，依次沿用已有 ~/.apibuddy、~/.apiabc、~/.apix，否则使用 ~/.apibuddy。不会复制、合并或迁移数据。CLI 三个变量均未设置时仍使用生成时绑定的目录。

保存先验证数据，再创建 `.write-lock` 目录以排斥其他写入进程。更新时比较修订号；写入独立的 0600 临时文件、fsync、关闭后 rename 到 UUID 文件，最后释放锁。新建目录权限为 0700。进程崩溃可能留下锁或临时文件；恢复方式见 ENGINEERING。此机制不保证任意网络/云同步文件系统上的事务语义，也不是多机协同协议。

列表读取按文件隔离错误：损坏文件保留并提示，不把整个库清空，不擅自迁移未知版本。模板的名称不参与文件路径。删除是明确的单模板操作，界面有确认，并校验修订号。

## 请求生命周期

UI 先生成 requestId 并注册监听，再调用 perform-request，避免第一条响应消息早于监听。主进程独立验证/替换变量，以 Fetch 发起网络请求，发送 headers、chunk、end。end 包含 completed、cancelled、timed-out 或 failed 之一。HTTP 4xx/5xx 也是一次已完成交换，仍展示响应。

请求状态按标签 ID 隔离，切换标签不取消后台请求；关闭标签会取消该标签的请求。每次重发先注销旧监听并取消旧请求；组件卸载和窗口关闭也取消请求。主进程在 finally 清理计时器、取消/释放 reader、移除请求 Map，并尝试发送唯一终态。渲染器断开不阻止资源清理。RPC 有 15 秒确认超时，网络请求超时由模板单独设置。

接收上限为 8 MiB，超过上限保留前面的内容并失败。流式读取使用 TextDecoder 保持跨块 UTF-8 状态；流式模式每次读到可解码文本就发送，尚未实现帧率合并。数据只进入 React 文本节点，不执行响应 HTML。

## CLI 打包

scripts/desktop.mjs 使用已有 esbuild 将 src/cli/runtime.ts 编译为 Node 18 目标的单文件 CommonJS，随应用放进 Resources/app/cli/runtime.cjs。点击生成时主进程读取随包资源，注入当前数据根目录，保存到 APIBUDDY_DIR/cli/apibuddy.cjs，并生成唯一入口 APIBUDDY_DIR/bin/apibuddy（0700）；说明文件为 0600。不需要运行时安装依赖或从网络下载代码。

Node CLI 和 Cottontail 主进程共用存储、变量替换与 HTTP 生命周期逻辑；CLI 只读模板，覆写参数保留在本次内存。命名接口以直接子命令调用；request METHOD /path 按服务地址直接请求未保存的路径，不进行变量插值。APIBUDDY_DIR 可覆盖数据根目录；运行时读取该目录的服务清单，按第一个参数匹配唯一服务标识，再将后续参数交给接口执行器。无参数或 --help 列服务，服务后无参数或 --help 列接口；未知或重复服务标识明确失败，不回退到其他服务。协议详见 AGENT-CLI.md。

服务环境保存在该服务目录的 environments.json；整个恢复工作区保存在数据根目录的 workspace.json。主进程验证文件类型、尺寸及修订号；WebView 只通过 RPC 读写，不直接访问磁盘。自动保存串行处理最新快照，遇到冲突停止并保留文件。环境覆盖在桌面预览/发送/cURL 导出和 CLI 中使用同一 applyEnvironment 规则。
