/// OxiBrowser ortak veri tipleri

// CSS renk dönüşümü - cssparser bağımlılığı olmadan
// impl From<&cssparser::color::PredefinedColorSpace> for Color {
//     fn from(cs: &cssparser::color::PredefinedColorSpace) -> Self {
//         Color::BLACK
//     }
// }

/// RGBA rengi
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    pub const fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_tiny_skia(self) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    pub const BLACK: Color = Color::new(0, 0, 0, 255);
    pub const WHITE: Color = Color::new(255, 255, 255, 255);
    pub const TRANSPARENT: Color = Color::new(0, 0, 0, 0);
    pub const BLUE: Color = Color::new(0, 0, 255, 255);
    pub const RED: Color = Color::new(255, 0, 0, 255);
    pub const GRAY: Color = Color::new(128, 128, 128, 255);
    pub const LIGHT_GRAY: Color = Color::new(200, 200, 200, 255);
    pub const DARK_GRAY: Color = Color::new(50, 50, 50, 255);
}

/// 2D noktası
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Boyut
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub const fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

/// Dikdörtgen
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }
}

/// CSS birim değeri
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssValue {
    Pixels(f32),
    Percentage(f32),
    Auto,
    None,
    Inherit,
    Zero,
}

impl CssValue {
    pub fn resolve(&self, parent: f32, self_val: f32) -> f32 {
        match self {
            CssValue::Pixels(v) => *v,
            CssValue::Percentage(p) => parent * p / 100.0,
            CssValue::Auto => self_val,
            CssValue::None => 0.0,
            CssValue::Inherit => parent,
            CssValue::Zero => 0.0,
        }
    }

    pub fn pixels(&self) -> Option<f32> {
        if let CssValue::Pixels(v) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl Default for CssValue {
    fn default() -> Self {
        CssValue::Auto
    }
}

/// Görüntüleme tipi
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayType {
    Block,
    Inline,
    None,
    Flex,
    Grid,
    InlineBlock,
}

impl Default for DisplayType {
    fn default() -> Self {
        DisplayType::Block
    }
}

/// Pozisyon tipi
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionType {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

impl Default for PositionType {
    fn default() -> Self {
        PositionType::Static
    }
}

/// Metin hizalama
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

impl Default for TextAlign {
    fn default() -> Self {
        TextAlign::Left
    }
}

/// Font stili
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Normal
    }
}

/// Font ağırlığı
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Bolder,
    Lighter,
    Weight(u16),
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Normal
    }
}

impl FontWeight {
    pub fn to_number(&self) -> u16 {
        match self {
            FontWeight::Normal => 400,
            FontWeight::Bold => 700,
            FontWeight::Bolder => 700,
            FontWeight::Lighter => 300,
            FontWeight::Weight(w) => *w,
        }
    }
}
