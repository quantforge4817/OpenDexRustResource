use iced::{Alignment, Element, Length};
use iced::widget::{Container, Row, Text};
use crate::app::Message;
use crate::model::GuiModel;
use crate::style::{BG_PANEL, TEXT_SECONDARY, ACCENT_COLOR};

pub fn status_bar(model: &GuiModel) -> Element<'_, Message> {
    let peer_id_short = if model.node_status.peer_id.len() > 12 {
        format!("...{}", &model.node_status.peer_id[model.node_status.peer_id.len() - 9..])
    } else {
        model.node_status.peer_id.clone()
    };
    let listen_addr = model.node_status.listen_addr.clone();
    let peer_count = if model.node_status.peer_count == 1 {
        "1 peer".to_string()
    } else {
        format!("{} peers", model.node_status.peer_count)
    };
    let wallet_status = if model.wallet_connected {
        "XMR ✓".to_string()
    } else {
        "XMR ✗".to_string()
    };
    let btc_status = if model.btc_wallet_connected {
        "BTC ✓".to_string()
    } else {
        "BTC ✗".to_string()
    };
    let status_text = model.status_message.clone().unwrap_or_else(|| "Ready".to_string());

    let content = Row::new()
        .spacing(20)
        .padding(iced::Padding::new(8.0).top(4).bottom(4))
        .push(Text::new(peer_id_short).size(12))
        .push(Text::new(listen_addr).size(12).color(TEXT_SECONDARY))
        .push(Text::new(peer_count).size(12).color(ACCENT_COLOR))
        .push(Text::new(wallet_status).size(12))
        .push(Text::new(btc_status).size(12))
        .push(Container::new(Text::new(status_text).size(12))
            .width(Length::Fill)
            .align_x(Alignment::End));

    Container::new(content)
        .padding(iced::Padding::new(8.0).top(4).bottom(4))
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
