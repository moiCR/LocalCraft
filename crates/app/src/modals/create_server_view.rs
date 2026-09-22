use super::create_server::CreateServerModal;
use gpui::{Context, IntoElement, MouseButton, Render, Window, div, prelude::*, px};
use services::AppState;
use ui::components::{button::button, input::Input, select};

pub fn input_field(label: &'static str, input: &gpui::Entity<Input>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .gap_2()
        .child(label)
        .child(input.clone())
}

impl Render for CreateServerModal {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = cx.global::<AppState>().theme_manager.palette();
        div().id("create-server-form").size_full().flex().flex_col().track_focus(&self.focus)
            .bg(palette.background).text_color(palette.text).text_sm()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" { this.close(cx); cx.stop_propagation(); }
            }))
            .child(div().flex().items_center().justify_between().p_6().border_b_1().border_color(palette.border)
                .child(div().flex().flex_col().gap_1().child(div().text_lg().child("Create server"))
                    .child(div().text_color(palette.muted).child("Your next world starts here.")))
                .child(button("dismiss-create", "×", palette, false, !self.busy)
                    .on_click(cx.listener(|this, _, _, cx| this.close(cx)))))
            .child(div().id("create-server-fields").flex_1().min_h_0().overflow_y_scroll().p_6().flex().flex_col().gap_4()
                .child(input_field("Name", &self.name))
                .child(div().flex().gap_4().child(select::field("Software", &self.software)).child(select::field("Minecraft version", &self.version)))
                .child(div().flex().gap_4().child(select::field("Build / loader", &self.build)).child(select::field("Java version", &self.java)))
                .when(self.loading, |el| el.child(div().text_color(palette.muted).child("Fetching available releases…")))
                .child(div().flex().gap_4().child(input_field("Memory (MiB)", &self.ram)).child(input_field("Port", &self.port)))
                .child(div().text_xs().text_color(palette.muted).child("Java is installed automatically when needed. Servers are created stopped."))
                .when(self.needs_checksum(cx), |el| el.child(div().flex().flex_col().gap_2()
                    .child(input_field("Expected SHA256", &self.checksum))
                    .child(div().text_xs().text_color(palette.muted).child("This provider does not publish SHA256. Enter the trusted checksum for the selected build."))))
                .child(div().flex().items_center().gap_2()
                    .child(div().id("accept-eula").cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                        if !this.busy { this.accepted_eula = !this.accepted_eula; cx.notify(); }
                    })).child(if self.accepted_eula { "☑" } else { "☐" }))
                    .child("I agree to the Minecraft")
                    .child(div().id("minecraft-eula").cursor_pointer().underline().child("EULA")
                        .on_click(|_, _, cx| cx.open_url("https://www.minecraft.net/eula"))))
                .children(self.error.as_ref().map(|error| div().text_color(gpui::rgb(0xe87878)).child(error.clone())))
                .when(self.error.is_some() && !self.busy, |el| el.child(button("retry-catalog", "Reload versions", palette, false, true)
                    .on_click(cx.listener(|this, _, _, cx| this.fetch_versions(cx)))))
                .children(self.status.as_ref().map(|status| div().text_color(palette.muted).child(status.clone()))))
            .child(div().flex().justify_end().gap_2().p_4().border_t_1().border_color(palette.border)
                .child(button("cancel-create", "Cancel", palette, false, !self.busy).on_click(cx.listener(|this, _, _, cx| this.close(cx))))
                .child(button("confirm-create", if self.busy { "Creating…" } else { "Create server" }, palette, true, !self.busy && !self.loading)
                    .on_click(cx.listener(|this, _, _, cx| this.create(cx)))))
            .min_h(px(0.))
    }
}
