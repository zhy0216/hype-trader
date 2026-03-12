use anyhow::{Result, bail};
use ethers::signers::LocalWallet;
use hyperliquid_rust_sdk::{
    BaseUrl, ExchangeClient, ExchangeResponseStatus,
    ClientOrderRequest, ClientOrder, ClientLimit, ClientTrigger,
    ClientCancelRequest, MarketOrderParams, MarketCloseParams,
};
use crate::models::*;

pub struct ExchangeService {
    client: Option<ExchangeClient>,
    base_url: BaseUrl,
}

impl ExchangeService {
    pub fn new(network: Network) -> Self {
        let base_url = match network {
            Network::Mainnet => BaseUrl::Mainnet,
            Network::Testnet => BaseUrl::Testnet,
        };
        Self { client: None, base_url }
    }

    pub async fn connect(&mut self, wallet: LocalWallet) -> Result<()> {
        let client = ExchangeClient::new(
            None,
            wallet,
            Some(self.base_url),
            None,
            None,
        ).await?;
        self.client = Some(client);
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    fn client(&self) -> Result<&ExchangeClient> {
        self.client.as_ref().ok_or_else(|| anyhow::anyhow!("exchange not connected"))
    }

    pub async fn place_limit_order(
        &self, coin: &str, is_buy: bool, price: f64, size: f64, reduce_only: bool,
    ) -> Result<String> {
        let client = self.client()?;
        let order = ClientOrderRequest {
            asset: coin.to_string(),
            is_buy,
            reduce_only,
            limit_px: price,
            sz: size,
            cloid: None,
            order_type: ClientOrder::Limit(ClientLimit { tif: "Gtc".to_string() }),
        };
        match client.order(order, None).await? {
            ExchangeResponseStatus::Ok(resp) => Ok(format!("{:?}", resp)),
            ExchangeResponseStatus::Err(e) => bail!("order failed: {}", e),
        }
    }

    pub async fn place_market_order(&self, coin: &str, is_buy: bool, size: f64) -> Result<String> {
        let client = self.client()?;
        let params = MarketOrderParams {
            asset: coin,
            is_buy,
            sz: size,
            px: None,
            slippage: None,
            cloid: None,
            wallet: None,
        };
        match client.market_open(params).await? {
            ExchangeResponseStatus::Ok(resp) => Ok(format!("{:?}", resp)),
            ExchangeResponseStatus::Err(e) => bail!("market order failed: {}", e),
        }
    }

    pub async fn place_trigger_order(
        &self, coin: &str, is_buy: bool, trigger_price: f64, size: f64, is_tp: bool,
    ) -> Result<String> {
        let client = self.client()?;
        let order = ClientOrderRequest {
            asset: coin.to_string(),
            is_buy,
            reduce_only: true,
            limit_px: trigger_price,
            sz: size,
            cloid: None,
            order_type: ClientOrder::Trigger(ClientTrigger {
                is_market: true,
                trigger_px: trigger_price,
                tpsl: if is_tp { "tp".to_string() } else { "sl".to_string() },
            }),
        };
        match client.order(order, None).await? {
            ExchangeResponseStatus::Ok(resp) => Ok(format!("{:?}", resp)),
            ExchangeResponseStatus::Err(e) => bail!("trigger order failed: {}", e),
        }
    }

    pub async fn cancel_order(&self, coin: &str, oid: u64) -> Result<()> {
        let client = self.client()?;
        let cancel = ClientCancelRequest { asset: coin.to_string(), oid };
        match client.cancel(cancel, None).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("cancel failed: {}", e),
        }
    }

    pub async fn bulk_cancel(&self, cancels: Vec<(String, u64)>) -> Result<()> {
        let client = self.client()?;
        let requests: Vec<ClientCancelRequest> = cancels.into_iter()
            .map(|(asset, oid)| ClientCancelRequest { asset, oid })
            .collect();
        match client.bulk_cancel(requests, None).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("bulk cancel failed: {}", e),
        }
    }

    pub async fn update_leverage(&self, coin: &str, leverage: u32, is_cross: bool) -> Result<()> {
        let client = self.client()?;
        match client.update_leverage(leverage, coin, is_cross, None).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("update leverage failed: {}", e),
        }
    }

    pub async fn market_close(&self, coin: &str, size: Option<f64>) -> Result<()> {
        let client = self.client()?;
        let params = MarketCloseParams {
            asset: coin,
            sz: size,
            px: None,
            slippage: None,
            cloid: None,
            wallet: None,
        };
        match client.market_close(params).await? {
            ExchangeResponseStatus::Ok(_) => Ok(()),
            ExchangeResponseStatus::Err(e) => bail!("market close failed: {}", e),
        }
    }
}
