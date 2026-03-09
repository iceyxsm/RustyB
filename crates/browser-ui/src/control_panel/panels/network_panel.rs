//! Network tools panel - Proxy, Ad Blocker, DNS settings

use iced::{
    widget::{button, column, container, row, text, toggler, scrollable, text_input},
    Element, Length, Alignment,
};
use crate::theme::{container_background, text_color, ContainerStyle, TextStyle, BrowserTheme};

/// Messages for the network panel
#[derive(Debug, Clone)]
pub enum NetworkMessage {
    ToggleProxy(bool),
    ToggleAdBlocker(bool),
    TogglePrivacyMode(bool),
    ProxyPortChanged(u16),
    DnsProviderChanged(String),
    ClearCache,
    ViewNetworkLog,
}

/// Network tools panel
pub struct NetworkPanel {
    proxy_enabled: bool,
    adblock_enabled: bool,
    privacy_mode: bool,
    proxy_port: u16,
    dns_provider: String,
    theme: BrowserTheme,
}

impl NetworkPanel {
    pub fn new() -> Self {
        Self {
            proxy_enabled: false,
            adblock_enabled: true,
            privacy_mode: false,
            proxy_port: 8080,
            dns_provider: "Automatic".to_string(),
            theme: BrowserTheme::default(),
        }
    }

    pub fn theme(mut self, theme: BrowserTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn title(&self) -> &'static str {
        "Network Tools"
    }

    pub fn update(&mut self, message: NetworkMessage) {
        match message {
            NetworkMessage::ToggleProxy(enabled) => {
                self.proxy_enabled = enabled;
            }
            NetworkMessage::ToggleAdBlocker(enabled) => {
                self.adblock_enabled = enabled;
            }
            NetworkMessage::TogglePrivacyMode(enabled) => {
                self.privacy_mode = enabled;
            }
            NetworkMessage::ProxyPortChanged(port) => {
                self.proxy_port = port;
            }
            NetworkMessage::DnsProviderChanged(provider) => {
                self.dns_provider = provider;
            }
            NetworkMessage::ClearCache => {
                // Trigger cache clear
            }
            NetworkMessage::ViewNetworkLog => {
                // Show network log
            }
        }
    }

    pub fn view(&self) -> Element<'_, NetworkMessage> {
        let theme = &self.theme;

        // Title
        let title = text(self.title())
            .size(18)
            .color(text_color(theme, TextStyle::Primary));

        // MITM Proxy section
        let proxy_section = container(
            column![
                row![
                    text("🔒 MITM Proxy").size(14).color(text_color(theme, TextStyle::Primary)),
                    iced::widget::Space::new().width(Length::Fill),
                    toggler(self.proxy_enabled)
                        .on_toggle(NetworkMessage::ToggleProxy)
                        .label("Enabled"),
                ]
                .align_y(Alignment::Center),
                text("Intercept and inspect HTTPS traffic")
                    .size(11)
                    .color(text_color(theme, TextStyle::Secondary)),
                row![
                    text("Port:").size(12),
                    text_input("8080", &self.proxy_port.to_string())
                        .on_input(|s| {
                            if let Ok(port) = s.parse::<u16>() {
                                NetworkMessage::ProxyPortChanged(port)
                            } else {
                                NetworkMessage::ProxyPortChanged(8080)
                            }
                        })
                        .width(80),
                ]
                .spacing(8)
                .padding(8),
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

        // Ad Blocker section
        let adblock_section = container(
            column![
                row![
                    text("🛡️ Ad Blocker").size(14).color(text_color(theme, TextStyle::Primary)),
                    iced::widget::Space::new().width(Length::Fill),
                    toggler(self.adblock_enabled)
                        .on_toggle(NetworkMessage::ToggleAdBlocker)
                        .label("Enabled"),
                ]
                .align_y(Alignment::Center),
                text("Block ads and trackers using EasyList")
                    .size(11)
                    .color(text_color(theme, TextStyle::Secondary)),
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
                row![
                    text("🕵️ Privacy Mode").size(14).color(text_color(theme, TextStyle::Primary)),
                    iced::widget::Space::new().width(Length::Fill),
                    toggler(self.privacy_mode)
                        .on_toggle(NetworkMessage::TogglePrivacyMode)
                        .label("Enabled"),
                ]
                .align_y(Alignment::Center),
                text("Enhanced privacy protection and fingerprint randomization")
                    .size(11)
                    .color(text_color(theme, TextStyle::Secondary)),
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

        // DNS section
        let dns_section = container(
            column![
                text("🌐 DNS Settings").size(14).color(text_color(theme, TextStyle::Primary)),
                text("Configure DNS-over-HTTPS or DNS-over-TLS")
                    .size(11)
                    .color(text_color(theme, TextStyle::Secondary)),
                row![
                    text("Provider:").size(12),
                    text_input("Automatic", &self.dns_provider)
                        .on_input(NetworkMessage::DnsProviderChanged)
                        .width(Length::Fill),
                ]
                .spacing(8)
                .padding(8),
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

        // Actions
        let actions = row![
            button("Clear Cache")
                .on_press(NetworkMessage::ClearCache)
                .width(Length::FillPortion(1)),
            button("View Network Log")
                .on_press(NetworkMessage::ViewNetworkLog)
                .width(Length::FillPortion(1)),
        ]
        .spacing(8);

        // Main content
        let content = column![
            title,
            proxy_section,
            adblock_section,
            privacy_section,
            dns_section,
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

impl Default for NetworkPanel {
    fn default() -> Self {
        Self::new()
    }
}
