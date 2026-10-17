//! Authentication screens: login, registration, and email verification.

use iced::widget::{Column, Container, Row, Text, TextInput, Button};
use iced::{Length, Element};

use crate::app::Message;
use crate::model::GuiModel;

/// Render the authentication overlay screen.
pub fn auth_panel<'a>(model: &'a GuiModel) -> Element<'a, Message> {
    let content = match model.auth_screen {
        crate::model::AuthScreen::Login => login_form(model),
        crate::model::AuthScreen::Register => register_form(model),
        crate::model::AuthScreen::VerifyEmail => verify_email_form(model),
        crate::model::AuthScreen::Profile | crate::model::AuthScreen::ResetPassword => login_form(model),
    };

    let centered = Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    Container::new(centered)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}

fn login_form<'a>(model: &'a GuiModel) -> Column<'a, Message> {
    let mut column = Column::new()
        .width(Length::Fixed(400.0))
        .padding(30)
        .spacing(15);

    column = column.push(Text::new("OpenDex Login").size(28));

    if let Some(ref msg) = model.auth_message {
        column = column.push(Text::new(msg.clone()).color(iced::Color::from_rgb(1.0, 0.3, 0.3)));
    }

    column = column.push(
        TextInput::new("Username", &model.login_username)
            .on_input(Message::LoginUsernameChanged),
    );

    column = column.push(
        TextInput::new("Password", &model.login_password)
            .on_input(Message::LoginPasswordChanged)
            .on_submit(Message::LoginSubmit)
            .secure(true),
    );

    column = column.push(
        Button::new(Text::new("Login"))
            .on_press(Message::LoginSubmit)
            .width(Length::Fill),
    );

    column = column.push(
        Row::new()
            .spacing(10)
            .push(Text::new("Don't have an account?"))
            .push(
                Button::new(Text::new("Register"))
                    .on_press(Message::SwitchToRegister),
            ),
    );

    column
}

fn register_form<'a>(model: &'a GuiModel) -> Column<'a, Message> {
    let mut column = Column::new()
        .width(Length::Fixed(450.0))
        .padding(30)
        .spacing(10);

    column = column.push(Text::new("Create Account").size(28));

    if let Some(ref msg) = model.auth_message {
        column = column.push(Text::new(msg.clone()).color(iced::Color::from_rgb(1.0, 0.3, 0.3)));
    }

    column = column.push(
        TextInput::new("Username (3-32 chars)", &model.reg_username)
            .on_input(Message::RegUsernameChanged),
    );

    column = column.push(
        TextInput::new("Password (min 8 chars)", &model.reg_password)
            .on_input(Message::RegPasswordChanged)
            .secure(true),
    );

    column = column.push(
        TextInput::new("Email address", &model.reg_email)
            .on_input(Message::RegEmailChanged),
    );

    column = column.push(Text::new("Wallet Addresses (optional)").size(14));

    column = column.push(
        TextInput::new("Bitcoin wallet address", &model.reg_btc_wallet)
            .on_input(Message::RegBtcWalletChanged),
    );

    column = column.push(
        TextInput::new("Monero wallet address", &model.reg_xmr_wallet)
            .on_input(Message::RegXmrWalletChanged),
    );

    column = column.push(
        Button::new(Text::new("Create Account"))
            .on_press(Message::RegisterSubmit)
            .width(Length::Fill),
    );

    column = column.push(
        Row::new()
            .spacing(10)
            .push(Text::new("Already have an account?"))
            .push(
                Button::new(Text::new("Login"))
                    .on_press(Message::SwitchToLogin),
            ),
    );

    column
}

fn verify_email_form<'a>(model: &'a GuiModel) -> Column<'a, Message> {
    let mut column = Column::new()
        .width(Length::Fixed(400.0))
        .padding(30)
        .spacing(15);

    column = column.push(Text::new("Verify Email").size(28));

    column = column.push(
        Text::new(
            "Enter the 6-digit verification code sent to your email. \
             (For development, check the console logs for the code.)",
        )
        .size(14),
    );

    if let Some(ref msg) = model.auth_message {
        column = column.push(Text::new(msg.clone()).color(iced::Color::from_rgb(1.0, 0.3, 0.3)));
    }

    column = column.push(
        TextInput::new("Verification Code", &model.verify_code)
            .on_input(Message::VerifyCodeChanged)
            .on_submit(Message::VerifyEmailSubmit),
    );

    column = column.push(
        Button::new(Text::new("Verify"))
            .on_press(Message::VerifyEmailSubmit)
            .width(Length::Fill),
    );

    column = column.push(
        Button::new(Text::new("Resend Code"))
            .on_press(Message::ResendVerificationCode)
            .width(Length::Fill),
    );

    column
}
