use gpui::*;

pub fn card_surface(surface: Rgba, raised: Rgba, border: Rgba) -> Div {
    div()
        .rounded(px(16.0))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(raised, 0.0),
            linear_color_stop(surface, 1.0),
        ))
        .border_1()
        .border_color(border)
        .shadow_md()
}
