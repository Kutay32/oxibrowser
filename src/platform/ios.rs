pub fn handle_ios_events() {
}

pub fn init_ios() {
    setup_webview_config();
}

fn setup_webview_config() {
}

pub fn is_running_on_ios() -> bool {
    #[cfg(target_os = "ios")]
    { true }
    #[cfg(not(target_os = "ios"))]
    { false }
}
