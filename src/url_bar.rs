/// URL bar - URL parsing ve gezinme
use url::Url;

/// URL ayrıştırma sonucu
#[derive(Debug, Clone)]
pub struct ParsedUrl {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub original: String,
}

/// URL'yi parse et
pub fn parse_url(url_str: &str) -> Result<ParsedUrl, String> {
    let normalized = crate::net::normalize_url(url_str);
    let parsed = Url::parse(&normalized).map_err(|e| format!("URL ayrıştırma hatası: {}", e))?;

    Ok(ParsedUrl {
        scheme: parsed.scheme().to_string(),
        host: parsed.host_str().unwrap_or("").to_string(),
        port: parsed.port(),
        path: parsed.path().to_string(),
        query: parsed.query().map(|s| s.to_string()),
        fragment: parsed.fragment().map(|s| s.to_string()),
        original: normalized,
    })
}

/// URL'yi güvenli gösterimle döndür (parola varsa gizle)
pub fn safe_url(url_str: &str) -> String {
    if let Ok(parsed) = Url::parse(url_str) {
        if parsed.password().is_some() {
            let mut safe = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
            if let Some(port) = parsed.port() {
                safe.push_str(&format!(":{}", port));
            }
            safe.push_str(parsed.path());
            if let Some(query) = parsed.query() {
                safe.push_str(&format!("?{}", query));
            }
            return safe;
        }
    }
    url_str.to_string()
}

/// URL'nin temel kısmını döndür (path'ler için base URL)
pub fn base_url(url_str: &str) -> String {
    if let Ok(parsed) = Url::parse(url_str) {
        if let Some(base) = parsed.join(".").ok() {
            if let Some(base_str) = base.as_str().strip_suffix(".") {
                return base_str.to_string();
            }
        }
        format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""))
    } else {
        url_str.to_string()
    }
}

/// URL'deki dosya uzantısını döndür
pub fn file_extension(url_str: &str) -> Option<String> {
    let path = if let Ok(parsed) = Url::parse(url_str) {
        parsed.path().to_string()
    } else {
        url_str.to_string()
    };

    let path = path.trim_end_matches('/');
    if let Some(dot_pos) = path.rfind('.') {
        let ext = path[dot_pos + 1..].to_lowercase();
        if ext.len() <= 5 && ext.chars().all(|c| c.is_alphanumeric()) {
            return Some(ext);
        }
    }
    None
}
