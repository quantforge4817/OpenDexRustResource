use iced::widget::{Column, Container, Row, Scrollable, Text};
use iced::Element;

use crate::gui_app::Message;
use crate::gui_model::OrderBookView;
use crate::gui_style::{ASK_COLOR, BID_COLOR};

fn header_row() -> Element<'static, Message> {
    Row::new()
        .spacing(10)
        .push(Text::new("Price").width(iced::Length::Fill))
        .push(Text::new("Amount").width(iced::Length::Fill))
        .push(Text::new("ID").width(iced::Length::Fill))
        .into()
}

fn order_row(order: &protocol::message::Order, color: iced::Color) -> Element<'_, Message> {
    let price_str = crate::gui_model::GuiModel::format_price(order.price);
    let amount_str = crate::gui_model::GuiModel::format_amount(order.amount);
    let id_str = order.id.to_string()[..8].to_string();

    Row::new()
        .spacing(10)
        .push(Text::new(price_str).width(iced::Length::Fill))
        .push(Text::new(amount_str).width(iced::Length::Fill))
        .push(Text::new(id_str).width(iced::Length::Fill))
        .into()
}

pub fn order_book_panel(order_book: &OrderBookView) -> Element<'_, Message> {
    let asks: Vec<Element<'_, Message>> = order_book
        .asks
        .iter()
        .rev()
        .map(|o| order_row(o, ASK_COLOR))
        .collect();

    let bids: Vec<Element<'_, Message>> = order_book
        .bids
        .iter()
        .map(|o| order_row(o, BID_COLOR))
        .collect();

    let asks_header = Text::new("Asks");
    let bids_header = Text::new("Bids");

    let content = Column::new()
        .spacing(5)
        .push(asks_header)
        .push(header_row())
        .push(Column::with_children(asks))
        .push(bids_header)
        .push(Column::with_children(bids));

    let scrollable = Scrollable::new(content).height(iced::Length::Fill);

    Container::new(scrollable)
        .padding(10)
        .into()
}
