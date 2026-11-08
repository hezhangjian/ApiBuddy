use nsh_base::{
    ButtonElement, SelectElement, SelectItem, SelectValue, button, input, select, theme,
};
use nsh_core::prelude::*;

pub const MONO: &str = if cfg!(target_arch = "wasm32") {
    "monospace"
} else if cfg!(target_os = "windows") {
    "Consolas"
} else {
    "Menlo"
};

pub fn label(value: impl IntoReactive<SharedString>) -> Text {
    text(value)
        .text_size(px(13.0))
        .text_color(|| theme().foreground)
}

pub fn muted(value: impl IntoReactive<SharedString>) -> Text {
    label(value).text_color(|| theme().muted_foreground)
}

pub fn action(
    value: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .h(px(32.0))
        .px_3()
        .rounded(px(5.0))
        .justify_center()
        .items_center()
        .bg(|| theme().secondary)
        .hover(|s| s.bg(|| theme().accent))
        .focus(|s| s.border_1().border_color(|| theme().ring))
        .on_press(on_press)
        .child(label(value))
}

pub fn primary(
    value: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .h(px(36.0))
        .px_4()
        .rounded(px(5.0))
        .justify_center()
        .items_center()
        .bg(|| theme().primary)
        .hover(|s| s.opacity(0.88))
        .focus(|s| s.border_1().border_color(|| theme().ring))
        .on_press(on_press)
        .child(
            label(value)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(|| theme().primary_foreground),
        )
}

pub fn field(
    name: &'static str,
    value: impl IntoReactive<String>,
    changed: impl Fn(&str) + 'static,
) -> Input {
    input()
        .label(name)
        .test_id(name)
        .value(value)
        .on_change(changed)
        .h(px(34.0))
        .px_2()
        .text_size(px(13.0))
        .text_color(|| theme().foreground)
        .bg(|| theme().background)
        .placeholder_color(|| color(theme().muted_foreground))
        .border_1()
        .border_color(|| theme().input)
        .rounded(px(4.0))
        .focus(|s| s.border_color(|| theme().ring))
}

pub fn editor(
    name: &'static str,
    value: impl IntoReactive<String>,
    changed: impl Fn(&str) + 'static,
) -> Input {
    field(name, value, changed)
        .multiline(true)
        .font_family(MONO)
        .h(auto())
        .flex_1()
        .min_h(px(80.0))
        .py_2()
}

pub fn choice(
    name: &'static str,
    items: impl IntoReactive<Vec<SelectItem>>,
    value: impl Fn() -> String + 'static,
    title: impl Fn() -> String + 'static,
    changed: impl Fn(String) + 'static,
) -> SelectElement {
    let items = items.into_reactive();
    select().accessibility_label(name).items(move || items.get())
        .value(move || SelectValue::Controlled(Some(value().into())))
        .on_change(move |value: &Option<SharedString>| { if let Some(value) = value { changed(value.to_string()); } })
        .popup_class("bg-popover border border-border rounded-md p-1")
        .item_class("h-8 px-3 justify-center rounded text-foreground text-sm focus:bg-accent selected:bg-accent")
        .list_class("max-h-72")
        .trigger(view().h(px(34.0)).px_3().flex_row().gap_3().items_center()
            .border_1().border_color(|| theme().border).rounded(px(4.0)).bg(|| theme().background)
            .child(label(title).flex_1().number_of_lines(1)).child(muted("⌄")))
}

pub fn tabs(labels: &'static [&'static str], selected: Signal<usize>) -> impl IntoElement {
    view()
        .flex_row()
        .gap_1()
        .border_b_1()
        .border_color(|| theme().border)
        .child(
            labels
                .iter()
                .enumerate()
                .map(move |(index, title)| {
                    button()
                        .h(px(38.0))
                        .px_3()
                        .justify_center()
                        .selected(move || selected.get() == index)
                        .role(AccessibilityRole::Tab)
                        .accessibility_label(*title)
                        .border_b(px(2.0))
                        .border_color(move || {
                            if selected.get() == index {
                                theme().primary
                            } else {
                                theme().border
                            }
                        })
                        .hover(|s| s.bg(|| theme().secondary))
                        .on_press(move || selected.set(index))
                        .child(label(*title).text_color(move || {
                            if selected.get() == index {
                                theme().foreground
                            } else {
                                theme().muted_foreground
                            }
                        }))
                })
                .collect::<Vec<_>>(),
        )
}
