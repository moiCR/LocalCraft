use gpui::{
    Animation, AnimationExt, App, ClickEvent, IntoElement, Window, div, ease_out_quint, prelude::*,
    px, svg,
};
use services::java::JavaInstallation;
use std::time::Duration;
use ui::theme::Palette;

pub struct RuntimeItem {
    installation: JavaInstallation,
}

impl RuntimeItem {
    pub fn new(installation: JavaInstallation) -> Self {
        Self { installation }
    }

    pub fn render(
        self,
        palette: &Palette,
        on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
        on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> impl IntoElement {
        let version = self.installation.major_version;
        let binary = self.installation.binary_path().display().to_string();
        div()
            .id(("runtime-item", version as u32))
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .w_full()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(palette.border)
            .bg(palette.surface)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .flex_basis(px(360.))
                    .min_w_0()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(format!("Java {version}")),
                    )
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(palette.muted)
                            .child(binary),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .child(action_button(
                        ("runtime-folder", version as u32),
                        "folder-open",
                        palette,
                        on_open,
                    ))
                    .child(action_button(
                        ("runtime-delete", version as u32),
                        "trash",
                        palette,
                        on_delete,
                    )),
            )
            .with_animation(
                ("runtime-item-enter", version as u32),
                Animation::new(Duration::from_millis(260)).with_easing(ease_out_quint()),
                |item, progress| {
                    item.relative()
                        .top(px(8.0 * (1.0 - progress)))
                        .opacity(progress)
                },
            )
    }
}

fn action_button(
    id: (&'static str, u32),
    icon: &'static str,
    palette: &Palette,
    action: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size_8()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(palette.background))
        .on_click(action)
        .child(
            svg()
                .path(format!("icons/{icon}.svg"))
                .size_4()
                .text_color(palette.muted),
        )
}
