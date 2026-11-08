# 工程状态与限制（NSH 分支）

## 双版本边界

- 原生版与 WebView 版同时维护，UI/状态和 Host/存储/HTTP 均复用。WASM 与本地能力通过类型化异步接口衔接。
- WebView 版是桌面 App，不能脱离宿主作为独立浏览器 API 客户端使用。页面资源随包分发；API 请求不经过浏览器网络栈。
- 两版可共用数据目录，外部保存需刷新集合；已打开草稿保留，修订冲突仍拒绝覆盖。
- 系统 WebView 仍存在平台差异。本机验证 macOS，Windows/Linux 的编译及交互需分别验证，不能据此宣称已通过。

## 运行与构建

- Rust 1.99.1，NSH v0.4.0 固定 Git revision，reqwest 0.12.28，Tokio。纯 Rust UI，不链接 QuickJS。
- 日常命令：`cargo check`、`cargo test -p apibuddy-core`、`cargo test -p apibuddy-app --lib`。交付使用 `cargo build --release -p apibuddy-app` 和 `python3 scripts/package-macos.py`；安装包只包含桌面程序。
- CLI 代码保留但默认不构建、不打包；桌面相关代码由默认关闭的 `cli-integration` feature 保留，当前不是产品入口。
- 本轮目标为 macOS 原生桌面。Linux/Windows 已配置对应后端依赖，尚未验证；Linux 需要 GTK 4.14+。
- 本地 .app 未做 Developer ID 签名、公证或自动更新。打包不代表所有交互已验收。

## 数据

- 模板、环境变量明文保存；Unix 文件权限 0600，Windows 继承数据目录 ACL。没有钥匙串；Basic 密码输入框遮蔽字符，但磁盘数据、复制 cURL 和回显响应不会脱敏。复制 cURL 会包含当前解析后的真实参数和请求头。
- 模板使用修订号检测应用间并发修改。外部手工改文件但不增加 revision，不在此保证内；刷新集合不会覆盖已打开的草稿，可另存副本后关闭并重新打开旧模板。
- 原子写入、写锁、schemaVersion、大小及符号链接校验沿用共享核心。损坏文件跳过并提示，原文件保留。
- 异常退出可能留下 `.write-lock` 或隐藏 `.tmp` 文件。确认无进程写入并备份后手动处理，不按锁龄自动删除。
- 标签、未保存草稿和环境选择每约 0.5 秒自动写入 `desktop/native-workspace.json` 或 `desktop/webview-workspace.json`；两个入口独立恢复，原有其他 workspace 文件不迁移。写入使用修订号、写锁与原子替换；同入口的另一个窗口改动会停止覆盖并在底栏提示。损坏文件保留，停止自动覆盖。退出前确认底栏已保存；立即退出或强制终止可能丢失最后半秒改动。响应正文不落盘。

## HTTP

- 最多 8 个并发请求，每个响应 ≤ 8 MiB。支持七种常用方法，GET/HEAD 不发送正文；重复查询参数和请求头保留。
- TLS 校验开启；不自动跟随重定向。无代理、客户端证书、Cookie jar 或 OAuth 流程配置。
- 正文变量是文本替换，不自动 JSON 转义；Form 模式按启用字段替换变量后自动 URL 编码；原始 URL-encoded cURL 导入仍保留原文，可切换为 Form 字段。无 multipart、文件上传、二进制响应保存、非 UTF-8 字符集选择。
- JSON / Text 共用原始正文，Form 独立保留字段草稿（包括禁用字段）。切换模式不做隐式格式转换；仅首次进入 Form 且原始 Content-Type 为 URL-encoded 时解析已有正文。模板可选 `formDraft` 保存非活动 Form 内容，旧模板缺省为空；只有活动模式参与变量检查与发送。Form 模式禁用 JSON 格式化。
- 网络反馈区分连接拒绝、连接中断、连接失败及响应读取中断；不把底层错误中的完整 URL/参数加入提示。响应读取中断保留已收到的字节与文本，流式/非流式均适用。取消和超时沿用独立状态。
- SSE 按流式文本显示，无事件解析或重连。中文与 emoji 跨 HTTP 数据块使用增量 UTF-8 解码。
- 响应使用共享 NSH 只读代码编辑器，支持行号、JSON 着色和自动换行开关。Pretty 尝试格式化 JSON；大响应只显示前 256 KiB，并明确提示；复制仍保留完整内容。HTML 不执行。
- 响应查找是字面文本匹配，默认忽略大小写，支持 Unicode；只查找当前 Pretty / Raw / 响应头 / 字段视图实际显示的文本，最多导航前 2000 处命中（超过显示 `2000+`）。选区使用原文 UTF-16 坐标，不通过整段转小写计算位置。显示时统一换行符，复制完整响应仍保留原文。
- JSON 字段使用 JSON Pointer：`/data/0/name`、`~1` 表示键名中的 `/`、`~0` 表示 `~`，留空查看根值。响应完成后才解析，基于完整的 ≤ 8 MiB 正文；无效 JSON、非法路径和不存在的字段分别提示，`null` 是有效值。字段复制输出完整 JSON（字符串带引号），不受预览长度限制；「复制响应」仍复制完整原始响应。
- 查看模式、查找词、大小写开关和字段路径随请求标签保留，仅用于当前会话，不写入模板、历史或草稿文件。Escape 在查找展开时优先关闭查找，再次按下仍可取消请求。

## 导入与历史

- cURL 支持单条 POSIX 命令的 URL、`-X/--request`、`-H/--header`、`-d/--data`、`--data-raw`、`--data-binary`（字面文本）、`--json`、`--data-urlencode`、`-G/--get`、`-u/--user`（Basic）、`-I/--head`、`-m/--max-time` 和静默输出选项。支持 `-sS`、`-XPOST`、`-H'Name: Value'`、长参数 `--name=value` 和 `--` 参数终止符。
- `--json` 补齐 Content-Type / Accept，重复 JSON 直接拼接；显式请求头优先，包括 Authorization 优先于 `-u`。URL 编码按 UTF-8，保留重复字段；`-G` 将数据追加到查询中并清空正文，保留已有查询与片段。`--data-urlencode` 的 `@file`、`name@file` 不读取，字面 `@` 可写为 `name=内容` 或 `=内容`。Basic 必须提供冒号分隔的用户名和密码，不弹出密码提示。
- 无法保持请求语义的组合明确拒绝：GET / HEAD 正文、`--head` 搭配其他显式方法、GET 数据含未编码的空白/非 ASCII/`#`（改用 `--data-urlencode`）、`-H 'Name:'` 删除默认头。空请求头用 `-H 'Name;'`，导出也使用该形式。导入摘要不展示认证值；错误时禁用导入按钮，修正后可继续。解析结果复用于预览和导入，不执行请求。
- 未支持参数明确报错；不执行 shell、展开变量/命令、访问引用文件或跟随 `-L`。可用单引号包裹含 shell 字符的字面值。
- 参数语义参考 [cURL 官方手册](https://curl.se/docs/manpage.html)，并用本机 cURL 与临时回显服务对照；这里只实现上述子集，不读取 `.curlrc` 或复制其他 cURL 运行环境。
- 历史落盘到 `desktop/history.json`，保留最近 50 条请求且总量不超过 8 MiB（超限从最旧条目淘汰），不包含响应正文。请求头、正文和认证配置仍为明文；两个入口追加时合并，不覆盖彼此历史。重新打开后需主动发送；环境取发送时保存的 ID，重发时使用该环境当前值。

## 桌面验证边界

原生窗口需实际验证中文输入、剪贴板、选择器、对话框、滚动和缩放；无头测试验证状态与事件，不能证明视觉质量。真实拼音 IME、屏幕阅读器、多显示器和跨平台需要额外验收。验证记录见 [DEVELOPMENT](DEVELOPMENT.md)。
