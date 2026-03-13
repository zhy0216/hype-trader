use gpui::prelude::*;
use gpui::{div, Entity, EventEmitter};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::InputState;

use crate::components::theme::*;
use crate::components::input_field::input_field;
use crate::models::{AppConfig, Network, WalletConfig};
use crate::services::{config_service, wallet_service};

/// Events emitted by WelcomeView to signal screen transitions.
pub enum WelcomeEvent {
    /// User connected with a valid wallet. Carries (private_key, network).
    ConnectWallet { private_key: String, network: Network },
    /// User chose read-only browsing. Carries the selected network.
    BrowseReadOnly { network: Network },
}

impl EventEmitter<WelcomeEvent> for WelcomeView {}

/// Whether we show the "unlock saved wallet" UI or the fresh key-entry UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WelcomeMode {
    /// No saved wallet -- show private key input + optional remember toggle.
    Fresh,
    /// A saved encrypted wallet exists -- show password input to unlock.
    Saved,
}

pub struct WelcomeView {
    network: Network,
    mode: WelcomeMode,
    /// Encrypted key loaded from config (if any).
    saved_encrypted_key: Option<String>,
    /// Private-key input (fresh mode).
    key_input: Entity<InputState>,
    /// Password input (used in both modes -- encrypt on save, decrypt on unlock).
    password_input: Entity<InputState>,
    /// "Remember wallet" toggle (fresh mode only).
    remember: bool,
    error_message: Option<String>,
}

impl WelcomeView {
    pub fn new_with_config(
        config: Option<AppConfig>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Self {
        let config = config.unwrap_or_default();

        let has_saved = config
            .wallet
            .as_ref()
            .map(|w| w.remember && w.encrypted_key.is_some())
            .unwrap_or(false);

        let saved_encrypted_key = if has_saved {
            config.wallet.as_ref().and_then(|w| w.encrypted_key.clone())
        } else {
            None
        };

        let mode = if has_saved {
            WelcomeMode::Saved
        } else {
            WelcomeMode::Fresh
        };

        let key_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter private key (0x...)")
                .masked(true)
        });
        let password_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(if has_saved {
                    "Enter password to unlock"
                } else {
                    "Password (required to remember)"
                })
                .masked(true)
        });

        Self {
            network: config.network,
            mode,
            saved_encrypted_key,
            key_input,
            password_input,
            remember: false,
            error_message: None,
        }
    }

    /// Attempt to unlock the saved wallet with the given password.
    fn try_unlock(&mut self, cx: &mut gpui::Context<Self>) {
        let password = self.password_input.read(cx).value().to_string();
        if password.is_empty() {
            self.error_message = Some("Please enter your password.".into());
            return;
        }
        let encrypted = match &self.saved_encrypted_key {
            Some(e) => e.clone(),
            None => {
                self.error_message = Some("No saved wallet found.".into());
                return;
            }
        };
        match wallet_service::decrypt_key(&encrypted, &password) {
            Ok(private_key) => {
                self.error_message = None;
                cx.emit(WelcomeEvent::ConnectWallet {
                    private_key,
                    network: self.network,
                });
            }
            Err(e) => {
                self.error_message = Some(format!("Unlock failed: {}", e));
            }
        }
    }

    /// Attempt to connect with the entered private key (fresh mode).
    fn try_connect(&mut self, cx: &mut gpui::Context<Self>) {
        let key_str = self.key_input.read(cx).value().to_string().trim().to_string();
        if key_str.is_empty() {
            self.error_message = Some("Please enter a private key.".into());
            return;
        }
        match wallet_service::address_from_key(&key_str) {
            Ok(_) => {
                // If remember is checked, encrypt & save config.
                if self.remember {
                    let password = self.password_input.read(cx).value().to_string();
                    if password.is_empty() {
                        self.error_message =
                            Some("Password is required when 'Remember wallet' is enabled.".into());
                        return;
                    }
                    match wallet_service::encrypt_key(&key_str, &password) {
                        Ok(encrypted) => {
                            let cfg = AppConfig {
                                network: self.network,
                                theme: crate::models::ThemeMode::Dark,
                                wallet: Some(WalletConfig {
                                    encrypted_key: Some(encrypted),
                                    remember: true,
                                }),
                            };
                            if let Err(e) = config_service::save_config(&cfg) {
                                eprintln!("Warning: failed to save config: {}", e);
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Encryption error: {}", e));
                            return;
                        }
                    }
                }
                self.error_message = None;
                cx.emit(WelcomeEvent::ConnectWallet {
                    private_key: key_str,
                    network: self.network,
                });
            }
            Err(e) => {
                self.error_message = Some(format!("Invalid key: {}", e));
            }
        }
    }

    fn render_network_selector(&self, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(gpui::px(6.))
            .child(
                div()
                    .text_size(gpui::px(12.))
                    .text_color(text_muted())
                    .child("Network"),
            )
            .child(
                div()
                    .flex()
                    .gap(gpui::px(4.))
                    .child(
                        Button::new("mainnet")
                            .label("Mainnet")
                            .compact()
                            .map(|btn| {
                                if self.network == Network::Mainnet {
                                    btn.primary()
                                } else {
                                    btn.outline()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.network = Network::Mainnet;
                            })),
                    )
                    .child(
                        Button::new("testnet")
                            .label("Testnet")
                            .compact()
                            .map(|btn| {
                                if self.network == Network::Testnet {
                                    btn.primary()
                                } else {
                                    btn.outline()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.network = Network::Testnet;
                            })),
                    ),
            )
    }

    fn render_saved_mode(&self, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(gpui::px(16.))
            // Saved-wallet indicator
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .px(gpui::px(12.))
                            .py(gpui::px(6.))
                            .rounded(gpui::px(6.))
                            .bg(color_buy_bg())
                            .child(
                                div()
                                    .text_size(gpui::px(12.))
                                    .text_color(color_success())
                                    .child("Saved wallet found"),
                            ),
                    ),
            )
            // Password input
            .child(input_field("Password", &self.password_input))
            // Unlock button
            .child(
                Button::new("unlock")
                    .label("Unlock Wallet")
                    .primary()
                    .w_full()
                    .on_click(cx.listener(|this, _, _w, cx| {
                        this.try_unlock(cx);
                    })),
            )
            // "Use different wallet" link
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(
                        Button::new("use-different")
                            .label("Use a different wallet")
                            .ghost()
                            .compact()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.mode = WelcomeMode::Fresh;
                                this.saved_encrypted_key = None;
                                this.error_message = None;
                                // Reset password input
                                this.password_input = cx.new(|cx| {
                                    InputState::new(window, cx)
                                        .placeholder("Password (required to remember)")
                                        .masked(true)
                                });
                            })),
                    ),
            )
    }

    fn render_fresh_mode(&self, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(gpui::px(16.))
            // Private key input
            .child(input_field("Private Key", &self.key_input))
            // Remember wallet toggle
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(gpui::px(8.))
                    .child(
                        Button::new("remember-toggle")
                            .label(if self.remember { "[x] Remember wallet" } else { "[ ] Remember wallet" })
                            .compact()
                            .map(|btn| {
                                if self.remember {
                                    btn.primary()
                                } else {
                                    btn.ghost()
                                }
                            })
                            .on_click(cx.listener(|this, _, _w, _cx| {
                                this.remember = !this.remember;
                            })),
                    ),
            )
            // Password field (visible when remember is checked)
            .when(self.remember, |el| {
                el.child(input_field("Encryption Password", &self.password_input))
            })
            // Connect button
            .child(
                Button::new("connect")
                    .label("Connect Wallet")
                    .primary()
                    .w_full()
                    .on_click(cx.listener(|this, _, _w, cx| {
                        this.try_connect(cx);
                    })),
            )
    }
}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(bg_primary())
            .child(
                div()
                    .w(gpui::px(400.))
                    .p(gpui::px(36.))
                    .rounded(gpui::px(12.))
                    .bg(bg_card())
                    .border_1()
                    .border_color(border_accent())
                    .flex()
                    .flex_col()
                    .gap(gpui::px(24.))
                    // Title
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(gpui::px(8.))
                            .child(
                                div()
                                    .text_size(gpui::px(22.))
                                    .text_color(color_brand())
                                    .child("Hype Trader"),
                            )
                            .child(
                                div()
                                    .text_size(gpui::px(13.))
                                    .text_color(text_dim())
                                    .child("Hyperliquid Trading Client"),
                            ),
                    )
                    // Network selector
                    .child(self.render_network_selector(cx))
                    // Mode-specific content
                    .map(|el| {
                        match self.mode {
                            WelcomeMode::Saved => el.child(self.render_saved_mode(cx)),
                            WelcomeMode::Fresh => el.child(self.render_fresh_mode(cx)),
                        }
                    })
                    // Error message (if any)
                    .when_some(self.error_message.clone(), |el, msg| {
                        el.child(
                            div()
                                .px(gpui::px(12.))
                                .py(gpui::px(8.))
                                .rounded(gpui::px(6.))
                                .bg(color_sell_bg())
                                .child(
                                    div()
                                        .text_size(gpui::px(12.))
                                        .text_color(color_red())
                                        .child(msg),
                                ),
                        )
                    })
                    // Divider
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap(gpui::px(12.))
                            .child(div().flex_1().h(gpui::px(1.)).bg(border_primary()))
                            .child(
                                div()
                                    .text_size(gpui::px(11.))
                                    .text_color(text_dimmest())
                                    .child("or"),
                            )
                            .child(div().flex_1().h(gpui::px(1.)).bg(border_primary())),
                    )
                    // Read-only mode
                    .child(
                        Button::new("readonly")
                            .label("Browse Market (Read-only)")
                            .outline()
                            .w_full()
                            .on_click(cx.listener(|this, _, _w, cx| {
                                cx.emit(WelcomeEvent::BrowseReadOnly {
                                    network: this.network,
                                });
                            })),
                    ),
            )
    }
}
