#![doc = include_str!("../README.md")]

pub mod auth;
pub mod trader;
pub mod types;
pub mod utils;

pub use auth::{AuthStore, clear_auth, load_auth, save_auth};
pub use trader::TurboflowTrader;
pub use types::{
    AuthInfo, AuthStatus, BalanceInfo, DurationConfig, Error, EventContract,
    EventContractConfig, OrderConfig, Result, BTC_USDT_PAIR_ID, DEFAULT_POOL_ID,
    DURATION_10M, DURATION_15M, DURATION_1H, DURATION_30S, DURATION_3M, DURATION_5M,
    DURATION_60S, ETH_USDT_PAIR_ID, ORDER_WAY_HIGHER, ORDER_WAY_LOWER, XAU_USDT_PAIR_ID,
};
