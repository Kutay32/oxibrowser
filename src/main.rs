#![allow(dead_code)]
#![cfg_attr(target_os = "ios", allow(unused_imports))]

/// OxiBrowser - Rust shell + native WebView web tarayıcısı
///
/// # Mimarisi
/// - Browser shell: winit + softbuffer + tiny-skia
/// - Page engine: wry/native WebView (WKWebView/WebView2/WebKitGTK)
/// - Profile persistence: JSON state
/// - Window: winit + softbuffer
/// - Legacy custom parser/layout modules remain for local tests and fallback pages.
mod browser;
mod css;
mod dom;
mod html;
pub mod icons;
mod js;
mod layout;
mod net;
mod platform;
mod render;
mod style;
mod types;
mod ui;
mod url_bar;

use browser::Browser;

#[cfg(not(target_os = "ios"))]
use ui::BrowserUI;

#[cfg(target_os = "macos")]
fn setup_macos_app() {
    platform::init_platform();
    platform::macos::setup_application_menu();
}

#[cfg(not(target_os = "macos"))]
fn setup_macos_app() {}

#[cfg(target_os = "ios")]
fn main() {
    // iOS: This entry point is for reference only.
    // iOS builds use a separate UIKit-based entry via wry.
    // Build with: cargo build --target aarch64-apple-ios --features ios-app
    println!("OxiBrowser iOS - build with Xcode or cargo-xcode");
}

#[cfg(not(target_os = "ios"))]
fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("OxiBrowser başlatılıyor...");
    log::info!(
        "Rust versiyon: {}",
        std::env::var("CARGO_PKG_RUST_VERSION").unwrap_or_else(|_| "bilinmiyor".to_string())
    );

    setup_macos_app();

    let mut browser = Browser::new();

    let args: Vec<String> = std::env::args().collect();
    let startup_url = args.get(1).map(|url| {
        log::info!("URL yükleniyor: {}", url);
        url.as_str()
    });
    browser.restore_session_or_welcome(startup_url);

    if browser.tabs.is_empty() {
        browser.new_tab("");
    }

    log::info!("UI başlatılıyor...");

    let mut browser_ui = BrowserUI::new(browser);
    match browser_ui.run() {
        Ok(()) => {
            log::info!("OxiBrowser kapatıldı.");
        }
        Err(e) => {
            log::error!("UI hatası: {}", e);
            eprintln!("OxiBrowser hatası: {}", e);
        }
    }
}
