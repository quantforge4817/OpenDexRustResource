use iced::{Element, Length};
use iced::widget::{Button, Column, Container, Row, Text, TextInput};
use crate::app::Message;
use crate::model::GuiModel;
use crate::style::{BG_PANEL, TEXT_SECONDARY, ACCENT_COLOR};

pub fn wallet_panel(model: &GuiModel) -> Element<'_, Message> {
    // Display local balance book entries
    let balances_col = model.balances.iter().fold(
        Column::new().spacing(2),
        |col, b| {
            let available = GuiModel::format_amount(b.available);
            let locked = GuiModel::format_amount(b.locked);
            let escrowed = GuiModel::format_amount(b.escrowed);
            let asset = b.asset.to_string();
            col.push(Row::new()
                .spacing(10)
                .push(Text::new(format!("{}:", asset)).size(11).width(Length::Fixed(40.0)))
                .push(Text::new(format!("Avail: {}", available)).size(11).width(Length::Fill))
                .push(Text::new(format!("Locked: {}", locked)).size(11).width(Length::Fill))
                .push(Text::new(format!("Escrow: {}", escrowed)).size(11).width(Length::Fill)))
        }
    );

    // On-chain wallet balances
    let xmr_balance = model.wallet_balance_available
        .map(|b| format!("XMR: {:.4}", b))
        .unwrap_or_else(|| "XMR: —".to_string());

    let btc_balance = model.btc_wallet_balance
        .map(|b| format!("BTC: {:.8}", b))
        .unwrap_or_else(|| "BTC: —".to_string());

    // Fund controls
    let fund_asset_label = model.fund_asset.to_string();
    let fund_button = Button::new(Text::new("Fund").size(11))
        .on_press(Message::FundSubmit)
        .padding(iced::Padding::new(8.0).top(4).bottom(4));

    let fund_amount_input = TextInput::new("Amount", &model.fund_amount)
        .on_input(Message::FundAmountChanged)
        .padding(4)
        .width(Length::Fixed(80.0));

    let fund_asset_button = Button::new(Text::new(fund_asset_label).size(11))
        .on_press(Message::FundAssetSelected(
            if model.fund_asset == protocol::message::Asset::BTC {
                "XMR".to_string()
            } else {
                "BTC".to_string()
            }
        ))
        .padding(iced::Padding::new(6.0).top(4).bottom(4));

    let refresh_button = Button::new(Text::new("Refresh").size(11))
        .on_press(Message::RefreshBalances)
        .padding(iced::Padding::new(8.0).top(4).bottom(4));

    let content = Row::new()
        .spacing(20)
        .padding(iced::Padding::new(10.0).top(6).bottom(6))
        .push(Text::new("Wallet").size(12).width(Length::Fixed(50.0)).color(ACCENT_COLOR))
        .push(balances_col)
        .push(Text::new(xmr_balance).size(11))
        .push(Text::new(btc_balance).size(11))
        .push(fund_asset_button)
        .push(fund_amount_input)
        .push(fund_button)
        .push(refresh_button)
        .push(Text::new("Fund (test)").size(10).color(TEXT_SECONDARY).width(Length::Fill));

    Container::new(content)
        .padding(iced::Padding::new(10.0).top(6).bottom(6))
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(BG_PANEL)),
            ..Default::default()
        })
        .into()
}
