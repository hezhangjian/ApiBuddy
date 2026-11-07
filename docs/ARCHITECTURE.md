# 当前架构

```text
NSH 原生桌面 (apibuddy-app)
  ├─ workbench / request / dialogs / ui：界面与原生控件
  ├─ state：请求标签、集合、环境、进行中的请求和会话历史
  ├─ ServiceStore / TemplateStore / EnvironmentStore → 本地 JSON
  └─ Engine (reqwest + Tokio) → HTTP API

保留构想（默认关闭、不随桌面打包）：
AI Agent → apibuddy <服务标识> <接口名> --参数 值
  ├─ 同一份服务、模板和环境
  └─ 同一份 apply_environment / resolve_request / Engine
```

交付物为独立 Rust 桌面二进制。CLI 源码保留，桌面的 `cli-integration` feature 默认关闭。NSH 固定在 v0.4.0 提交 `e626a8dd8f8d8b277b9507cfd5ac74d0e67020f3`。禁用平台默认的 `js` feature；macOS AppKit、Linux GTK、Windows Win32 原生后端分别由目标条件选择，本轮验证 macOS。

## UI 状态与生命周期

NSH 组件运行一次，值通过 Signal / Memo 的绑定更新。标签状态由工作台的 Owner 持有，不依附于可关闭的弹窗或当前页面；切换标签只重新挂载视图。保存基线与当前草稿分别保留，模板修订号用于冲突检测。

请求开始后按标签 ID 保存 Operation 和事件接收器。每 50 ms 收集 Headers / Text / End；响应状态与内容刷新版本分离，计时变化不会反复格式化 JSON。取消或关闭标签通过 Operation 终止网络请求。历史捕获发送时的模板及环境 ID，最多 50 条，不存响应。

动态集合列表虚拟化；请求标签最多 20 个，参数/头/变量最多各 100 行。请求与响应各自滚动，分隔由 NSH ResizablePanel 管理。

## 存储与请求

默认服务使用根目录的 `templates/`，其他服务使用 `services/<UUID>/templates/`；服务元数据为 `service.json`，环境为 `environments.json`。写入采用 `.write-lock` 和临时文件 rename，模板 ≤ 2 MiB，环境读取 ≤ 2 MiB。

发送与 cURL 导出均重新读取对应服务的环境，按所选 ID 找到环境、合并变量、解析请求，再交给 Engine。未定义变量、非法请求头、无效 URL 和超时设置都报告错误。HTTP 4xx/5xx 作为响应显示，重定向保持原始 3xx，不自动跟随。

响应上限 8 MiB，UTF-8 以增量解码器处理跨块字符。界面最多显示前 256 KiB，显式提示截断；复制操作使用完整响应。远端 HTML 只按文本展示。
