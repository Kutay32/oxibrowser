#![allow(dead_code)]

/// OxiBrowser - %100 Rust web tarayıcısı
///
/// # Mimarisi
/// - HTML parsing: Kendi implementasyonumuz (SimpleHtmlParser)
/// - CSS parsing: Kendi implementasyonumuz
/// - Layout engine: Box model, block/inline layout
/// - Render engine: tiny-skia ile 2D çizim
/// - Networking: reqwest + rustls
/// - Window: winit + softbuffer
/// - JavaScript: placeholder (MVP'de devre dışı)
///
/// # %100 Rust
/// Hiçbir C/C++ kütüphanesi kullanılmamıştır.
/// Tüm bağımlılıklar saf Rust implementasyonlarıdır.
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

    // Varsayılan sekmeyi aç
    let _tab_id = browser.new_tab("");

    // Varsa komut satırı argümanından URL al
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let url = &args[1];
        log::info!("URL yükleniyor: {}", url);
        browser.load_url_sync(url);
    } else {
        browser.load_welcome_page();
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
