//! Tab bar component

use iced::{
    widget::{button, row, scrollable, text},
    Element, Length,
};

/// Tab bar component
pub struct TabBar<Message> {
    on_new_tab: Option<Message>,
    on_close_tab: Option<fn(uuid::Uuid) -> Message>,
    on_switch_tab: Option<fn(uuid::Uuid) -> Message>,
}

impl<Message: Clone> TabBar<Message> {
    pub fn new() -> Self {
        Self {
            on_new_tab: None,
            on_close_tab: None,
            on_switch_tab: None,
        }
    }

    pub fn on_new_tab(mut self, msg: Message) -> Self {
        self.on_new_tab = Some(msg);
        self
    }

    pub fn on_close_tab(mut self, f: fn(uuid::Uuid) -> Message) -> Self {
        self.on_close_tab = Some(f);
        self
    }

    pub fn on_switch_tab(mut self, f: fn(uuid::Uuid) -> Message) -> Self {
        self.on_switch_tab = Some(f);
        self
    }

    pub fn view<'a>(&self) -> Element<'a, Message> where Message: 'a {
        let new_tab_button = button("+").on_press_maybe(self.on_new_tab.clone());

        // Placeholder tabs
        let tabs = row![
            self.tab_button(uuid::Uuid::new_v4(), "Tab 1", true),
            self.tab_button(uuid::Uuid::new_v4(), "Tab 2", false),
        ]
        .spacing(4);

        row![
            scrollable(tabs).width(Length::Fill),
            new_tab_button,
        ]
        .spacing(8)
        .padding(8)
        .into()
    }

    fn tab_button<'a>(&self, id: uuid::Uuid, title: &'a str, active: bool) -> Element<'a, Message> where Message: 'a {
        let close_button = button("×")
            .on_press_maybe(self.on_close_tab.map(|f| f(id)))
            .style(iced::widget::button::danger);

        let tab_content = row![
            text(title).size(14),
            close_button,
        ]
        .spacing(8);

        button(tab_content)
            .on_press_maybe(self.on_switch_tab.map(|f| f(id)))
            .style(if active {
                button::primary
            } else {
                button::secondary
            })
            .into()
    }
}

impl<Message: Clone> Default for TabBar<Message> {
    fn default() -> Self {
        Self::new()
    }
}
