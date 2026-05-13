/// Browser - ana browser durumu ve işlemleri
use crate::css;
use crate::dom::{self, Node};
use crate::html;
use crate::layout::{self, LayoutBox};
use crate::net;
use crate::render;
use crate::style;
use crate::url_bar;
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
        }
    }

    /// Yeni sekme aç
    pub fn new_tab(&mut self, url: &str) -> u32 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let tab = Tab {
            id,
            title: if url.is_empty() {
                "Yeni Sekme".to_string()
            } else {
                url.to_string()
            },
            url: url.to_string(),
            history: vec![url.to_string()],
            history_pos: 0,
        };

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        id
    }

    /// URL'ye git
    pub fn navigate(&mut self, url_str: &str) {
        let normalized = net::normalize_url(url_str);
        self.current_url = normalized.clone();
        self.loading = true;
        self.status_message = format!("{} yükleniyor...", normalized);
    }

    /// Sayfayı yükle (senkron)
    pub fn load_url_sync(&mut self, url_str: &str) {
        self.navigate(url_str);

        let url = net::normalize_url(url_str);
        self.status_message = format!("{} yükleniyor...", url);

        // Blocking HTTP isteği
        let response = match Self::blocking_fetch(&url) {
            Ok(resp) => resp,
            Err(e) => {
                self.status_message = format!("Hata: {}", e);
                self.loading = false;
                // Hata sayfası oluştur
                let error_html = format!(
                    "<html><body><h1>Bağlantı Hatası</h1><p>{}</p></body></html>",
                    e
                );
                self.parse_and_render(&error_html, &url);
                return;
            }
        };

        self.status_message = format!("Durum: {}", response.status);

        // Content-Type kontrol et
        let is_html = response
            .headers
            .get("content-type")
            .map(|ct| ct.contains("text/html") || ct.contains("text/plain"))
            .unwrap_or(true);

        if is_html || response.body.trim().starts_with('<') {
            self.parse_and_render(&response.body, &url);
        } else {
            // HTML değil - metin olarak göster
            let wrapped = format!(
                "<html><body><pre>{}</pre></body></html>",
                response.body.chars().take(10000).collect::<String>()
            );
            self.parse_and_render(&wrapped, &url);
        }

        self.current_url = response.url;
        self.loading = false;
    }

    /// Blocking HTTP isteği
    fn blocking_fetch(url: &str) -> Result<net::HttpResponse, String> {
        // Tokio runtime ile blocking fetch
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Runtime: {}", e))?;
        rt.block_on(net::fetch_url(url))
    }

    /// HTML'i parse et, stilleri uygula, layout ve render
    pub fn parse_and_render(&mut self, html_text: &str, base_url: &str) {
        // HTML parse et
        self.dom_tree = Some(html::parse_html_simple(html_text));

        let dom = self.dom_tree.as_ref().unwrap();

        // Sayfa başlığını bul
        self.page_title = Self::extract_title(dom);

        // Satır içi CSS'i bul
        let inline_css = Self::extract_styles(dom);
        if !inline_css.is_empty() {
            self.stylesheet = css::parse_css(&inline_css);
        }

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
            self.load_url_sync(&url);
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
            self.load_url_sync(&url);
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
            self.load_url_sync(&url);
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
        }
    }
}
