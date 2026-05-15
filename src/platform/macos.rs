pub fn init_macos() {
    // macOS initialization is handled by winit/wry internally.
    // winit sets up NSApplication and the event loop.
    // wry handles WKWebView configuration.
}

pub fn setup_application_menu() {
    // winit creates the basic application menu automatically.
    // Custom NSMenu/NSMenuItem setup can be added via objc2-app-kit.
}
