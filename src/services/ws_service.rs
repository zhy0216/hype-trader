use crate::models::Network;

pub struct WsService {
    base_url: String,
}

impl WsService {
    pub fn new(network: Network) -> Self {
        let base_url = match network {
            Network::Mainnet => "wss://api.hyperliquid.xyz/ws".to_string(),
            Network::Testnet => "wss://api.hyperliquid-testnet.xyz/ws".to_string(),
        };
        Self { base_url }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // WebSocket subscription will be implemented when we wire up real-time data
    // For now this is a placeholder
}
