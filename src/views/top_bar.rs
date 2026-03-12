use gpui::prelude::*;
use gpui::div;
use gpui_component::button::{Button, ButtonVariants as _};

use crate::models::{ConnectionStatus, Network, ThemeMode};

pub struct TopBar {
    pub network: Network,
    pub connection_status: ConnectionStatus,
    pub theme: ThemeMode,
    pub balance: f64,
    pub address: Option<String>,
}

impl TopBar {
    pub fn new(
        network: Network,
        connection_status: ConnectionStatus,
        theme: ThemeMode,
        balance: f64,
        address: Option<String>,
    ) -> Self {
        Self {
            network,
            connection_status,
            theme,
            balance,
            address,
        }
    }
}

impl Render for TopBar {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let status_color = match self.connection_status {
            ConnectionStatus::Connected => gpui::rgb(0x00ff88),
            ConnectionStatus::Connecting => gpui::rgb(0xffaa00),
            ConnectionStatus::Disconnected => gpui::rgb(0xff4444),
        };

        div()
            .h(gpui::px(48.))
            .w_full()
            .bg(gpui::rgb(0x0f3460))
            .border_b_1()
            .border_color(gpui::rgb(0x1a1a4e))
            .flex()
            .items_center()
            .justify_between()
            .px(gpui::px(16.))
            // Left section - branding
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(12.))
                    .child(
                        div()
                            .text_size(gpui::px(18.))
                            .text_color(gpui::rgb(0xe94560))
                            .child("Hype Trader"),
                    ),
            )
            // Center section - network toggle + connection status
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(8.))
                    // Status dot
                    .child(
                        div()
                            .w(gpui::px(8.))
                            .h(gpui::px(8.))
                            .rounded(gpui::px(4.))
                            .bg(status_color),
                    )
                    .child(
                        Button::new("mainnet-btn")
                            .label("Mainnet")
                            .compact()
                            .map(|b| {
                                if self.network == Network::Mainnet {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.network = Network::Mainnet;
                            })),
                    )
                    .child(
                        Button::new("testnet-btn")
                            .label("Testnet")
                            .compact()
                            .map(|b| {
                                if self.network == Network::Testnet {
                                    b.primary()
                                } else {
                                    b.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.network = Network::Testnet;
                            })),
                    ),
            )
            // Right section - balance + address + theme toggle + settings
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(12.))
                    .child(
                        div()
                            .text_size(gpui::px(14.))
                            .text_color(gpui::rgb(0x00ff88))
                            .child(format!("${:.2}", self.balance)),
                    )
                    .when_some(self.address.clone(), |el, addr| {
                        el.child(
                            div()
                                .text_size(gpui::px(12.))
                                .text_color(gpui::rgb(0xaaaaaa))
                                .child(addr),
                        )
                    })
                    .child(
                        Button::new("theme-toggle")
                            .label(match self.theme {
                                ThemeMode::Dark => "Light",
                                ThemeMode::Light => "Dark",
                            })
                            .compact()
                            .ghost()
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.theme = match this.theme {
                                    ThemeMode::Dark => ThemeMode::Light,
                                    ThemeMode::Light => ThemeMode::Dark,
                                };
                            })),
                    )
                    .child(
                        Button::new("settings-btn")
                            .label("Settings")
                            .compact()
                            .ghost(),
                    ),
            )
    }
}
