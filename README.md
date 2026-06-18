# 接口逆向

Turboflow 事件合约接口逆向（Rust SDK），利用 API 下单，可实现批量化、自动化交易；项目仅供学习交流使用，请勿用于非法用途，否则后果自负。

Turboflow 使用 Privy 邮箱验证码登录，登录后获取 JWT Token 进行接口调用。

## 认证

Web 端使用 Privy 邮箱验证码登录，获取 JWT Token。Token 通过 `Authorization: Bearer <token>` header 传递，同时需要 `uid` header 传用户 ID。

凭证可通过浏览器开发者工具获取（F12 → Network → 任意请求的 Request Headers），也可通过 SDK 的登录流程自动获取。

SDK 支持两种认证方式：
1. `auth.json` 文件持久化（推荐）
2. 环境变量 `TURBOFLOW_JWT_TOKEN` + `TURBOFLOW_USER_ID`

## 接口列表

### 1. 下单

```
POST https://api.turboflow.xyz/account/pm/order/submit
Content-Type: application/json
Authorization: Bearer <jwt_token>
uid: <user_id>

{
  "account_id": "<user_id>",
  "amount": "2",
  "duration": 3600,
  "order_way": 1,
  "pair_id": "6",
  "pool_id": 0,
  "coin_code": "1",
  "return_rate": 85
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `account_id` | string | 用户 ID，与 `uid` header 一致 |
| `pair_id` | string | 交易对 ID，见下方交易对表 |
| `duration` | i64 | 持仓时长（秒）：30, 60, 180, 300, 600, 900, 3600 |
| `order_way` | i32 | `1` = 看涨（Higher），`2` = 看跌（Lower） |
| `amount` | string | 下单金额（USDT），最低 2.0 |
| `return_rate` | f64 | 赔率百分比（如 85 表示 85%），可选，不填则用默认值 |
| `pool_id` | i64 | 抵押池 ID，默认 `0` |
| `coin_code` | string | 币种，USDT 固定 `"1"` |

**赔率说明：** 赔率影响盈亏计算。例如赔率 85%，下注 10 USDT，赢了获得 10 + 10×0.85 = 18.5 USDT，输了损失 10 USDT。赔率越高，潜在收益越高但胜率可能越低。赔率随市场波动实时变化，建议通过 `/public/pm/config` 接口获取当前可用赔率。

### 2. 查询持仓

```
GET https://api.turboflow.xyz/account/pm/positions?page_num=1&page_size=100
Authorization: Bearer <jwt_token>
uid: <user_id>
```

| 参数 | 说明 |
|------|------|
| `page_num` | 页码，从 1 开始 |
| `page_size` | 每页数量，默认 100 |

### 3. 查询订单历史

```
GET https://api.turboflow.xyz/account/pm/histories?page_num=1&page_size=20
Authorization: Bearer <jwt_token>
uid: <user_id>
```

| 参数 | 说明 |
|------|------|
| `page_num` | 页码，从 1 开始 |
| `page_size` | 每页数量，默认 20 |

### 4. 查询余额

```
GET https://api.turboflow.xyz/account/assets/v2?fill_coin_sub_info=yes
Authorization: Bearer <jwt_token>
uid: <user_id>
```

| 参数 | 说明 |
|------|------|
| `fill_coin_sub_info` | 是否填充币种子信息，固定 `yes` |

### 5. 查询用户信息

```
GET https://api.turboflow.xyz/account/info
Authorization: Bearer <jwt_token>
uid: <user_id>
```

### 6. K 线数据

```
GET https://api.turboflow.xyz/market/kline?spot_token_key=6&granularity=1m&limit=100
```

| 参数 | 说明 |
|------|------|
| `spot_token_key` | 交易对 ID（即 pair_id） |
| `granularity` | K 线周期：`1m`, `5m`, `15m`, `1h`, `4h`, `1d` 等 |
| `limit` | 返回条数，最大 100 |

### 7. 事件合约配置

```
GET https://api.turboflow.xyz/public/pm/config?version=2
```

返回所有交易对的 pair_id、duration、return_rate 等配置。无需认证。

## 交易对 pair_id

| pair_id | 交易对 | 说明 |
|---------|--------|------|
| `6` | BTC/USDT | 比特币 |
| `5` | ETH/USDT | 以太坊 |
| `467401036654819328` | XAU/USDT | 黄金 |

## 持仓时长

| 秒数 | 时长 |
|------|------|
| 30 | 30 秒 |
| 60 | 1 分钟 |
| 180 | 3 分钟 |
| 300 | 5 分钟 |
| 600 | 10 分钟 |
| 900 | 15 分钟 |
| 3600 | 1 小时 |

## 赔率（return_rate）

赔率通过 `/public/pm/config?version=2` 接口获取，格式为小数（如 0.85 表示 85%）。下单时 `return_rate` 字段传百分比整数（如 85）。

各交易对赔率示例（实时变化，以下为某一时刻的快照）：

**BTC/USDT：**
| 时长 | 看涨赔率 | 看跌赔率 |
|------|----------|----------|
| 30s | 87.2% | 88.8% |
| 1min | 86.8% | 87.1% |
| 5min | 84.9% | 85.1% |
| 1h | 90.0% | 87.0% |

**ETH/USDT：**
| 时长 | 看涨赔率 | 看跌赔率 |
|------|----------|----------|
| 30s | 89.1% | 80.9% |
| 1min | 88.2% | 82.0% |
| 5min | 86.7% | 83.3% |
| 1h | 90.0% | 85.4% |

**XAU/USDT：**
| 时长 | 看涨赔率 | 看跌赔率 |
|------|----------|----------|
| 全部 | 80.0% | 80.0% |

## WebSocket

### 行情 WS

```
wss://apis.turboflow.xyz/realtime?isDex=true
```

登录态：

```
wss://apis.turboflow.xyz/realtime?PLATFORM=web&Authorization=<jwt>&isDex=true
```

**协议：**

```json
// 订阅
{"action":"subscribe","args":["dex_all_ticker"]}

// 取消订阅
{"action":"unsubscribe","args":["dex_all_ticker"]}

// 心跳
{"action":"ping"}
```

**已知 channel：**

| channel | 说明 | 需要登录 |
|---------|------|----------|
| `dex_all_ticker` | 所有交易对 ticker | 否 |
| `dex_ticker.${pair_id}` | 单个交易对 ticker | 否 |
| `dex_trade` | 用户交易状态 | 是 |
| `dex_position` | 持仓变动 | 是 |
| `dex_asset` | 资产变动 | 是 |
| `dex_public_trade` | 公开成交流水 | 否 |
| `dex_cross_equity` | 全仓权益 | 是 |
| `dex_predict_ticker` | 事件合约 ticker | 否 |
| `dex_predict_market` | 事件合约行情 | 否 |
| `dex_predict_risk` | 事件合约风险 | 否 |
| `dex_predict_config` | 事件合约配置 | 否 |

### RPC WS

```
wss://rpc.turboflow.xyz/ws
```

用于链上 RPC 调用，非交易数据。

## Rust 使用

### 下单示例

```rust
use turboflow::{TurboflowTrader, EventContract, ORDER_WAY_HIGHER, ORDER_WAY_LOWER};
use turboflow::{DURATION_30S, DURATION_60S, DURATION_3M, DURATION_5M, DURATION_10M, DURATION_15M, DURATION_1H};

// 从 auth.json 加载凭证
let auth = turboflow::load_auth()?;
let trader = TurboflowTrader::from_auth(&auth)?;

// 查询余额
let balance = trader.get_balance()?;

// 下单：BTC/USDT, 2 USDT, 5分钟, 看涨
let result = trader.place_btc_order("2", DURATION_5M, ORDER_WAY_HIGHER, None)?;

// 下单：ETH/USDT, 5 USDT, 1小时, 看跌
let result = trader.place_eth_order("5", DURATION_1H, ORDER_WAY_LOWER, None)?;

// 下单：XAU/USDT（黄金）, 10 USDT, 5分钟, 看涨
let result = trader.place_event_order(EventContract::XauUsdt, "10", DURATION_5M, ORDER_WAY_HIGHER, None)?;

// 指定赔率下单（赔率从 /public/pm/config 获取，传百分比整数）
let result = trader.place_btc_order("2", DURATION_60S, ORDER_WAY_HIGHER, Some(87.0))?;

// 查询持仓
let positions = trader.get_positions()?;

// 查询历史订单
let history = trader.get_order_history(1, 20)?;
```

### 验证码登录

```rust
use turboflow::TurboflowLogin;

let login = TurboflowLogin::new();

// 发送验证码
login.send_code("your_email@example.com")?;

// 验证码验证，获取 token
let auth = login.verify_code("your_email@example.com", "123456")?;

// 保存凭证到 auth.json
turboflow::save_auth(&auth.token, &auth.user_id, &auth.email)?;
```

### 环境变量认证

```bash
export TURBOFLOW_JWT_TOKEN="your_token"
export TURBOFLOW_USER_ID="your_user_id"
```

```rust
let trader = TurboflowTrader::new(None, None)?;
```

### 获取凭证

有两种方式获取凭证：

**方式一：浏览器开发者工具**
1. 登录 `https://turboflow.xyz`
2. F12 → Network → 任意 API 请求
3. 复制 Request Headers 中的 `Authorization`（去掉 `Bearer ` 前缀）和 `uid`
4. 保存到 `auth.json` 或设置环境变量

**方式二：SDK 登录流程**
1. 调用 `TurboflowLogin::send_code()` 发送邮箱验证码
2. 调用 `TurboflowLogin::verify_code()` 验证并获取 token
3. SDK 自动保存到 `auth.json`

## 构建

```bash
cargo check
cargo test
cargo build --release
```

## 项目结构

```
turboflow-rs/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs        # 公共导出
    ├── types.rs      # 数据结构和常量定义
    ├── auth.rs       # 认证模块（加载/保存凭证）
    ├── login.rs      # 登录模块（邮箱验证码）
    ├── trader.rs     # 交易模块（下单/查询）
    ├── config.rs     # 配置管理
    ├── totp.rs       # TOTP 验证码
    └── utils.rs      # 工具函数
```
