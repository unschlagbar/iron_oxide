#![allow(unused)]

use crate::{
    graphics::formats::RGBA,
    ui::{Absolute, Button, UiUnit},
};

// --- Hilfs-Types + Style ---
#[derive(Default, Clone)]
pub struct UiStyle {
    pub color: Option<RGBA>,
    pub border_color: Option<RGBA>,
    pub border: Option<[f32; 4]>,
    pub corner: Option<[UiUnit; 4]>,
    pub width: Option<UiUnit>,
    pub height: Option<UiUnit>,
}

// Helper: check if a UiUnit is the "default" (Px(0.0) or Relative(0.0) etc.)
fn is_unit_default(u: &UiUnit) -> bool {
    match u {
        UiUnit::Px(x) => *x == 0.0,
        UiUnit::Relative(r) => *r == 0.0,
        UiUnit::Fill => false,
        UiUnit::Auto => true, // treat Auto as default
        _ => false,
    }
}

// Style-Apply functions (erweitere nach Bedarf)
fn apply_style_absolute(mut a: Absolute, style: &UiStyle) -> Absolute {
    if a.color == RGBA::ZERO {
        if let Some(c) = style.color {
            a.color = c;
        }
    }
    if a.border_color == RGBA::ZERO {
        if let Some(c) = style.border_color {
            a.border_color = c;
        }
    }
    if a.border == [0.0; 4] {
        if let Some(b) = style.border {
            a.border = b;
        }
    }
    let default_corner = [UiUnit::Px(0.0); 4];
    if a.corner == default_corner {
        if let Some(c) = style.corner {
            a.corner = c;
        }
    }
    if is_unit_default(&a.width) {
        if let Some(w) = style.width {
            a.width = w;
        }
    }
    if is_unit_default(&a.height) {
        if let Some(h) = style.height {
            a.height = h;
        }
    }
    a
}

fn apply_style_button(mut b: Button, style: &UiStyle) -> Button {
    if b.color == RGBA::ZERO {
        if let Some(c) = style.color {
            b.color = c;
        }
    }
    if b.border_color == RGBA::ZERO {
        if let Some(c) = style.border_color {
            b.border_color = c;
        }
    }
    if b.border == [0.0; 4] {
        if let Some(br) = style.border {
            b.border = br;
        }
    }
    let default_corner = [UiUnit::Px(0.0); 4];
    if b.corner == default_corner {
        if let Some(c) = style.corner {
            b.corner = c;
        }
    }
    if is_unit_default(&b.width) {
        if let Some(w) = style.width {
            b.width = w;
        }
    }
    if is_unit_default(&b.height) {
        if let Some(h) = style.height {
            b.height = h;
        }
    }
    b
}

#[macro_export]
// --- Macros ---
// Note: die Macros erzeugen direkt die gewrappten Kinder (sie rufen `.wrap(&ui)` auf).
macro_rules! button {
    // Variante: button!(ui, style_opt, { field: expr, ... , text: "Label" })
    ($ui:expr, $style:expr, { $($field:ident : $val:expr),* $(,)? }) => {{
        // build button with fields provided
        let mut b = Button {
            $( $field: $val, )*
            ..Default::default()
        };
        // optional style apply
        if let Some(s) = $style {
            b = apply_style_button(b, s);
        }
        // if user provided `text` as a field, they may have already put a Text child,
        // but provide helper: if no childs, leave empty.
        b.wrap(&$ui)
    }};

    // Variante ohne style-Argument: button!(ui, { ... })
    ($ui:expr, { $($field:ident : $val:expr),* $(,)? }) => {
        button!($ui, None::<&UiStyle>, { $($field : $val),* })
    };

    // Kurzform: button_with_text!(ui, style_opt, "Label")
    ($ui:expr, $style:expr, $text:expr) => {{
        let mut b = Button { ..Default::default() };
        // create Text child and wrap it
        let txt = Text { text: $text.to_string(), ..Default::default() }.wrap(&$ui);
        b.childs = vec![ txt ];
        if let Some(s) = $style {
            b = apply_style_button(b, s);
        }
        b.wrap(&$ui)
    }};

    // Kurzform ohne style
    ($ui:expr, $text:expr) => {
        button!($ui, None::<&UiStyle>, $text)
    };
}

#[macro_export]
macro_rules! absolute_layout {
    ($ui:expr, $style:expr, { $($field:ident : $val:expr),* , childs : [$($child:expr),* $(,)?] $(,)? }) => {{
        // Build element
        let mut el = AbsoluteLayout {
            $( $field : $val, )*
            ..Default::default()
        };
        // attach childs (they are expected to already be wrapped by child macros or calls)
        el.childs = vec![ $($child),* ];
        if let Some(s) = $style {
            el = apply_style_absolute(el, s);
        }
        el
    }};

    // ohne style param
    ($ui:expr, { $($field:ident : $val:expr),* , childs : [$($child:expr),* $(,)?] $(,)? }) => {
        absolute_layout!($ui, None::<&UiStyle>, { $($field : $val),* , childs: [$($child),*] })
    };
}

#[macro_export]
macro_rules! style {
    (
        $( $field:ident : $val:expr ),* $(,)?
    ) => {{
        UiStyle {
            $(
                $field: Some($val),
            )*
            ..Default::default()
        }
    }};
}
