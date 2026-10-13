use iced::{Element, Length};
use iced::widget::{Container, Column, Row, Scrollable, Text};
use crate::app::Message;
use crate::model::TradeView;
use crate::style::{BG_PANEL, TEXT_SECONDARY, BID_COLOR, ASK_COLOR, WARNING_COLOR};

pub fn trades_panel(trades: &[TradeView]) -> Element<'_, Message> {
    let header = Row::new()
        .spacing(10)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("ID").size(12).width(Length::Fixed(60.0)).color(TEXT_SECONDARY))
        .push(Text::new("Market").size(12).width(Length::Fixed(60.0)).color(TEXT_SECONDARY))
        .push(Text::new("Price").size(12).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("Amount").size(12).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("State").size(12).width(Length::Fill).color(TEXT_SECONDARY));

    let list = trades.iter().fold(
        Column::new().spacing(0),
        |col, trade| {
            let id_short = trade.id.to_string()[..8].to_string();
            let state_color = match trade.state {
                protocol::message::TradeState::Settled => BID_COLOR,
                protocol::message::TradeState::Cancelled
                | protocol::message::TradeState::Expired => ASK_COLOR,
                protocol::message::TradeState::Disputed => WARNING_COLOR,
                _ => TEXT_SECONDARY,
            };
            let state_str = format!("{:?}", trade.state);
            col.push(Row::new()
                .spacing(10)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(id_short).size(10).width(Length::Fixed(60.0)).color(TEXT_SECONDARY))
                .push(Text::new(&trade.market).size(11).width(Length::Fixed(60.0)))
                .push(Text::new(format!("{:.4}", trade.price)).size(11).width(Length::Fill))
                .push(Text::new(format!("{:.4}", trade.amount)).size(11).width(Length::Fill))
                .push(Text::new(state_str).size(10).width(Length::Fill).color(state_color)))
        }
    );

    let title = Container::new(Text::new("Trades").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let content = Column::new()
        .spacing(0)
        .push(title)
        .push(header)
        .push(Scrollable::new(list).height(Length::Fill));

    Container::new(content)
        .padding(0)
        .height(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
