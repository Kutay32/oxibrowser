/// CSS parser - stylesheet, rules, selectors, declarations
use std::collections::HashMap;

/// CSS stil sayfası
#[derive(Debug, Clone)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }
}

/// CSS kuralı (selector { declarations })
#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// CSS seçici
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// div, p, h1
    Tag(String),
    /// .classname
    Class(String),
    /// #id
    Id(String),
    /// * (evrensel)
    Universal,
    /// div.class
    Compound(Vec<Selector>),
    /// div p (torun)
    Descendant {
        ancestor: Box<Selector>,
        descendant: Box<Selector>,
    },
    /// div > p (çocuk)
    Child {
        parent: Box<Selector>,
        child: Box<Selector>,
    },
    /// div + p (bitişik kardeş)
    AdjacentSibling {
        first: Box<Selector>,
        second: Box<Selector>,
    },
    /// [attr]
    AttributeExists(String),
    /// [attr="value"]
    AttributeEquals(String, String),
    /// :hover, :first-child, etc.
    PseudoClass(String),
}

/// CSS bildirimi (property: value)
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: CssDeclarationValue,
    pub important: bool,
}

/// CSS değer türleri
#[derive(Debug, Clone, PartialEq)]
pub enum CssDeclarationValue {
    /// Renk: #ff0000, red, rgb(255,0,0)
    ColorValue(u8, u8, u8, u8),
    /// Uzunluk: 16px, 1em, 2rem
    Length(f32, String), // (value, unit)
    /// Yüzde: 50%
    Percentage(f32),
    /// Anahtar kelime: auto, none, block, bold, etc.
    Keyword(String),
    /// String değer
    StringValue(String),
    /// Sayı (birimsiz)
    Number(f32),
    /// Birden çok değer
    Multiple(Vec<CssDeclarationValue>),
    /// URL: url(...)
    Url(String),
}

impl CssDeclarationValue {
    pub fn to_keyword(&self) -> Option<&str> {
        match self {
            CssDeclarationValue::Keyword(k) => Some(k.as_str()),
            _ => None,
        }
    }

    pub fn to_color(&self) -> Option<(u8, u8, u8, u8)> {
        match self {
            CssDeclarationValue::ColorValue(r, g, b, a) => Some((*r, *g, *b, *a)),
            _ => None,
        }
    }

    pub fn to_length_px(&self) -> Option<f32> {
        match self {
            CssDeclarationValue::Length(v, unit) if unit == "px" => Some(*v),
            _ => None,
        }
    }

    pub fn to_number(&self) -> Option<f32> {
        match self {
            CssDeclarationValue::Number(n) => Some(*n),
            CssDeclarationValue::Length(v, _) => Some(*v),
            CssDeclarationValue::Percentage(p) => Some(*p),
            _ => None,
        }
    }
}

/// CSS'i string'den parse et
pub fn parse_css(css_text: &str) -> Stylesheet {
    let mut parser = CssParser::new(css_text);
    parser.parse_stylesheet()
}

/// CSS parser
struct CssParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> CssParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.remaining().chars().next() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() || c == '\n' || c == '\r' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.remaining().starts_with("/*") {
            if let Some(end) = self.remaining().find("*/") {
                self.pos += end + 2;
            }
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            self.skip_whitespace();
            if self.remaining().starts_with("/*") {
                self.skip_comment();
            } else {
                break;
            }
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.remaining().starts_with(s)
    }

    fn parse_stylesheet(&mut self) -> Stylesheet {
        let mut rules = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.pos >= self.input.len() {
                break;
            }
            if let Some(rule) = self.parse_rule() {
                rules.push(rule);
            } else {
                // Geçersiz - karakteri atla
                self.advance();
            }
        }
        Stylesheet { rules }
    }

    fn parse_rule(&mut self) -> Option<Rule> {
        self.skip_whitespace_and_comments();

        // @-rules'ları geç (ör: @media, @import)
        if self.starts_with("@") {
            while self.pos < self.input.len() && !self.starts_with("{") {
                self.advance();
            }
            if self.starts_with("{") {
                let mut depth = 1;
                self.advance(); // skip {
                while self.pos < self.input.len() && depth > 0 {
                    if self.starts_with("{") {
                        depth += 1;
                    } else if self.starts_with("}") {
                        depth -= 1;
                    }
                    self.advance();
                }
            }
            return None;
        }

        // Selector'ları oku
        let mut selector_text = String::new();
        let mut depth = 0;
        while self.pos < self.input.len() {
            let c = self.peek().unwrap();
            if c == '{' && depth == 0 {
                break;
            }
            if c == '(' || c == '[' {
                depth += 1;
            } else if c == ')' || c == ']' {
                depth -= 1;
            }
            selector_text.push(c);
            self.advance();
        }

        if !self.starts_with("{") {
            return None;
        }
        self.advance(); // skip {

        // Bildirimleri oku
        let mut declarations = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.starts_with("}") {
                self.advance(); // skip }
                break;
            }
            if let Some(decl) = self.parse_declaration() {
                declarations.push(decl);
            } else {
                // Geçersiz karakter - atla
                if self.pos < self.input.len() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if selector_text.trim().is_empty() {
            return None;
        }

        let selectors = parse_selectors(&selector_text);

        Some(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_declaration(&mut self) -> Option<Declaration> {
        self.skip_whitespace_and_comments();

        // Property adını oku
        let mut property = String::new();
        while let Some(c) = self.peek() {
            if c == ':' || c == '}' || c == ';' || c.is_ascii_whitespace() {
                break;
            }
            property.push(c);
            self.advance();
        }

        if property.is_empty() {
            return None;
        }

        self.skip_whitespace_and_comments();

        if !self.starts_with(":") {
            return None;
        }
        self.advance(); // skip :

        // Değeri oku
        let mut value_text = String::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.starts_with(";") {
                self.advance(); // skip ;
                break;
            }
            if self.starts_with("}") {
                break;
            }
            if self.starts_with("!important") || self.starts_with("! important") {
                // !important işaretini geç
                if self.starts_with("!important") {
                    self.pos += 10;
                } else {
                    self.pos += 11;
                }
                // TODO: important flag'ini bildirime ekle
                continue;
            }
            if let Some(c) = self.peek() {
                value_text.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let property = property.trim().to_lowercase();
        let value_text = value_text.trim();

        if property.is_empty() || value_text.is_empty() {
            return None;
        }

        let value = parse_css_value(value_text);

        Some(Declaration {
            property,
            value,
            important: false,
        })
    }
}

/// Selector string'ini parse et (ör: "div.container > p.highlight")
fn parse_selectors(text: &str) -> Vec<Selector> {
    // Virgülle ayrılmış selector grupları
    text.split(',')
        .map(|s| parse_single_selector(s.trim()))
        .filter(|s| *s != Selector::Universal || !text.trim().is_empty())
        .collect()
}

/// Tek bir selector string'ini parse et
fn parse_single_selector(text: &str) -> Selector {
    let text = text.trim();

    // Combinator'ları kontrol et: boşluk (descendant), >, +, ~
    // Önce complex selector'ları kontrol et
    if let Some(result) = parse_complex_selector(text) {
        return result;
    }

    // Simple selector
    parse_simple_selector(text)
}

/// Complex selector'ları parse et (descendant, child, sibling)
fn parse_complex_selector(text: &str) -> Option<Selector> {
    // Child combinator: div > p
    if let Some(pos) = text.find(">") {
        let before = text[..pos].trim();
        let after = text[pos + 1..].trim();
        if !before.is_empty() && !after.is_empty() {
            return Some(Selector::Child {
                parent: Box::new(parse_single_selector(before)),
                child: Box::new(parse_single_selector(after)),
            });
        }
    }

    // Adjacent sibling: div + p
    if let Some(pos) = text.find('+') {
        let before = text[..pos].trim();
        let after = text[pos + 1..].trim();
        if !before.is_empty() && !after.is_empty() {
            return Some(Selector::AdjacentSibling {
                first: Box::new(parse_single_selector(before)),
                second: Box::new(parse_single_selector(after)),
            });
        }
    }

    // Descendant combinator (boşluk): div p
    // En sağdaki boşluğu bul
    let mut last_space = None;
    let mut in_paren = 0;
    let mut in_bracket = 0;
    for (i, c) in text.char_indices() {
        match c {
            '(' | '[' => {
                if c == '(' {
                    in_paren += 1
                } else {
                    in_bracket += 1
                }
            }
            ')' | ']' => {
                if c == ')' {
                    in_paren -= 1
                } else {
                    in_bracket -= 1
                }
            }
            c if c.is_ascii_whitespace() && in_paren == 0 && in_bracket == 0 => {
                last_space = Some(i);
            }
            _ => {}
        }
    }

    if let Some(pos) = last_space {
        let before = text[..pos].trim();
        let after = text[pos..].trim();
        if !before.is_empty() && !after.is_empty() {
            return Some(Selector::Descendant {
                ancestor: Box::new(parse_single_selector(before)),
                descendant: Box::new(parse_single_selector(after)),
            });
        }
    }

    None
}

/// Simple selector parse et
fn parse_simple_selector(text: &str) -> Selector {
    let text = text.trim();

    if text == "*" {
        return Selector::Universal;
    }

    // Birden çok parçayı birleştir (ör: div.container#main)
    let mut parts = Vec::new();
    let mut current = String::new();

    for c in text.chars() {
        match c {
            '.' | '#' | '[' | ':' => {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
                current.push(c);
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() {
        return Selector::Universal;
    }

    let selectors: Vec<Selector> = parts.iter().map(|part| parse_selector_part(part)).collect();

    if selectors.len() == 1 {
        selectors.into_iter().next().unwrap()
    } else {
        Selector::Compound(selectors)
    }
}

/// Tek bir selector parçasını parse et
fn parse_selector_part(text: &str) -> Selector {
    if text.starts_with('.') {
        Selector::Class(text[1..].to_string())
    } else if text.starts_with('#') {
        Selector::Id(text[1..].to_string())
    } else if text.starts_with('[') {
        // Attribute selector: [attr], [attr=value], [attr~=value], etc.
        let inner = text.trim_start_matches('[').trim_end_matches(']');
        if let Some(eq_pos) = inner.find('=') {
            let name = inner[..eq_pos].trim();
            let _op = if inner[..eq_pos].ends_with('~') {
                "~="
            } else if inner[..eq_pos].ends_with('|') {
                "|="
            } else if inner[..eq_pos].ends_with('^') {
                "^="
            } else if inner[..eq_pos].ends_with('$') {
                "$="
            } else if inner[..eq_pos].ends_with('*') {
                "*="
            } else {
                "="
            };
            let name = name.trim_end_matches(&['~', '|', '^', '$', '*']);
            let value = inner[eq_pos + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            Selector::AttributeEquals(name.to_string(), value.to_string())
        } else {
            Selector::AttributeExists(inner.trim().to_string())
        }
    } else if text.starts_with(':') {
        Selector::PseudoClass(text[1..].to_string())
    } else {
        Selector::Tag(text.to_lowercase())
    }
}

/// CSS değer string'ini parse et
pub fn parse_css_value(text: &str) -> CssDeclarationValue {
    let text = text.trim();

    if text.is_empty() {
        return CssDeclarationValue::Keyword("".to_string());
    }

    // URL değeri
    if text.starts_with("url(") {
        let inner = text.trim_start_matches("url(").trim_end_matches(')').trim();
        let inner = inner.trim_matches('"').trim_matches('\'');
        return CssDeclarationValue::Url(inner.to_string());
    }

    // Renk değerleri
    if let Some(color) = parse_color(text) {
        return color;
    }

    // Sayısal değerler
    if let Some(val) = parse_numeric_value(text) {
        return val;
    }

    // Birden çok değer (boşlukla ayrılmış)
    if text.contains(char::is_whitespace) && !text.contains('(') {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() > 1 {
            let values: Vec<CssDeclarationValue> =
                parts.iter().map(|p| parse_css_value(p)).collect();
            return CssDeclarationValue::Multiple(values);
        }
    }

    // Anahtar kelime
    CssDeclarationValue::Keyword(text.to_lowercase())
}

fn parse_color(text: &str) -> Option<CssDeclarationValue> {
    let text = text.trim();

    // #rrggbb
    if text.starts_with('#') {
        let hex = &text[1..];
        if hex.len() == 6 {
            if let Ok(val) = u32::from_str_radix(hex, 16) {
                let r = ((val >> 16) & 0xFF) as u8;
                let g = ((val >> 8) & 0xFF) as u8;
                let b = (val & 0xFF) as u8;
                return Some(CssDeclarationValue::ColorValue(r, g, b, 255));
            }
        } else if hex.len() == 3 {
            if let Ok(val) = u32::from_str_radix(hex, 16) {
                let r = ((val >> 8) & 0xF) as u8 * 17;
                let g = ((val >> 4) & 0xF) as u8 * 17;
                let b = (val & 0xF) as u8 * 17;
                return Some(CssDeclarationValue::ColorValue(r, g, b, 255));
            }
        }
    }

    // rgb(r, g, b)
    if text.starts_with("rgb(") && text.ends_with(')') {
        let inner = text.trim_start_matches("rgb(").trim_end_matches(')');
        let parts: Vec<&str> = inner
            .split(&[',', ' '][..])
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 3 {
            let r = parts[0].trim().parse::<u8>().unwrap_or(0);
            let g = parts[1].trim().parse::<u8>().unwrap_or(0);
            let b = parts[2].trim().parse::<u8>().unwrap_or(0);
            let a = if parts.len() > 3 {
                parts[3].trim().parse::<u8>().unwrap_or(255)
            } else {
                255
            };
            return Some(CssDeclarationValue::ColorValue(r, g, b, a));
        }
    }

    // rgba(r, g, b, a)
    if text.starts_with("rgba(") && text.ends_with(')') {
        let inner = text.trim_start_matches("rgba(").trim_end_matches(')');
        let parts: Vec<&str> = inner
            .split(&[',', ' '][..])
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 4 {
            let r = parts[0].trim().parse::<u8>().unwrap_or(0);
            let g = parts[1].trim().parse::<u8>().unwrap_or(0);
            let b = parts[2].trim().parse::<u8>().unwrap_or(0);
            let a = (parts[3].trim().parse::<f32>().unwrap_or(1.0) * 255.0) as u8;
            return Some(CssDeclarationValue::ColorValue(r, g, b, a));
        }
    }

    // İsimlendirilmiş renkler
    let named_colors: HashMap<&str, (u8, u8, u8)> = [
        ("black", (0, 0, 0)),
        ("white", (255, 255, 255)),
        ("red", (255, 0, 0)),
        ("green", (0, 128, 0)),
        ("blue", (0, 0, 255)),
        ("yellow", (255, 255, 0)),
        ("orange", (255, 165, 0)),
        ("purple", (128, 0, 128)),
        ("gray", (128, 128, 128)),
        ("grey", (128, 128, 128)),
        ("pink", (255, 192, 203)),
        ("brown", (165, 42, 42)),
        ("navy", (0, 0, 128)),
        ("teal", (0, 128, 128)),
        ("aqua", (0, 255, 255)),
        ("cyan", (0, 255, 255)),
        ("magenta", (255, 0, 255)),
        ("fuchsia", (255, 0, 255)),
        ("lime", (0, 255, 0)),
        ("maroon", (128, 0, 0)),
        ("olive", (128, 128, 0)),
        ("silver", (192, 192, 192)),
        ("transparent", (0, 0, 0)),
    ]
    .iter()
    .cloned()
    .collect();

    if let Some(&(r, g, b)) = named_colors.get(text.to_lowercase().as_str()) {
        let a = if text.eq_ignore_ascii_case("transparent") {
            0
        } else {
            255
        };
        return Some(CssDeclarationValue::ColorValue(r, g, b, a));
    }

    None
}

fn parse_numeric_value(text: &str) -> Option<CssDeclarationValue> {
    let text = text.trim();

    // Yüzde
    if text.ends_with('%') {
        let num = text[..text.len() - 1].trim().parse::<f32>().ok()?;
        return Some(CssDeclarationValue::Percentage(num));
    }

    // Uzunluk birimleri
    let units = [
        "px", "em", "rem", "pt", "cm", "mm", "in", "vw", "vh", "vmin", "vmax", "ch", "ex",
    ];
    for unit in &units {
        if text.ends_with(unit) {
            let num_str = text[..text.len() - unit.len()].trim();
            let num = num_str.parse::<f32>().ok()?;
            return Some(CssDeclarationValue::Length(num, unit.to_string()));
        }
    }

    // Düz sayı
    if let Ok(num) = text.parse::<f32>() {
        return Some(CssDeclarationValue::Number(num));
    }

    None
}

/// Selector'u elemente karşı test et
pub fn matches_selector(selector: &Selector, element: &crate::dom::ElementData) -> bool {
    match selector {
        Selector::Universal => true,
        Selector::Tag(name) => element.tag_name == *name,
        Selector::Class(name) => element.has_class(name),
        Selector::Id(id) => element.id() == Some(id.as_str()),
        Selector::Compound(parts) => parts.iter().all(|s| matches_selector(s, element)),
        Selector::Descendant {
            ancestor: _,
            descendant,
        } => {
            // Basit - sadece descendant kısmını kontrol et
            matches_selector(descendant, element)
        }
        Selector::Child { parent: _, child } => matches_selector(child, element),
        Selector::AdjacentSibling { first: _, second } => matches_selector(second, element),
        Selector::AttributeExists(name) => element.attributes.contains_key(name),
        Selector::AttributeEquals(name, value) => element
            .attributes
            .get(name)
            .map(|v| v == value)
            .unwrap_or(false),
        Selector::PseudoClass(_) => {
            // Pseudo-class'ları basitçe true döndür (gerçek implementasyon daha karmaşık)
            true
        }
    }
}

/// Specificity hesapla (selector önceliği)
pub fn selector_specificity(selector: &Selector) -> (u32, u32, u32) {
    match selector {
        Selector::Universal => (0, 0, 0),
        Selector::Tag(_) => (0, 0, 1),
        Selector::Class(_)
        | Selector::PseudoClass(_)
        | Selector::AttributeExists(_)
        | Selector::AttributeEquals(_, _) => (0, 1, 0),
        Selector::Id(_) => (1, 0, 0),
        Selector::Compound(parts) => {
            let mut spec = (0, 0, 0);
            for part in parts {
                let s = selector_specificity(part);
                spec.0 += s.0;
                spec.1 += s.1;
                spec.2 += s.2;
            }
            spec
        }
        Selector::Descendant {
            ancestor,
            descendant,
        }
        | Selector::Child {
            parent: ancestor,
            child: descendant,
        }
        | Selector::AdjacentSibling {
            first: ancestor,
            second: descendant,
        } => {
            let a = selector_specificity(ancestor);
            let d = selector_specificity(descendant);
            (a.0 + d.0, a.1 + d.1, a.2 + d.2)
        }
    }
}
