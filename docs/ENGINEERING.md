# 工程状态与限制（NSH 分支）

## 运行与构建

- Rust 1.99.1，NSH v0.4.0 固定 Git revision，reqwest 0.12.28，Tokio。纯 Rust UI，不链接 QuickJS。
- 日常命令：`cargo check`、`cargo test -p apibuddy-core`、`cargo test -p apibuddy-app --lib`。交付使用 `cargo build --release -p apibuddy-app` 和 `python3 scripts/package-macos.py`；安装包只包含桌面程序。
- CLI 代码保留但默认不构建、不打包；桌面相关代码由默认关闭的 `cli-integration` feature 保留，当前不是产品入口。
- 本轮目标为 macOS 原生桌面。Linux/Windows 已配置对应后端依赖，尚未验证；Linux 需要 GTK 4.14+。
- 本地 .app 未做 Developer ID 签名、公证或自动更新。打包不代表所有交互已验收。

## 数据

- 模板、环境变量明文保存；Unix 文件权限 0600，Windows 继承数据目录 ACL。没有钥匙串和凭据遮蔽。复制 cURL 会包含当前解析后的真实参数和请求头。
- 模板使用修订号检测应用间并发修改。外部手工改文件但不增加 revision，不在此保证内；刷新集合不会覆盖已打开的草稿，可另存副本后关闭并重新打开旧模板。
- 原子写入、写锁、schemaVersion、大小及符号链接校验沿用共享核心。损坏文件跳过并提示，原文件保留。
- 异常退出可能留下 `.write-lock` 或隐藏 `.tmp` 文件。确认无进程写入并备份后手动处理，不按锁龄自动删除。
- 标签、未保存草稿、环境选择、响应和历史仅在本次会话保留；不写 workspace.json。关闭未保存标签会提示，直接退出应用前请保存需要保留的接口。

## HTTP

- 最多 8 个并发请求，每个响应 ≤ 8 MiB。支持七种常用方法，GET/HEAD 不发送正文；重复查询参数和请求头保留。
- TLS 校验开启；不自动跟随重定向。无代理、客户端证书、Cookie jar 或 OAuth 流程配置。
- 正文变量是文本替换，不自动 JSON 转义；Form 模式提供 URL-encoded 原始正文，不自动编码字段。无 multipart、文件上传、二进制响应保存、非 UTF-8 字符集选择。
- SSE 按流式文本显示，无事件解析或重连。中文与 emoji 跨 HTTP 数据块使用增量 UTF-8 解码。
- Pretty 尝试格式化 JSON；大响应只显示前 256 KiB，并明确提示；复制仍保留完整内容。HTML 不执行。

## 导入与历史

- cURL 支持单条 POSIX 命令的 URL、`-X/--request`、`-H/--header`、`-d/--data`、`--data-raw`、`--data-binary`（字面文本）、`-I/--head`、`-m/--max-time` 和静默输出选项。
- 未支持参数明确报错；不执行 shell、展开变量/命令、访问引用文件或跟随 `-L`。可用单引号包裹含 shell 字符的字面值。
- 历史保留本次会话最近 50 条已结束请求，不包含响应正文。重新打开后需主动发送；环境取发送时保存的 ID，重发时使用该环境当前值。

## 桌面验证边界

原生窗口需实际验证中文输入、剪贴板、选择器、对话框、滚动和缩放；无头测试验证状态与事件，不能证明视觉质量。真实拼音 IME、屏幕阅读器、多显示器和跨平台需要额外验收。验证记录见 [DEVELOPMENT](DEVELOPMENT.md)。
