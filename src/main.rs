#![allow(dead_code)]

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
mod js;
mod layout;
mod net;
mod render;
mod style;
mod types;
mod ui;
mod url_bar;

use browser::Browser;
use ui::BrowserUI;

fn main() {
    // Logger başlat
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("OxiBrowser başlatılıyor...");
    log::info!(
        "Rust versiyon: {}",
        std::env::var("CARGO_PKG_RUST_VERSION").unwrap_or_else(|_| "bilinmiyor".to_string())
    );

    // Browser oluştur
    let mut browser = Browser::new();

    // Varsa komut satırı argümanından URL al
    let args: Vec<String> = std::env::args().collect();
    let startup_url = args.get(1).map(|url| {
        log::info!("URL yükleniyor: {}", url);
        url.as_str()
    });
    browser.restore_session_or_welcome(startup_url);

    if browser.tabs.is_empty() {
        browser.new_tab("");
    }

    // UI'ı başlat
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
