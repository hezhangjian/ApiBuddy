# 统一入口：apibuddy 服务 接口

假设你的订单服务里有三个接口，生成的 CLI 就是这样：

```sh
apibuddy orders list-orders --status pending   # 查询列表
apibuddy orders get-order --id 123             # 查询一笔
apibuddy orders create-order --name 手机       # 创建订单
```

只生成一个 apibuddy 程序，第一层是服务标识，第二层是接口名。每次调用按服务读取自己的接口库。英文名称中的空格转短横线，中文名称也能直接用。`apibuddy --help` 列出服务，`apibuddy orders --help` 列出订单服务的全部已保存接口；新增、修改保存后立即生效。

## 开始使用

1. 在 ApiBuddy 创建服务，例如「订单服务」，标识填 `orders`，地址填实际 API 地址。
2. 保存该服务的接口，例如名称 `get-order`，URL `{{base_url}}/orders/{{id}}`。
3. 点击「CLI 与 AI 接入」，按面板里的两行命令设置当前终端，然后调用接口。

```sh
export APIBUDDY_DIR="$HOME/.apibuddy"              # 换成面板显示的数据目录
export PATH="$APIBUDDY_DIR/bin:$PATH"

apibuddy --help                              # 查看服务
apibuddy orders --help                       # 查看订单服务接口
apibuddy orders get-order --help
apibuddy orders get-order --id 123
apibuddy orders get-order --id 123 --dry-run    # 只预览，不发送
```

需要 Node.js 18+。不用启动桌面，也不用安装项目依赖。不想改当前终端 PATH，可直接运行 `"$APIBUDDY_DIR/bin/apibuddy" orders get-order --id 123`。

## 与桌面共用环境

```sh
apibuddy orders environments
apibuddy orders get-order --environment local
apibuddy orders get-order --environment staging --id 123
```

环境来自桌面“管理环境”，每次调用实时读取。优先级为模板默认值 → 所选环境 → 显式变量参数（按参数顺序）。`environments` 只列环境名与变量名，不输出环境值。选择不存在的环境会失败，不会回退到默认地址。

桌面选择环境后打开“CLI 与 AI 接入”，面板和复制的 Markdown 会带上该环境；环境中已提供的变量不会再生成多余的占位参数。环境值本身不会写入 Markdown。`request` 使用所选环境的 `base_url`；其他认证头仍须显式传入，不从模板或环境自动注入。

## 没存模板的接口

同一个 CLI 可以直接调用服务地址下的 HTTP 接口：

```sh
apibuddy orders request GET /orders --query status=pending
apibuddy orders request POST /orders --data '{"name":"手机"}' --header 'Content-Type: application/json'
apibuddy orders request GET /orders --header "Authorization: Bearer $MY_API_TOKEN"
```

服务地址为 `https://api.example.com/v1` 时，`/orders` 会访问 `/v1/orders`。request 支持 GET、HEAD、POST、PUT、PATCH、DELETE、OPTIONS。参数、请求头可重复；正文按原文发送。此命令不读取其他接口的模板或认证头，凭据通过请求头显式传入。

## 给 AI Agent

点击「复制给 AI Agent」，把说明粘贴到 Agent 会读取的任意 Markdown。内容包含初始化命令和该服务全部已保存接口的具体调用示例；Agent 后续通过 `--help` 获取最新接口。

Agent 需要本机命令执行能力。Markdown 是给它看的工具说明，不会自动改动任何 Agent 配置。说明里的 `<参数名>` 由 Agent 根据用户任务填写。

## 参数与结果

- `--id 123` 或 `--id=123`：覆盖本次变量，未传时使用所选环境或模板默认值。
- `--environment local`：使用当前服务的环境，也接受环境 ID。
- `--env token=MY_API_TOKEN`：从已有环境变量读取凭据。
- `--vars-file /path/vars.json`：批量提供字符串键值；最大 64 KiB。
- `--dry-run`：预览解析后的请求，不发送；预览可能含凭据。

变量覆盖不保存回模板。命名接口中的正文变量只作文本替换，不自动转义 JSON。接口名冲突或与内置命令重名时，用 `--help` 中显示的命令；旧的 list/show/run 调用仍兼容。

执行输出一个 JSON，主要字段为 `ok`、`status`、`body`。body 是字符串。退出码：0 成功；2 参数/存储错误；3 网络/超时/大小限制；4 HTTP 4xx/5xx；130 取消。最大接收 8 MiB，可用 Ctrl+C 取消。

`APIBUDDY_DIR` 是桌面和 CLI 唯一的数据根目录配置。CLI 未设置时沿用生成时的目录；设置后读取新目录的服务清单，按命令中的服务标识选择。所有服务共用 `$APIBUDDY_DIR/bin/apibuddy` 与 `$APIBUDDY_DIR/cli/apibuddy.cjs`，无需逐个生成。不会全局安装或修改 shell 配置。调用范围按服务组织，不是操作系统沙箱。

旧名称 `APIABC_DIR`、`APIX_DIR` 作为兼容别名继续有效，优先级为 `APIBUDDY_DIR > APIABC_DIR > APIX_DIR`；显式空值会报错。升级后重新生成 CLI、复制新的 Markdown，即可将调用改为 `apibuddy 服务 接口`。旧的 `apiabc`、`apix` 文件不自动删除或改写。
