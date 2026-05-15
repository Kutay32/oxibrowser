/// Icon drawing utilities - tiny-skia ile vektÃ¶rel ikonlar
use crate::types::Color;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

fn make_stroke(width: f32) -> Stroke {
    Stroke {
        width,
        miter_limit: 4.0,
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        dash: None,
    }
}

pub fn draw_icon(pixmap: &mut Pixmap, icon: &str, x: f32, y: f32, size: f32, color: Color) {
    let s = size;
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );
    let stroke = make_stroke(s * 0.1);

    match icon {
        "back" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.65, y + s * 0.15);
            pb.line_to(x + s * 0.3, y + s * 0.5);
            pb.line_to(x + s * 0.65, y + s * 0.85);
            pb.move_to(x + s * 0.3, y + s * 0.5);
            pb.line_to(x + s * 0.85, y + s * 0.5);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "forward" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.35, y + s * 0.15);
            pb.line_to(x + s * 0.7, y + s * 0.5);
            pb.line_to(x + s * 0.35, y + s * 0.85);
            pb.move_to(x + s * 0.7, y + s * 0.5);
            pb.line_to(x + s * 0.15, y + s * 0.5);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "refresh" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let r = s * 0.3;
            let mut pb = PathBuilder::new();
            // Approximate circle with quad curves
            let k = r * 0.55;
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - r, cy - r, cx - k, cy - r);
            pb.quad_to(cx, cy - r, cx, cy - r);
            // Arrow tip
            pb.move_to(cx + r * 0.5, cy - r * 0.9);
            pb.line_to(cx + r * 0.9, cy - r * 0.5);
            pb.line_to(cx + r * 0.9, cy - r * 0.1);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "close" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.25, y + s * 0.25);
            pb.line_to(x + s * 0.75, y + s * 0.75);
            pb.move_to(x + s * 0.75, y + s * 0.25);
            pb.line_to(x + s * 0.25, y + s * 0.75);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "plus" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.5, y + s * 0.2);
            pb.line_to(x + s * 0.5, y + s * 0.8);
            pb.move_to(x + s * 0.2, y + s * 0.5);
            pb.line_to(x + s * 0.8, y + s * 0.5);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "star" | "star_outline" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let outer_r = s * 0.42;
            let inner_r = s * 0.2;
            let mut pb = PathBuilder::new();
            for i in 0..10 {
                let angle = std::f32::consts::PI / 2.0 + (i as f32) * std::f32::consts::PI / 5.0;
                let r = if i % 2 == 0 { outer_r } else { inner_r };
                let px = cx + r * angle.cos();
                let py = cy - r * angle.sin();
                if i == 0 {
                    pb.move_to(px, py);
                } else {
                    pb.line_to(px, py);
                }
            }
            pb.close();
            if let Some(p) = pb.finish() {
                if icon == "star" {
                    pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
                } else {
                    pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
                }
            }
        }
        "user" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let mut pb = PathBuilder::new();
            // Head circle
            let hr = s * 0.18;
            let hk = hr * 0.55;
            pb.move_to(cx, cy - s * 0.33);
            pb.quad_to(cx + hk, cy - s * 0.33, cx + hr, cy - s * 0.33 + hk);
            pb.quad_to(cx + hr, cy - s * 0.15, cx + hk, cy - s * 0.15);
            pb.quad_to(cx, cy - s * 0.15, cx - hk, cy - s * 0.15);
            pb.quad_to(cx - hr, cy - s * 0.15, cx - hr, cy - s * 0.33 + hk);
            pb.quad_to(cx - hr, cy - s * 0.33, cx - hk, cy - s * 0.33);
            pb.quad_to(cx, cy - s * 0.33, cx, cy - s * 0.33);
            pb.close();
            // Body
            pb.move_to(cx - s * 0.35, y + s * 0.85);
            pb.line_to(cx - s * 0.35, y + s * 0.6);
            pb.quad_to(cx - s * 0.35, y + s * 0.35, cx, y + s * 0.35);
            pb.quad_to(cx + s * 0.35, y + s * 0.35, cx + s * 0.35, y + s * 0.6);
            pb.line_to(cx + s * 0.35, y + s * 0.85);
            pb.close();
            if let Some(p) = pb.finish() {
                pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        "menu" => {
            let mut pb = PathBuilder::new();
            for i in 0..3 {
                let dy = y + s * (0.25 + i as f32 * 0.25);
                pb.move_to(x + s * 0.2, dy);
                pb.line_to(x + s * 0.8, dy);
            }
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "lock" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.55;
            let mut pb = PathBuilder::new();
            // Lock body
            pb.move_to(cx - s * 0.25, cy);
            pb.line_to(cx - s * 0.25, cy + s * 0.35);
            pb.line_to(cx + s * 0.25, cy + s * 0.35);
            pb.line_to(cx + s * 0.25, cy);
            pb.close();
            if let Some(p) = pb.finish() {
                pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
            // Lock shackle
            let mut pb2 = PathBuilder::new();
            pb2.move_to(cx - s * 0.15, cy);
            pb2.line_to(cx - s * 0.15, cy - s * 0.15);
            pb2.quad_to(cx, cy - s * 0.35, cx + s * 0.15, cy - s * 0.15);
            pb2.line_to(cx + s * 0.15, cy);
            if let Some(p) = pb2.finish() {
                let fill_paint = Paint::default();
                pixmap.stroke_path(&p, &fill_paint, &stroke, Transform::identity(), None);
            }
        }
        "search" => {
            let cx = x + s * 0.4;
            let cy = y + s * 0.4;
            let r = s * 0.28;
            let k = r * 0.55;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - k, cy - r, cx, cy - r);
            pb.close();
            pb.move_to(cx + r * 0.7, cy + r * 0.7);
            pb.line_to(cx + s * 0.85, cy + s * 0.85);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "download" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.5, y + s * 0.15);
            pb.line_to(x + s * 0.5, y + s * 0.65);
            pb.move_to(x + s * 0.3, y + s * 0.45);
            pb.line_to(x + s * 0.5, y + s * 0.65);
            pb.line_to(x + s * 0.7, y + s * 0.45);
            pb.move_to(x + s * 0.2, y + s * 0.75);
            pb.line_to(x + s * 0.8, y + s * 0.75);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "history" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let r = s * 0.35;
            let k = r * 0.55;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - k, cy - r, cx, cy - r);
            pb.close();
            pb.move_to(cx, cy);
            pb.line_to(cx, cy - r * 0.6);
            pb.move_to(cx, cy);
            pb.line_to(cx + r * 0.5, cy);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "bookmark" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.2, y + s * 0.1);
            pb.line_to(x + s * 0.8, y + s * 0.1);
            pb.line_to(x + s * 0.8, y + s * 0.85);
            pb.line_to(x + s * 0.5, y + s * 0.65);
            pb.line_to(x + s * 0.2, y + s * 0.85);
            pb.close();
            if let Some(p) = pb.finish() {
                pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        "globe" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let r = s * 0.38;
            let k = r * 0.55;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - k, cy - r, cx, cy - r);
            pb.close();
            pb.move_to(cx - r, cy);
            pb.line_to(cx + r, cy);
            // Vertical ellipse approximation
            let vr = r * 0.5;
            let vk = vr * 0.55;
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + vk, cy - r, cx + vr, cy - r + vk);
            pb.quad_to(cx + vr, cy, cx + vr, cy + r - vk);
            pb.quad_to(cx + vk, cy + r, cx, cy + r);
            pb.quad_to(cx - vk, cy + r, cx - vr, cy + r - vk);
            pb.quad_to(cx - vr, cy, cx - vr, cy - r + vk);
            pb.quad_to(cx - vk, cy - r, cx, cy - r);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "settings" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let r = s * 0.3;
            let k = r * 0.55;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - k, cy - r, cx, cy - r);
            pb.close();
            if let Some(p) = pb.finish() {
                pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
            // Center dot
            let mut pb2 = PathBuilder::new();
            let dr = s * 0.08;
            pb2.move_to(cx, cy - dr);
            pb2.quad_to(cx + dr * 0.55, cy - dr, cx + dr, cy - dr * 0.55);
            pb2.quad_to(cx + dr, cy, cx + dr * 0.55, cy + dr);
            pb2.quad_to(cx, cy + dr, cx - dr * 0.55, cy + dr);
            pb2.quad_to(cx - dr, cy + dr, cx - dr, cy + dr * 0.55);
            pb2.quad_to(cx - dr, cy, cx - dr * 0.55, cy - dr);
            pb2.quad_to(cx, cy - dr, cx, cy - dr);
            pb2.close();
            if let Some(p) = pb2.finish() {
                let mut white_paint = Paint::default();
                white_paint.set_color_rgba8(255, 255, 255, 255);
                pixmap.fill_path(&p, &white_paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        "key" => {
            let cx = x + s * 0.35;
            let cy = y + s * 0.35;
            let r = s * 0.22;
            let k = r * 0.55;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - k, cy - r, cx, cy - r);
            pb.close();
            pb.move_to(cx + r * 0.7, cy + r * 0.7);
            pb.line_to(x + s * 0.85, y + s * 0.85);
            pb.move_to(x + s * 0.7, y + s * 0.7);
            pb.line_to(x + s * 0.75, y + s * 0.65);
            pb.move_to(x + s * 0.8, y + s * 0.75);
            pb.line_to(x + s * 0.85, y + s * 0.7);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "cookie" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let r = s * 0.38;
            let k = r * 0.55;
            let mut pb = PathBuilder::new();
            pb.move_to(cx, cy - r);
            pb.quad_to(cx + k, cy - r, cx + r, cy - k);
            pb.quad_to(cx + r, cy, cx + r, cy + k);
            pb.quad_to(cx + r, cy + r, cx + k, cy + r);
            pb.quad_to(cx, cy + r, cx - k, cy + r);
            pb.quad_to(cx - r, cy + r, cx - r, cy + k);
            pb.quad_to(cx - r, cy, cx - r, cy - k);
            pb.quad_to(cx - k, cy - r, cx, cy - r);
            pb.close();
            if let Some(p) = pb.finish() {
                pixmap.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
            // Chips
            let mut fill_paint = Paint::default();
            fill_paint.set_color_rgba8(255, 255, 255, 200);
            for (dx, dy, dr) in &[(0.15, -0.1, 0.06), (-0.12, 0.12, 0.05), (0.1, 0.15, 0.04), (-0.15, -0.08, 0.05)] {
                let mut pb2 = PathBuilder::new();
                let chip_cx = cx + s * dx;
                let chip_cy = cy + s * dy;
                let chip_r = s * dr;
                let chip_k = chip_r * 0.55;
                pb2.move_to(chip_cx, chip_cy - chip_r);
                pb2.quad_to(chip_cx + chip_k, chip_cy - chip_r, chip_cx + chip_r, chip_cy - chip_k);
                pb2.quad_to(chip_cx + chip_r, chip_cy, chip_cx + chip_r, chip_cy + chip_k);
                pb2.quad_to(chip_cx + chip_r, chip_cy + chip_r, chip_cx + chip_k, chip_cy + chip_r);
                pb2.quad_to(chip_cx, chip_cy + chip_r, chip_cx - chip_k, chip_cy + chip_r);
                pb2.quad_to(chip_cx - chip_r, chip_cy + chip_r, chip_cx - chip_r, chip_cy + chip_k);
                pb2.quad_to(chip_cx - chip_r, chip_cy, chip_cx - chip_r, chip_cy - chip_k);
                pb2.quad_to(chip_cx - chip_k, chip_cy - chip_r, chip_cx, chip_cy - chip_r);
                pb2.close();
                if let Some(p) = pb2.finish() {
                    pixmap.fill_path(&p, &fill_paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        "sync" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let r = s * 0.3;
            let mut pb = PathBuilder::new();
            // Top arc
            pb.move_to(cx - r * 0.7, cy - r * 0.3);
            pb.quad_to(cx, cy - r * 0.8, cx + r * 0.7, cy - r * 0.3);
            pb.move_to(cx + r * 0.4, cy - r * 0.5);
            pb.line_to(cx + r * 0.7, cy - r * 0.3);
            pb.line_to(cx + r * 0.4, cy - r * 0.1);
            // Bottom arc
            pb.move_to(cx + r * 0.7, cy + r * 0.3);
            pb.quad_to(cx, cy + r * 0.8, cx - r * 0.7, cy + r * 0.3);
            pb.move_to(cx - r * 0.4, cy + r * 0.1);
            pb.line_to(cx - r * 0.7, cy + r * 0.3);
            pb.line_to(cx - r * 0.4, cy + r * 0.5);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "extension" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.2, y + s * 0.2);
            pb.line_to(x + s * 0.8, y + s * 0.2);
            pb.line_to(x + s * 0.8, y + s * 0.8);
            pb.line_to(x + s * 0.2, y + s * 0.8);
            pb.close();
            pb.move_to(x + s * 0.5, y + s * 0.2);
            pb.line_to(x + s * 0.5, y + s * 0.8);
            pb.move_to(x + s * 0.2, y + s * 0.5);
            pb.line_to(x + s * 0.8, y + s * 0.5);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "devtools" => {
            let mut pb = PathBuilder::new();
            pb.move_to(x + s * 0.25, y + s * 0.2);
            pb.line_to(x + s * 0.1, y + s * 0.5);
            pb.line_to(x + s * 0.25, y + s * 0.8);
            pb.move_to(x + s * 0.75, y + s * 0.2);
            pb.line_to(x + s * 0.9, y + s * 0.5);
            pb.line_to(x + s * 0.75, y + s * 0.8);
            pb.move_to(x + s * 0.55, y + s * 0.15);
            pb.line_to(x + s * 0.45, y + s * 0.85);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        "incognito" => {
            let cx = x + s * 0.5;
            let cy = y + s * 0.5;
            let lr = s * 0.15;
            let lk = lr * 0.55;
            let mut pb = PathBuilder::new();
            // Left lens
            pb.move_to(cx - s * 0.2, cy - lr);
            pb.quad_to(cx - s * 0.2 + lk, cy - lr, cx - s * 0.2 + lr, cy - lr + lk);
            pb.quad_to(cx - s * 0.2 + lr, cy, cx - s * 0.2 + lr, cy + lr - lk);
            pb.quad_to(cx - s * 0.2 + lk, cy + lr, cx - s * 0.2, cy + lr);
            pb.quad_to(cx - s * 0.2 - lk, cy + lr, cx - s * 0.2 - lr, cy + lr - lk);
            pb.quad_to(cx - s * 0.2 - lr, cy, cx - s * 0.2 - lr, cy - lr + lk);
            pb.quad_to(cx - s * 0.2 - lk, cy - lr, cx - s * 0.2, cy - lr);
            pb.close();
            // Right lens
            pb.move_to(cx + s * 0.2, cy - lr);
            pb.quad_to(cx + s * 0.2 + lk, cy - lr, cx + s * 0.2 + lr, cy - lr + lk);
            pb.quad_to(cx + s * 0.2 + lr, cy, cx + s * 0.2 + lr, cy + lr - lk);
            pb.quad_to(cx + s * 0.2 + lk, cy + lr, cx + s * 0.2, cy + lr);
            pb.quad_to(cx + s * 0.2 - lk, cy + lr, cx + s * 0.2 - lr, cy + lr - lk);
            pb.quad_to(cx + s * 0.2 - lr, cy, cx + s * 0.2 - lr, cy - lr + lk);
            pb.quad_to(cx + s * 0.2 - lk, cy - lr, cx + s * 0.2, cy - lr);
            pb.close();
            // Bridge
            pb.move_to(cx - s * 0.05, cy - lr);
            pb.line_to(cx + s * 0.05, cy - lr);
            if let Some(p) = pb.finish() {
                pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
        }
        _ => {
            crate::render::draw_ui_text(pixmap, icon, x, y, size, size * 0.8, color, crate::types::TextAlign::Center);
        }
    }
}
