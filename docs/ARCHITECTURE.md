# 当前架构

```text
同一份 apibuddy-app：workbench / request / dialogs / ui / state
  ├─ 原生入口 → NSH 原生控件 → Backend 本地调用
  └─ WASM UI → NSH DOM 运行时 → 系统 WebView → 消息桥
                                      │
apibuddy-core::host::Host ←────────────┘
  ├─ ServiceStore / TemplateStore / EnvironmentStore / DesktopStore → 本地 JSON
  └─ Engine (reqwest + Tokio) → HTTP API
```

原生入口默认构建。WebView 宿主为 `apibuddy-webview`，使用 NSH 自带的 WebView 控件：macOS WKWebView、Windows WebView2、Linux WebKitGTK。NSH 固定在 v0.4.0 提交 `e626a8dd8f8d8b277b9507cfd5ac74d0e67020f3`，平台依赖关闭默认 JS feature。CLI 保持默认关闭。

`protocol::Command` 是两版共用的类型化能力边界：列服务、载入集合及环境、保存、解析请求、开始、取消、轮询事件。WebView 不传数据目录或任意文件路径；宿主使用启动时确定的数据根目录。WASM 编译时不包含 reqwest、Tokio、文件存储或 CLI 模块。

本地静态资源服务绑定 `127.0.0.1` 的随机端口，仅提供打包资源；路径随机前缀、Host 校验、规范化路径和 CSP 限制资源访问，没有 HTTP 后端代理接口。消息通道另有每次启动的随机标识，只有宿主加载的页面持有；响应 HTML 一律作为文本呈现。剪贴板操作交回原生宿主。退出或重载页面时释放 Host 并取消所有所属网络操作。

## UI 状态与生命周期

NSH 组件运行一次，值通过 Signal / Memo 的绑定更新。标签状态由工作台的 Owner 持有，不依附于可关闭的弹窗或当前页面；切换标签只重新挂载视图。保存基线与当前草稿分别保留，模板修订号用于冲突检测。

Host 持有 Operation 和事件接收器，UI 按标签记录操作 ID。每 50 ms 收集已确认 ID 的 Headers / Text / End；响应状态与内容刷新版本分离，计时变化不会反复格式化 JSON。取消通过 Host 中的 Operation 终止网络请求；关闭标签同时丢弃操作与待收事件。保存异步返回时只更新保存基线，不覆盖在等待期间继续编辑的草稿。历史捕获发送时的模板及环境 ID，持久化最近 50 条且最多 8 MiB，不存响应。草稿保存状态只有在最新快照落盘后才显示完成，异步保存不覆盖后续输入。

`response.rs` 负责响应预览、字面文本查找和 JSON Pointer 字段提取。查看偏好归属于请求标签，切换标签时不丢失；不参与持久草稿快照。JSON 解析仅在字段查看开启且响应完成时触发，路径编辑复用解析结果。大正文先截取显示范围再分配预览字符串；搜索位置从原文转为行号与 UTF-16 列坐标，交由 NSH 只读代码编辑器选择并滚动定位，原生与 WebView 共用。

动态集合列表虚拟化；请求标签最多 20 个，参数/头/变量最多各 100 行。请求与响应各自滚动，分隔由 NSH ResizablePanel 管理。

## 存储与请求

默认服务使用根目录的 `templates/`，其他服务使用 `services/<UUID>/templates/`；服务元数据为 `service.json`，环境为 `environments.json`。写入采用 `.write-lock` 和临时文件 rename，模板 ≤ 2 MiB，环境读取 ≤ 2 MiB。

桌面恢复文件单独存于 `desktop/`，不修改旧模板布局；原生与 WebView 各自保存标签快照并以修订号拒绝过期写入，历史在写锁内读取合并后追加。未完成的 URL、变量名和超时输入也能按原文恢复。

发送与 cURL 导出均重新读取对应服务的环境，按所选 ID 找到环境、合并变量、解析认证（Basic 在替换后做 Base64，API Key 支持 Header/Query）、编码 Form 字段，再交给 Engine。未定义变量、非法请求头、无效 URL 和超时设置都报告错误。HTTP 4xx/5xx 作为响应显示，重定向保持原始 3xx，不自动跟随。

响应上限 8 MiB，UTF-8 以增量解码器处理跨块字符。界面最多显示前 256 KiB，显式提示截断；复制操作使用完整响应。远端 HTML 只按文本展示。
