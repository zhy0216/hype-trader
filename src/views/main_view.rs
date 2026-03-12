use gpui::prelude::*;
use gpui::{div, px, rgb, Entity};

use crate::models::*;
use crate::views::bottom_panel::BottomPanel;
use crate::views::candle_chart::CandleChart;
use crate::views::order_book::OrderBookView;
use crate::views::order_panel::OrderPanel;
use crate::views::symbol_list::SymbolList;
use crate::views::top_bar::TopBar;

pub struct MainView {
    top_bar: Entity<TopBar>,
    symbol_list: Entity<SymbolList>,
    candle_chart: Entity<CandleChart>,
    order_book: Entity<OrderBookView>,
    order_panel: Entity<OrderPanel>,
    bottom_panel: Entity<BottomPanel>,
}

impl MainView {
    pub fn new(window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> Self {
        // Create TopBar
        let top_bar = cx.new(|_cx| {
            TopBar::new(
                Network::Mainnet,
                ConnectionStatus::Connected,
                ThemeMode::Dark,
                10000.0,
                Some("0x1234...abcd".to_string()),
            )
        });

        // Create SymbolList with mock data
        let symbol_list = cx.new(|cx| {
            let mut list = SymbolList::new(window, cx);
            list.symbols = vec![
                Symbol { name: "ETH-USD".into(), base: "ETH".into(), quote: "USD".into(), last_price: 3500.0, change_24h: 2.5, volume_24h: 1_000_000.0 },
                Symbol { name: "BTC-USD".into(), base: "BTC".into(), quote: "USD".into(), last_price: 65000.0, change_24h: -1.2, volume_24h: 5_000_000.0 },
                Symbol { name: "SOL-USD".into(), base: "SOL".into(), quote: "USD".into(), last_price: 145.0, change_24h: 5.3, volume_24h: 800_000.0 },
                Symbol { name: "ARB-USD".into(), base: "ARB".into(), quote: "USD".into(), last_price: 1.15, change_24h: -0.5, volume_24h: 200_000.0 },
                Symbol { name: "DOGE-USD".into(), base: "DOGE".into(), quote: "USD".into(), last_price: 0.12, change_24h: 3.1, volume_24h: 400_000.0 },
                Symbol { name: "LINK-USD".into(), base: "LINK".into(), quote: "USD".into(), last_price: 18.5, change_24h: 1.8, volume_24h: 300_000.0 },
                Symbol { name: "AVAX-USD".into(), base: "AVAX".into(), quote: "USD".into(), last_price: 35.0, change_24h: -2.1, volume_24h: 250_000.0 },
                Symbol { name: "MATIC-USD".into(), base: "MATIC".into(), quote: "USD".into(), last_price: 0.85, change_24h: 0.7, volume_24h: 150_000.0 },
            ];
            list
        });

        // Create CandleChart with mock candle data
        let candle_chart = cx.new(|_cx| {
            let mut chart = CandleChart::new();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            chart.candles = (0..100)
                .rev()
                .map(|i| {
                    let base = 3400.0
                        + (i as f64 * 0.1).sin() * 100.0
                        + (i as f64 * 0.3).cos() * 50.0;
                    let vol = (500.0 + (i as f64 * 0.2).sin() * 200.0).abs();
                    Candle {
                        time: now - i * 3600_000,
                        open: base,
                        high: base + 15.0 + (i as f64 * 0.5).sin().abs() * 30.0,
                        low: base - 10.0 - (i as f64 * 0.7).sin().abs() * 25.0,
                        close: base + (i as f64 * 0.4).cos() * 20.0,
                        volume: vol,
                    }
                })
                .collect();
            chart
        });

        // Create OrderBook with mock data
        let order_book = cx.new(|_cx| {
            let mut book = OrderBookView::new();
            let mut cum = 0.0;
            book.data.asks = (0..15)
                .map(|i| {
                    let size = 5.0 + (i as f64 * 0.3).sin().abs() * 10.0;
                    cum += size;
                    OrderBookLevel {
                        price: 3501.0 + i as f64 * 0.5,
                        size,
                        cumulative: cum,
                    }
                })
                .collect();
            cum = 0.0;
            book.data.bids = (0..15)
                .map(|i| {
                    let size = 4.0 + (i as f64 * 0.4).cos().abs() * 8.0;
                    cum += size;
                    OrderBookLevel {
                        price: 3500.0 - i as f64 * 0.5,
                        size,
                        cumulative: cum,
                    }
                })
                .collect();
            book.data.last_price = 3500.50;
            book
        });

        // Create OrderPanel
        let order_panel = cx.new(|cx| OrderPanel::new(window, cx));

        // Create BottomPanel with mock data
        let bottom_panel = cx.new(|cx| {
            let mut panel = BottomPanel::new(window, cx);
            panel.positions = vec![
                Position {
                    symbol: "ETH-USD".into(),
                    side: OrderSide::Buy,
                    size: 2.0,
                    entry_price: 3400.0,
                    mark_price: 3500.0,
                    unrealized_pnl: 200.0,
                    leverage: 5.0,
                },
                Position {
                    symbol: "BTC-USD".into(),
                    side: OrderSide::Sell,
                    size: 0.1,
                    entry_price: 66000.0,
                    mark_price: 65000.0,
                    unrealized_pnl: 100.0,
                    leverage: 3.0,
                },
            ];
            panel.pnl = PnlSummary {
                total_pnl: 300.0,
                daily_pnl: 150.0,
                total_balance: 10000.0,
                available_balance: 7500.0,
                margin_used: 2500.0,
            };
            panel.balances = vec![Balance {
                asset: "USDC".into(),
                total: 10000.0,
                available: 7500.0,
                in_margin: 2500.0,
            }];
            panel
        });

        Self {
            top_bar,
            symbol_list,
            candle_chart,
            order_book,
            order_panel,
            bottom_panel,
        }
    }
}

impl Render for MainView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1a1a2e))
            // TopBar
            .child(self.top_bar.clone())
            // Main content area (flex row)
            .child(
                div()
                    .flex_1()
                    .flex()
                    // Left: SymbolList
                    .child(self.symbol_list.clone())
                    // Center: Chart + Order panel
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            // Chart
                            .child(div().flex_1().child(self.candle_chart.clone()))
                            // Order Panel
                            .child(self.order_panel.clone()),
                    )
                    // Right: OrderBook
                    .child(self.order_book.clone()),
            )
            // BottomPanel
            .child(div().h(px(250.)).child(self.bottom_panel.clone()))
    }
}
