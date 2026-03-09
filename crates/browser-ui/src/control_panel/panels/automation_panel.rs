//! Automation panel - Scripts, Commands, Macros

use iced::{
    widget::{button, column, container, row, text, text_input, scrollable, toggler},
    Element, Length, Alignment,
};
use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme};

/// Messages for the automation panel
#[derive(Debug, Clone)]
pub enum AutomationMessage {
    RunScript,
    StopScript,
    RecordMacro,
    CommandTextChanged(String),
    ExecuteCommand,
    ToggleRecording(bool),
}

/// Automation panel
pub struct AutomationPanel {
    command_text: String,
    is_recording: bool,
    is_running: bool,
    macros: Vec<String>,
    theme: BrowserTheme,
}

impl AutomationPanel {
    pub fn new() -> Self {
        Self {
            command_text: String::new(),
            is_recording: false,
            is_running: false,
            macros: vec![
                "Login Sequence".to_string(),
                "Form Filler".to_string(),
                "Data Scraper".to_string(),
            ],
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn title(&self) -> &'static str {
        "Automation"
    }

    pub fn update(&mut self, message: AutomationMessage) {
        match message {
            AutomationMessage::RunScript => {
                self.is_running = true;
            }
            AutomationMessage::StopScript => {
                self.is_running = false;
            }
            AutomationMessage::RecordMacro => {
                self.is_recording = !self.is_recording;
            }
            AutomationMessage::CommandTextChanged(text) => {
                self.command_text = text;
            }
            AutomationMessage::ExecuteCommand => {
                // Execute the command
                self.command_text.clear();
            }
            AutomationMessage::ToggleRecording(enabled) => {
                self.is_recording = enabled;
            }
        }
    }

    pub fn view(&self) -> Element<AutomationMessage> {
        let theme = &self.theme;

        let title = text(self.title())
            .size(18)
            .color(text_color(theme, TextStyle::Primary));

        // Recording controls
        let recording_section = container(
            column![
                row![
                    text("🔴 Recording").size(14).color(text_color(theme, TextStyle::Primary)),
                    iced::widget::Space::new().width(Length::Fill),
                    toggler(self.is_recording)
                        .on_toggle(AutomationMessage::ToggleRecording),
                ]
                .align_y(Alignment::Center),
                text("Record user interactions as replayable macros")
                    .size(11)
                    .color(text_color(theme, TextStyle::Secondary)),
                row![
                    button("Start Recording").on_press(AutomationMessage::RecordMacro),
                    button("Stop").on_press(AutomationMessage::StopScript),
                ]
                .spacing(8)
                .padding(8),
            ]
            .spacing(8)
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::Card).into()),
            border: iced::Border {
                color: if self.is_recording { theme.error_color() } else { theme.border_color() },
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .padding(12)
        .width(Length::Fill);

        // Command console
        let command_section = container(
            column![
                text("⌨️ Command Console").size(14).color(text_color(theme, TextStyle::Primary)),
                row![
                    text_input("Enter command...", &self.command_text)
                        .on_input(AutomationMessage::CommandTextChanged)
                        .on_submit(AutomationMessage::ExecuteCommand)
                        .width(Length::Fill),
                    button("▶ Run").on_press(AutomationMessage::RunScript),
                ]
                .spacing(8),
            ]
            .spacing(8)
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::Card).into()),
            border: iced::Border {
                color: theme.border_color(),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .padding(12)
        .width(Length::Fill);

        // Saved macros
        let mut macros_list = column![]
            .spacing(4);

        for macro_name in &self.macros {
            let macro_item = row![
                text(macro_name).size(12),
                iced::widget::Space::new().width(Length::Fill),
                button("▶").on_press(AutomationMessage::RunScript),
                button("✏️"),
                button("🗑️"),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            macros_list = macros_list.push(macro_item);
        }

        let macros_section = container(
            column![
                text("📋 Saved Macros").size(14).color(text_color(theme, TextStyle::Primary)),
                macros_list,
            ]
            .spacing(8)
        )
        .style(move |_| container::Style {
            background: Some(container_background(theme, ContainerStyle::Card).into()),
            border: iced::Border {
                color: theme.border_color(),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .padding(12)
        .width(Length::Fill);

        // Quick actions
        let quick_actions = row![
            button("⏱️ Wait"),
            button("🖱️ Click"),
            button("⌨️ Type"),
            button("📜 Scroll"),
        ]
        .spacing(8);

        let content = column![
            title,
            recording_section,
            command_section,
            quick_actions,
            macros_section,
        ]
        .spacing(12)
        .padding(12);

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Default for AutomationPanel {
    fn default() -> Self {
        Self::new()
    }
}
