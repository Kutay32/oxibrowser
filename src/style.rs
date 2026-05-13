/// Style engine - CSS kurallarını DOM elementlerine uygula
use crate::css::{self, Declaration};
use crate::dom::{ElementData, Node};
use crate::types::*;
use std::collections::HashMap;

/// Hesaplanmış stiller
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: DisplayType,
    pub position: PositionType,
    pub width: CssValue,
    pub height: CssValue,
    pub min_width: CssValue,
    pub min_height: CssValue,
    pub max_width: CssValue,
    pub max_height: CssValue,

    // Margin, padding, border
    pub margin_top: CssValue,
    pub margin_right: CssValue,
    pub margin_bottom: CssValue,
    pub margin_left: CssValue,
    pub padding_top: CssValue,
    pub padding_right: CssValue,
    pub padding_bottom: CssValue,
    pub padding_left: CssValue,

    pub border_top_width: CssValue,
    pub border_right_width: CssValue,
    pub border_bottom_width: CssValue,
    pub border_left_width: CssValue,
    pub border_top_color: Color,
    pub border_right_color: Color,
    pub border_bottom_color: Color,
    pub border_left_color: Color,
    pub border_style: String,

    // Renk ve arkaplan
    pub color: Color,
    pub background_color: Color,
    pub background_image: Option<String>,

    // Font
    pub font_family: Vec<String>,
    pub font_size: CssValue,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,

    // Metin
    pub text_align: TextAlign,
    pub line_height: CssValue,

    // Overflow
    pub overflow_x: String,
    pub overflow_y: String,

    // Visibility
    pub visibility: String,

    // Opacity
    pub opacity: f32,

    // Z-index
    pub z_index: i32,

    // Cursor
    pub cursor: String,

    // Position coordinates
    pub top: CssValue,
    pub right: CssValue,
    pub bottom: CssValue,
    pub left: CssValue,
}

impl ComputedStyle {
    pub fn default() -> Self {
        Self {
            display: DisplayType::Block,
            position: PositionType::Static,
            width: CssValue::Auto,
            height: CssValue::Auto,
            min_width: CssValue::Zero,
            min_height: CssValue::Zero,
            max_width: CssValue::None,
            max_height: CssValue::None,
            margin_top: CssValue::Zero,
            margin_right: CssValue::Zero,
            margin_bottom: CssValue::Zero,
            margin_left: CssValue::Zero,
            padding_top: CssValue::Zero,
            padding_right: CssValue::Zero,
            padding_bottom: CssValue::Zero,
            padding_left: CssValue::Zero,
            border_top_width: CssValue::Zero,
            border_right_width: CssValue::Zero,
            border_bottom_width: CssValue::Zero,
            border_left_width: CssValue::Zero,
            border_top_color: Color::TRANSPARENT,
            border_right_color: Color::TRANSPARENT,
            border_bottom_color: Color::TRANSPARENT,
            border_left_color: Color::TRANSPARENT,
            border_style: "none".to_string(),
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            background_image: None,
            font_family: vec!["serif".to_string()],
            font_size: CssValue::Pixels(16.0),
            font_weight: FontWeight::Normal,
            font_style: FontStyle::Normal,
            text_align: TextAlign::Left,
            line_height: CssValue::Pixels(1.2),
            overflow_x: "visible".to_string(),
            overflow_y: "visible".to_string(),
            visibility: "visible".to_string(),
            opacity: 1.0,
            z_index: 0,
            cursor: "auto".to_string(),
            top: CssValue::Auto,
            right: CssValue::Auto,
            bottom: CssValue::Auto,
            left: CssValue::Auto,
        }
    }

    /// Inline elementler için varsayılan stiller
    pub fn default_inline() -> Self {
        let mut s = Self::default();
        s.display = DisplayType::Inline;
        s
    }

    /// Block elementler için varsayılan stiller
    pub fn default_block() -> Self {
        let mut s = Self::default();
        s.display = DisplayType::Block;
        s
    }
}

/// DOM ağacına CSS kurallarını uygula
pub fn apply_stylesheet(dom: &Node, stylesheet: &css::Stylesheet) -> HashMap<crate::dom::NodeId, ComputedStyle> {
    let mut styles = HashMap::new();
    let mut inherited = ComputedStyle::default();
    apply_styles_recursive(dom, stylesheet, &mut styles, &mut inherited);
    styles
}

fn apply_styles_recursive(
    node: &Node,
    stylesheet: &css::Stylesheet,
    mut styles: &mut HashMap<crate::dom::NodeId, ComputedStyle>,
    inherited: &mut ComputedStyle,
) {
    match &node.node_type {
        crate::dom::NodeType::Element(el) => {
            let mut style = apply_rules_to_element(el, stylesheet, inherited);

            // Inline styles (style attribute)
            apply_inline_style(el, &mut style);

            // Element tipine göre varsayılanları düzelt
            apply_defaults_for_tag(&el.tag_name, &mut style);

            styles.insert(node.id, style.clone());
            *inherited = style;
        }
        crate::dom::NodeType::Document => {
            // Document root - varsayılan stilleri ekle
            styles.insert(node.id, ComputedStyle::default());
        }
        crate::dom::NodeType::Text(_) => {
            // Metin düğümleri - parent'dan inherit et
            styles.insert(node.id, inherited.clone());
        }
        crate::dom::NodeType::Comment(_) => {}
    }

    for child in &node.children {
        // Metin düğümleri için inherit edilen stili geç (elementler zaten yeni style oluşturur)
        let mut child_inherited = match &node.node_type {
            crate::dom::NodeType::Element(_) => styles.get(&node.id).cloned().unwrap_or_else(|| inherited.clone()),
            _ => inherited.clone(),
        };
        apply_styles_recursive(child, stylesheet, &mut styles, &mut child_inherited);
    }
}

/// CSS kurallarını elemente uygula
fn apply_rules_to_element(
    el: &ElementData,
    stylesheet: &css::Stylesheet,
    inherited: &ComputedStyle,
) -> ComputedStyle {
    let mut style = inherited.clone();

    // Eşleşen kuralları topla (specificity hesaplama)
    struct RuleWithSpec {
        spec: (u32, u32, u32),
        declarations: Vec<css::Declaration>,
    }

    let mut matching_rules: Vec<RuleWithSpec> = Vec::new();

    for rule in &stylesheet.rules {
        for selector in &rule.selectors {
            if css::matches_selector(selector, el) {
                let spec = css::selector_specificity(selector);
                matching_rules.push(RuleWithSpec {
                    spec,
                    declarations: rule.declarations.clone(),
                });
                break;
            }
        }
    }

    // Specificity'ye göre sırala (artan)
    matching_rules.sort_by(|a, b| a.spec.cmp(&b.spec));

    // Bildirimleri uygula
    for rule in &matching_rules {
        for decl in &rule.declarations {
            apply_declaration(&mut style, decl);
        }
    }

    style
}

/// Rule + specificity wrapper
struct RuleWithSpec<'a> {
    rule: &'a css::Rule,
    spec: (u32, u32, u32),
}

/// Inline style attribute'unu uygula
fn apply_inline_style(el: &ElementData, style: &mut ComputedStyle) {
    if let Some(inline_css) = el.attributes.get("style") {
        if !inline_css.is_empty() {
            let decls = parse_inline_style(inline_css);
            for decl in &decls {
                apply_declaration(style, decl);
            }
        }
    }
}

/// Inline CSS bildirimlerini parse et
fn parse_inline_style(css_text: &str) -> Vec<Declaration> {
    let mut decls = Vec::new();
    for part in css_text.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = part.find(':') {
            let property = part[..pos].trim().to_lowercase();
            let value = part[pos + 1..].trim();
            if !property.is_empty() && !value.is_empty() {
                decls.push(Declaration {
                    property,
                    value: css::parse_css_value(value),
                    important: false,
                });
            }
        }
    }
    decls
}

/// Element tipine göre varsayılan stilleri uygula
fn apply_defaults_for_tag(tag: &str, style: &mut ComputedStyle) {
    match tag {
        // Block-level elements
        "html" | "body" | "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        | "ul" | "ol" | "li" | "dl" | "dt" | "dd" | "table" | "tr" | "td" | "th"
        | "form" | "fieldset" | "blockquote" | "pre" | "hr" | "figure" | "figcaption"
        | "header" | "footer" | "nav" | "main" | "section" | "article" | "aside"
        | "details" | "summary" | "dialog" => {
            style.display = DisplayType::Block;
        }

        // Inline elements
        "span" | "a" | "b" | "i" | "em" | "strong" | "u" | "s" | "sub" | "sup"
        | "code" | "kbd" | "q" | "cite" | "abbr" | "time" | "mark" | "small"
        | "label" | "button" | "input" | "textarea" | "select" | "option" => {
            style.display = DisplayType::Inline;
        }

        // Images
        "img" => {
            style.display = DisplayType::InlineBlock;
        }

        // Script, style, etc.
        "script" | "style" | "noscript" | "meta" | "link" | "head" => {
            style.display = DisplayType::None;
        }

        _ => {}
    }

    // Başlık boyutları
    let heading_sizes: [f32; 6] = [32.0, 24.0, 19.0, 16.0, 13.0, 11.0];
    if let Some(n) = tag.strip_prefix('h') {
        if let Ok(idx) = n.parse::<usize>() {
            if idx >= 1 && idx <= 6 {
                style.font_size = CssValue::Pixels(heading_sizes[idx - 1]);
                style.font_weight = FontWeight::Bold;
            }
        }
    }

    // Body margin
    if tag == "body" {
        style.margin_top = CssValue::Pixels(8.0);
        style.margin_right = CssValue::Pixels(8.0);
        style.margin_bottom = CssValue::Pixels(8.0);
        style.margin_left = CssValue::Pixels(8.0);
    }

    // p margin
    if tag == "p" {
        style.margin_top = CssValue::Pixels(16.0);
        style.margin_bottom = CssValue::Pixels(16.0);
    }

    // a tag - underline and color
    if tag == "a" {
        style.color = Color::BLUE;
        // TODO: text-decoration: underline
    }
}

/// Tek bir CSS bildirimini ComputedStyle'a uygula
fn apply_declaration(style: &mut ComputedStyle, decl: &Declaration) {
    match decl.property.as_str() {
        "display" => {
            style.display = match decl.value.to_keyword() {
                Some("block") => DisplayType::Block,
                Some("inline") => DisplayType::Inline,
                Some("none") => DisplayType::None,
                Some("flex") => DisplayType::Flex,
                Some("grid") => DisplayType::Grid,
                Some("inline-block") => DisplayType::InlineBlock,
                _ => style.display,
            };
        }
        "position" => {
            style.position = match decl.value.to_keyword() {
                Some("static") => PositionType::Static,
                Some("relative") => PositionType::Relative,
                Some("absolute") => PositionType::Absolute,
                Some("fixed") => PositionType::Fixed,
                Some("sticky") => PositionType::Sticky,
                _ => style.position,
            };
        }
        "color" => {
            if let Some((r, g, b, a)) = decl.value.to_color() {
                style.color = Color::new(r, g, b, a);
            }
        }
        "background-color" | "background" => {
            if let Some((r, g, b, a)) = decl.value.to_color() {
                style.background_color = Color::new(r, g, b, a);
            }
        }
        "width" => style.width = css_value_from_decl(&decl.value),
        "height" => style.height = css_value_from_decl(&decl.value),
        "min-width" => style.min_width = css_value_from_decl(&decl.value),
        "min-height" => style.min_height = css_value_from_decl(&decl.value),
        "max-width" => style.max_width = css_value_from_decl(&decl.value),
        "max-height" => style.max_height = css_value_from_decl(&decl.value),

        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            let val = css_value_from_decl(&decl.value);
            match decl.property.as_str() {
                "margin" => {
                    style.margin_top = val;
                    style.margin_right = val;
                    style.margin_bottom = val;
                    style.margin_left = val;
                }
                "margin-top" => style.margin_top = val,
                "margin-right" => style.margin_right = val,
                "margin-bottom" => style.margin_bottom = val,
                "margin-left" => style.margin_left = val,
                _ => {}
            }
        }

        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            let val = css_value_from_decl(&decl.value);
            match decl.property.as_str() {
                "padding" => {
                    style.padding_top = val;
                    style.padding_right = val;
                    style.padding_bottom = val;
                    style.padding_left = val;
                }
                "padding-top" => style.padding_top = val,
                "padding-right" => style.padding_right = val,
                "padding-bottom" => style.padding_bottom = val,
                "padding-left" => style.padding_left = val,
                _ => {}
            }
        }

        "border" | "border-top" | "border-right" | "border-bottom" | "border-left" => {
            // TODO: Full border parsing
            if decl.property == "border" {
                if let Some((r, g, b, a)) = decl.value.to_color() {
                    style.border_top_color = Color::new(r, g, b, a);
                    style.border_right_color = Color::new(r, g, b, a);
                    style.border_bottom_color = Color::new(r, g, b, a);
                    style.border_left_color = Color::new(r, g, b, a);
                }
                if let Some(px) = decl.value.to_length_px() {
                    style.border_top_width = CssValue::Pixels(px);
                    style.border_right_width = CssValue::Pixels(px);
                    style.border_bottom_width = CssValue::Pixels(px);
                    style.border_left_width = CssValue::Pixels(px);
                }
            }
        }

        "border-width" => {
            if let Some(px) = decl.value.to_length_px() {
                style.border_top_width = CssValue::Pixels(px);
                style.border_right_width = CssValue::Pixels(px);
                style.border_bottom_width = CssValue::Pixels(px);
                style.border_left_width = CssValue::Pixels(px);
            }
        }

        "border-color" => {
            if let Some((r, g, b, a)) = decl.value.to_color() {
                let c = Color::new(r, g, b, a);
                style.border_top_color = c;
                style.border_right_color = c;
                style.border_bottom_color = c;
                style.border_left_color = c;
            }
        }

        "border-style" => {
            if let Some(kw) = decl.value.to_keyword() {
                style.border_style = kw.to_string();
            }
        }

        "font-size" => {
            if let Some(px) = decl.value.to_length_px() {
                style.font_size = CssValue::Pixels(px);
            } else if let Some(pct) = decl.value.to_number() {
                style.font_size = CssValue::Pixels(16.0 * pct / 100.0);
            }
        }
        "font-weight" => {
            style.font_weight = match decl.value.to_keyword() {
                Some("bold") | Some("700") => FontWeight::Bold,
                Some("normal") | Some("400") => FontWeight::Normal,
                Some("bolder") => FontWeight::Bolder,
                Some("lighter") => FontWeight::Lighter,
                _ => {
                    if let Some(n) = decl.value.to_number() {
                        FontWeight::Weight(n as u16)
                    } else {
                        style.font_weight
                    }
                }
            };
        }
        "font-style" => {
            style.font_style = match decl.value.to_keyword() {
                Some("italic") => FontStyle::Italic,
                Some("oblique") => FontStyle::Oblique,
                _ => FontStyle::Normal,
            };
        }
        "font-family" => {
            if let Some(kw) = decl.value.to_keyword() {
                style.font_family = kw.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
            }
        }
        "text-align" => {
            style.text_align = match decl.value.to_keyword() {
                Some("left") => TextAlign::Left,
                Some("center") | Some("middle") => TextAlign::Center,
                Some("right") => TextAlign::Right,
                Some("justify") => TextAlign::Justify,
                _ => style.text_align,
            };
        }
        "line-height" => {
            if let Some(px) = decl.value.to_length_px() {
                style.line_height = CssValue::Pixels(px);
            } else if let Some(n) = decl.value.to_number() {
                style.line_height = CssValue::Pixels(n);
            }
        }
        "opacity" => {
            if let Some(n) = decl.value.to_number() {
                style.opacity = n.max(0.0).min(1.0);
            }
        }
        "z-index" => {
            if let Some(n) = decl.value.to_number() {
                style.z_index = n as i32;
            }
        }
        "cursor" => {
            if let Some(kw) = decl.value.to_keyword() {
                style.cursor = kw.to_string();
            }
        }
        "overflow" | "overflow-x" | "overflow-y" => {
            let val = decl.value.to_keyword().unwrap_or("visible").to_string();
            match decl.property.as_str() {
                "overflow" => {
                    style.overflow_x = val.clone();
                    style.overflow_y = val;
                }
                "overflow-x" => style.overflow_x = val,
                "overflow-y" => style.overflow_y = val,
                _ => {}
            }
        }
        "visibility" => {
            if let Some(kw) = decl.value.to_keyword() {
                style.visibility = kw.to_string();
            }
        }
        "top" => style.top = css_value_from_decl(&decl.value),
        "right" => style.right = css_value_from_decl(&decl.value),
        "bottom" => style.bottom = css_value_from_decl(&decl.value),
        "left" => style.left = css_value_from_decl(&decl.value),
        _ => {} // Bilinmeyen property - ignore
    }
}

/// DeclarationValue'den CssValue'e dönüştür
fn css_value_from_decl(value: &css::CssDeclarationValue) -> CssValue {
    match value {
        css::CssDeclarationValue::Length(v, unit) if unit == "px" => CssValue::Pixels(*v),
        css::CssDeclarationValue::Percentage(v) => CssValue::Percentage(*v),
        css::CssDeclarationValue::Keyword(k) => match k.as_str() {
            "auto" => CssValue::Auto,
            "none" => CssValue::None,
            "inherit" => CssValue::Inherit,
            "0" => CssValue::Zero,
            _ => CssValue::Auto,
        },
        css::CssDeclarationValue::Number(n) => CssValue::Pixels(*n),
        _ => CssValue::Auto,
    }
}
