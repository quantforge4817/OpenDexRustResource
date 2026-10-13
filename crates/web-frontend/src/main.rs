use leptos::prelude::*;
use web_frontend::shell;

fn main() {
    _ = console_error_panic_hook::set_once();

    mount_to_body(|| {
        shell()
    });
}
