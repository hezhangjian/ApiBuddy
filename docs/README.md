# ApiBuddy 文档

当前实现：`~/code/apibuddy/ApiBuddy-dev` 的 `dev` 分支，Electrobun 服务请求模板与 Agent CLI 工作台。2026-10-26 已从原 `dev2` 重命名并调整为 Git worktree 布局。

- [AGENT-CLI](AGENT-CLI.md)：服务 CLI 命令、变量、退出码与 Markdown 接入。
- [ARCHITECTURE](ARCHITECTURE.md)：模块、RPC、存储和请求生命周期。
- [DECISIONS](DECISIONS.md)：当前选型与历史试验。
- [DEVELOPMENT](DEVELOPMENT.md)：命令、检查和桌面验证记录。
- [ENGINEERING](ENGINEERING.md)：已知限制和失败恢复。
- [PRODUCT](PRODUCT.md)：实际能力和后续边界。

## 对话归档后的接续入口

2026-10-26 工作区调整后的基线（后续以实际 Git 状态为准）：

- `~/code/apibuddy/ApiBuddy-main` 检出 `main`，提交 `06533e2`，仍为初始化版本。
- `~/code/apibuddy/ApiBuddy-dev` 检出 `dev`，提交 `c43a6a0`，承接原 `dev2` 的完整实现和已有依赖。
- 原 `dev` 保留为 `dev-old`，提交 `17d42eb`，不单独检出。两个 worktree 共享 Git 历史，分别保有工作区。
- 已配置远端 `https://github.com/hezhangjian/ApiBuddy.git`。本次只调整本地分支和目录，未推送或改名远端分支；现有 `origin/dev2` 引用保留。

以下是 2026-10-23 归档前的历史快照，其中分支、路径和 Git 状态已被上述记录替代，验证记录保留当时的范围：

- 当前开发只在 `/Users/akka/code/apibuddy`；已创建 `dev2`，HEAD 为 `06533e2`，最新实现仍是未提交的工作区改动，多数源文件尚未跟踪。当前仓库没有配置远端，尚未推送或发布 npm。
- 来源是 `/Users/akka/code/ApiX/ApiX-dev` 的 `dev` 分支及其未提交修复；旧仓库、旧数据、历史分支与 stash 保留原处。不要把旧仓库中的历史提交误认为本仓库已经包含的 Git 历史。
- 产品名称确定为 ApiBuddy，统一命令 `apibuddy <服务 ID> <接口>`。继续使用 Electrobun 与 TypeScript；优先把现有桌面、模板、CLI 和 Agent Markdown 流程做得简单可用。
- 已完成冻结安装、类型检查、31 项自动测试、stable 安装包构建，以及真实桌面创建服务/接口、发送请求、生成 CLI 和仓库外调用验证。详细证据与未验证范围见 [DEVELOPMENT](DEVELOPMENT.md)。文档核对未改动运行代码，未重复运行这些测试。
- 上次日常窗口沿用了 `/Users/akka/.apix` 数据目录；`APIBUDDY_DIR` 可显式覆盖，旧变量和旧目录兼容规则见 [README](../README.md#存储)。测试数据位于临时目录，没有混入日常库。
- 安装包、QA 数据和运行中的窗口都不能作为长期交付状态；临时目录可能被系统清理，需要时从源码重新构建。回显服务已停止。

新对话先读本页、[开发约定](../AGENTS.md) 和 [当前决策](DECISIONS.md)，然后确认工作区并启动：

```sh
cd ~/code/apibuddy/ApiBuddy-dev
git status --short --branch
hutch run dev
```

当前依赖已安装；若需要重新安装，使用 `hutch run install`。重新生成 CLI 后，以桌面接入面板显示的初始化命令和实际目录为准。

2026-10-26 已加入多标签、自动草稿恢复、服务环境、cURL 导入/复制及 JSON 响应定位；桌面与 CLI 环境共用，详见 [开发记录](DEVELOPMENT.md)。仍缺少服务编辑/删除、秘密存储、OpenAPI 导入和持久请求历史。工作台已按空间反馈改为固定工具栏与可调分区，支持收起侧栏、单区放大和窄窗上下排列；长响应支持语法着色、搜索导航及流式跟随。下一步从实际使用反馈继续；提交、推送和发布需用户明确要求。npm 名称检查仅用于预检，不再通过真实发布来试探名字。
