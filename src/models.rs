// src/models.rs

use serde::{Deserialize, Serialize};

// === Network ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
}

// === Connection Status ===
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

// === Theme ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

// === Wallet ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub encrypted_key: Option<String>,
    pub remember: bool,
}

// === Market Data ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub base: String,
    pub quote: String,
    pub last_price: f64,
    pub change_24h: f64, // percentage
    pub volume_24h: f64,
}

#[derive(Debug, Clone)]
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
    pub cumulative: f64,
}

#[derive(Debug, Clone, Default)]
pub struct OrderBook {
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub last_price: f64,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub time: u64,   // unix ms
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
}

#[derive(Debug, Clone)]
pub struct Candle {
    pub time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

// === Time intervals for candles ===
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleInterval {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl CandleInterval {
    pub fn label(&self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1d",
        }
    }

    pub fn to_sdk_string(&self) -> &'static str {
        match self {
            Self::M1 => "1m",
            Self::M5 => "5m",
            Self::M15 => "15m",
            Self::H1 => "1h",
            Self::H4 => "4h",
            Self::D1 => "1d",
        }
    }
}

// === Account ===
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
    TakeProfit,
    StopLoss,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub side: OrderSide,
    pub size: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub unrealized_pnl: f64,
    pub leverage: f64,
}

#[derive(Debug, Clone)]
pub struct OpenOrder {
    pub id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: f64,
    pub size: f64,
    pub filled: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct TradeHistory {
    pub id: String,
    pub symbol: String,
    pub side: OrderSide,
    pub price: f64,
    pub size: f64,
    pub fee: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Balance {
    pub asset: String,
    pub total: f64,
    pub available: f64,
    pub in_margin: f64,
}

#[derive(Debug, Clone, Default)]
pub struct PnlSummary {
    pub total_pnl: f64,
    pub daily_pnl: f64,
    pub total_balance: f64,
    pub available_balance: f64,
    pub margin_used: f64,
}

// === UI State ===
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomTab {
    Positions,
    OpenOrders,
    TradeHistory,
    Funds,
}

#[derive(Debug, Clone)]
pub struct OrderFormState {
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: String,
    pub size: String,
    pub leverage: f64,
}

impl Default for OrderFormState {
    fn default() -> Self {
        Self {
            order_type: OrderType::Limit,
            side: OrderSide::Buy,
            price: String::new(),
            size: String::new(),
            leverage: 1.0,
        }
    }
}

// === Config ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub network: Network,
    pub theme: ThemeMode,
    pub wallet: Option<WalletConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            network: Network::Mainnet,
            theme: ThemeMode::Dark,
            wallet: None,
        }
    }
}
