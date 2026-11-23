use gpui::{App, ClickEvent, Div, FontWeight, SharedString, Window, div, prelude::*, px, svg};
use services::java::JavaInstallation;
use ui::{
    components::{IconButton, card_surface},
    theme::Palette,
};

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
    ) -> Div {
        let version = self.installation.major_version;
        let path_label: SharedString = self
            .installation
            .directory()
            .unwrap_or(self.installation.binary_path().as_path())
            .display()
            .to_string()
            .into();
        let can_open = self.installation.directory().is_some();

        card_surface(palette.surface, palette.surface_hover, palette.border)
            .w_full()
            .px_4()
            .py_3()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .size(px(36.0))
                            .rounded(px(16.0))
                            .bg(palette.card_surface())
                            .border_1()
                            .border_color(palette.border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .child(
                                svg()
                                    .path("icons/coffee.svg")
                                    .size(px(16.0))
                                    .text_color(palette.text),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(palette.text)
                                            .child(format!("Java {version}")),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(palette.muted)
                                            .child(self.installation.vendor),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(palette.muted)
                                    .truncate()
                                    .child(path_label),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_shrink_0()
                    .child(
                        IconButton::new(
                            ("open-runtime", u32::from(version)),
                            "icons/folder-open.svg",
                        )
                        .tooltip("Open folder")
                        .disabled(!can_open)
                        .on_click(on_open),
                    )
                    .child(
                        IconButton::new(("delete-runtime", u32::from(version)), "icons/trash.svg")
                            .tooltip("Delete runtime")
                            .color(palette.error)
                            .on_click(on_delete),
                    ),
            )
    }
}
