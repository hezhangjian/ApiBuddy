use nsh_base::{
    ButtonElement, SelectElement, SelectItem, SelectValue, button, input, select, theme,
};
use nsh_core::{
    canvas::{LineCap, LineJoin},
    prelude::*,
};

pub const MONO: &str = if cfg!(target_os = "windows") {
    "Consolas"
} else {
    "Menlo"
};

pub fn tone(name: &str) -> AdaptiveColor {
    theme().color(name).unwrap_or(theme().foreground)
}

pub fn method_color(method: &str) -> AdaptiveColor {
    tone(match method {
        "GET" | "HEAD" => "success",
        "POST" => "warning",
        "DELETE" => "destructive",
        "PATCH" => "purple",
        _ => "primary",
    })
}

pub fn label(value: impl IntoReactive<SharedString>) -> Text {
    text(value)
        .text_size(px(13.0))
        .text_color(|| theme().foreground)
}

pub fn muted(value: impl IntoReactive<SharedString>) -> Text {
    label(value).text_color(|| theme().muted_foreground)
}

pub fn caption(value: impl IntoReactive<SharedString>) -> Text {
    muted(value)
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
}

pub fn action(
    value: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .h(px(30.0))
        .px_3()
        .rounded(px(7.0))
        .justify_center()
        .items_center()
        .border_1()
        .border_color(|| theme().border)
        .bg(|| theme().background)
        .hover(|s| s.bg(|| theme().secondary).border_color(|| theme().input))
        .focus(|s| s.border_color(|| theme().ring))
        .on_press(on_press)
        .child(label(value).text_size(px(12.0)))
}

pub fn ghost(
    value: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .h(px(30.0))
        .px_3()
        .rounded(px(7.0))
        .justify_center()
        .items_center()
        .border_1()
        .border_color(rgba(0x00000000))
        .hover(|s| s.bg(|| theme().secondary))
        .focus(|s| s.border_color(|| theme().ring))
        .on_press(on_press)
        .child(label(value).text_size(px(12.0)))
}

pub fn primary(
    value: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .h(px(38.0))
        .px_4()
        .rounded(px(8.0))
        .justify_center()
        .items_center()
        .bg(|| theme().primary)
        .hover(|s| s.opacity(0.9))
        .border_1()
        .border_color(|| theme().primary)
        .focus(|s| s.border_color(|| theme().foreground))
        .on_press(on_press)
        .child(
            label(value)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(|| theme().primary_foreground),
        )
}

#[derive(Clone, Copy)]
pub enum Glyph {
    Code,
    Folder,
    Import,
    Sliders,
    Sidebar,
    Plus,
    Copy,
    Search,
    Refresh,
    Arrow,
}

pub fn icon(glyph: Glyph) -> impl IntoElement {
    // Small native vector glyphs share a 20-point grid and adapt with the theme.
    view()
        .size(px(16.0))
        .flex_shrink(0.0)
        .pointer_events(PointerEvents::None)
        .child(
            canvas(move || {
                let p = Painter::new()
                    .scale(0.8, 0.8)
                    .stroke_color(color(theme().muted_foreground))
                    .line_width(1.5)
                    .line_cap(LineCap::Round)
                    .line_join(LineJoin::Round);
                let line = |p: Painter, points: &[f32]| {
                    let mut p = p.begin_path().move_to(points[0], points[1]);
                    for xy in points[2..].chunks_exact(2) {
                        p = p.line_to(xy[0], xy[1]);
                    }
                    p.stroke()
                };
                match glyph {
                    Glyph::Code => {
                        let p = line(p, &[6., 5., 2., 10., 6., 15.]);
                        let p = line(p, &[14., 5., 18., 10., 14., 15.]);
                        line(p, &[11.5, 3., 8.5, 17.])
                    }
                    Glyph::Folder => line(
                        p,
                        &[
                            2., 6., 2., 4., 8., 4., 10., 6., 18., 6., 18., 16., 2., 16., 2., 6.,
                        ],
                    ),
                    Glyph::Import => {
                        let p = line(p, &[3., 12., 3., 17., 17., 17., 17., 12.]);
                        let p = line(p, &[10., 2., 10., 12.]);
                        line(p, &[6., 8., 10., 12., 14., 8.])
                    }
                    Glyph::Sliders => {
                        let p = line(p, &[3., 6., 17., 6.]);
                        let p = line(p, &[3., 14., 17., 14.]);
                        let p = line(p, &[7., 3., 7., 9.]);
                        line(p, &[13., 11., 13., 17.])
                    }
                    Glyph::Sidebar => {
                        let p = line(p, &[3., 3., 17., 3., 17., 17., 3., 17., 3., 3.]);
                        line(p, &[8., 3., 8., 17.])
                    }
                    Glyph::Plus => {
                        let p = line(p, &[4., 10., 16., 10.]);
                        line(p, &[10., 4., 10., 16.])
                    }
                    Glyph::Copy => {
                        let p = line(p, &[7., 7., 17., 7., 17., 17., 7., 17., 7., 7.]);
                        line(p, &[3., 13., 3., 3., 13., 3.])
                    }
                    Glyph::Search => {
                        let p = p
                            .begin_path()
                            .arc(8.5, 8.5, 5.5, 0., std::f32::consts::TAU)
                            .stroke();
                        line(p, &[12.5, 12.5, 17., 17.])
                    }
                    Glyph::Refresh => {
                        let p = line(p, &[16., 7., 16., 3., 12., 3.]);
                        let p = line(p, &[4., 13., 4., 17., 8., 17.]);
                        let p = line(p, &[16., 3., 7., 3., 3., 7.]);
                        line(p, &[4., 17., 13., 17., 17., 13.])
                    }
                    Glyph::Arrow => {
                        let p = line(p, &[3., 10., 17., 10.]);
                        line(p, &[12., 5., 17., 10., 12., 15.])
                    }
                }
            })
            .size_full(),
        )
}

pub fn icon_action(
    glyph: Glyph,
    title: &'static str,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .h(px(30.0))
        .px_2()
        .rounded(px(7.0))
        .flex_row()
        .gap_2()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(rgba(0x00000000))
        .hover(|s| s.bg(|| theme().secondary))
        .focus(|s| s.border_color(|| theme().ring))
        .accessibility_label(title)
        .on_press(on_press)
        .child(icon(glyph))
        .child(label(title).text_size(px(12.0)))
}

pub fn icon_button(
    glyph: Glyph,
    title: &'static str,
    on_press: impl Fn() + 'static,
) -> ButtonElement {
    button()
        .size(px(28.0))
        .rounded(px(7.0))
        .items_center()
        .justify_center()
        .border_1()
        .border_color(rgba(0x00000000))
        .hover(|s| s.bg(|| theme().accent))
        .focus(|s| s.border_color(|| theme().ring))
        .accessibility_label(title)
        .on_press(on_press)
        .child(icon(glyph))
}

pub fn empty_state(glyph: Glyph, title: &'static str, detail: &'static str) -> impl IntoElement {
    view()
        .flex_1()
        .min_h(px(0.0))
        .items_center()
        .justify_center()
        .gap_2()
        .p_4()
        .child(
            view()
                .size(px(40.0))
                .rounded(px(12.0))
                .bg(|| theme().secondary)
                .items_center()
                .justify_center()
                .mb_2()
                .child(icon(glyph)),
        )
        .child(label(title).font_weight(FontWeight::MEDIUM))
        .child(muted(detail).text_size(px(12.0)).number_of_lines(2))
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
        .h(px(32.0))
        .px_3()
        .text_size(px(13.0))
        .text_color(|| theme().foreground)
        .bg(|| theme().background)
        .placeholder_color(|| color(theme().muted_foreground))
        .border_1()
        .border_color(|| theme().input)
        .rounded(px(7.0))
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
        .popup_class("bg-popover border border-border rounded-lg p-1 shadow-lg")
        .item_class("h-8 px-3 justify-center rounded-md text-foreground text-sm focus:bg-accent selected:bg-accent")
        .list_class("max-h-72")
        .trigger(view().h(px(32.0)).px_3().flex_row().gap_2().items_center()
            .border_1().border_color(|| theme().input).rounded(px(7.0)).bg(|| theme().background)
            .child(label(title).flex_1().number_of_lines(1)).child(caption("⌄")))
}

pub fn tabs(labels: &'static [&'static str], selected: Signal<usize>) -> impl IntoElement {
    view().flex_row().gap_1().child(
        labels
            .iter()
            .enumerate()
            .map(move |(index, title)| {
                button()
                    .h(px(35.0))
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
                            AdaptiveColor::TRANSPARENT
                        }
                    })
                    .hover(|s| s.bg(|| theme().secondary))
                    .on_press(move || selected.set(index))
                    .child(
                        label(*title)
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(move || {
                                if selected.get() == index {
                                    theme().primary
                                } else {
                                    theme().muted_foreground
                                }
                            }),
                    )
            })
            .collect::<Vec<_>>(),
    )
}

pub fn segments(labels: &'static [&'static str], selected: Signal<usize>) -> impl IntoElement {
    view()
        .flex_row()
        .p(px(3.0))
        .gap_1()
        .rounded(px(8.0))
        .bg(|| theme().muted)
        .child(
            labels
                .iter()
                .enumerate()
                .map(move |(index, title)| {
                    button()
                        .h(px(27.0))
                        .flex_1()
                        .rounded(px(6.0))
                        .justify_center()
                        .items_center()
                        .selected(move || selected.get() == index)
                        .role(AccessibilityRole::Tab)
                        .accessibility_label(*title)
                        .bg(move || {
                            if selected.get() == index {
                                theme().background
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            if selected.get() == index {
                                theme().border
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        })
                        .on_press(move || selected.set(index))
                        .child(
                            label(*title)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(move || {
                                    if selected.get() == index {
                                        theme().foreground
                                    } else {
                                        theme().muted_foreground
                                    }
                                }),
                        )
                })
                .collect::<Vec<_>>(),
        )
}
