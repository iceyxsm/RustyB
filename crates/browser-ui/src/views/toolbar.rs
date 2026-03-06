//! Navigation toolbar

use iced::{
    widget::{button, row},
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
        }
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
        let back_button = button("←")
            .on_press_maybe(self.on_back.clone())
            .style(if self.can_go_back {
                button::primary
            } else {
                button::secondary
            });

        let forward_button = button("→")
            .on_press_maybe(self.on_forward.clone())
            .style(if self.can_go_forward {
                button::primary
            } else {
                button::secondary
            });

        let reload_button = button(if self.is_loading { "✕" } else { "⟳" })
            .on_press_maybe(if self.is_loading {
                self.on_stop.clone()
            } else {
                self.on_reload.clone()
            });

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
