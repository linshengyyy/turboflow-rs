use crate::types::{AuthInfo, Error, JsonObject, Result};
use crate::utils::{find_first_key, normalize_token};
use reqwest::blocking::Client;
use serde_json::json;
use std::time::Duration;

const PRIVY_API_URL: &str = "https://auth.privy.io/api/v1";
const PRIVY_APP_ID: &str = "cmcjy9lbg0028l70m9owhg0oa";
const PRIVY_CA_ID: &str = "6d8288aa-9264-48ec-b90a-f05ff16b0087";
const TURBOFLOW_API: &str = "https://api.turboflow.xyz";

const USER_AGENT: &str = concat!(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) ",
    "AppleWebKit/537.36 (KHTML, like Gecko) ",
    "Chrome/148.0.0.0 Safari/537.36"
);

fn base_headers() -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Content-Type", "application/json".parse().unwrap());
    headers.insert("Accept", "application/json".parse().unwrap());
    headers.insert("Origin", "https://www.turboflow.xyz".parse().unwrap());
    headers.insert("Referer", "https://www.turboflow.xyz/".parse().unwrap());
    headers.insert("privy-app-id", PRIVY_APP_ID.parse().unwrap());
    headers.insert("privy-ca-id", PRIVY_CA_ID.parse().unwrap());
    headers.insert("privy-client", "react-auth:3.22.1".parse().unwrap());
    headers.insert("privy-ui", "t".parse().unwrap());
    headers.insert("User-Agent", USER_AGENT.parse().unwrap());
    headers
}

fn build_client() -> Result<Client> {
    Ok(Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?)
}

/// 发送邮箱验证码
pub fn send_verification_code(email: &str) -> Result<JsonObject> {
    let client = build_client()?;
    let resp = client
        .post(format!("{PRIVY_API_URL}/passwordless/init"))
        .headers(base_headers())
        .json(&json!({"email": email}))
        .send()?
        .error_for_status()?;
    Ok(resp.json()?)
}

/// 验证验证码并获取 Privy tokens
pub fn verify_code_and_get_privy_tokens(email: &str, code: &str) -> Result<JsonObject> {
    let client = build_client()?;
    let resp = client
        .post(format!("{PRIVY_API_URL}/passwordless/authenticate"))
        .headers(base_headers())
        .json(&json!({
            "email": email,
            "code": code,
            "mode": "login-or-sign-up",
        }))
        .send()?
        .error_for_status()?;
    Ok(resp.json()?)
}

/// 用 Privy tokens 交换 Turboflow API token
pub fn exchange_for_turboflow_token(privy_tokens: &JsonObject, email: &str) -> Result<JsonObject> {
    let access_token = find_first_key(
        &serde_json::to_value(privy_tokens).unwrap_or_default(),
        &["token"],
    )
    .unwrap_or_default();

    let identity_token = find_first_key(
        &serde_json::to_value(privy_tokens).unwrap_or_default(),
        &["identity_token"],
    )
    .unwrap_or_default();

    let auth_value = json!({
        "pf": "privy",
        "method": "email",
        "access_token": access_token,
        "address": email,
        "identity_token": identity_token,
        "referral_code": serde_json::Value::Null,
    })
    .to_string();

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Accept", "application/json, text/plain, */*".parse().unwrap());
    headers.insert("Authorization", auth_value.parse().unwrap());
    headers.insert("Biz-pf", "6".parse().unwrap());
    headers.insert("LANG", "zh-cn".parse().unwrap());
    headers.insert(
        "Content-Type",
        "application/x-www-form-urlencoded".parse().unwrap(),
    );
    headers.insert("Origin", "https://www.turboflow.xyz".parse().unwrap());
    headers.insert("Referer", "https://www.turboflow.xyz/".parse().unwrap());
    headers.insert("User-Agent", USER_AGENT.parse().unwrap());

    let client = build_client()?;
    let resp = client
        .post(format!("{TURBOFLOW_API}/login"))
        .headers(headers)
        .send()?
        .error_for_status()?;
    Ok(resp.json()?)
}

/// 完整的登录流程：验证码 → Privy token → Turboflow token
pub fn login_with_email_code(email: &str, code: &str) -> Result<AuthInfo> {
    let privy_tokens = verify_code_and_get_privy_tokens(email, code)?;
    let turboflow_data = exchange_for_turboflow_token(&privy_tokens, email)?;
    let token = extract_turboflow_token(&turboflow_data);
    let user_id = extract_user_id(&turboflow_data, &token);

    if token.is_empty() || user_id.is_empty() {
        return Err(Error::Auth(format!(
            "login succeeded but token or user_id is empty; raw response: {turboflow_data:?}"
        )));
    }

    Ok(AuthInfo {
        token,
        user_id,
        email: email.to_string(),
    })
}

fn extract_turboflow_token(turboflow_data: &JsonObject) -> String {
    let token_keys = ["access_token", "token", "jwt_token", "jwt", "authorization"];
    let value = serde_json::to_value(turboflow_data).unwrap_or_default();
    normalize_token(find_first_key(&value, &token_keys).as_deref())
}

fn extract_user_id(turboflow_data: &JsonObject, token: &str) -> String {
    let user_keys = ["account_id", "accountId", "user_id", "userId", "uid"];
    let value = serde_json::to_value(turboflow_data).unwrap_or_default();
    if let Some(id) = find_first_key(&value, &user_keys) {
        return id;
    }
    crate::utils::infer_user_id_from_token(token).unwrap_or_default()
}

/// 登录客户端（与 Python 版本风格保持一致）
#[derive(Debug, Default)]
pub struct TurboflowLogin;

impl TurboflowLogin {
    pub fn new() -> Self {
        Self
    }

    pub fn send_code(&self, email: &str) -> Result<JsonObject> {
        send_verification_code(email)
    }

    pub fn verify_code(&self, email: &str, code: &str) -> Result<AuthInfo> {
        login_with_email_code(email, code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_turboflow_token() {
        let mut data = JsonObject::new();
        data.insert(
            "data".to_string(),
            json!({ "access_token": "Bearer abc.def.ghi" }),
        );
        assert_eq!(extract_turboflow_token(&data), "abc.def.ghi");
    }

    #[test]
    fn test_extract_user_id() {
        let mut data = JsonObject::new();
        data.insert("account_id".to_string(), json!("12345"));
        assert_eq!(extract_user_id(&data, ""), "12345");
    }
}
