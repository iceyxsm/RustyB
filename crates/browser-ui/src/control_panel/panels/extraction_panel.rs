//! Extraction panel - Web-to-API, Data Scraping

use iced::{
    widget::{button, column, container, row, text, text_input, scrollable, toggler},
    Element, Length, Alignment,
};
use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme};

/// Messages for the extraction panel
#[derive(Debug, Clone)]
pub enum ExtractionMessage {
    SchemaNameChanged(String),
    SelectorChanged(String),
    AddField,
    RemoveField(usize),
    SaveSchema,
    LoadSchema,
    RunExtraction,
    ExportData,
    ToggleLivePreview(bool),
}

/// Extraction panel
pub struct ExtractionPanel {
    schema_name: String,
    selector: String,
    fields: Vec<ExtractionField>,
    live_preview: bool,
    #[allow(dead_code)]
    extracted_data: Vec<String>,
    theme: BrowserTheme,
}

#[derive(Debug, Clone)]
pub struct ExtractionField {
    pub name: String,
    pub selector: String,
    pub attribute: String,
}

impl ExtractionPanel {
    pub fn new() -> Self {
        Self {
            schema_name: String::new(),
            selector: String::new(),
            fields: vec![
                ExtractionField {
                    name: "title".to_string(),
                    selector: "h1".to_string(),
                    attribute: "text".to_string(),
                },
            ],
            live_preview: false,
            extracted_data: vec![],
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn title(&self) -> &'static str {
        "Data Extraction"
    }

    pub fn update(&mut self, message: ExtractionMessage) {
        match message {
            ExtractionMessage::SchemaNameChanged(name) => {
                self.schema_name = name;
            }
            ExtractionMessage::SelectorChanged(selector) => {
                self.selector = selector;
            }
            ExtractionMessage::AddField => {
                self.fields.push(ExtractionField {
                    name: format!("field_{}", self.fields.len()),
                    selector: String::new(),
                    attribute: "text".to_string(),
                });
            }
            ExtractionMessage::RemoveField(index) => {
                if index < self.fields.len() {
                    self.fields.remove(index);
                }
            }
            ExtractionMessage::SaveSchema => {}
            ExtractionMessage::LoadSchema => {}
            ExtractionMessage::RunExtraction => {}
            ExtractionMessage::ExportData => {}
            ExtractionMessage::ToggleLivePreview(enabled) => {
                self.live_preview = enabled;
            }
        }
    }

    pub fn view(&self) -> Element<'_, ExtractionMessage> {
        let theme = &self.theme;

        let title = text(self.title())
            .size(18)
            .color(text_color(theme, TextStyle::Primary));

        // Schema builder
        let schema_section = container(
            column![
                text("📐 Schema Builder").size(14).color(text_color(theme, TextStyle::Primary)),
                row![
                    text("Name:").size(12),
                    text_input("Schema name...", &self.schema_name)
                        .on_input(ExtractionMessage::SchemaNameChanged)
                        .width(Length::Fill),
                ]
                .spacing(8),
                row![
                    text("Base Selector:").size(12),
                    text_input("CSS selector...", &self.selector)
                        .on_input(ExtractionMessage::SelectorChanged)
                        .width(Length::Fill),
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

        // Fields list
        let mut fields_content = column![]
            .spacing(4);

        for (i, field) in self.fields.iter().enumerate() {
            let field_row = row![
                text(&field.name).size(11).width(80),
                text(&field.selector).size(11).width(Length::Fill),
                text(&field.attribute).size(11).width(60),
                button("🗑️").on_press(ExtractionMessage::RemoveField(i)),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            fields_content = fields_content.push(field_row);
        }

        let fields_section = container(
            column![
                row![
                    text("🔍 Fields").size(14).color(text_color(theme, TextStyle::Primary)),
                    iced::widget::Space::new().width(Length::Fill),
                    button("+ Add").on_press(ExtractionMessage::AddField),
                ]
                .align_y(Alignment::Center),
                fields_content,
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

        // Live preview toggle
        let preview_row = row![
            text("Live Preview").size(12),
            iced::widget::Space::new().width(Length::Fill),
            toggler(self.live_preview)
                .on_toggle(ExtractionMessage::ToggleLivePreview),
        ]
        .align_y(Alignment::Center);

        // Actions
        let actions = row![
            button("💾 Save").on_press(ExtractionMessage::SaveSchema),
            button("📂 Load").on_press(ExtractionMessage::LoadSchema),
            button("▶ Extract").on_press(ExtractionMessage::RunExtraction),
            button("📤 Export").on_press(ExtractionMessage::ExportData),
        ]
        .spacing(8);

        let content = column![
            title,
            schema_section,
            fields_section,
            preview_row,
            actions,
        ]
        .spacing(12)
        .padding(12);

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Default for ExtractionPanel {
    fn default() -> Self {
        Self::new()
    }
}
