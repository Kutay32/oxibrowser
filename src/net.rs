/// HTTP networking - reqwest ile web sayfası yükleme
use std::collections::HashMap;

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

/// URL'yi normalize et (varsayılan protokol, düzeltmeler)
pub fn normalize_url(input: &str) -> String {
    let input = input.trim();

    if input.is_empty() {
        return String::new();
    }

    // Zaten protokol varsa olduğu gibi kullan
    if input.starts_with("http://") || input.starts_with("https://") {
        return input.to_string();
    }

    // "example.com" veya "www.example.com" → https:// ile başla
    if !input.contains(' ') {
        return format!("https://{}", input);
    }

    // Arama sorgusu olabilir → varsayılan arama motoruna yönlendir
    let encoded: String = input
        .chars()
        .map(|c| if c == ' ' { '+' } else { c })
        .collect();
    format!("https://www.google.com/search?q={}", encoded)
}
