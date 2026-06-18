use crate::types::{ApiResponse, AuthStatus, AuthInfo, BalanceInfo, DEFAULT_POOL_ID, Error, JsonObject, OrderConfig, PlaceOrderRequest, Result};
use crate::utils::{get_token_expiry, infer_user_id_from_token, is_success_code, normalize_token, parse_number};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

const BASE_URL: &str = "https://api.turboflow.xyz";

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/148.0.0.0 Safari/537.36"
);

const TOKEN_ENV_KEYS: &[&str] = &["TURBOFLOW_JWT_TOKEN", "JWT_TOKEN"];
const USER_ID_ENV_KEYS: &[&str] = &["TURBOFLOW_USER_ID", "USER_ID"];

fn build_base_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("User-Agent", USER_AGENT.parse().unwrap());
    headers.insert("Accept", "application/json, text/plain, */*".parse().unwrap());
    headers.insert("Accept-Language", "zh-CN,zh;q=0.9".parse().unwrap());
    headers.insert("Origin", "https://www.turboflow.xyz".parse().unwrap());
    headers.insert("Referer", "https://www.turboflow.xyz/".parse().unwrap());
    headers.insert("biz-pf", "6".parse().unwrap());
    headers.insert("lang", "zh-cn".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers
}

fn read_first_env(keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| std::env::var(k).ok())
        .map(|v| v.trim().to_string())
        .find(|v| !v.is_empty())
}

fn resolve_credentials(token: Option<&str>, user_id: Option<&str>) -> (Option<String>, Option<String>) {
    let resolved_token = normalize_token(token)
        .is_empty()
        .then(|| read_first_env(TOKEN_ENV_KEYS))
        .flatten()
        .or_else(|| Some(normalize_token(token)).filter(|s| !s.is_empty()));

    let resolved_user_id = user_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| read_first_env(USER_ID_ENV_KEYS));

    let resolved_user_id = resolved_user_id.or_else(|| {
        resolved_token
            .as_deref()
            .and_then(|t| infer_user_id_from_token(t))
    });

    (resolved_token, resolved_user_id)
}

fn build_headers(token: Option<&str>, user_id: Option<&str>) -> reqwest::header::HeaderMap {
    let (resolved_token, resolved_user_id) = resolve_credentials(token, user_id);
    let mut headers = build_base_headers();
    if let Some(t) = resolved_token {
        headers.insert("Authorization", format!("Bearer {t}").parse().unwrap());
    }
    if let Some(u) = resolved_user_id {
        headers.insert("uid", u.parse().unwrap());
    }
    headers
}

/// Turboflow 交易客户端
pub struct TurboflowTrader {
    base_url: String,
    token: Option<String>,
    user_id: Option<String>,
    client: Client,
}

impl TurboflowTrader {
    /// 创建客户端；token / user_id 可来自参数或环境变量
    pub fn new(token: Option<&str>, user_id: Option<&str>) -> Result<Self> {
        Self::with_base_url(BASE_URL, token, user_id)
    }

    /// 从认证信息创建
    pub fn from_auth(auth: &AuthInfo) -> Result<Self> {
        Self::new(
            Some(&auth.token).filter(|s| !s.is_empty()).map(|s| s.as_str()),
            Some(&auth.user_id).filter(|s| !s.is_empty()).map(|s| s.as_str()),
        )
    }

    /// 自定义 base_url（测试或代理场景）
    pub fn with_base_url(base_url: &str, token: Option<&str>, user_id: Option<&str>) -> Result<Self> {
        let (token, user_id) = resolve_credentials(token, user_id);
        let headers = build_headers(token.as_deref(), user_id.as_deref());
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            base_url: base_url.to_string(),
            token,
            user_id,
            client,
        })
    }

    /// 是否已配置 token 和 user_id
    pub fn is_configured(&self) -> bool {
        self.token.as_ref().is_some_and(|t| !t.is_empty())
            && self.user_id.as_ref().is_some_and(|u| !u.is_empty())
    }

    /// 获取认证状态
    pub fn auth_status(&self) -> AuthStatus {
        let expiry = get_token_expiry(self.token.as_deref());
        let token_preview = self
            .token
            .as_ref()
            .filter(|t| t.len() > 18)
            .map(|t| format!("{}...{}", &t[..8], &t[t.len() - 6..]))
            .unwrap_or_default();

        AuthStatus {
            configured: self.is_configured(),
            user_id: self.user_id.clone().unwrap_or_default(),
            token_preview,
            expires_at: expiry.map(|dt| dt.to_rfc3339()),
        }
    }

    /// 获取 USDT 余额
    pub fn get_balance(&self) -> Result<f64> {
        let info = self.get_balance_info()?;
        if info.ok {
            Ok(info.balance.unwrap_or(0.0))
        } else {
            Err(Error::Other(info.error.unwrap_or_else(|| "unknown balance error".to_string())))
        }
    }

    /// 获取详细余额信息
    pub fn get_balance_info(&self) -> Result<BalanceInfo> {
        let resp: ApiResponse = self
            .client
            .get(format!("{}/account/assets/v2", self.base_url))
            .query(&[("fill_coin_sub_info", "yes")])
            .send()?.json()?;

        let result = resp;
        if !is_success_code(result.errno.as_deref()) {
            return Ok(BalanceInfo {
                ok: false,
                balance: None,
                error: result.msg,
            });
        }

        let data = result.data.unwrap_or_default();
        let list = data
            .get("list")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut total = 0.0;
        let mut found = false;
        for coin in list {
            if coin.get("coin_code").and_then(|v| v.as_str()) == Some("1") {
                found = true;
                if let Some(avail) = parse_number(coin.get("available_balance").unwrap_or(&json!(0))) {
                    total += avail;
                }
            }
        }

        if !found {
            Ok(BalanceInfo {
                ok: false,
                balance: None,
                error: Some("USDT not found".to_string()),
            })
        } else {
            Ok(BalanceInfo {
                ok: true,
                balance: Some(total),
                error: None,
            })
        }
    }

    /// 获取 K 线数据
    pub fn get_kline(
        &self,
        pair_id: &str,
        granularity: &str,
        limit: i32,
    ) -> Result<Vec<JsonObject>> {
        let resp: ApiResponse<Vec<JsonObject>> = self
            .client
            .get(format!("{}/market/kline", self.base_url))
            .query(&[
                ("spot_token_key", pair_id),
                ("granularity", granularity),
                ("limit", &limit.to_string()),
            ])
            .send()?.json()?;

        if is_success_code(resp.errno.as_deref()) {
            Ok(resp.data.unwrap_or_default())
        } else {
            Ok(vec![])
        }
    }

    fn get_pm_order_config(
        &self,
        pair_id: &str,
        duration: i64,
        order_way: i32,
    ) -> Result<OrderConfig> {
        let resp: ApiResponse = self
            .client
            .get(format!("{}/public/pm/config", self.base_url))
            .query(&[("version", "2")])
            .send()?.json()?;

        if !is_success_code(resp.errno.as_deref()) {
            return Ok(OrderConfig::default());
        }

        let data = resp.data.unwrap_or_default();
        let configs = data
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let pair_cfg = configs
            .into_iter()
            .find(|cfg| cfg.get("pair_id").and_then(|v| v.as_str()) == Some(pair_id));

        let pair_cfg = match pair_cfg {
            Some(v) => v,
            None => return Ok(OrderConfig::default()),
        };

        let order_cfgs = pair_cfg
            .get("order_configs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let match_cfg = order_cfgs
            .into_iter()
            .find(|cfg| cfg.get("duration").and_then(|v| v.as_i64()) == Some(duration));

        let match_cfg = match match_cfg {
            Some(v) => v,
            None => return Ok(OrderConfig::default()),
        };

        let rr_key = if order_way == 1 {
            "ask_return_rate"
        } else {
            "bid_return_rate"
        };

        Ok(OrderConfig {
            return_rate: parse_number(match_cfg.get(rr_key).unwrap_or(&json!(0))),
            min_amount: parse_number(match_cfg.get("min_amount").unwrap_or(&json!(0))),
            max_amount: parse_number(match_cfg.get("max_amount").unwrap_or(&json!(0))),
        })
    }

    /// 下单
    pub fn place_order(
        &self,
        pair_id: &str,
        amount: impl Into<String>,
        duration: i64,
        order_way: i32,
        return_rate: Option<f64>,
        pool_id: i32,
    ) -> Result<JsonObject> {
        if !self.is_configured() {
            return Err(Error::Config("missing token/user_id".to_string()));
        }

        let pair_id = pair_id.to_string();
        let amount_text = amount.into().trim().to_string();
        let cfg = self.get_pm_order_config(&pair_id, duration, order_way)?;

        // 金额校验
        let amount_value: f64 = amount_text
            .parse()
            .map_err(|_| Error::InvalidAmount(format!("amount '{amount_text}' is not a number")))?;

        if let Some(min) = cfg.min_amount {
            if amount_value < min {
                return Err(Error::InvalidAmount(format!(
                    "amount too small, min={min}"
                )));
            }
        }
        if let Some(max) = cfg.max_amount {
            if max > 0.0 && amount_value > max {
                return Err(Error::InvalidAmount(format!(
                    "amount too large, max={max}"
                )));
            }
        }

        let effective_rr = return_rate.or(cfg.return_rate);
        let return_rate_value = effective_rr.and_then(|rr| {
            let scaled = if rr <= 1.5 { rr * 100.0 } else { rr };
            Some(scaled.round() as i64)
        });

        let mut payload = PlaceOrderRequest {
            account_id: self.user_id.clone().unwrap(),
            amount: amount_text,
            duration,
            order_way,
            pair_id,
            pool_id,
            coin_code: "1".to_string(),
            return_rate: None,
        };
        payload.return_rate = return_rate_value;

        let resp: ApiResponse = self
            .client
            .post(format!("{}/account/pm/order/submit", self.base_url))
            .json(&payload)
            .send()?.json()?;

        Ok(resp.data.unwrap_or_default().as_object().cloned().unwrap_or_default())
    }

    /// 获取当前持仓
    pub fn get_positions(&self) -> Result<Vec<JsonObject>> {
        let resp: ApiResponse = self
            .client
            .get(format!("{}/account/pm/positions", self.base_url))
            .query(&[("page_num", "1"), ("page_size", "100")])
            .send()?.json()?;

        if is_success_code(resp.errno.as_deref()) {
            if let Some(data) = resp.data {
                if let Some(list) = data.get("data").and_then(|v| v.as_array()).cloned() {
                    return Ok(list.into_iter().filter_map(|v| v.as_object().cloned()).collect());
                }
                if let Some(list) = data.as_array().cloned() {
                    return Ok(list.into_iter().filter_map(|v| v.as_object().cloned()).collect());
                }
            }
        }

        // 备用接口
        let resp: ApiResponse = self
            .client
            .get(format!("{}/account/position/list", self.base_url))
            .query(&[("status", "Holding"), ("page_num", "1"), ("page_size", "100")])
            .send()?.json()?;

        if is_success_code(resp.errno.as_deref()) {
            Ok(resp
                .data
                .and_then(|d| d.get("data").and_then(|v| v.as_array()).cloned())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_object().cloned())
                .collect())
        } else {
            Ok(vec![])
        }
    }

    /// 获取订单历史
    pub fn get_order_history(&self, page_num: i32, page_size: i32) -> Result<Vec<JsonObject>> {
        let resp: ApiResponse = self
            .client
            .get(format!("{}/account/pm/histories", self.base_url))
            .query(&[
                ("page_num", page_num.to_string()),
                ("page_size", page_size.to_string()),
            ])
            .send()?.json()?;

        if is_success_code(resp.errno.as_deref()) {
            Ok(resp
                .data
                .and_then(|d| d.get("data").and_then(|v| v.as_array()).cloned())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_object().cloned())
                .collect())
        } else {
            Ok(vec![])
        }
    }

    /// 下事件合约订单（通用方法）
    ///
    /// `order_way`: [`ORDER_WAY_HIGHER`] 看涨 / [`ORDER_WAY_LOWER`] 看跌
    pub fn place_event_order(
        &self,
        contract: crate::types::EventContract,
        amount: impl Into<String>,
        duration: i64,
        order_way: i32,
        return_rate: Option<f64>,
    ) -> Result<JsonObject> {
        let pair_id = contract.pair_id();
        let effective_rr = return_rate.or_else(|| contract.return_rate_for_way(duration, order_way));
        self.place_order(pair_id, amount, duration, order_way, effective_rr, DEFAULT_POOL_ID)
    }

    /// 下 BTC/USDT 事件合约订单
    pub fn place_btc_order(
        &self,
        amount: impl Into<String>,
        duration: i64,
        order_way: i32,
        return_rate: Option<f64>,
    ) -> Result<JsonObject> {
        self.place_event_order(crate::types::EventContract::BtcUsdt, amount, duration, order_way, return_rate)
    }

    /// 下 ETH/USDT 事件合约订单
    pub fn place_eth_order(
        &self,
        amount: impl Into<String>,
        duration: i64,
        order_way: i32,
        return_rate: Option<f64>,
    ) -> Result<JsonObject> {
        self.place_event_order(crate::types::EventContract::EthUsdt, amount, duration, order_way, return_rate)
    }

    /// 下 XAU/USDT 事件合约订单（快捷方法）
    pub fn place_xau_order(
        &self,
        amount: impl Into<String>,
        duration: i64,
        order_way: i32,
        return_rate: Option<f64>,
    ) -> Result<JsonObject> {
        self.place_event_order(crate::types::EventContract::XauUsdt, amount, duration, order_way, return_rate)
    }

    /// 获取用户信息
    pub fn get_user_info(&self) -> Result<JsonObject> {
        let resp: ApiResponse = self
            .client
            .get(format!("{}/account/info", self.base_url))
            .send()?.json()?;

        if is_success_code(resp.errno.as_deref()) {
            Ok(resp.data.unwrap_or_default().as_object().cloned().unwrap_or_default())
        } else {
            Ok(JsonObject::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_status_not_configured() {
        let trader = TurboflowTrader::new(None, None).unwrap();
        assert!(!trader.is_configured());
    }
}
