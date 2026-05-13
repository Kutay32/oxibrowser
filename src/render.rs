/// Render engine - tiny-skia ile sayfa çizimi
use crate::layout::{BoxType, LayoutBox};
use crate::types::*;
use ab_glyph::{point, Font, FontArc, FontVec, PxScale, ScaleFont};
use std::sync::OnceLock;

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
        line_height: f32,
        max_width: f32,
        text_align: TextAlign,
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
    let has_border = dim.border.top > 0.0
        || dim.border.right > 0.0
        || dim.border.bottom > 0.0
        || dim.border.left > 0.0;

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
            let line_height =
                crate::layout::resolve_line_height(&box_node.style.line_height, font_size);
            let color = box_node.style.color;

            commands.push(DisplayCommand::Text {
                text: box_node.text.clone().unwrap_or_default(),
                color,
                x: dim.content.x,
                y: dim.content.y,
                font_size,
                line_height,
                max_width: dim.content.width,
                text_align: box_node.style.text_align,
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
pub fn render_to_pixmap(commands: &[DisplayCommand], width: u32, height: u32) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("Pixmap oluşturulamadı");
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

            fill_rect_safe(pixmap, rect.x, rect.y, rect.width, rect.height, &pb);
        }
        DisplayCommand::Text {
            text,
            color,
            x,
            y,
            font_size,
            line_height,
            max_width,
            text_align,
        } => {
            draw_text(
                pixmap,
                text,
                *x,
                *y,
                *max_width,
                *font_size,
                *line_height,
                *color,
                *text_align,
            );
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
                fill_colored_rect(pixmap, x, y, w, *top_width, *top_color);
            }
            if *right_width > 0.0 && right_color.a > 0.0 {
                fill_colored_rect(
                    pixmap,
                    x + w - *right_width,
                    y,
                    *right_width,
                    h,
                    *right_color,
                );
            }
            if *bottom_width > 0.0 && bottom_color.a > 0.0 {
                fill_colored_rect(
                    pixmap,
                    x,
                    y + h - *bottom_width,
                    w,
                    *bottom_width,
                    *bottom_color,
                );
            }
            if *left_width > 0.0 && left_color.a > 0.0 {
                fill_colored_rect(pixmap, x, y, *left_width, h, *left_color);
            }
        }
        DisplayCommand::Image { rect } => {
            // Resim placeholder - mavi dikdörtgen
            let mut pb = tiny_skia::Paint::default();
            pb.set_color_rgba8(100, 150, 255, 200);
            fill_rect_safe(pixmap, rect.x, rect.y, rect.width, rect.height, &pb);
        }
    }
}

fn draw_text(
    pixmap: &mut tiny_skia::Pixmap,
    text: &str,
    x: f32,
    y: f32,
    max_width: f32,
    font_size: f32,
    line_height: f32,
    color: Color,
    text_align: TextAlign,
) {
    let Some(font) = default_font() else {
        draw_text_fallback(pixmap, text, x, y, max_width, font_size, line_height, color);
        return;
    };

    let font_size = font_size.max(1.0);
    let line_height = line_height.max(font_size * 1.05);
    let scale = PxScale::from(font_size);
    let scaled_font = font.as_scaled(scale);
    let ascent = scaled_font.ascent();
    let lines = wrap_text(font, text, max_width.max(1.0), font_size);
    let mut line_top = y;

    for line in lines {
        let line_width = measure_text(font, &line, font_size);
        let line_x = match text_align {
            TextAlign::Center => x + ((max_width - line_width) / 2.0).max(0.0),
            TextAlign::Right => x + (max_width - line_width).max(0.0),
            TextAlign::Left | TextAlign::Justify => x,
        };
        let baseline = line_top + ascent;
        let mut caret = line_x;

        for ch in line.chars() {
            let glyph_id = font.glyph_id(ch);
            let glyph = glyph_id.with_scale_and_position(scale, point(caret, baseline));

            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, coverage| {
                    let px = gx as i32 + bounds.min.x as i32;
                    let py = gy as i32 + bounds.min.y as i32;
                    blend_pixel(pixmap, px, py, color, coverage);
                });
            }

            caret += scaled_font.h_advance(glyph_id);
        }

        line_top += line_height;
    }
}

pub fn draw_ui_text(
    pixmap: &mut tiny_skia::Pixmap,
    text: &str,
    x: f32,
    y: f32,
    max_width: f32,
    font_size: f32,
    color: Color,
    text_align: TextAlign,
) {
    draw_text(
        pixmap,
        text,
        x,
        y,
        max_width,
        font_size,
        font_size * 1.25,
        color,
        text_align,
    );
}

fn draw_text_fallback(
    pixmap: &mut tiny_skia::Pixmap,
    text: &str,
    x: f32,
    y: f32,
    max_width: f32,
    font_size: f32,
    line_height: f32,
    color: Color,
) {
    let char_width = (font_size * 0.55).max(1.0);
    let lines = wrap_text_by_estimate(text, max_width, char_width);
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );

    for (idx, line) in lines.iter().enumerate() {
        let width = (line.chars().count() as f32 * char_width).min(max_width);
        let line_y = y + idx as f32 * line_height + font_size * 0.75;
        fill_rect_safe(pixmap, x, line_y, width, 2.0, &paint);
    }
}

fn default_font() -> Option<&'static FontArc> {
    static FONT: OnceLock<Option<FontArc>> = OnceLock::new();
    FONT.get_or_init(load_system_font).as_ref()
}

fn load_system_font() -> Option<FontArc> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let families = [
        fontdb::Family::Name("Helvetica Neue"),
        fontdb::Family::Name("Helvetica"),
        fontdb::Family::Name("Arial"),
        fontdb::Family::SansSerif,
        fontdb::Family::Serif,
    ];
    let query = fontdb::Query {
        families: &families,
        ..fontdb::Query::default()
    };

    let id = db
        .query(&query)
        .or_else(|| db.faces().next().map(|face| face.id))?;
    db.with_face_data(id, |data, face_index| {
        FontVec::try_from_vec_and_index(data.to_vec(), face_index)
            .ok()
            .map(FontArc::new)
    })
    .flatten()
}

fn wrap_text(font: &FontArc, text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in collapsed.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current, word)
        };

        if current.is_empty() || measure_text(font, &candidate, font_size) <= max_width {
            current = candidate;
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn wrap_text_by_estimate(text: &str, max_width: f32, char_width: f32) -> Vec<String> {
    let max_chars = (max_width / char_width).floor().max(1.0) as usize;
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let candidate_len = if current.is_empty() {
            word.chars().count()
        } else {
            current.chars().count() + 1 + word.chars().count()
        };

        if current.is_empty() || candidate_len <= max_chars {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn measure_text(font: &FontArc, text: &str, font_size: f32) -> f32 {
    let scale = PxScale::from(font_size.max(1.0));
    let scaled_font = font.as_scaled(scale);
    text.chars()
        .map(|ch| scaled_font.h_advance(font.glyph_id(ch)))
        .sum()
}

fn blend_pixel(pixmap: &mut tiny_skia::Pixmap, x: i32, y: i32, color: Color, coverage: f32) {
    if x < 0 || y < 0 {
        return;
    }

    let width = pixmap.width();
    let height = pixmap.height();
    let x = x as u32;
    let y = y as u32;
    if x >= width || y >= height {
        return;
    }

    let alpha = (color.a * coverage).clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }

    let idx = ((y * width + x) as usize) * 4;
    let data = pixmap.data_mut();
    let dst_r = data[idx] as f32 / 255.0;
    let dst_g = data[idx + 1] as f32 / 255.0;
    let dst_b = data[idx + 2] as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;

    data[idx] = ((color.r * alpha + dst_r * inv_alpha) * 255.0).round() as u8;
    data[idx + 1] = ((color.g * alpha + dst_g * inv_alpha) * 255.0).round() as u8;
    data[idx + 2] = ((color.b * alpha + dst_b * inv_alpha) * 255.0).round() as u8;
    data[idx + 3] = 255;
}

fn fill_colored_rect(
    pixmap: &mut tiny_skia::Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) {
    let mut pb = tiny_skia::Paint::default();
    pb.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );
    fill_rect_safe(pixmap, x, y, width, height, &pb);
}

pub fn fill_solid_rect(
    pixmap: &mut tiny_skia::Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) {
    fill_colored_rect(pixmap, x, y, width, height, color);
}

fn fill_rect_safe(
    pixmap: &mut tiny_skia::Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    paint: &tiny_skia::Paint,
) {
    if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
        return;
    }

    let x0 = x.max(0.0).floor();
    let y0 = y.max(0.0).floor();
    let x1 = (x + width).min(pixmap.width() as f32).ceil();
    let y1 = (y + height).min(pixmap.height() as f32).ceil();
    let width = x1 - x0;
    let height = y1 - y0;

    if width <= 0.0 || height <= 0.0 {
        return;
    }

    if let Some(rect) = tiny_skia::Rect::from_xywh(x0, y0, width, height) {
        pixmap.fill_rect(rect, paint, tiny_skia::Transform::identity(), None);
    }
}
