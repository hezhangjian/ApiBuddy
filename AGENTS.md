# ApiBuddy 开发约定

- 当前工作目录为 ~/code/apibuddy，用户于 2026-10-23 要求新建 dev2，承接 ApiX-dev 的 Electrobun 实现、未提交的新建接口修复和产品思考，优先快速可用。不要自动同步或修改旧仓库。
- ApiBuddy 是桌面 API 客户端：创建服务 → 构筑并保存接口模板 → 桌面发送 → 生成统一 CLI → 复制 Markdown 给 AI Agent。服务 ID 由用户输入，命令形式为 apibuddy <服务> <接口>，不是一个服务一个可执行文件。
- 产品和安装包名 ApiBuddy；包名、CLI 为 apibuddy；数据目录变量 APIBUDDY_DIR，默认 ~/.apibuddy。兼容 APIABC_DIR、APIX_DIR（按此顺序）；无显式配置时依次选已有 ~/.apibuddy、~/.apiabc、~/.apix，否则创建 ~/.apibuddy。不自动搬动或合并用户数据。
- 技术栈为 Electrobun 2.0.1、Cottontail、React、TypeScript、Vite；系统 WebView，不捆绑 Chromium。不要自行切换框架。
- 开发前阅读 README.md、docs/README.md、docs/DECISIONS.md 和 docs/ENGINEERING.md。
- 主进程在 src/main，界面在 src/mainview，共享模型和 RPC 契约在 src/shared。桌面文件访问只放主进程；独立 Node CLI 在 src/cli，复用存储和请求逻辑。
- 保留 hutch.lock，冻结安装用 hutch run install；其他包操作使用 pnpm，不使用 npm/yarn。不编辑生成的 .hutch/devkit。
- 运行 hutch run check、hutch run test、hutch run build；构建和测试输出放临时目录，不提交缓存和产物。
- 桌面输入、剪贴板和渲染需实际交互验证；打包成功不等于端到端通过，流式文本不代表 SSE 解析。
- 外部文件和 RPC 参数需运行时验证；保持请求身份、成功/取消/超时/错误终态和资源清理明确。
- 响应只按数据展示，不执行远端 HTML；日志、截图和测试不包含真实凭据。
- 文档默认中文，代码注释英文。模块职责清楚，避免无实际用途的抽象。
- 未经明确要求，不提交、推送、批量迁移、删除、上传或发布。本次已授权把旧实现复制到本仓库 dev2，但未授权 npm 发布或 Git 推送。项目 private 配置保留，CLI 由桌面生成。
- 如需提交，使用 Conventional Commits 和 git commit -s。
