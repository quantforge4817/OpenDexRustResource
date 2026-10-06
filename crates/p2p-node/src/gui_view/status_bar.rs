use iced::widget::{Container, Row, Text};
use iced::Element;

use crate::gui_app::Message;
use crate::gui_model::GuiModel;

pub fn status_bar(model: &GuiModel) -> Element<'_, Message> {
    let status_text = model
        .status_message
        .as_deref()
        .unwrap_or("Ready");

    let peer_text = format!("Peers: {}", model.node_status.peer_count);

    let container = Container::new(
        Row::new()
            .spacing(20)
            .push(Text::new(&model.node_status.peer_id[..12.min(model.node_status.peer_id.len())]))
            .push(Text::new(peer_text))
            .push(Text::new(status_text).width(iced::Length::Fill)),
    )
    .padding(5);

    container.into()
}
