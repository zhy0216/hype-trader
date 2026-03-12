use anyhow::Result;
use crate::models::*;

pub struct InfoService {
    base_url: String,
}

impl InfoService {
    pub fn new(network: Network) -> Self {
        let base_url = match network {
            Network::Mainnet => "https://api.hyperliquid.xyz".to_string(),
            Network::Testnet => "https://api.hyperliquid-testnet.xyz".to_string(),
        };
        Self { base_url }
    }

    /// Fetch all available trading symbols/assets metadata
    pub async fn fetch_symbols(&self) -> Result<Vec<Symbol>> {
        // Use reqwest or hyperliquid_rust_sdk InfoClient to call /info endpoint
        // For now, create a stub that returns mock data for compilation
        // We'll wire up the real SDK calls later
        Ok(vec![
            Symbol { name: "ETH-USD".into(), base: "ETH".into(), quote: "USD".into(), last_price: 3500.0, change_24h: 2.5, volume_24h: 1_000_000.0 },
            Symbol { name: "BTC-USD".into(), base: "BTC".into(), quote: "USD".into(), last_price: 65000.0, change_24h: -1.2, volume_24h: 5_000_000.0 },
            Symbol { name: "SOL-USD".into(), base: "SOL".into(), quote: "USD".into(), last_price: 145.0, change_24h: 5.3, volume_24h: 800_000.0 },
            Symbol { name: "ARB-USD".into(), base: "ARB".into(), quote: "USD".into(), last_price: 1.15, change_24h: -0.5, volume_24h: 200_000.0 },
            Symbol { name: "DOGE-USD".into(), base: "DOGE".into(), quote: "USD".into(), last_price: 0.12, change_24h: 3.1, volume_24h: 400_000.0 },
        ])
    }

    /// Fetch order book for a symbol
    pub async fn fetch_orderbook(&self, _symbol: &str) -> Result<OrderBook> {
        // Mock data for now
        let asks: Vec<OrderBookLevel> = (0..20).map(|i| {
            let price = 3500.0 + (i as f64) * 0.5;
            let size = 10.0 - (i as f64) * 0.3;
            OrderBookLevel { price, size: size.max(0.1), cumulative: 0.0 }
        }).collect();
        let bids: Vec<OrderBookLevel> = (0..20).map(|i| {
            let price = 3499.5 - (i as f64) * 0.5;
            let size = 8.0 - (i as f64) * 0.25;
            OrderBookLevel { price, size: size.max(0.1), cumulative: 0.0 }
        }).collect();
        Ok(OrderBook { bids, asks, last_price: 3500.0 })
    }

    /// Fetch candle data
    pub async fn fetch_candles(&self, _symbol: &str, _interval: CandleInterval, _limit: usize) -> Result<Vec<Candle>> {
        // Mock candle data
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        let candles: Vec<Candle> = (0..100).rev().map(|i| {
            let base = 3400.0 + (i as f64 * 1.5).sin() * 100.0;
            Candle {
                time: now - i * 3600_000,
                open: base,
                high: base + 20.0,
                low: base - 15.0,
                close: base + 5.0,
                volume: 1000.0 + (i as f64) * 10.0,
            }
        }).collect();
        Ok(candles)
    }

    /// Fetch recent trades
    pub async fn fetch_recent_trades(&self, _symbol: &str) -> Result<Vec<Trade>> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64;
        Ok((0..50).map(|i| Trade {
            time: now - i * 1000,
            price: 3500.0 + (i as f64 * 0.7).sin() * 5.0,
            size: 0.5 + (i as f64) * 0.1,
            is_buy: i % 2 == 0,
        }).collect())
    }
}
