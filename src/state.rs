// src/state.rs
use crate::models::*;

/// Global application state
pub struct AppState {
    // Config
    pub network: Network,
    pub theme: ThemeMode,
    pub connection_status: ConnectionStatus,

    // Wallet
    pub wallet_address: Option<String>,
    pub private_key: Option<String>, // held in memory only

    // Market data
    pub symbols: Vec<Symbol>,
    pub selected_symbol: String,
    pub orderbook: OrderBook,
    pub recent_trades: Vec<Trade>,
    pub candles: Vec<Candle>,
    pub candle_interval: CandleInterval,

    // Account
    pub positions: Vec<Position>,
    pub open_orders: Vec<OpenOrder>,
    pub trade_history: Vec<TradeHistory>,
    pub balances: Vec<Balance>,
    pub pnl: PnlSummary,

    // UI
    pub active_tab: BottomTab,
    pub order_form: OrderFormState,
    pub symbol_filter: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            network: Network::Mainnet,
            theme: ThemeMode::Dark,
            connection_status: ConnectionStatus::Disconnected,
            wallet_address: None,
            private_key: None,
            symbols: Vec::new(),
            selected_symbol: "ETH-USD".to_string(),
            orderbook: OrderBook::default(),
            recent_trades: Vec::new(),
            candles: Vec::new(),
            candle_interval: CandleInterval::H1,
            positions: Vec::new(),
            open_orders: Vec::new(),
            trade_history: Vec::new(),
            balances: Vec::new(),
            pnl: PnlSummary::default(),
            active_tab: BottomTab::Positions,
            order_form: OrderFormState::default(),
            symbol_filter: String::new(),
        }
    }
}
