//! AI Engine panel - LLM, RAG, Embeddings

use iced::{
    widget::{button, column, container, row, text, text_input, scrollable, pick_list, toggler},
    Element, Length, Alignment,
};
use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme};

/// Messages for the AI panel
#[derive(Debug, Clone)]
pub enum AiMessage {
    PromptChanged(String),
    SendPrompt,
    ClearChat,
    ToggleRag(bool),
    ModelSelected(String),
    ExtractPage,
    SummarizePage,
}

/// AI Engine panel
pub struct AiPanel {
    prompt: String,
    chat_history: Vec<ChatMessage>,
    rag_enabled: bool,
    selected_model: String,
    available_models: Vec<String>,
    theme: BrowserTheme,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl AiPanel {
    pub fn new() -> Self {
        Self {
            prompt: String::new(),
            chat_history: vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: "Welcome to Rusty AI! I can help you analyze web pages, answer questions, and assist with automation.".to_string(),
                },
            ],
            rag_enabled: false,
            selected_model: "default".to_string(),
            available_models: vec![
                "default".to_string(),
                "local-llm".to_string(),
                "embeddings".to_string(),
            ],
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn title(&self) -> &'static str {
        "AI Engine"
    }

    pub fn update(&mut self, message: AiMessage) {
        match message {
            AiMessage::PromptChanged(text) => {
                self.prompt = text;
            }
            AiMessage::SendPrompt => {
                if !self.prompt.is_empty() {
                    self.chat_history.push(ChatMessage {
                        role: MessageRole::User,
                        content: self.prompt.clone(),
                    });
                    // Simulate AI response
                    self.chat_history.push(ChatMessage {
                        role: MessageRole::Assistant,
                        content: "I'm processing your request... (AI integration pending)".to_string(),
                    });
                    self.prompt.clear();
                }
            }
            AiMessage::ClearChat => {
                self.chat_history.retain(|m| m.role == MessageRole::System);
            }
            AiMessage::ToggleRag(enabled) => {
                self.rag_enabled = enabled;
            }
            AiMessage::ModelSelected(model) => {
                self.selected_model = model;
            }
            AiMessage::ExtractPage => {
                self.chat_history.push(ChatMessage {
                    role: MessageRole::User,
                    content: "Extract data from this page".to_string(),
                });
            }
            AiMessage::SummarizePage => {
                self.chat_history.push(ChatMessage {
                    role: MessageRole::User,
                    content: "Summarize this page".to_string(),
                });
            }
        }
    }

    pub fn view(&self) -> Element<AiMessage> {
        let theme = &self.theme;

        // Title
        let title = text(self.title())
            .size(18)
            .color(text_color(theme, TextStyle::Primary));

        // Model selection
        let model_row = row![
            text("Model:").size(12),
            pick_list(
                self.available_models.clone(),
                Some(self.selected_model.clone()),
                AiMessage::ModelSelected,
            )
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // RAG toggle
        let rag_row = row![
            text("RAG (Retrieval Augmented Generation)").size(12),
            iced::widget::Space::new().width(Length::Fill),
            toggler(self.rag_enabled)
                .on_toggle(AiMessage::ToggleRag),
        ]
        .align_y(Alignment::Center);

        // Chat history
        let mut chat_content = column![]
            .spacing(8)
            .padding(8);

        for msg in &self.chat_history {
            let (label, color) = match msg.role {
                MessageRole::User => ("You:", theme.accent_color()),
                MessageRole::Assistant => ("AI:", theme.success_color()),
                MessageRole::System => ("System:", theme.info_color()),
            };

            let msg_container = container(
                column![
                    text(label).size(10).color(color),
                    text(&msg.content).size(12).color(text_color(theme, TextStyle::Primary)),
                ]
                .spacing(2)
            )
            .style(move |_| container::Style {
                background: Some(container_background(theme, ContainerStyle::Card).into()),
                border: iced::Border {
                    color: color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .padding(8)
            .width(Length::Fill);

            chat_content = chat_content.push(msg_container);
        }

        let chat_area = scrollable(chat_content)
            .height(Length::FillPortion(3));

        // Quick actions
        let quick_actions = row![
            button("📄 Extract Page").on_press(AiMessage::ExtractPage),
            button("📝 Summarize").on_press(AiMessage::SummarizePage),
            button("🧹 Clear").on_press(AiMessage::ClearChat),
        ]
        .spacing(8);

        // Input area
        let input_row = row![
            text_input("Ask me anything...", &self.prompt)
                .on_input(AiMessage::PromptChanged)
                .on_submit(AiMessage::SendPrompt)
                .width(Length::Fill),
            button("Send").on_press(AiMessage::SendPrompt),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        // Main content
        let content = column![
            title,
            model_row,
            rag_row,
            chat_area,
            quick_actions,
            input_row,
        ]
        .spacing(12)
        .padding(12);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Default for AiPanel {
    fn default() -> Self {
        Self::new()
    }
}
