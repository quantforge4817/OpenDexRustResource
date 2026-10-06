use iced::widget::{button, Column, Container, Row, Text, TextInput};
use iced::Element;

use crate::gui_app::Message;
use crate::gui_model::GuiModel;
use crate::gui_style::{ACCENT_COLOR, ASK_COLOR, BID_COLOR};
use protocol::message::Asset;

pub fn trade_form_panel(model: &GuiModel) -> Element<'_, Message> {
    let side_label = match model.trade_form.side {
        protocol::message::Side::Buy => "Buy",
        protocol::message::Side::Sell => "Sell",
    };

    let market_label = format!("{}/{}", model.trade_form.market.base, model.trade_form.market.quote);

    let buy_button = button(Text::new("Buy").width(iced::Length::Fill))
        .on_press(Message::SideSelected("Buy".to_string()))
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(BID_COLOR)),
            ..Default::default()
        });

    let sell_button = button(Text::new("Sell").width(iced::Length::Fill))
        .on_press(Message::SideSelected("Sell".to_string()))
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(ASK_COLOR)),
            ..Default::default()
        });

    let side_selector = Row::new()
        .spacing(5)
        .push(buy_button)
        .push(sell_button);

    let market_button = button(Text::new(market_label).width(iced::Length::Fill))
        .on_press(Message::MarketSelected("BTC/XMR".to_string()))
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(ACCENT_COLOR)),
            ..Default::default()
        });

    let quote_asset = match model.trade_form.market.quote {
        Asset::BTC => "BTC",
        Asset::XMR => "XMR",
    };
    let base_asset = match model.trade_form.market.base {
        Asset::BTC => "BTC",
        Asset::XMR => "XMR",
    };

    let price_input = TextInput::new(format!("Price ({})", quote_asset).as_str(), &model.trade_form.price)
        .on_input(Message::PriceChanged)
        .padding(5);

    let amount_input = TextInput::new(format!("Amount ({})", base_asset).as_str(), &model.trade_form.amount)
        .on_input(Message::AmountChanged)
        .padding(5);

    let submit_button = button(
        Text::new(format!("{} Order", side_label)).width(iced::Length::Fill),
    )
    .on_press(Message::SubmitOrder)
    .style(|_, _| iced::widget::button::Style {
        background: Some(iced::Background::Color(ACCENT_COLOR)),
        ..Default::default()
    });

    let content = Column::new()
        .spacing(10)
        .push(Text::new("Place Order"))
        .push(market_button)
        .push(side_selector)
        .push(price_input)
        .push(amount_input)
        .push(submit_button);

    Container::new(content)
        .padding(10)
        .into()
}
