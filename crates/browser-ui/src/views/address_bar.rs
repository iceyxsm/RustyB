//! Address bar component

use iced::{
    widget::{text_input, TextInput},
    Element, Length,
};

/// Address bar component
pub struct AddressBar<'a, Message> {
    url: &'a str,
    on_change: Option<fn(String) -> Message>,
    on_submit: Option<Message>,
}

impl<'a, Message: Clone> AddressBar<'a, Message> {
    pub fn new(url: &'a str) -> Self {
        Self {
            url,
            on_change: None,
            on_submit: None,
        }
    }

    pub fn on_change(mut self, f: fn(String) -> Message) -> Self {
        self.on_change = Some(f);
        self
    }

    pub fn on_submit(mut self, msg: Message) -> Self {
        self.on_submit = Some(msg);
        self
    }

    pub fn view(&self) -> Element<Message> {
        let input = text_input("Enter URL or search...", self.url)
            .on_input_maybe(self.on_change)
            .on_submit_maybe(self.on_submit.clone())
            .padding(10)
            .size(16)
            .width(Length::Fill);

        input.into()
    }
}
