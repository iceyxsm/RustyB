//! Navigation toolbar

use crate::theme::{button_style, text_color, ButtonStyle, TextStyle, BrowserTheme};
use iced::{
    widget::{button, row, text},
    Element,
};

/// Navigation toolbar component
pub struct Toolbar<Message> {
    on_back: Option<Message>,
    on_forward: Option<Message>,
    on_reload: Option<Message>,
    on_stop: Option<Message>,
    can_go_back: bool,
    can_go_forward: bool,
    is_loading: bool,
    theme: BrowserTheme,
}

impl<Message: Clone> Toolbar<Message> {
    pub fn new() -> Self {
        Self {
            on_back: None,
            on_forward: None,
            on_reload: None,
            on_stop: None,
            can_go_back: false,
            can_go_forward: false,
            is_loading: false,
            theme: BrowserTheme::default(),
        }
    }
    
    /// Set the theme for the toolbar
    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn on_back(mut self, msg: Message) -> Self {
        self.on_back = Some(msg);
        self
    }

    pub fn on_forward(mut self, msg: Message) -> Self {
        self.on_forward = Some(msg);
        self
    }

    pub fn on_reload(mut self, msg: Message) -> Self {
        self.on_reload = Some(msg);
        self
    }

    pub fn on_stop(mut self, msg: Message) -> Self {
        self.on_stop = Some(msg);
        self
    }

    pub fn can_go_back(mut self, can: bool) -> Self {
        self.can_go_back = can;
        self
    }

    pub fn can_go_forward(mut self, can: bool) -> Self {
        self.can_go_forward = can;
        self
    }

    pub fn is_loading(mut self, loading: bool) -> Self {
        self.is_loading = loading;
        self
    }

    pub fn view<'a>(&self) -> Element<'a, Message> where Message: 'a {
        let theme = &self.theme;
        
        let back_button = button(
            text("←")
                .color(if self.can_go_back {
                    text_color(theme, TextStyle::Toolbar)
                } else {
                    text_color(theme, TextStyle::Disabled)
                })
        )
        .on_press_maybe(self.on_back.clone())
        .style(move |_, _| button_style(theme, ButtonStyle::Navigation));

        let forward_button = button(
            text("→")
                .color(if self.can_go_forward {
                    text_color(theme, TextStyle::Toolbar)
                } else {
                    text_color(theme, TextStyle::Disabled)
                })
        )
        .on_press_maybe(self.on_forward.clone())
        .style(move |_, _| button_style(theme, ButtonStyle::Navigation));

        let reload_button = button(
            text(if self.is_loading { "✕" } else { "⟳" })
                .color(text_color(theme, TextStyle::Toolbar))
        )
        .on_press_maybe(if self.is_loading {
            self.on_stop.clone()
        } else {
            self.on_reload.clone()
        })
        .style(move |_, _| button_style(theme, ButtonStyle::Toolbar));

        row![back_button, forward_button, reload_button]
            .spacing(8)
            .padding(8)
            .into()
    }
}

impl<Message: Clone> Default for Toolbar<Message> {
    fn default() -> Self {
        Self::new()
    }
}
