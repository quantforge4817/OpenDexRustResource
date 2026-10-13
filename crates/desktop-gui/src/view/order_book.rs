use iced::{Element, Length};
use iced::widget::{Container, Column, Row, Scrollable, Text};
use crate::app::Message;
use crate::model::{GuiModel, OrderBookView};
use crate::style::{BG_PANEL, TEXT_SECONDARY, BID_COLOR, ASK_COLOR};

pub fn order_book_panel(order_book: &OrderBookView) -> Element<'_, Message> {
    let header = Row::new()
        .spacing(10)
        .padding(iced::Padding::new(8.0).left(10).right(10))
        .push(Text::new("Price").size(12).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("Amount").size(12).width(Length::Fill).color(TEXT_SECONDARY))
        .push(Text::new("ID").size(12).width(Length::Fill).color(TEXT_SECONDARY));

    let asks_col = order_book.asks.iter().rev().fold(
        Column::new().spacing(0),
        |col, order| {
            let price = GuiModel::format_price(order.price);
            let amount = GuiModel::format_amount(order.amount);
            let id_short = order.id.to_string()[..8].to_string();
            col.push(Row::new()
                .spacing(10)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(price).size(11).width(Length::Fill).color(ASK_COLOR))
                .push(Text::new(amount).size(11).width(Length::Fill))
                .push(Text::new(id_short).size(10).width(Length::Fill).color(TEXT_SECONDARY)))
        }
    );

    let bids_col = order_book.bids.iter().fold(
        Column::new().spacing(0),
        |col, order| {
            let price = GuiModel::format_price(order.price);
            let amount = GuiModel::format_amount(order.amount);
            let id_short = order.id.to_string()[..8].to_string();
            col.push(Row::new()
                .spacing(10)
                .padding(iced::Padding::new(4.0).left(10).right(10))
                .push(Text::new(price).size(11).width(Length::Fill).color(BID_COLOR))
                .push(Text::new(amount).size(11).width(Length::Fill))
                .push(Text::new(id_short).size(10).width(Length::Fill).color(TEXT_SECONDARY)))
        }
    );

    let separator = Row::new()
        .padding(iced::Padding::new(4.0).left(10).right(10))
        .push(Text::new("—").size(11).width(Length::Fill).color(TEXT_SECONDARY));

    let all_rows = Column::new()
        .spacing(0)
        .push(asks_col)
        .push(separator)
        .push(bids_col);

    let title = Container::new(Text::new("Order Book").size(14))
        .padding(iced::Padding::new(8.0).left(10).bottom(4));

    let content = Column::new()
        .spacing(0)
        .push(title)
        .push(header)
        .push(Scrollable::new(all_rows).height(Length::Fill));

    Container::new(content)
        .padding(0)
        .height(Length::Fill)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
