use gpui::prelude::*;
use gpui::{ElementId, SharedString};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Sizable as _;

/// A button that shows primary style when active, secondary (subtle bg) when inactive.
pub fn toggle_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    active: bool,
) -> Button {
    Button::new(id)
        .label(label)
        .small()
        .map(|b| if active { b.primary() } else { b })
}
