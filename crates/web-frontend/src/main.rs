use leptos::prelude::*;
use web_frontend::shell;

fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(shell);
}
