//! Address bar component

use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme};
use iced::{
    widget::{container, text_input},
    Element, Length,
};

/// Address bar component
pub struct AddressBar<'a, Message> {
    url: &'a str,
    on_change: Option<fn(String) -> Message>,
    on_submit: Option<Message>,
    theme: BrowserTheme,
}

impl<'a, Message: Clone> AddressBar<'a, Message> {
    pub fn new(url: &'a str) -> Self {
        Self {
            url,
            on_change: None,
            on_submit: None,
            theme: BrowserTheme::default(),
        }
    }
    
    /// Set the theme for the address bar
    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn on_change(mut self, f: fn(String) -> Message) -> Self {
        self.on_change = Some(f);
        self
    }

    pub fn on_submit(mut self, msg: Message) -> Self {
        self.on_submit = Some(msg);
        self
    }

    pub fn view(&self) -> Element<Message> where Message: 'a {
        let theme = &self.theme;
        
        let input = text_input("Enter URL or search...", self.url)
            .on_input_maybe(self.on_change)
            .on_submit_maybe(self.on_submit.clone())
            .padding(10)
            .size(16)
            .width(Length::Fill);

        container(input)
            .style(move |_| container::Style {
                background: Some(container_background(theme, ContainerStyle::AddressBar).into()),
                ..Default::default()
            })
            .width(Length::Fill)
            .into()
    }
}
