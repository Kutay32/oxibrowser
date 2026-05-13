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
            let trimmed = text.trim();
            if trimmed.is_empty() {
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
        children,
    }
}

/// Layout hesapla
pub fn calculate_layout(
    layout_root: &mut LayoutBox,
    containing_width: f32,
    containing_height: f32,
) {
    match layout_root.box_type {
        BoxType::Block | BoxType::AnonymousBlock => {
            layout_block(layout_root, containing_width);
        }
        BoxType::Inline | BoxType::Text => {
            // Inline layout, block layout içinde hesaplanır
        }
        BoxType::Flex => {
            layout_block(layout_root, containing_width);
        }
    }
}

/// Block layout hesapla
fn layout_block(box_node: &mut LayoutBox, parent_width: f32) {
    let style = &box_node.style;

    // Kenar değerlerini hesapla
    let margin_top = resolve_css_value(&style.margin_top, parent_width);
    let margin_right = resolve_css_value(&style.margin_right, parent_width);
    let margin_bottom = resolve_css_value(&style.margin_bottom, parent_width);
    let margin_left = resolve_css_value(&style.margin_left, parent_width);

    let padding_top = resolve_css_value(&style.padding_top, parent_width);
    let padding_right = resolve_css_value(&style.padding_right, parent_width);
    let padding_bottom = resolve_css_value(&style.padding_bottom, parent_width);
    let padding_left = resolve_css_value(&style.padding_left, parent_width);

    let border_top = resolve_css_value(&style.border_top_width, parent_width);
    let border_right = resolve_css_value(&style.border_right_width, parent_width);
    let border_bottom = resolve_css_value(&style.border_bottom_width, parent_width);
    let border_left = resolve_css_value(&style.border_left_width, parent_width);

    box_node.dimensions.margin = EdgeSizes::new(margin_top, margin_right, margin_bottom, margin_left);
    box_node.dimensions.padding = EdgeSizes::new(padding_top, padding_right, padding_bottom, padding_left);
    box_node.dimensions.border = EdgeSizes::new(border_top, border_right, border_bottom, border_left);

    // Content genişliğini hesapla
    let width = resolve_css_value(&style.width, parent_width);
    let available_width = parent_width
        - margin_left - margin_right
        - padding_left - padding_right
        - border_left - border_right;

    let content_width = if width == 0.0 || width.is_nan() {
        available_width.max(0.0)
    } else if width > 0.0 {
        width
    } else {
        available_width.max(0.0)
    };

    box_node.dimensions.content.width = content_width;

    // İçindeki block'ları yerleştir
    let mut current_y = 0.0;

    for child in &mut box_node.children {
        match child.box_type {
            BoxType::Block | BoxType::AnonymousBlock => {
                // Child'ın layout'unu hesapla
                layout_block(child, content_width);

                // Y pozisyonunu ayarla
                child.dimensions.content.x = padding_left + border_left;
                child.dimensions.content.y = current_y;

                current_y += child.dimensions.total_height();
            }
            BoxType::Inline | BoxType::Text => {
                // Inline layout
                layout_inline(child, content_width, &mut current_y, padding_left + border_left);
            }
            BoxType::Flex => {
                layout_block(child, content_width);
                child.dimensions.content.x = padding_left + border_left;
                child.dimensions.content.y = current_y;
                current_y += child.dimensions.total_height();
            }
        }
    }

    // Content yüksekliğini hesapla
    let height = resolve_css_value(&style.height, current_y);
    box_node.dimensions.content.height = if height > 0.0 {
        height
    } else {
        current_y + padding_top + padding_bottom + border_top + border_bottom
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
            let line_height = resolve_css_value(&box_node.style.line_height, font_size);
            let line_height = if line_height <= 0.0 {
                font_size * 1.2
            } else {
                line_height
            };

            box_node.dimensions.content.width = containing_width; // max genişlik
            box_node.dimensions.content.height = line_height;
            box_node.dimensions.content.x = x_offset;
            box_node.dimensions.content.y = *current_y;

            *current_y += line_height;
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
