use gpui::{Context, IntoElement, Window, div, prelude::*};
use gpui_pages::{Page, PageAnimation, PageOptions};

use crate::views::AppViews;

pub struct InstancesPage;

impl Page for InstancesPage {
    fn new(cx: &mut Context<Self>) -> Self {
        AppViews::init(cx);
        Self
    }

    fn options() -> PageOptions {
        PageOptions::default().on_open(PageAnimation::SlideUpFade)
    }

    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let servers = cx.global::<AppViews>().servers.clone();
        div()
            .size_full()
            .min_w_0()
            .min_h_0()
            .flex_1()
            .child(servers)
    }
}
