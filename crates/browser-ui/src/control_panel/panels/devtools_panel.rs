//! Developer Tools panel - Inspector, Console, Network

use iced::{
    widget::{button, column, container, row, text, text_input, scrollable},
    Element, Length,
};
use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme};

/// Messages for the devtools panel
#[derive(Debug, Clone)]
pub enum DevToolsMessage {
    TabSelected(DevToolsTab),
    ConsoleInputChanged(String),
    ExecuteConsole,
    ClearConsole,
    InspectElement,
    ViewSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevToolsTab {
    Console,
    Network,
    Elements,
    Storage,
}

impl DevToolsTab {
    pub fn name(&self) -> &'static str {
        match self {
            DevToolsTab::Console => "Console",
            DevToolsTab::Network => "Network",
            DevToolsTab::Elements => "Elements",
            DevToolsTab::Storage => "Storage",
        }
    }
}

/// DevTools panel
pub struct DevToolsPanel {
    active_tab: DevToolsTab,
    console_input: String,
    console_output: Vec<String>,
    theme: BrowserTheme,
}

impl DevToolsPanel {
    pub fn new() -> Self {
        Self {
            active_tab: DevToolsTab::Console,
            console_input: String::new(),
            console_output: vec![
                "Rusty Browser DevTools initialized".to_string(),
                "Waiting for page load...".to_string(),
            ],
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn title(&self) -> &'static str {
        "Developer Tools"
    }

    pub fn update(&mut self, message: DevToolsMessage) {
        match message {
            DevToolsMessage::TabSelected(tab) => {
                self.active_tab = tab;
            }
            DevToolsMessage::ConsoleInputChanged(text) => {
                self.console_input = text;
            }
            DevToolsMessage::ExecuteConsole => {
                if !self.console_input.is_empty() {
                    self.console_output.push(format!("> {}", self.console_input));
                    self.console_output.push("< undefined".to_string());
                    self.console_input.clear();
                }
            }
            DevToolsMessage::ClearConsole => {
                self.console_output.clear();
            }
            DevToolsMessage::InspectElement => {}
            DevToolsMessage::ViewSource => {}
        }
    }

    pub fn view(&self) -> Element<'_, DevToolsMessage> {
        let theme = &self.theme;

        let title = text(self.title())
            .size(18)
            .color(text_color(theme, TextStyle::Primary));

        // Tab selector
        let tabs = [DevToolsTab::Console, DevToolsTab::Network, DevToolsTab::Elements, DevToolsTab::Storage];
        let mut tab_row = row![]
            .spacing(4);

        for tab in tabs {
            let is_active = tab == self.active_tab;
            tab_row = tab_row.push(
                button(text(tab.name()).size(11))
                    .on_press(DevToolsMessage::TabSelected(tab))
                    .style(move |_, _| {
                        let style = if is_active {
                            button_style_active(theme)
                        } else {
                            button_style_inactive(theme)
                        };
                        style
                    }),
            );
        }

        // Tab content
        let content = match self.active_tab {
            DevToolsTab::Console => self.view_console(),
            DevToolsTab::Network => self.view_network(),
            DevToolsTab::Elements => self.view_elements(),
            DevToolsTab::Storage => self.view_storage(),
        };

        let main_content = column![
            title,
            tab_row,
            content,
        ]
        .spacing(12)
        .padding(12);

        container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_console(&self) -> Element<'_, DevToolsMessage> {
        let theme = &self.theme;

        let mut output = column![]
            .spacing(2);

        for line in &self.console_output {
            output = output.push(
                text(line).size(11).color(text_color(theme, TextStyle::Secondary))
            );
        }

        let output_area = scrollable(
            container(output)
                .style(move |_| container::Style {
                    background: Some(container_background(theme, ContainerStyle::Content).into()),
                    ..Default::default()
                })
                .padding(8)
                .width(Length::Fill)
                .height(Length::FillPortion(4))
        );

        let input_row = row![
            text_input("> ", &self.console_input)
                .on_input(DevToolsMessage::ConsoleInputChanged)
                .on_submit(DevToolsMessage::ExecuteConsole)
                .width(Length::Fill),
            button("Clear").on_press(DevToolsMessage::ClearConsole),
        ]
        .spacing(8);

        column![output_area, input_row]
            .spacing(8)
            .into()
    }

    fn view_network(&self) -> Element<'_, DevToolsMessage> {
        let theme = &self.theme;

        container(
            column![
                row![
                    text("Method").size(11).width(60),
                    text("Status").size(11).width(50),
                    text("URL").size(11).width(Length::Fill),
                    text("Time").size(11).width(50),
                ]
                .spacing(4),
                text("Network log will appear here...")
                    .size(12)
                    .color(text_color(theme, TextStyle::Secondary)),
            ]
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::Card).into()),
            ..Default::default()
        })
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_elements(&self) -> Element<'_, DevToolsMessage> {
        let theme = &self.theme;

        container(
            column![
                button("🔍 Inspect Element").on_press(DevToolsMessage::InspectElement),
                text("DOM tree will appear here...")
                    .size(12)
                    .color(text_color(theme, TextStyle::Secondary)),
            ]
            .spacing(8)
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::Card).into()),
            ..Default::default()
        })
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_storage(&self) -> Element<'_, DevToolsMessage> {
        let theme = &self.theme;

        container(
            column![
                row![
                    button("Cookies"),
                    button("LocalStorage"),
                    button("SessionStorage"),
                ]
                .spacing(8),
                text("Storage data will appear here...")
                    .size(12)
                    .color(text_color(theme, TextStyle::Secondary)),
            ]
            .spacing(8)
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::Card).into()),
            ..Default::default()
        })
        .padding(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn button_style_active(theme: &BrowserTheme) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(theme.accent_color().into()),
        text_color: theme.background_color(),
        border: iced::Border {
            color: theme.accent_color(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn button_style_inactive(theme: &BrowserTheme) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(theme.surface_color().into()),
        text_color: text_color(theme, TextStyle::Primary),
        border: iced::Border {
            color: theme.border_color(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

impl Default for DevToolsPanel {
    fn default() -> Self {
        Self::new()
    }
}
