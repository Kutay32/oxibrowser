/// Layout engine - box tree oluşturma ve yerleşim hesaplama
use crate::dom::{Node, NodeType};
use crate::style::ComputedStyle;
use crate::types::*;
use std::collections::HashMap;

/// Layout kutusu
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub node_id: crate::dom::NodeId,
    pub box_type: BoxType,
    pub style: ComputedStyle,
    pub dimensions: BoxDimensions,
    pub text: Option<String>,
    pub children: Vec<LayoutBox>,
}

/// Kutu tipi
#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    Block,
    Inline,
    AnonymousBlock,
    Text,
    Flex,
}

/// Kutu boyutları (box model)
#[derive(Debug, Clone, Copy)]
pub struct BoxDimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl BoxDimensions {
    pub fn new() -> Self {
        Self {
            content: Rect::new(0.0, 0.0, 0.0, 0.0),
            padding: EdgeSizes::zero(),
            border: EdgeSizes::zero(),
            margin: EdgeSizes::zero(),
        }
    }

    /// Padding + border genişliği
    pub fn padding_border_width(&self) -> f32 {
        self.padding.left
            + self.padding.right
            + self.border.left
            + self.border.right
    }

    /// Padding + border yüksekliği
    pub fn padding_border_height(&self) -> f32 {
        self.padding.top
            + self.padding.bottom
            + self.border.top
            + self.border.bottom
    }

    /// Toplam genişlik (margin + border + padding + content)
    pub fn total_width(&self) -> f32 {
        self.margin.left + self.margin.right + self.content.width + self.padding_border_width()
    }

    /// Toplam yükseklik
    pub fn total_height(&self) -> f32 {
        self.margin.top + self.margin.bottom + self.content.height + self.padding_border_height()
    }

    /// Border box rect
    pub fn border_box(&self) -> Rect {
        Rect::new(
            self.content.x - self.border.left - self.padding.left,
            self.content.y - self.border.top - self.padding.top,
            self.content.width + self.padding_border_width(),
            self.content.height + self.padding_border_height(),
        )
    }

    /// Margin box rect
    pub fn margin_box(&self) -> Rect {
        Rect::new(
            self.content.x - self.margin.left - self.border.left - self.padding.left,
            self.content.y - self.margin.top - self.border.top - self.padding.top,
            self.total_width(),
            self.total_height(),
        )
    }
}

/// Kenar boyutları (margin, padding, border için)
#[derive(Debug, Clone, Copy)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeSizes {
    pub fn zero() -> Self {
        Self {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        }
    }

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// Layout ağacı oluştur
pub fn build_layout_tree(
    dom: &Node,
    styles: &HashMap<crate::dom::NodeId, ComputedStyle>,
) -> Option<LayoutBox> {
    build_tree_recursive(dom, styles)
}

fn build_tree_recursive(
    node: &Node,
    styles: &HashMap<crate::dom::NodeId, ComputedStyle>,
) -> Option<LayoutBox> {
    match &node.node_type {
        NodeType::Element(el) => {
            let style = styles.get(&node.id)?;

            // display: none olan elementleri atla
            if style.display == DisplayType::None {
                return None;
            }

            // Sadece block/inline elementleri layout'a ekle
            let box_type = match style.display {
                DisplayType::Block => BoxType::Block,
                DisplayType::Inline => BoxType::Inline,
                DisplayType::InlineBlock => BoxType::Block, // Treat as block
                _ => BoxType::Block,
            };

            // script/style elementlerini atla
            match el.tag_name.as_str() {
                "script" | "style" | "meta" | "link" | "head" => return None,
                _ => {}
            }

            let mut layout_box = LayoutBox {
                node_id: node.id,
                box_type,
                style: style.clone(),
                dimensions: BoxDimensions::new(),
                text: None,
                children: Vec::new(),
            };

            for child in &node.children {
                if let Some(child_box) = build_tree_recursive(child, styles) {
                    layout_box.children.push(child_box);
                }
            }

            // Anonymous block kutuları oluştur (inline-child'ları grupla)
            if layout_box.box_type == BoxType::Block {
                layout_box.wrap_inline_children();
            }

            Some(layout_box)
        }
        NodeType::Text(text) => {
            let text = collapse_whitespace(text);
            if text.is_empty() {
                return None;
            }

            let style = styles
                .get(&node.id)
                .cloned()
                .unwrap_or_else(ComputedStyle::default);

            Some(LayoutBox {
                node_id: node.id,
                box_type: BoxType::Text,
                style,
                dimensions: BoxDimensions::new(),
                text: Some(text),
                children: Vec::new(),
            })
        }
        NodeType::Document => {
            // Document'in çocuklarını işle (html elementi)
            for child in &node.children {
                if let Some(result) = build_tree_recursive(child, styles) {
                    return Some(result);
                }
            }
            None
        }
        NodeType::Comment(_) => None,
    }
}

impl LayoutBox {
    /// Inline child'ları anonymous block'lara sar
    fn wrap_inline_children(&mut self) {
        let mut new_children: Vec<LayoutBox> = Vec::new();
        let mut current_inline_group: Vec<LayoutBox> = Vec::new();

        for child in self.children.drain(..) {
            match child.box_type {
                BoxType::Inline | BoxType::Text => {
                    current_inline_group.push(child);
                }
                BoxType::Block => {
                    if !current_inline_group.is_empty() {
                        new_children.push(create_anonymous_block(current_inline_group.drain(..).collect()));
                    }
                    new_children.push(child);
                }
                _ => {
                    current_inline_group.push(child);
                }
            }
        }

        if !current_inline_group.is_empty() {
            new_children.push(create_anonymous_block(current_inline_group));
        }

        self.children = new_children;
    }
}

fn create_anonymous_block(children: Vec<LayoutBox>) -> LayoutBox {
    LayoutBox {
        node_id: children.first().map(|c| c.node_id).unwrap_or(crate::dom::NodeId::zero()),
        box_type: BoxType::AnonymousBlock,
        style: ComputedStyle::default(),
        dimensions: BoxDimensions::new(),
        text: None,
        children,
    }
}

/// Layout hesapla
pub fn calculate_layout(
    layout_root: &mut LayoutBox,
    containing_width: f32,
    _containing_height: f32,
) {
    match layout_root.box_type {
        BoxType::Block | BoxType::AnonymousBlock => {
            layout_block(layout_root, containing_width, 0.0, 0.0);
        }
        BoxType::Inline | BoxType::Text => {
            // Inline layout, block layout içinde hesaplanır
        }
        BoxType::Flex => {
            layout_block(layout_root, containing_width, 0.0, 0.0);
        }
    }
}

/// Block layout hesapla
fn layout_block(box_node: &mut LayoutBox, parent_width: f32, x: f32, y: f32) {
    let style = &box_node.style;

    // Kenar değerlerini hesapla
    let margin_top = resolve_edge_value(&style.margin_top, parent_width);
    let mut margin_right = resolve_edge_value(&style.margin_right, parent_width);
    let margin_bottom = resolve_edge_value(&style.margin_bottom, parent_width);
    let mut margin_left = resolve_edge_value(&style.margin_left, parent_width);

    let padding_top = resolve_css_value(&style.padding_top, parent_width);
    let padding_right = resolve_css_value(&style.padding_right, parent_width);
    let padding_bottom = resolve_css_value(&style.padding_bottom, parent_width);
    let padding_left = resolve_css_value(&style.padding_left, parent_width);

    let border_top = resolve_css_value(&style.border_top_width, parent_width);
    let border_right = resolve_css_value(&style.border_right_width, parent_width);
    let border_bottom = resolve_css_value(&style.border_bottom_width, parent_width);
    let border_left = resolve_css_value(&style.border_left_width, parent_width);

    let left_auto = matches!(style.margin_left, CssValue::Auto);
    let right_auto = matches!(style.margin_right, CssValue::Auto);

    // Content genişliğini hesapla
    let available_width = parent_width
        - margin_left - margin_right
        - padding_left - padding_right
        - border_left - border_right;

    let mut content_width = match style.width {
        CssValue::Auto | CssValue::None => available_width.max(0.0),
        _ => resolve_css_value(&style.width, parent_width).max(0.0),
    };

    let min_width = resolve_css_value(&style.min_width, parent_width);
    if min_width > 0.0 {
        content_width = content_width.max(min_width);
    }

    if !matches!(style.max_width, CssValue::None | CssValue::Auto) {
        let max_width = resolve_css_value(&style.max_width, parent_width);
        if max_width > 0.0 {
            content_width = content_width.min(max_width);
        }
    }

    let remaining_width = (parent_width
        - content_width
        - padding_left
        - padding_right
        - border_left
        - border_right
        - if left_auto { 0.0 } else { margin_left }
        - if right_auto { 0.0 } else { margin_right })
        .max(0.0);

    match (left_auto, right_auto) {
        (true, true) => {
            margin_left = remaining_width / 2.0;
            margin_right = remaining_width / 2.0;
        }
        (true, false) => margin_left = remaining_width,
        (false, true) => margin_right = remaining_width,
        (false, false) => {}
    }

    box_node.dimensions.margin = EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left);
    box_node.dimensions.padding = EdgeSizes::new(padding_top, padding_right, padding_bottom, padding_left);
    box_node.dimensions.border = EdgeSizes::new(border_top, border_right, border_bottom, border_left);

    box_node.dimensions.content.x = x + margin_left + border_left + padding_left;
    box_node.dimensions.content.y = y + margin_top + border_top + padding_top;
    box_node.dimensions.content.width = content_width;

    // İçindeki block'ları yerleştir
    let mut current_y = box_node.dimensions.content.y;

    for child in &mut box_node.children {
        match child.box_type {
            BoxType::Block | BoxType::AnonymousBlock => {
                // Child'ın layout'unu hesapla
                layout_block(child, content_width, box_node.dimensions.content.x, current_y);

                current_y = child.dimensions.margin_box().bottom();
            }
            BoxType::Inline | BoxType::Text => {
                // Inline layout
                layout_inline(child, content_width, &mut current_y, box_node.dimensions.content.x);
            }
            BoxType::Flex => {
                layout_block(child, content_width, box_node.dimensions.content.x, current_y);
                current_y = child.dimensions.margin_box().bottom();
            }
        }
    }

    // Content yüksekliğini hesapla
    let height = resolve_css_value(&style.height, current_y - box_node.dimensions.content.y);
    box_node.dimensions.content.height = if height > 0.0 && !matches!(style.height, CssValue::Auto | CssValue::None) {
        height
    } else {
        (current_y - box_node.dimensions.content.y).max(0.0)
    };
}

/// Inline layout - metin satırları
fn layout_inline(
    box_node: &mut LayoutBox,
    containing_width: f32,
    current_y: &mut f32,
    x_offset: f32,
) {
    match &box_node.box_type {
        BoxType::Text => {
            // Metin kutusu - font metriklerine göre boyutlandır
            let font_size = resolve_css_value(&box_node.style.font_size, containing_width);
            let line_height = resolve_line_height(&box_node.style.line_height, font_size);
            let line_count = estimate_wrapped_line_count(
                box_node.text.as_deref().unwrap_or(""),
                containing_width,
                font_size,
            );

            box_node.dimensions.content.width = containing_width; // max genişlik
            box_node.dimensions.content.height = line_height * line_count as f32;
            box_node.dimensions.content.x = x_offset;
            box_node.dimensions.content.y = *current_y;

            *current_y += box_node.dimensions.content.height;
        }
        _ => {
            // Inline element
            for child in &mut box_node.children {
                layout_inline(child, containing_width, current_y, x_offset);
            }
        }
    }
}

/// Block layout'u baştan sona hesapla
pub fn layout_document(layout_root: &mut LayoutBox, viewport_width: f32, viewport_height: f32) {
    calculate_layout(layout_root, viewport_width, viewport_height);

    // Position absolute/fixed elementleri varsa onları da hesapla (şimdilik atla)
    // TODO: Positioned layout
}

/// CSS değerini çözümle
pub fn resolve_css_value(value: &CssValue, parent: f32) -> f32 {
    match value {
        CssValue::Pixels(v) => *v,
        CssValue::Percentage(p) => parent * p / 100.0,
        CssValue::Auto | CssValue::None => 0.0,
        CssValue::Inherit => parent,
        CssValue::Zero => 0.0,
    }
}

pub fn resolve_line_height(value: &CssValue, font_size: f32) -> f32 {
    match value {
        CssValue::Pixels(v) => {
            if *v > 0.0 {
                *v
            } else {
                font_size * 1.2
            }
        }
        CssValue::Percentage(p) => font_size * p / 100.0,
        CssValue::Auto | CssValue::None | CssValue::Inherit | CssValue::Zero => font_size * 1.2,
    }
}

fn resolve_edge_value(value: &CssValue, parent: f32) -> f32 {
    match value {
        CssValue::Auto | CssValue::None => 0.0,
        _ => resolve_css_value(value, parent),
    }
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn estimate_wrapped_line_count(text: &str, max_width: f32, font_size: f32) -> usize {
    if text.trim().is_empty() {
        return 1;
    }

    let char_width = (font_size * 0.55).max(1.0);
    let max_chars = (max_width / char_width).floor().max(1.0) as usize;
    let mut lines = 1usize;
    let mut current = 0usize;

    for word in text.split_whitespace() {
        let len = word.chars().count();
        if current == 0 {
            current = len;
        } else if current + 1 + len > max_chars {
            lines += 1;
            current = len;
        } else {
            current += 1 + len;
        }

        if current > max_chars {
            let extra = (current - 1) / max_chars;
            lines += extra;
            current %= max_chars;
        }
    }

    lines.max(1)
}

/// Layout ağacını yazdır (debug)
pub fn print_layout(box_node: &LayoutBox, indent: usize) {
    let prefix = "  ".repeat(indent);
    let box_type_str = match box_node.box_type {
        BoxType::Block => "Block",
        BoxType::Inline => "Inline",
        BoxType::AnonymousBlock => "AnonymousBlock",
        BoxType::Text => "Text",
        BoxType::Flex => "Flex",
    };

    let dim = &box_node.dimensions;
    println!(
        "{}[{}] x={:.0} y={:.0} w={:.0} h={:.0} (margin={:.0},{:.0} padding={:.0},{:.0})",
        prefix,
        box_type_str,
        dim.content.x,
        dim.content.y,
        dim.content.width,
        dim.content.height,
        dim.margin.left,
        dim.margin.top,
        dim.padding.left,
        dim.padding.top,
    );

    for child in &box_node.children {
        print_layout(child, indent + 1);
    }
}
