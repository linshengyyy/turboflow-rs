use serde::{Deserialize, Serialize};

/// 通用错误类型
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("json (de)serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("api error: errno={errno}, msg={msg}")]
    Api { errno: String, msg: String },

    #[error("missing configuration: {0}")]
    Config(String),

    #[error("authentication missing: {0}")]
    Auth(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// 通用 API 响应包装
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResponse<T = serde_json::Value> {
    #[serde(default, rename = "errno")]
    pub errno: Option<String>,

    #[serde(default)]
    pub msg: Option<String>,

    #[serde(default)]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn is_ok(&self) -> bool {
        self.errno.as_deref() == Some("200")
    }

    pub fn into_result(self) -> Result<T> {
        if self.is_ok() {
            self.data.ok_or_else(|| Error::Other("response data is empty".to_string()))
        } else {
            Err(Error::Api {
                errno: self.errno.unwrap_or_default(),
                msg: self.msg.unwrap_or_default(),
            })
        }
    }
}

/// 认证信息
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthInfo {
    pub token: String,
    pub user_id: String,
    #[serde(default)]
    pub email: String,
}

/// 余额信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BalanceInfo {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 下单配置项
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OrderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_amount: Option<f64>,
}

/// 下单请求
#[derive(Debug, Clone, Serialize)]
pub struct PlaceOrderRequest {
    #[serde(rename = "account_id")]
    pub account_id: String,
    pub amount: String,
    pub duration: i64,
    pub order_way: i32,
    pub pair_id: String,
    pub pool_id: i32,
    #[serde(rename = "coin_code")]
    pub coin_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_rate: Option<i64>,
}

/// 默认 USDT 抵押池 pool_id
pub const DEFAULT_POOL_ID: i32 = 1;

/// 事件合约方向
pub const ORDER_WAY_HIGHER: i32 = 1;
pub const ORDER_WAY_LOWER: i32 = 2;

/// 事件合约常用持仓时长（秒）
pub const DURATION_30S: i64 = 30;
pub const DURATION_60S: i64 = 60;
pub const DURATION_3M: i64 = 180;
pub const DURATION_5M: i64 = 300;
pub const DURATION_10M: i64 = 600;
pub const DURATION_15M: i64 = 900;
pub const DURATION_1H: i64 = 3600;

/// 事件合约交易对 pair_id（逆向自 `/public/pm/config?version=2`）
pub const BTC_USDT_PAIR_ID: &str = "6";
pub const ETH_USDT_PAIR_ID: &str = "5";
pub const XAU_USDT_PAIR_ID: &str = "467401036654819328";

/// 事件合约某个持仓时长的配置
#[derive(Debug, Clone, Copy)]
pub struct DurationConfig {
    pub duration: i64,
    pub ask_return_rate: f64,
    pub bid_return_rate: f64,
    pub min_amount: f64,
    pub max_amount: f64,
}

/// 事件合约交易对完整配置
#[derive(Debug, Clone, Copy)]
pub struct EventContractConfig {
    pub pair_id: &'static str,
    pub pair_name: &'static str,
    pub enabled: bool,
    pub durations: &'static [i64],
    pub order_configs: &'static [DurationConfig],
}

/// 事件合约枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventContract {
    BtcUsdt,
    EthUsdt,
    XauUsdt,
}

impl EventContract {
    pub const fn config(&self) -> EventContractConfig {
        match self {
            EventContract::BtcUsdt => EventContractConfig {
                pair_id: BTC_USDT_PAIR_ID,
                pair_name: "BTC/USDT",
                enabled: true,
                durations: &[DURATION_30S, DURATION_60S, DURATION_3M, DURATION_5M, DURATION_10M, DURATION_15M, DURATION_1H],
                order_configs: &[
                    DurationConfig { duration: DURATION_30S, ask_return_rate: 0.872, bid_return_rate: 0.887_662, min_amount: 2.0, max_amount: 200.0 },
                    DurationConfig { duration: DURATION_60S, ask_return_rate: 0.868, bid_return_rate: 0.870_907, min_amount: 2.0, max_amount: 200.0 },
                    DurationConfig { duration: DURATION_3M, ask_return_rate: 0.851, bid_return_rate: 0.849, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_5M, ask_return_rate: 0.849, bid_return_rate: 0.851, min_amount: 2.0, max_amount: 600.0 },
                    DurationConfig { duration: DURATION_10M, ask_return_rate: 0.858, bid_return_rate: 0.842, min_amount: 2.0, max_amount: 600.0 },
                    DurationConfig { duration: DURATION_15M, ask_return_rate: 0.856, bid_return_rate: 0.9, min_amount: 2.0, max_amount: 800.0 },
                    DurationConfig { duration: DURATION_1H, ask_return_rate: 0.9, bid_return_rate: 0.87, min_amount: 2.0, max_amount: 1000.0 },
                ],
            },
            EventContract::EthUsdt => EventContractConfig {
                pair_id: ETH_USDT_PAIR_ID,
                pair_name: "ETH/USDT",
                enabled: true,
                durations: &[DURATION_30S, DURATION_60S, DURATION_3M, DURATION_5M, DURATION_10M, DURATION_15M, DURATION_1H],
                order_configs: &[
                    DurationConfig { duration: DURATION_30S, ask_return_rate: 0.891, bid_return_rate: 0.809, min_amount: 2.0, max_amount: 200.0 },
                    DurationConfig { duration: DURATION_60S, ask_return_rate: 0.882, bid_return_rate: 0.820_001, min_amount: 2.0, max_amount: 200.0 },
                    DurationConfig { duration: DURATION_3M, ask_return_rate: 0.87, bid_return_rate: 0.83, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_5M, ask_return_rate: 0.867, bid_return_rate: 0.833, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_10M, ask_return_rate: 0.9, bid_return_rate: 0.840_001, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_15M, ask_return_rate: 0.891_727, bid_return_rate: 0.816, min_amount: 2.0, max_amount: 800.0 },
                    DurationConfig { duration: DURATION_1H, ask_return_rate: 0.9, bid_return_rate: 0.854, min_amount: 2.0, max_amount: 800.0 },
                ],
            },
            EventContract::XauUsdt => EventContractConfig {
                pair_id: XAU_USDT_PAIR_ID,
                pair_name: "XAU/USDT",
                enabled: true,
                durations: &[DURATION_60S, DURATION_3M, DURATION_5M, DURATION_10M, DURATION_15M, DURATION_1H],
                order_configs: &[
                    DurationConfig { duration: DURATION_60S, ask_return_rate: 0.8, bid_return_rate: 0.8, min_amount: 2.0, max_amount: 200.0 },
                    DurationConfig { duration: DURATION_3M, ask_return_rate: 0.8, bid_return_rate: 0.8, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_5M, ask_return_rate: 0.8, bid_return_rate: 0.8, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_10M, ask_return_rate: 0.8, bid_return_rate: 0.8, min_amount: 2.0, max_amount: 400.0 },
                    DurationConfig { duration: DURATION_15M, ask_return_rate: 0.8, bid_return_rate: 0.8, min_amount: 2.0, max_amount: 800.0 },
                    DurationConfig { duration: DURATION_1H, ask_return_rate: 0.8, bid_return_rate: 0.8, min_amount: 2.0, max_amount: 800.0 },
                ],
            },
        }
    }

    pub const fn pair_id(&self) -> &'static str {
        self.config().pair_id
    }

    pub const fn pair_name(&self) -> &'static str {
        self.config().pair_name
    }

    pub fn from_pair_id(pair_id: &str) -> Option<Self> {
        match pair_id {
            BTC_USDT_PAIR_ID => Some(EventContract::BtcUsdt),
            ETH_USDT_PAIR_ID => Some(EventContract::EthUsdt),
            XAU_USDT_PAIR_ID => Some(EventContract::XauUsdt),
            _ => None,
        }
    }

    pub fn duration_config(&self, duration: i64) -> Option<&'static DurationConfig> {
        self.config()
            .order_configs
            .iter()
            .find(|cfg| cfg.duration == duration)
    }

    pub fn return_rate_for_way(&self, duration: i64, order_way: i32) -> Option<f64> {
        self.duration_config(duration).map(|cfg| match order_way {
            ORDER_WAY_HIGHER => cfg.ask_return_rate,
            ORDER_WAY_LOWER => cfg.bid_return_rate,
            _ => cfg.ask_return_rate,
        })
    }
}

/// 认证状态
#[derive(Debug, Clone, Serialize)]
pub struct AuthStatus {
    pub configured: bool,
    pub user_id: String,
    pub token_preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// 宽松的 JSON 对象别名
pub type JsonObject = serde_json::Map<String, serde_json::Value>;
