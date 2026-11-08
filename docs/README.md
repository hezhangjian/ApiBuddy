# ApiBuddy 文档

当前开发分支：`dev`（原 dev 备份为 `dev-before-nsh-20261108`），Rust + Native Shell 原生桌面；双版本基线存档于 `archive/nsh-webview-20261108`；由 `ApiBuddy-dev` 的现有 Rust 工作区切出。

- [PRODUCT](PRODUCT.md)：当前能力与边界。
- [ARCHITECTURE](ARCHITECTURE.md)：NSH 状态、共享核心和存储。
- [DECISIONS](DECISIONS.md)：选型与历史决策。
- [ENGINEERING](ENGINEERING.md)：限制与失败恢复。
- [DEVELOPMENT](DEVELOPMENT.md)：构建命令与验证记录。
- [AGENT-CLI](AGENT-CLI.md)：服务 CLI、变量与 Agent 接入。

`ApiBuddy-main` 保留 main 基线；当前实现在 dev，不合并 main。Windows 克隆和构建步骤见 [README](../README.md#在-windows-构建)。旧版 Electrobun/GPUI 的历史能力不代表当前 NSH 分支已实现；以 README 当前范围为准。
