pub mod macos;
pub mod ios;

pub fn init_platform() {
    #[cfg(target_os = "macos")]
    macos::init_macos();
}

pub fn handle_apple_events() {
    #[cfg(target_os = "ios")]
    ios::handle_ios_events();
}
