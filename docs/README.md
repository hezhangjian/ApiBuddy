# ApiBuddy 文档

当前实现：2026-10-23，`~/code/apibuddy` 的 `dev2` 分支，Electrobun 服务请求模板与 Agent CLI 工作台。

- [AGENT-CLI](AGENT-CLI.md)：服务 CLI 命令、变量、退出码与 Markdown 接入。
- [ARCHITECTURE](ARCHITECTURE.md)：模块、RPC、存储和请求生命周期。
- [DECISIONS](DECISIONS.md)：当前选型与历史试验。
- [DEVELOPMENT](DEVELOPMENT.md)：命令、检查和桌面验证记录。
- [ENGINEERING](ENGINEERING.md)：已知限制和失败恢复。
- [PRODUCT](PRODUCT.md)：实际能力和后续边界。

## 对话归档后的接续入口

以下是 2026-10-23 归档前核对的快照，后续以实际 Git 状态为准：

- 当前开发只在 `/Users/akka/code/apibuddy`；已创建 `dev2`，HEAD 为 `06533e2`，最新实现仍是未提交的工作区改动，多数源文件尚未跟踪。当前仓库没有配置远端，尚未推送或发布 npm。
- 来源是 `/Users/akka/code/ApiX/ApiX-dev` 的 `dev` 分支及其未提交修复；旧仓库、旧数据、历史分支与 stash 保留原处。不要把旧仓库中的历史提交误认为本仓库已经包含的 Git 历史。
- 产品名称确定为 ApiBuddy，统一命令 `apibuddy <服务 ID> <接口>`。继续使用 Electrobun 与 TypeScript；优先把现有桌面、模板、CLI 和 Agent Markdown 流程做得简单可用。
- 已完成冻结安装、类型检查、31 项自动测试、stable 安装包构建，以及真实桌面创建服务/接口、发送请求、生成 CLI 和仓库外调用验证。详细证据与未验证范围见 [DEVELOPMENT](DEVELOPMENT.md)。文档核对未改动运行代码，未重复运行这些测试。
- 上次日常窗口沿用了 `/Users/akka/.apix` 数据目录；`APIBUDDY_DIR` 可显式覆盖，旧变量和旧目录兼容规则见 [README](../README.md#存储)。测试数据位于临时目录，没有混入日常库。
- 安装包、QA 数据和运行中的窗口都不能作为长期交付状态；临时目录可能被系统清理，需要时从源码重新构建。回显服务已停止。

新对话先读本页、[开发约定](../AGENTS.md) 和 [当前决策](DECISIONS.md)，然后确认工作区并启动：

```sh
cd ~/code/apibuddy
git status --short --branch
hutch run dev
```

当前依赖已安装；若需要重新安装，使用 `hutch run install`。重新生成 CLI 后，以桌面接入面板显示的初始化命令和实际目录为准。

后续功能尚未排定优先级；服务编辑/删除、环境集、秘密存储、导入导出等属于明确记录的缺项，不是已完成能力。下一步从实际使用反馈继续；提交、推送和发布需用户明确要求。npm 名称检查仅用于预检，不再通过真实发布来试探名字。
