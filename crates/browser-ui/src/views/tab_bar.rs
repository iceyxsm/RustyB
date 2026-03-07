//! Tab bar component

use crate::theme::{button_style, container_background, text_color, ButtonStyle, ContainerStyle, TextStyle, BrowserTheme};
use iced::{
    widget::{button, container, row, scrollable, text},
    Element, Length,
};

/// Tab bar component
pub struct TabBar<Message> {
    on_new_tab: Option<Message>,
    on_close_tab: Option<fn(uuid::Uuid) -> Message>,
    on_switch_tab: Option<fn(uuid::Uuid) -> Message>,
    theme: BrowserTheme,
}

impl<Message: Clone> TabBar<Message> {
    pub fn new() -> Self {
        Self {
            on_new_tab: None,
            on_close_tab: None,
            on_switch_tab: None,
            theme: BrowserTheme::default(),
        }
    }
    
    /// Set the theme for the tab bar
    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
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
        let theme = &self.theme;
        
        let new_tab_button = button(
            text("+").color(text_color(theme, TextStyle::Secondary))
        )
        .on_press_maybe(self.on_new_tab.clone())
        .style(move |_, _| button_style(theme, ButtonStyle::Toolbar));

        // Placeholder tabs
        let tabs = row![
            self.tab_button(uuid::Uuid::new_v4(), "Tab 1", true),
            self.tab_button(uuid::Uuid::new_v4(), "Tab 2", false),
        ]
        .spacing(4);

        container(
            row![
                scrollable(tabs).width(Length::Fill),
                new_tab_button,
            ]
            .spacing(8)
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::TabBar).into()),
            ..Default::default()
        })
        .padding(8)
        .width(Length::Fill)
        .into()
    }

    fn tab_button<'a>(&self, id: uuid::Uuid, title: &'a str, active: bool) -> Element<'a, Message> where Message: 'a {
        let theme = &self.theme;
        
        let close_button = button(
            text("×").color(text_color(theme, TextStyle::Secondary))
        )
        .on_press_maybe(self.on_close_tab.map(|f| f(id)))
        .style(move |_, _| button_style(theme, ButtonStyle::Danger));

        let tab_style = if active { ButtonStyle::TabActive } else { ButtonStyle::TabInactive };
        let text_style = if active { TextStyle::Primary } else { TextStyle::Secondary };

        let tab_content = row![
            text(title).size(14).color(text_color(theme, text_style)),
            close_button,
        ]
        .spacing(8);

        button(tab_content)
            .on_press_maybe(self.on_switch_tab.map(|f| f(id)))
            .style(move |_, _| button_style(theme, tab_style))
            .into()
    }
}

impl<Message: Clone> Default for TabBar<Message> {
    fn default() -> Self {
        Self::new()
    }
}
