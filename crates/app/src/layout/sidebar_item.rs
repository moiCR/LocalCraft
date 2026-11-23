use gpui::{prelude::FluentBuilder as _, *};
use gpui_kit::base::Button;
use gpui_pages::router;
use ui::{components::ActionTooltip, theme::ActiveUiTheme};

use crate::modals::SettingsModalState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NavigationItem {
    Instances,
    Runtimes,
    Playit,
    Settings,
}

impl NavigationItem {
    pub(super) const ALL: [Self; 4] = [
        Self::Instances,
        Self::Runtimes,
        Self::Playit,
        Self::Settings,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::Instances => "nav-instances",
            Self::Runtimes => "nav-runtimes",
            Self::Playit => "nav-playit",
            Self::Settings => "nav-settings",
        }
    }

    pub(super) fn label(self) -> SharedString {
        match self {
            Self::Instances => "Instances".into(),
            Self::Runtimes => "Runtimes".into(),
            Self::Playit => "Playit".into(),
            Self::Settings => "Settings".into(),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Instances => "icons/server.svg",
            Self::Runtimes => "icons/coffee.svg",
            Self::Playit => "icons/computer.svg",
            Self::Settings => "icons/settings.svg",
        }
    }

    pub(super) fn active(window: &Window, cx: &App) -> Option<Self> {
        if cx
            .try_global::<SettingsModalState>()
            .is_some_and(|state| state.open)
        {
            return Some(Self::Settings);
        }

        let path = router(window, cx).map(|manager| manager.read(cx).current_path().to_owned());
        match path.as_deref() {
            Some("/") => Some(Self::Instances),
            Some("/runtimes") => Some(Self::Runtimes),
            Some("/playit") => Some(Self::Playit),
            _ => None,
        }
    }

    fn activate(self, window: &mut Window, cx: &mut App) {
        let path = match self {
            Self::Instances => "/",
            Self::Runtimes => "/runtimes",
            Self::Playit => "/playit",
            Self::Settings => {
                cx.set_global(SettingsModalState {
                    open: true,
                    ..Default::default()
                });
                window.refresh();
                return;
            }
        };

        if let Some(manager) = router(window, cx) {
            manager.update(cx, |manager, cx| {
                if manager.current_path() != path
                    && let Err(error) = manager.navigate(path, cx)
                {
                    eprintln!("{error}");
                }
            });
        }
    }

    pub(super) fn sidebar_button(self, active: bool, cx: &App) -> Button {
        let palette = cx.ui_palette();
        let text = palette.text;
        let color = if active { text } else { palette.muted };
        let label = self.label();

        Button::new(self.id())
            .group(self.id())
            .accessibility_label(label.clone())
            .w_full()
            .justify_start()
            .gap_2p5()
            .py_2()
            .px_1()
            .cursor_pointer()
            .text_size(px(18.0))
            .line_height(relative(1.2))
            .font_weight(if active {
                FontWeight::BOLD
            } else {
                FontWeight::MEDIUM
            })
            .text_color(color)
            .hover(move |style| style.text_color(text))
            .on_click(move |_, window, cx| self.activate(window, cx))
            .child(
                svg()
                    .path(self.icon())
                    .size_4()
                    .flex_shrink_0()
                    .text_color(color)
                    .group_hover(self.id(), move |style| style.text_color(text)),
            )
            .child(div().flex_1().truncate().child(label))
    }

    pub(super) fn dock_button(
        self,
        size: Pixels,
        color: Rgba,
        hover_color: Option<Rgba>,
        label: SharedString,
    ) -> Button {
        let tooltip_label = label.clone();
        Button::new(self.id())
            .group(self.id())
            .accessibility_label(label)
            .size(size)
            .flex_shrink_0()
            .rounded_full()
            .cursor_pointer()
            .tooltip(move |_, cx| cx.new(|_| ActionTooltip::new(tooltip_label.clone())).into())
            .on_click(move |_, window, cx| self.activate(window, cx))
            .child(
                svg()
                    .path(self.icon())
                    .size(px(19.0))
                    .text_color(color)
                    .when_some(hover_color, |icon, hover_color| {
                        icon.group_hover(self.id(), move |style| style.text_color(hover_color))
                    }),
            )
    }
}
