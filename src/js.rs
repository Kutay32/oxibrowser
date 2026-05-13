/// JavaScript runtime - placeholder (boa_engine devre dışı)
/// Not: boa_engine v0.18 Rust 1.95 ile uyumlu değil.
/// İleride güncellenecek.
use crate::dom::Node;

/// JavaScript runtime durumu
#[derive(Debug)]
pub struct JsRuntime {
    ready: bool,
}

impl JsRuntime {
    pub fn new() -> Self {
        Self { ready: false }
    }

    /// JavaScript motorunu başlat
    pub fn init(&mut self) -> Result<(), String> {
        log::info!("JavaScript runtime (placeholder) başlatıldı");
        self.ready = true;
        Ok(())
    }

    /// JavaScript kodunu çalıştır
    pub fn evaluate(&mut self, code: &str, _dom_context: Option<&Node>) -> Result<String, String> {
        if !self.ready {
            self.init()?;
        }

        log::info!(
            "JavaScript kodu (placeholder) çalıştırıldı ({} karakter)",
            code.len()
        );

        Ok("[JavaScript: Bu sürümde JS çalıştırma devre dışı]".to_string())
    }

    /// DOM'dan script elementlerini bul ve çalıştır
    pub fn execute_scripts(&mut self, _dom: &Node) -> Vec<String> {
        Vec::new()
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}
