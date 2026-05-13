/// Render engine - tiny-skia ile sayfa çizimi
use crate::layout::{BoxType, LayoutBox};
use crate::types::*;
use std::collections::HashMap;

/// Render listesi öğesi
#[derive(Debug, Clone)]
pub enum DisplayCommand {
    /// Dikdörtgen dolgu (arkaplan rengi)
    SolidRect(Color, Rect),
    /// Metin
    Text {
        text: String,
        color: Color,
        x: f32,
        y: f32,
        font_size: f32,
    },
    /// Border
    Border {
        rect: Rect,
        top_width: f32,
        right_width: f32,
        bottom_width: f32,
        left_width: f32,
        top_color: Color,
        right_color: Color,
        bottom_color: Color,
        left_color: Color,
    },
    /// Resim
    Image { rect: Rect },
}

/// Sayfayı render et ve display command listesi oluştur
pub fn build_display_list(layout_root: &LayoutBox, commands: &mut Vec<DisplayCommand>) {
    render_box(layout_root, commands);
}

fn render_box(box_node: &LayoutBox, commands: &mut Vec<DisplayCommand>) {
    let dim = &box_node.dimensions;

    if box_node.style.visibility != "visible" || box_node.style.opacity <= 0.0 {
        return;
    }

    if dim.content.width <= 0.0 || dim.content.height <= 0.0 {
        for child in &box_node.children {
            render_box(child, commands);
        }
        return;
    }

    let border_rect = dim.border_box();

    // Arkaplan rengi
    let bg = box_node.style.background_color;
    if bg.a > 0.0 {
        commands.push(DisplayCommand::SolidRect(bg, border_rect));
    }

    // Border
    let has_border = (dim.border.top > 0.0
        || dim.border.right > 0.0
        || dim.border.bottom > 0.0
        || dim.border.left > 0.0);

    if has_border {
        commands.push(DisplayCommand::Border {
            rect: border_rect,
            top_width: dim.border.top,
            right_width: dim.border.right,
            bottom_width: dim.border.bottom,
            left_width: dim.border.left,
            top_color: box_node.style.border_top_color,
            right_color: box_node.style.border_right_color,
            bottom_color: box_node.style.border_bottom_color,
            left_color: box_node.style.border_left_color,
        });
    }

    match box_node.box_type {
        BoxType::Text => {
            let font_size = crate::layout::resolve_css_value(&box_node.style.font_size, 16.0);
            let color = box_node.style.color;

            commands.push(DisplayCommand::Text {
                text: "[Metin]".to_string(),
                color,
                x: dim.content.x,
                y: dim.content.y + font_size,
                font_size,
            });
        }
        _ => {
            for child in &box_node.children {
                render_box(child, commands);
            }
        }
    }
}

/// Display command'ları tiny-skia pixmap'e çiz
pub fn render_to_pixmap(
    commands: &[DisplayCommand],
    width: u32,
    height: u32,
) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .expect("Pixmap oluşturulamadı");
    pixmap.fill(tiny_skia::Color::WHITE);

    for cmd in commands {
        render_command(cmd, &mut pixmap);
    }

    pixmap
}

fn render_command(cmd: &DisplayCommand, pixmap: &mut tiny_skia::Pixmap) {
    match cmd {
        DisplayCommand::SolidRect(color, rect) => {
            let mut pb = tiny_skia::Paint::default();
            pb.set_color_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                (color.a * 255.0) as u8,
            );

            let r = tiny_skia::Rect::from_xywh(
                rect.x.max(0.0),
                rect.y.max(0.0),
                rect.width.max(0.0),
                rect.height.max(0.0),
            );

            if let Some(r) = r {
                pixmap.fill_rect(r, &pb, tiny_skia::Transform::identity(), None);
            }
        }
        DisplayCommand::Text {
            text,
            color,
            x,
            y,
            font_size: _,
        } => {
            // Basit metin çizimi - font olmadan dikdörtgen olarak işaretle
            // Gerçek metin render'ı ab_glyph ile yapılacak
            let mut pb = tiny_skia::Paint::default();
            pb.set_color_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                (color.a * 255.0) as u8,
            );

            // Metnin genişliğini karakter sayısına göre tahmin et
            let text_width = text.len() as f32 * 8.0;
            let text_height = 16.0;

            // Metnin arkaplanını yarı-saydam yap
            if let Some(r) = tiny_skia::Rect::from_xywh(*x, *y - text_height, text_width, text_height) {
                pb.set_color_rgba8(240, 240, 240, 100);
                pixmap.fill_rect(r, &pb, tiny_skia::Transform::identity(), None);
            }

            // Metin içeriğini görselleştirmek için basit bir gösterge çiz
            pb.set_color_rgba8(
                (color.r * 255.0) as u8,
                (color.g * 255.0) as u8,
                (color.b * 255.0) as u8,
                (color.a * 255.0) as u8,
            );
            if let Some(r) = tiny_skia::Rect::from_xywh(*x, *y - 2.0, text_width.min(200.0), 2.0) {
                pixmap.fill_rect(r, &pb, tiny_skia::Transform::identity(), None);
            }
        }
        DisplayCommand::Border {
            rect,
            top_width,
            right_width,
            bottom_width,
            left_width,
            top_color,
            right_color,
            bottom_color,
            left_color,
        } => {
            let x = rect.x;
            let y = rect.y;
            let w = rect.width;
            let h = rect.height;

            if *top_width > 0.0 && top_color.a > 0.0 {
                draw_line_rect(pixmap, x, y, x + w, y, *top_width, *top_color);
            }
            if *right_width > 0.0 && right_color.a > 0.0 {
                draw_line_rect(pixmap, x + w, y, x + w, y + h, *right_width, *right_color);
            }
            if *bottom_width > 0.0 && bottom_color.a > 0.0 {
                draw_line_rect(pixmap, x, y + h, x + w, y + h, *bottom_width, *bottom_color);
            }
            if *left_width > 0.0 && left_color.a > 0.0 {
                draw_line_rect(pixmap, x, y, x, y + h, *left_width, *left_color);
            }
        }
        DisplayCommand::Image { rect } => {
            // Resim placeholder - mavi dikdörtgen
            let mut pb = tiny_skia::Paint::default();
            pb.set_color_rgba8(100, 150, 255, 200);
            let r = tiny_skia::Rect::from_xywh(rect.x, rect.y, rect.width.max(0.0), rect.height.max(0.0));
            if let Some(r) = r {
                pixmap.fill_rect(r, &pb, tiny_skia::Transform::identity(), None);
            }
        }
    }
}

fn draw_line_rect(
    pixmap: &mut tiny_skia::Pixmap,
    x1: f32, y1: f32, x2: f32, y2: f32,
    width: f32,
    color: Color,
) {
    let mut pb = tiny_skia::Paint::default();
    pb.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );

    // Kalın çizgiyi ince bir dikdörtgen olarak çiz
    let (rx, ry, rw, rh) = if x1 == x2 {
        // Dikey çizgi
        (x1 - width / 2.0, y1.min(y2), width, (y2 - y1).abs())
    } else {
        // Yatay çizgi
        (x1.min(x2), y1 - width / 2.0, (x2 - x1).abs(), width)
    };

    if let Some(r) = tiny_skia::Rect::from_xywh(rx, ry, rw, rh) {
        pixmap.fill_rect(r, &pb, tiny_skia::Transform::identity(), None);
    }
}
