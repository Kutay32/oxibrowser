/// OxiBrowser - %100 Rust web tarayıcısı
///
/// # Mimarisi
/// - HTML parsing: Kendi implementasyonumuz (SimpleHtmlParser)
/// - CSS parsing: Kendi implementasyonumuz
/// - Layout engine: Box model, block/inline layout
/// - Render engine: tiny-skia ile 2D çizim
/// - Networking: reqwest + rustls
/// - Window: winit + softbuffer
/// - JavaScript: boa_engine (temel destek)
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
        // Demo sayfa yükle
        let demo_html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>OxiBrowser</title>
    <style>
        body {
            font-family: Helvetica, Arial, sans-serif;
            margin: 30px;
            background-color: #f5f5f5;
        }
        h1 {
            color: #e67e22;
            text-align: center;
            font-size: 36px;
        }
        .container {
            max-width: 700px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 10px;
            border: 1px solid #ddd;
        }
        p {
            font-size: 16px;
            line-height: 1.6;
            color: #333;
        }
        .feature {
            background: #fef9e7;
            padding: 10px 15px;
            margin: 10px 0;
            border-left: 4px solid #e67e22;
        }
        .feature h3 {
            margin: 0 0 5px 0;
            color: #e67e22;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🔥 OxiBrowser</h1>
        <p>%100 Rust ile yazılmış web tarayıcısına hoş geldiniz!</p>

        <div class="feature">
            <h3>🦀 HTML & CSS</h3>
            <p>Kendi HTML parser ve CSS motorumuz ile çalışır.</p>
        </div>

        <div class="feature">
            <h3>📐 Layout Engine</h3>
            <p>Block ve inline layout desteği ile sayfaları düzenler.</p>
        </div>

        <div class="feature">
            <h3>🎨 Render</h3>
            <p>tiny-skia ile GPU benzeri 2D çizim.</p>
        </div>

        <div class="feature">
            <h3>⚡ JavaScript</h3>
            <p>boa_engine ile JS desteği.</p>
        </div>

        <p style="text-align: center; margin-top: 20px; color: #999;">
            OxiBrowser v0.1.0
        </p>
    </div>
</body>
</html>
        "#;
        browser.parse_and_render(demo_html, "about:welcome");
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
