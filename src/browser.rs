/// Browser - ana browser durumu ve işlemleri
use crate::css;
use crate::dom::Node;
use crate::html;
use crate::layout::{self, LayoutBox};
use crate::net;
use crate::style;
use std::collections::HashMap;

/// Sekme bilgisi
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: u32,
    pub title: String,
    pub url: String,
    pub history: Vec<String>,
    pub history_pos: usize,
}

/// Browser ana durumu
pub struct Browser {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub current_url: String,
    pub page_title: String,
    pub dom_tree: Option<Node>,
    pub stylesheet: css::Stylesheet,
    pub layout_result: Option<LayoutBox>,
    pub styles: HashMap<crate::dom::NodeId, style::ComputedStyle>,
    pub loading: bool,
    pub status_message: String,
    pub next_tab_id: u32,
    /// Scroll offset (pixels scrolled down from top)
    pub scroll_offset: f32,
    /// URL bar interaction state
    pub url_bar_focused: bool,
    pub url_bar_text: String,
    pub url_bar_select_all: bool,
    /// Viewport height for scroll calculations
    pub viewport_height: f32,
}

impl Browser {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            current_url: String::new(),
            page_title: String::new(),
            dom_tree: None,
            stylesheet: css::Stylesheet::empty(),
            layout_result: None,
            styles: HashMap::new(),
            loading: false,
            status_message: "OxiBrowser'a hoş geldiniz!".to_string(),
            next_tab_id: 1,
            scroll_offset: 0.0,
            url_bar_focused: false,
            url_bar_text: String::new(),
            url_bar_select_all: false,
            viewport_height: 600.0,
        }
    }

    /// Yeni sekme aç
    pub fn new_tab(&mut self, url: &str) -> u32 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let initial_url = if url.trim().is_empty() {
            "about:welcome".to_string()
        } else {
            net::normalize_url(url)
        };

        let tab = Tab {
            id,
            title: if initial_url == "about:welcome" {
                "Yeni Sekme".to_string()
            } else {
                initial_url.clone()
            },
            url: initial_url.clone(),
            history: vec![initial_url],
            history_pos: 0,
        };

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        id
    }

    /// Yeni sekme aç ve içeriğini hazırla.
    pub fn open_new_tab(&mut self, url: Option<&str>) -> u32 {
        let id = self.new_tab(url.unwrap_or(""));
        if let Some(url) = url {
            self.load_url_sync(url);
        } else {
            self.load_welcome_page();
        }
        id
    }

    /// URL'ye git
    pub fn navigate(&mut self, url_str: &str) {
        let normalized = net::normalize_url(url_str);
        self.current_url = normalized.clone();
        self.url_bar_text = normalized.clone();
        self.loading = true;
        self.status_message = format!("{} yükleniyor...", normalized);
    }

    /// Sayfayı yükle (senkron)
    pub fn load_url_sync(&mut self, url_str: &str) {
        self.load_url_sync_with_history(url_str, true);
    }

    fn load_url_sync_without_history(&mut self, url_str: &str) {
        self.load_url_sync_with_history(url_str, false);
    }

    fn load_url_sync_with_history(&mut self, url_str: &str, update_history: bool) {
        self.navigate(url_str);

        let url = net::normalize_url(url_str);
        self.scroll_offset = 0.0;
        self.status_message = format!("{} yükleniyor...", url);

        if url.is_empty() || url == "about:welcome" {
            self.render_about_page("about:welcome", update_history);
            return;
        }

        if url.starts_with("about:") {
            self.render_about_page(&url, update_history);
            return;
        }

        if url.starts_with("file://") {
            self.load_response_result(net::load_file_url(&url), &url, update_history);
            return;
        }

        // Blocking HTTP isteği
        let response = match Self::blocking_fetch(&url) {
            Ok(resp) => resp,
            Err(e) => {
                self.status_message = format!("Hata: {}", e);
                self.loading = false;
                // Hata sayfası oluştur
                let error_html = self.error_page_html("Bağlantı Hatası", &url, &e);
                self.parse_and_render(&error_html, &url);
                self.finish_navigation(&url, update_history);
                return;
            }
        };

        self.load_response_result(Ok(response), &url, update_history);
    }

    fn load_response_result(
        &mut self,
        result: Result<net::HttpResponse, String>,
        requested_url: &str,
        update_history: bool,
    ) {
        let response = match result {
            Ok(response) => response,
            Err(e) => {
                self.status_message = format!("Hata: {}", e);
                self.loading = false;
                let error_html = self.error_page_html("Sayfa Açılamadı", requested_url, &e);
                self.parse_and_render(&error_html, requested_url);
                self.finish_navigation(requested_url, update_history);
                return;
            }
        };

        self.status_message = format!("Durum: {}", response.status);
        let final_url = response.url.clone();

        // Content-Type kontrol et
        let is_html = response
            .headers
            .get("content-type")
            .map(|ct| ct.contains("text/html") || ct.contains("text/plain"))
            .unwrap_or(true);

        if is_html || response.body.trim().starts_with('<') {
            self.parse_and_render(&response.body, &final_url);
        } else {
            // HTML değil - metin olarak göster
            let wrapped = format!(
                "<html><body><pre>{}</pre></body></html>",
                Self::escape_html(&response.body.chars().take(10000).collect::<String>())
            );
            self.parse_and_render(&wrapped, &final_url);
        }

        self.current_url = final_url;
        self.url_bar_text = self.current_url.clone();
        self.loading = false;
        let current_url = self.current_url.clone();
        self.finish_navigation(&current_url, update_history);
    }

    /// Varsayılan hoş geldin sayfasını yükle.
    pub fn load_welcome_page(&mut self) {
        self.render_about_page("about:welcome", false);
    }

    /// Blocking HTTP isteği
    fn blocking_fetch(url: &str) -> Result<net::HttpResponse, String> {
        // Tokio runtime ile blocking fetch
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime: {}", e))?;
        rt.block_on(net::fetch_url(url))
    }

    /// HTML'i parse et, stilleri uygula, layout ve render
    pub fn parse_and_render(&mut self, html_text: &str, base_url: &str) {
        self.current_url = base_url.to_string();
        self.url_bar_text = base_url.to_string();
        self.url_bar_select_all = false;

        // HTML parse et
        self.dom_tree = Some(html::parse_html_simple(html_text));

        let dom = self.dom_tree.as_ref().unwrap();

        // Sayfa başlığını bul
        self.page_title = Self::extract_title(dom);

        // Satır içi CSS'i bul
        let inline_css = Self::extract_styles(dom);
        self.stylesheet = css::parse_css(&inline_css);

        // Stilleri uygula
        self.styles = style::apply_stylesheet(dom, &self.stylesheet);

        // Layout ağacı oluştur
        self.layout_result = layout::build_layout_tree(dom, &self.styles);

        // Layout hesapla
        if let Some(ref mut layout_root) = self.layout_result {
            layout::layout_document(layout_root, 1024.0, 2000.0);
        }

        // Sekme başlığını güncelle
        if !self.tabs.is_empty() {
            let tab = &mut self.tabs[self.active_tab];
            tab.title = self.page_title.clone();
            tab.url = base_url.to_string();
        }
    }

    /// Mevcut layout'u yeni viewport boyutuna gore tekrar hesapla.
    pub fn relayout(&mut self, viewport_width: f32, viewport_height: f32) {
        self.viewport_height = viewport_height;
        if let Some(ref mut layout_root) = self.layout_result {
            layout::layout_document(layout_root, viewport_width, viewport_height);
        }
    }

    /// Sayfayı kaydır (delta pixels, pozitif = aşağı)
    pub fn scroll_by(&mut self, delta: f32) {
        let max_scroll = self.max_scroll_offset();
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll);
    }

    /// Maksimum kaydırma mesafesi
    pub fn max_scroll_offset(&self) -> f32 {
        if let Some(ref layout_root) = self.layout_result {
            let content_h = layout_root.dimensions.margin_box().bottom();
            let viewport_h = self.viewport_height.max(100.0);
            (content_h - viewport_h).max(0.0)
        } else {
            0.0
        }
    }

    /// URL bar metnini güncelle
    pub fn set_url_bar_text(&mut self, text: &str) {
        self.url_bar_text = text.to_string();
        self.url_bar_select_all = false;
    }

    /// URL bar'ı odakla.
    pub fn focus_url_bar(&mut self, select_all: bool) {
        self.url_bar_focused = true;
        self.url_bar_select_all = select_all;
        self.url_bar_text = self.current_url.clone();
    }

    /// URL bar odaktan çıksın.
    pub fn blur_url_bar(&mut self) {
        self.url_bar_focused = false;
        self.url_bar_select_all = false;
        self.url_bar_text = self.current_url.clone();
    }

    /// URL bar'a yazı ekle.
    pub fn push_url_bar_text(&mut self, text: &str) {
        if self.url_bar_select_all {
            self.url_bar_text.clear();
            self.url_bar_select_all = false;
        }
        self.url_bar_text.push_str(text);
    }

    /// URL bar'dan karakter sil.
    pub fn backspace_url_bar(&mut self) {
        if self.url_bar_select_all {
            self.url_bar_text.clear();
            self.url_bar_select_all = false;
        } else {
            self.url_bar_text.pop();
        }
    }

    /// URL bar'dan navigasyon yap
    pub fn navigate_from_url_bar(&mut self) {
        let url = self.url_bar_text.clone();
        if !url.is_empty() {
            self.scroll_offset = 0.0;
            self.load_url_sync(&url);
        }
    }

    /// Sayfa koordinatındaki link hedefini bul.
    pub fn link_at(&self, x: f32, y: f32) -> Option<String> {
        let dom = self.dom_tree.as_ref()?;
        let layout_root = self.layout_result.as_ref()?;
        let mut link_targets = HashMap::new();
        Self::collect_link_targets(dom, None, &mut link_targets);
        Self::hit_test_link(layout_root, x, y, &link_targets)
            .map(|href| net::resolve_url(&self.current_url, &href))
    }

    /// HTML'den başlık çıkar
    fn extract_title(dom: &Node) -> String {
        if let Some(title_node) = dom.get_element_by_id("title") {
            let text = title_node.text_content().trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }

        // title tag'ini ara
        let titles: Vec<&Node> = dom.elements_by_tag("title");
        if let Some(title) = titles.first() {
            let text = title.text_content().trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }

        // h1 tag'ini ara
        let h1s: Vec<&Node> = dom.elements_by_tag("h1");
        if let Some(h1) = h1s.first() {
            let text = h1.text_content().trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }

        "OxiBrowser".to_string()
    }

    /// Satır içi CSS stillerini çıkar
    fn extract_styles(dom: &Node) -> String {
        let mut css_text = String::new();
        let styles: Vec<&Node> = dom.elements_by_tag("style");
        for style_node in &styles {
            css_text.push_str(&style_node.text_content());
            css_text.push('\n');
        }
        css_text
    }

    /// Sayfayı yenile
    pub fn refresh(&mut self) {
        let url = self.current_url.clone();
        if !url.is_empty() {
            self.load_url_sync_without_history(&url);
        }
    }

    /// Geri git
    pub fn go_back(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.history_pos > 0 {
            tab.history_pos -= 1;
            let url = tab.history[tab.history_pos].clone();
            self.load_url_sync_without_history(&url);
            true
        } else {
            false
        }
    }

    /// İleri git
    pub fn go_forward(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.history_pos < tab.history.len().saturating_sub(1) {
            tab.history_pos += 1;
            let url = tab.history[tab.history_pos].clone();
            self.load_url_sync_without_history(&url);
            true
        } else {
            false
        }
    }

    /// Sekmeyi kapat
    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() && self.tabs.len() > 1 {
            self.tabs.remove(index);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            self.reload_active_tab_without_history();
        }
    }

    /// Sıradaki sekmeye geç.
    pub fn switch_to_next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.reload_active_tab_without_history();
        }
    }

    fn reload_active_tab_without_history(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let url = tab.url.clone();
            self.load_url_sync_without_history(&url);
        }
    }

    fn finish_navigation(&mut self, url: &str, update_history: bool) {
        self.current_url = url.to_string();
        self.url_bar_text = url.to_string();
        self.url_bar_select_all = false;
        self.loading = false;

        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        tab.url = url.to_string();
        if !self.page_title.trim().is_empty() {
            tab.title = self.page_title.clone();
        }

        if update_history {
            let already_current = tab
                .history
                .get(tab.history_pos)
                .map(|current| current == url)
                .unwrap_or(false);
            if !already_current {
                tab.history.truncate(tab.history_pos + 1);
                tab.history.push(url.to_string());
                tab.history_pos = tab.history.len().saturating_sub(1);
            }
        }
    }

    fn render_about_page(&mut self, url: &str, update_history: bool) {
        let html = match url {
            "about:blank" => {
                "<html><head><title>Boş Sayfa</title></head><body></body></html>".to_string()
            }
            "about:version" => self.about_version_html(),
            "about:welcome" => self.welcome_html(),
            _ => self.error_page_html("Bilinmeyen about sayfası", url, "Bu iç sayfa bulunamadı."),
        };

        self.status_message = "Hazır".to_string();
        self.parse_and_render(&html, url);
        self.finish_navigation(url, update_history);
    }

    fn collect_link_targets(
        node: &Node,
        current_href: Option<String>,
        targets: &mut HashMap<crate::dom::NodeId, String>,
    ) {
        let href = match node.as_element() {
            Some(el) if el.tag_name == "a" => el
                .attributes
                .get("href")
                .filter(|href| !href.trim().is_empty())
                .cloned()
                .or(current_href),
            _ => current_href,
        };

        if let Some(ref href) = href {
            targets.insert(node.id, href.clone());
        }

        for child in &node.children {
            Self::collect_link_targets(child, href.clone(), targets);
        }
    }

    fn hit_test_link(
        layout_box: &LayoutBox,
        x: f32,
        y: f32,
        targets: &HashMap<crate::dom::NodeId, String>,
    ) -> Option<String> {
        for child in layout_box.children.iter().rev() {
            if let Some(url) = Self::hit_test_link(child, x, y, targets) {
                return Some(url);
            }
        }

        let rect = layout_box.dimensions.border_box();
        if rect.contains(x, y) {
            return targets.get(&layout_box.node_id).cloned();
        }

        None
    }

    fn welcome_html(&self) -> String {
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>OxiBrowser</title>
    <style>
        body { font-family: Helvetica, Arial, sans-serif; margin: 36px; background: #f8fafc; color: #1f2937; }
        .container { max-width: 760px; margin: 0 auto; background: white; padding: 28px; border: 1px solid #d7dee8; }
        h1 { margin: 0 0 8px 0; color: #0f766e; font-size: 34px; }
        p { font-size: 16px; line-height: 1.55; color: #334155; }
        a { color: #2563eb; }
        .feature { margin: 12px 0; padding: 12px 14px; border-left: 4px solid #0f766e; background: #f0fdfa; }
        .feature h3 { margin: 0 0 5px 0; color: #115e59; font-size: 17px; }
        code { color: #7c2d12; }
    </style>
</head>
<body>
    <div class="container">
        <h1>OxiBrowser</h1>
        <p>%100 Rust ile yazılmış minimal web tarayıcısı.</p>
        <div class="feature"><h3>Gezinme</h3><p>URL bar, geri/ileri, yenileme, sekme ve kaydırma akışı çalışır.</p></div>
        <div class="feature"><h3>Sayfa motoru</h3><p>HTML parsing, CSS cascade, block layout ve tiny-skia render hattı yereldir.</p></div>
        <div class="feature"><h3>Yerel dosyalar</h3><p><code>file://</code> URL'leri ve yerel HTML dosyaları açılabilir.</p></div>
        <p><a href="about:version">Sürüm bilgisi</a></p>
    </div>
</body>
</html>
"#
        .to_string()
    }

    fn about_version_html(&self) -> String {
        format!(
            r#"
<html>
<head>
    <title>OxiBrowser Sürüm</title>
    <style>
        body {{ font-family: Helvetica, Arial, sans-serif; margin: 32px; color: #1f2937; }}
        .box {{ max-width: 680px; border: 1px solid #d7dee8; padding: 24px; }}
        h1 {{ margin-top: 0; color: #0f766e; }}
        p {{ line-height: 1.55; }}
    </style>
</head>
<body>
    <div class="box">
        <h1>OxiBrowser {}</h1>
        <p>Rust, winit, softbuffer ve tiny-skia ile çalışan MVP tarayıcı.</p>
        <p><a href="about:welcome">Başlangıç sayfasına dön</a></p>
    </div>
</body>
</html>
"#,
            env!("CARGO_PKG_VERSION")
        )
    }

    fn error_page_html(&self, title: &str, url: &str, message: &str) -> String {
        format!(
            r#"
<html>
<head>
    <title>{title}</title>
    <style>
        body {{ font-family: Helvetica, Arial, sans-serif; margin: 32px; background: #fff7ed; color: #1f2937; }}
        .box {{ max-width: 760px; background: white; border: 1px solid #fed7aa; padding: 24px; }}
        h1 {{ margin-top: 0; color: #c2410c; }}
        code {{ color: #7c2d12; }}
    </style>
</head>
<body>
    <div class="box">
        <h1>{title}</h1>
        <p><code>{url}</code></p>
        <p>{message}</p>
    </div>
</body>
</html>
"#,
            title = Self::escape_html(title),
            url = Self::escape_html(url),
            message = Self::escape_html(message)
        )
    }

    fn escape_html(text: &str) -> String {
        let mut escaped = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&#39;"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_navigation_updates_history() {
        let mut browser = Browser::new();
        browser.new_tab("");

        browser.load_url_sync("about:welcome");
        browser.load_url_sync("about:version");

        assert_eq!(browser.current_url, "about:version");
        assert!(browser.go_back());
        assert_eq!(browser.current_url, "about:welcome");
        assert!(browser.go_forward());
        assert_eq!(browser.current_url, "about:version");
    }

    #[test]
    fn link_hit_testing_resolves_relative_href() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            r#"<html><body><p><a href="/docs">Docs</a></p></body></html>"#,
            "https://example.com/start",
        );

        let mut href = None;
        for y in 0..120 {
            for x in 0..220 {
                href = browser.link_at(x as f32, y as f32);
                if href.is_some() {
                    break;
                }
            }
            if href.is_some() {
                break;
            }
        }

        assert_eq!(href.as_deref(), Some("https://example.com/docs"));
    }

    #[test]
    fn stylesheet_is_reset_between_pages() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            "<html><head><style>body { background: red; }</style></head><body>One</body></html>",
            "about:first",
        );
        assert!(!browser.stylesheet.rules.is_empty());

        browser.parse_and_render("<html><body>Two</body></html>", "about:second");
        assert!(browser.stylesheet.rules.is_empty());
    }
}
