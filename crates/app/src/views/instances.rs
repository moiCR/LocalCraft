use super::servers::Servers;
use gpui::{Div, Entity, div, prelude::*};

pub fn render(servers: &Entity<Servers>) -> Div {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex_1()
        .child(servers.clone())
}
