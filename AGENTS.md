# ApiBuddy 开发约定

当前功能分支为 `feat/nsh-postman-core`（2026-11-07 按用户要求从 dev 切出）；下方双 worktree 约定为仓库基线。

- 仓库采用 main、dev 双 worktree：`~/code/ApiBuddy/ApiBuddy-main` 检出 main，`~/code/ApiBuddy/ApiBuddy-dev` 检出 dev。日常开发在 ApiBuddy-dev，验证通过并经用户要求后再合并到 main。
- ApiBuddy 当前聚焦独立桌面 API 客户端：创建服务 → 编辑请求 → 发送 → 查看响应 → 保存接口。CLI 与 AI Agent 接入按用户最新要求暂缓，源码保留但入口默认隐藏、安装包不包含 CLI。
- 产品和安装包名 ApiBuddy；包名、CLI 为 `apibuddy`；数据目录变量 `APIBUDDY_DIR`，默认 `~/.apibuddy`。兼容 `APIABC_DIR`、`APIX_DIR`（按此顺序）。
- **本分支按用户要求改用 Rust 1.99.1、Native Shell（NSH）0.4.0、reqwest、Tokio。纯 Rust 模式，无 WebView、QuickJS、Node/Electrobun。** NSH 依赖固定 Git revision；不要将参考仓库并入本仓库。
- 开发前阅读 README.md、docs/README.md、docs/DECISIONS.md 和 docs/ENGINEERING.md。
- Workspace：`apibuddy-core`（共享域与存储）、`apibuddy-app`（NSH 原生桌面）、`apibuddy-cli`（保留的 CLI 构想）。默认只构建桌面；桌面 `cli-integration` feature 默认关闭，未经用户要求不重新开放入口。
- 存储规则沿用：版本校验、写锁、原子写入；模板 ≤ 2 MiB；响应上限 8 MiB；桌面与 CLI 共用 resolve / 环境合并规则。
- 运行 `cargo check`、`cargo test -p apibuddy-core`、`cargo test -p apibuddy-app --lib`。交付使用 `cargo build --release -p apibuddy-app` 与 `python3 scripts/package-macos.py`。构建产物在 `target/`，本地应用包在 `dist/`，不提交。
- 桌面输入、剪贴板和渲染需实际交互验证；打包成功不等于端到端通过。
- 响应只按数据展示，不执行远端 HTML；日志和测试不包含真实凭据。
- 文档默认中文，代码注释英文。模块职责清楚，避免无实际用途的抽象。
- 未经明确要求，不提交、推送、批量迁移、删除、上传或发布。
- 如需提交，使用 Conventional Commits 和 `git commit -s`。
