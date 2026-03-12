use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, rgb};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
    pub visible: bool,
}

impl Toast {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            kind: ToastKind::Info,
            visible: false,
        }
    }

    pub fn show(
        &mut self,
        message: impl Into<String>,
        kind: ToastKind,
        cx: &mut gpui::Context<Self>,
    ) {
        self.message = message.into();
        self.kind = kind;
        self.visible = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let _ = this.update(cx, |t, cx| {
                t.visible = false;
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for Toast {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        if !self.visible {
            return div();
        }

        let bg_color = match self.kind {
            ToastKind::Success => rgb(0x1b5e20), // green
            ToastKind::Error => rgb(0xb71c1c),   // red
            ToastKind::Info => rgb(0x0d47a1),    // blue
        };

        let text_color = rgb(0xffffff);

        div()
            .w_full()
            .px(px(16.))
            .py(px(8.))
            .bg(bg_color)
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(px(13.))
                    .text_color(text_color)
                    .child(self.message.clone()),
            )
    }
}
