//! Settings panel - General preferences, themes, advanced options

use iced::{
    widget::{button, column, container, row, text, toggler, slider, pick_list, scrollable},
    Element, Length, Alignment,
};
use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme, ThemeMode};

/// Messages for the settings panel
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    ToggleTheme,
    ThemeModeChanged(ThemeMode),
    ZoomChanged(f32),
    ToggleHardwareAcceleration(bool),
    ToggleNotifications(bool),
    ClearData,
    ExportSettings,
    ImportSettings,
}

/// Settings panel
pub struct SettingsPanel {
    theme_mode: ThemeMode,
    zoom_level: f32,
    hardware_acceleration: bool,
    notifications: bool,
    theme: BrowserTheme,
}

impl SettingsPanel {
    pub fn new() -> Self {
        Self {
            theme_mode: ThemeMode::Dark,
            zoom_level: 100.0,
            hardware_acceleration: true,
            notifications: true,
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn title(&self) -> &'static str {
        "Settings"
    }

    pub fn update(&mut self, message: SettingsMessage) {
        match message {
            SettingsMessage::ToggleTheme => {
                self.theme_mode = match self.theme_mode {
                    ThemeMode::Light => ThemeMode::Dark,
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::HighContrast => ThemeMode::Light,
                    ThemeMode::Auto => ThemeMode::Dark,
                };
            }
            SettingsMessage::ThemeModeChanged(mode) => {
                self.theme_mode = mode;
            }
            SettingsMessage::ZoomChanged(zoom) => {
                self.zoom_level = zoom;
            }
            SettingsMessage::ToggleHardwareAcceleration(enabled) => {
                self.hardware_acceleration = enabled;
            }
            SettingsMessage::ToggleNotifications(enabled) => {
                self.notifications = enabled;
            }
            SettingsMessage::ClearData => {}
            SettingsMessage::ExportSettings => {}
            SettingsMessage::ImportSettings => {}
        }
    }

    pub fn view(&self) -> Element<SettingsMessage> {
        let theme = &self.theme;

        let title = text(self.title())
            .size(18)
            .color(text_color(theme, TextStyle::Primary));

        // Appearance section
        let appearance_section = container(
            column![
                text("🎨 Appearance").size(14).color(text_color(theme, TextStyle::Primary)),
                row![
                    text("Theme:").size(12),
                    iced::widget::Space::new().width(Length::Fill),
                    {
                        let theme_options = vec![
                            ("Light", ThemeMode::Light),
                            ("Dark", ThemeMode::Dark),
                            ("High Contrast", ThemeMode::HighContrast),
                            ("Auto", ThemeMode::Auto),
                        ];
                        let selected = theme_options.iter().find(|(_, m)| *m == self.theme_mode).map(|(s, _)| *s);

                        pick_list(
                            theme_options.iter().map(|(s, _)| *s).collect::<Vec<_>>(),
                            selected,
                            |s| {
                                let mode = match s {
                                    "Light" => ThemeMode::Light,
                                    "Dark" => ThemeMode::Dark,
                                    "High Contrast" => ThemeMode::HighContrast,
                                    _ => ThemeMode::Auto,
                                };
                                SettingsMessage::ThemeModeChanged(mode)
                            },
                        )
                        .text_size(12)
                    },
                ]
                .align_y(Alignment::Center),
                row![
                    text("Zoom:").size(12).width(50),
                    slider(50.0..=200.0, self.zoom_level, SettingsMessage::ZoomChanged)
                        .width(Length::FillPortion(3)),
                    text(format!("{:.0}%", self.zoom_level)).size(12).width(40),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
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

        // Performance section
        let performance_section = container(
            column![
                text("⚡ Performance").size(14).color(text_color(theme, TextStyle::Primary)),
                row![
                    text("Hardware Acceleration").size(12),
                    iced::widget::Space::new().width(Length::Fill),
                    toggler(self.hardware_acceleration)
                        .on_toggle(SettingsMessage::ToggleHardwareAcceleration),
                ]
                .align_y(Alignment::Center),
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

        // Privacy section
        let privacy_section = container(
            column![
                text("🔒 Privacy & Security").size(14).color(text_color(theme, TextStyle::Primary)),
                row![
                    text("Notifications").size(12),
                    iced::widget::Space::new().width(Length::Fill),
                    toggler(self.notifications)
                        .on_toggle(SettingsMessage::ToggleNotifications),
                ]
                .align_y(Alignment::Center),
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

        // Data management
        let data_section = container(
            column![
                text("💾 Data Management").size(14).color(text_color(theme, TextStyle::Primary)),
                row![
                    button("Clear All Data").on_press(SettingsMessage::ClearData),
                    button("Export Settings").on_press(SettingsMessage::ExportSettings),
                    button("Import Settings").on_press(SettingsMessage::ImportSettings),
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

        // About
        let about_section = container(
            column![
                text("ℹ️ About Rusty Browser").size(14).color(text_color(theme, TextStyle::Primary)),
                text("Version: 0.1.0").size(11).color(text_color(theme, TextStyle::Secondary)),
                text("A custom Rust-based browser with Servo rendering").size(11).color(text_color(theme, TextStyle::Secondary)),
            ]
            .spacing(4)
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

        let content = column![
            title,
            appearance_section,
            performance_section,
            privacy_section,
            data_section,
            about_section,
        ]
        .spacing(12)
        .padding(12);

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}
