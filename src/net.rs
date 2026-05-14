/// HTTP networking - reqwest ile web sayfası yükleme
use std::collections::HashMap;
use std::path::Path;
use url::{form_urlencoded, Url};

/// HTTP yanıtı
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// Web sayfasını yükle
pub async fn fetch_url(url_str: &str) -> Result<HttpResponse, String> {
    let client = reqwest::Client::builder()
        .user_agent("OxiBrowser/0.1.0 (Rust)")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("İstemci oluşturulamadı: {}", e))?;

    let response = client
        .get(url_str)
        .send()
        .await
        .map_err(|e| format!("İstek hatası: {}", e))?;

    let status = response.status().as_u16();
    let final_url = response.url().to_string();

    // Header'ları al
    let mut headers = HashMap::new();
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string(), v.to_string());
        }
    }

    // Body'yi al
    let body = response
        .text()
        .await
        .map_err(|e| format!("Body okuma hatası: {}", e))?;

    Ok(HttpResponse {
        status,
        url: final_url,
        headers,
        body,
    })
}

/// Yerel dosyayı file:// URL'sinden yükle.
pub fn load_file_url(url_str: &str) -> Result<HttpResponse, String> {
    let parsed = Url::parse(url_str).map_err(|e| format!("Dosya URL'si hatalı: {}", e))?;
    let path = parsed
        .to_file_path()
        .map_err(|_| "Dosya yolu çözülemedi".to_string())?;
    let bytes = std::fs::read(&path).map_err(|e| format!("Dosya okunamadı: {}", e))?;
    let body = String::from_utf8_lossy(&bytes).into_owned();
    let mut headers = HashMap::new();
    headers.insert(
        "content-type".to_string(),
        content_type_for_path(&path).to_string(),
    );

    Ok(HttpResponse {
        status: 200,
        url: url_str.to_string(),
        headers,
        body,
    })
}

/// URL'yi normalize et (varsayılan protokol, düzeltmeler)
pub fn normalize_url(input: &str) -> String {
    let input = input.trim();

    if input.is_empty() {
        return String::new();
    }

    // Zaten desteklenen protokol varsa olduğu gibi kullan.
    if input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("file://")
        || input.starts_with("about:")
    {
        return input.to_string();
    }

    if let Some(file_url) = local_path_to_file_url(input) {
        return file_url;
    }

    if is_localhost_like(input) {
        return format!("http://{}", input);
    }

    // "example.com" veya "www.example.com" → https:// ile başla.
    if looks_like_host(input) {
        return format!("https://{}", input);
    }

    // Arama sorgusu olabilir → varsayılan arama motoruna yönlendir
    let encoded: String = form_urlencoded::byte_serialize(input.as_bytes()).collect();
    format!("https://www.google.com/search?q={}", encoded)
}

/// Göreli linkleri aktif sayfaya göre mutlak URL'ye çevir.
pub fn resolve_url(base_url: &str, href: &str) -> String {
    let href = href.trim();
    if href.is_empty() {
        return base_url.to_string();
    }
    if href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("file://")
        || href.starts_with("about:")
    {
        return href.to_string();
    }

    if let Ok(base) = Url::parse(base_url) {
        if let Ok(joined) = base.join(href) {
            return joined.to_string();
        }
    }

    normalize_url(href)
}

fn local_path_to_file_url(input: &str) -> Option<String> {
    let path = Path::new(input);
    let looks_like_path = path.is_absolute() || input.starts_with("./") || input.starts_with("../");
    if !looks_like_path && !path.exists() {
        return None;
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    Url::from_file_path(absolute)
        .ok()
        .map(|url| url.to_string())
}

fn looks_like_host(input: &str) -> bool {
    !input.contains(char::is_whitespace)
        && (input.contains('.') || input.starts_with("www.") || input.contains(':'))
}

fn is_localhost_like(input: &str) -> bool {
    !input.contains(char::is_whitespace)
        && (input == "localhost"
            || input.starts_with("localhost:")
            || input.starts_with("127.")
            || input.starts_with("[::1]"))
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "txt" | "md" | "rs" | "toml" | "lock" => "text/plain; charset=utf-8",
        _ => "text/plain; charset=utf-8",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_localhost_to_http() {
        assert_eq!(normalize_url("localhost:3000"), "http://localhost:3000");
        assert_eq!(normalize_url("127.0.0.1:8080"), "http://127.0.0.1:8080");
    }

    #[test]
    fn resolves_relative_links_against_base() {
        assert_eq!(
            resolve_url("https://example.com/a/b/index.html", "../next"),
            "https://example.com/a/next"
        );
    }
}
