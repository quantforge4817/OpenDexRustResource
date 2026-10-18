use crate::app::Message;
use crate::model::GuiModel;
use crate::style::BG_PANEL;
use iced::Element;
use iced::widget::{Button, Column, Container, Row, Text, TextInput};

pub fn trade_form_panel(model: &GuiModel) -> Element<'_, Message> {
    let side_label = match model.trade_form.side {
        protocol::message::Side::Buy => "Buy",
        protocol::message::Side::Sell => "Sell",
    };

    let side_button = Button::new(Text::new(side_label).size(12))
        .on_press(Message::SideSelected(
            if model.trade_form.side == protocol::message::Side::Buy {
                "Sell".to_string()
            } else {
                "Buy".to_string()
            },
        ))
        .padding(iced::Padding::new(8.0).top(6).bottom(6));

    let market_text = Text::new(model.trade_form.market.to_string()).size(12);

    let price_input = TextInput::new("Price", &model.trade_form.price)
        .on_input(Message::PriceChanged)
        .padding(6);

    let amount_input = TextInput::new("Amount", &model.trade_form.amount)
        .on_input(Message::AmountChanged)
        .padding(6);

    let submit_button = Button::new(Text::new("Submit Order").size(12))
        .on_press(Message::SubmitOrder)
        .padding(iced::Padding::new(12.0).top(8).bottom(8));

    let refresh_button = Button::new(Text::new("Refresh").size(11))
        .on_press(Message::RefreshOrderBook)
        .padding(iced::Padding::new(8.0).top(4).bottom(4));

    let content = Column::new()
        .spacing(8)
        .padding(10)
        .push(Text::new("Trade Execution").size(14))
        .push(Row::new().spacing(10).push(side_button).push(market_text))
        .push(price_input)
        .push(amount_input)
        .push(
            Row::new()
                .spacing(10)
                .push(submit_button)
                .push(refresh_button),
        );

    Container::new(content)
        .padding(10)
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
