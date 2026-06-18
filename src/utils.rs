use crate::types::{Error, JsonObject, Result};
use base64::{engine::general_purpose::URL_SAFE, Engine};
use chrono::{DateTime, Local};
use serde_json::Value;

/// 移除前导的 "Bearer " 并清理空白
pub fn normalize_token(token: Option<&str>) -> String {
    token
        .unwrap_or_default()
        .trim()
        .strip_prefix("Bearer ")
        .or_else(|| token.unwrap_or_default().trim().strip_prefix("bearer "))
        .map(str::trim)
        .unwrap_or(token.unwrap_or_default().trim())
        .to_string()
}

/// 解析 JWT payload（不验证签名）
pub fn decode_jwt_payload(token: Option<&str>) -> Result<JsonObject> {
    let normalized = normalize_token(token);
    if normalized.is_empty() {
        return Ok(JsonObject::new());
    }

    let parts: Vec<&str> = normalized.split('.').collect();
    if parts.len() < 2 {
        return Ok(JsonObject::new());
    }

    let payload = parts[1];
    let pad_len = (4 - payload.len() % 4) % 4;
    let padded = format!("{}{}", payload, "=".repeat(pad_len));
    let decoded = URL_SAFE.decode(padded)?;
    let text = String::from_utf8_lossy(&decoded);
    let parsed: Value = serde_json::from_str(&text)?;

    match parsed {
        Value::Object(map) => Ok(map.into_iter().collect()),
        _ => Ok(JsonObject::new()),
    }
}

/// 递归在 JSON 中查找第一个匹配的 key
pub fn find_first_key(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(v) = map.get(*key) {
                    if let Some(s) = as_non_empty_str(v) {
                        return Some(s);
                    }
                }
            }
            for v in map.values() {
                if let Some(found) = find_first_key(v, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(arr) => {
            for v in arr {
                if let Some(found) = find_first_key(v, keys) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn as_non_empty_str(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// 从 token 中推断 user_id
pub fn infer_user_id_from_token(token: &str) -> Option<String> {
    let payload = decode_jwt_payload(Some(token)).ok()?;
    let keys = ["userId", "user_id", "uid", "sub"];
    for key in keys {
        if let Some(v) = payload.get(key) {
            if let Some(s) = as_non_empty_str_from_value(v) {
                return Some(s);
            }
        }
    }
    None
}

fn as_non_empty_str_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// 获取 token 过期时间
pub fn get_token_expiry(token: Option<&str>) -> Option<DateTime<Local>> {
    let payload = decode_jwt_payload(token).ok()?;
    let exp = payload.get("exp")?.as_i64()?;
    DateTime::from_timestamp(exp, 0).map(|dt| dt.with_timezone(&Local))
}

/// 解析数字字段（处理字符串或数字）
pub fn parse_number(value: &serde_json::Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// 从 API 返回的 errno 判断是否为成功
pub fn is_success_code(errno: Option<&str>) -> bool {
    errno == Some("200")
}

/// 解析 JSON 对象里的字符串或数字
pub fn parse_json_value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

impl From<base64::DecodeError> for Error {
    fn from(e: base64::DecodeError) -> Self {
        Error::Other(format!("base64 decode error: {e}"))
    }
}
