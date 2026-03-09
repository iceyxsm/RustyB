//! Sidebar navigation for the control panel

use iced::{
    widget::{button, column, container, row, text, Space},
    Element, Length, Alignment,
};

use crate::control_panel::ToolCategory;
use crate::theme::{button_style, text_color, ButtonStyle, TextStyle, BrowserTheme};

/// Message type for sidebar interactions
#[derive(Debug, Clone)]
pub enum SidebarMessage {
    CategorySelected(ToolCategory),
    TogglePanel,
}

/// Sidebar component for tool navigation
pub struct Sidebar {
    categories: Vec<ToolCategory>,
    expanded: bool,
    selected: ToolCategory,
    theme: BrowserTheme,
}

impl Sidebar {
    pub fn new(expanded: bool, selected: ToolCategory) -> Self {
        Self {
            categories: vec![
                ToolCategory::Network,
                ToolCategory::AiEngine,
                ToolCategory::Automation,
                ToolCategory::Extraction,
                ToolCategory::DevTools,
                ToolCategory::Settings,
            ],
            expanded,
            selected,
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn update(&mut self, message: SidebarMessage) {
        match message {
            SidebarMessage::CategorySelected(category) => {
                self.selected = category;
            }
            SidebarMessage::TogglePanel => {
                self.expanded = !self.expanded;
            }
        }
    }

    pub fn view(&self) -> Element<SidebarMessage> {
        let theme = &self.theme;

        // Toggle button at the top
        let toggle_button = button(
            text(if self.expanded { "◀" } else { "▶" })
                .size(14)
                .color(text_color(theme, TextStyle::Toolbar))
        )
        .on_press(SidebarMessage::TogglePanel)
        .style(move |_, _| button_style(theme, ButtonStyle::Navigation))
        .width(Length::Fill);

        // Build category buttons
        let category_buttons: Vec<Element<SidebarMessage>> = self
            .categories
            .iter()
            .map(|category| {
                let is_selected = *category == self.selected;
                let icon = category.icon();
                let label = if self.expanded {
                    format!(" {} ", category.name())
                } else {
                    "".to_string()
                };

                let content = if self.expanded {
                    row![
                        text(icon).size(18),
                        text(label).size(12).color(text_color(theme, TextStyle::Secondary)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                } else {
                    row![text(icon).size(20)]
                        .align_y(Alignment::Center)
                };

                button(content)
                    .on_press(SidebarMessage::CategorySelected(*category))
                    .style(move |_, status| {
                        let mut style = button_style(theme, if is_selected {
                            ButtonStyle::Primary
                        } else {
                            ButtonStyle::Secondary
                        });
                        
                        // Highlight selected
                        if is_selected {
                            style.background = Some(theme.accent_color().into());
                        }
                        
                        style
                    })
                    .width(Length::Fill)
                    .padding(if self.expanded { 8 } else { 10 })
                    .into()
            })
            .collect();

        // Assemble sidebar
        let mut sidebar_content = column![]
            .spacing(4)
            .padding(4);

        sidebar_content = sidebar_content.push(toggle_button);
        sidebar_content = sidebar_content.push(Space::new().height(8));

        for btn in category_buttons {
            sidebar_content = sidebar_content.push(btn);
        }

        container(sidebar_content)
            .style(move |_| container::Style {
                background: Some(theme.surface_color().into()),
                border: iced::Border {
                    color: theme.border_color(),
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .width(if self.expanded { 160 } else { 50 })
            .height(Length::Fill)
            .into()
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn selected(&self) -> ToolCategory {
        self.selected
    }

    pub fn width(&self) -> u16 {
        if self.expanded { 160 } else { 50 }
    }
}
