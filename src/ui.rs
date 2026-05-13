/// Browser UI - winit + softbuffer ile pencere yönetimi
use crate::browser::Browser;
use crate::types::{Color, TextAlign};
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

const TAB_STRIP_HEIGHT: f32 = 40.0;
const TOOLBAR_HEIGHT: f32 = 52.0;
const CHROME_HEIGHT: f32 = TAB_STRIP_HEIGHT + TOOLBAR_HEIGHT + 1.0;

/// Browser UI yöneticisi
pub struct BrowserUI {
    browser: Arc<Mutex<Browser>>,
}

impl BrowserUI {
    pub fn new(browser: Browser) -> Self {
        Self {
            browser: Arc::new(Mutex::new(browser)),
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        let event_loop =
            EventLoop::new().map_err(|e| format!("Event loop oluşturulamadı: {}", e))?;

        let mut app = BrowserApp {
            ui: self.browser.clone(),
            pixmap: None,
            window: None,
            context: None,
            surface: None,
        };

        event_loop
            .run_app(&mut app)
            .map_err(|e| format!("Event loop hatası: {}", e))?;

        Ok(())
    }
}

/// winit ApplicationHandler implementasyonu
struct BrowserApp {
    ui: Arc<Mutex<Browser>>,
    pixmap: Option<Pixmap>,
    window: Option<Arc<Window>>,
    context: Option<softbuffer::Context<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
}

#[derive(Clone)]
struct ChromeSnapshot {
    title: String,
    url: String,
    status: String,
    can_back: bool,
    can_forward: bool,
    loading: bool,
    tab_count: usize,
}

impl ApplicationHandler for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("OxiBrowser")
            .with_inner_size(LogicalSize::new(1024.0, 768.0));

        let window = event_loop
            .create_window(window_attrs)
            .expect("Pencere oluşturulamadı");

        let window = Arc::new(window);

        // Context oluştur (window'un display handle'ını kullan)
        let context = softbuffer::Context::new(window.clone()).ok();
        self.context = context;

        // Surface oluştur
        if let Some(ref context) = self.context {
            let surface = Surface::new(context, window.clone()).ok();
            self.surface = surface;
        }

        self.window = Some(window.clone());

        let size = window.inner_size();
        self.pixmap = Pixmap::new(size.width.max(1), size.height.max(1));
        if let Some(ref mut p) = self.pixmap {
            p.fill(tiny_skia::Color::WHITE);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.pixmap = Pixmap::new(size.width, size.height);
                    if let Some(ref mut p) = self.pixmap {
                        p.fill(tiny_skia::Color::WHITE);
                    }
                    if let Some(ref mut surface) = self.surface {
                        let _ = surface.resize(
                            NonZeroU32::new(size.width).unwrap(),
                            NonZeroU32::new(size.height).unwrap(),
                        );
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::KeyCode;
                let key = event.physical_key;
                match key {
                    winit::keyboard::PhysicalKey::Code(KeyCode::Escape) => {
                        event_loop.exit();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl BrowserApp {
    fn render(&mut self) {
        let pixmap = match &mut self.pixmap {
            Some(p) => p,
            None => return,
        };

        let width = pixmap.width();
        let height = pixmap.height();
        pixmap.fill(tiny_skia::Color::WHITE);

        // Browser'dan render et
        let (page_pixmap, chrome) = {
            let mut browser = self.ui.lock().unwrap();
            let page_height = height.saturating_sub(CHROME_HEIGHT as u32).max(1);
            browser.relayout(width as f32, page_height as f32);
            let chrome = ChromeSnapshot::from_browser(&browser);

            let page_pixmap = if let Some(layout_root) = &browser.layout_result {
                let mut commands = Vec::new();
                crate::render::build_display_list(layout_root, &mut commands);
                Some(crate::render::render_to_pixmap(
                    &commands,
                    width,
                    page_height,
                ))
            } else {
                None
            };
            (page_pixmap, chrome)
        };

        if let Some(page_pixmap) = page_pixmap {
            blit_pixmap(pixmap, &page_pixmap, 0, CHROME_HEIGHT as u32);
        } else {
            draw_empty_page(pixmap, width, height);
        }
        draw_chrome_shell(pixmap, width, &chrome);

        // softbuffer: draw to screen
        if let Some(ref mut surface) = self.surface {
            if let Ok(mut buffer) = surface.buffer_mut() {
                let buf_w = buffer.width().get() as usize;
                let buf_h = buffer.height().get() as usize;
                for y in 0..buf_h.min(height as usize) {
                    for x in 0..buf_w.min(width as usize) {
                        if let Some(p) = pixmap.pixel(x as u32, y as u32) {
                            let idx = y * buf_w + x;
                            buffer[idx] = (p.blue() as u32)
                                | ((p.green() as u32) << 8)
                                | ((p.red() as u32) << 16);
                        }
                    }
                }
                let _ = buffer.present();
            }
        }
    }
}

impl ChromeSnapshot {
    fn from_browser(browser: &Browser) -> Self {
        let active_tab = browser.tabs.get(browser.active_tab);
        let title = active_tab
            .map(|tab| tab.title.as_str())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or("Yeni Sekme")
            .to_string();
        let url = if !browser.current_url.trim().is_empty() {
            browser.current_url.clone()
        } else {
            active_tab
                .map(|tab| tab.url.clone())
                .filter(|url| !url.trim().is_empty())
                .unwrap_or_else(|| "about:welcome".to_string())
        };
        let can_back = active_tab.map(|tab| tab.history_pos > 0).unwrap_or(false);
        let can_forward = active_tab
            .map(|tab| tab.history_pos + 1 < tab.history.len())
            .unwrap_or(false);

        Self {
            title,
            url,
            status: browser.status_message.clone(),
            can_back,
            can_forward,
            loading: browser.loading,
            tab_count: browser.tabs.len().max(1),
        }
    }
}

fn blit_pixmap(dest: &mut Pixmap, src: &Pixmap, x: u32, y: u32) {
    if x >= dest.width() || y >= dest.height() {
        return;
    }

    let copy_width = src.width().min(dest.width() - x) as usize;
    let copy_height = src.height().min(dest.height() - y) as usize;
    let dest_stride = dest.width() as usize * 4;
    let src_stride = src.width() as usize * 4;
    let dest_x = x as usize * 4;
    let dest_y = y as usize;
    let bytes = copy_width * 4;
    let src_data = src.data();
    let dest_data = dest.data_mut();

    for row in 0..copy_height {
        let src_start = row * src_stride;
        let dest_start = (dest_y + row) * dest_stride + dest_x;
        dest_data[dest_start..dest_start + bytes]
            .copy_from_slice(&src_data[src_start..src_start + bytes]);
    }
}

fn draw_empty_page(pixmap: &mut Pixmap, width: u32, height: u32) {
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        CHROME_HEIGHT,
        width as f32,
        (height as f32 - CHROME_HEIGHT).max(0.0),
        Color::new(248, 249, 250, 255),
    );
}

fn draw_chrome_shell(pixmap: &mut Pixmap, width: u32, chrome: &ChromeSnapshot) {
    let width = width as f32;
    draw_tab_strip(pixmap, width, chrome);
    draw_toolbar(pixmap, width, chrome);
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        CHROME_HEIGHT - 1.0,
        width,
        1.0,
        Color::new(218, 220, 224, 255),
    );
}

fn draw_tab_strip(pixmap: &mut Pixmap, width: f32, chrome: &ChromeSnapshot) {
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        0.0,
        width,
        TAB_STRIP_HEIGHT,
        Color::new(222, 225, 230, 255),
    );

    let tab_x = 12.0;
    let tab_y = 6.0;
    let tab_w = (width * 0.34)
        .clamp(170.0, 260.0)
        .min((width - 86.0).max(120.0));
    let tab_h = 34.0;
    fill_rounded_rect(
        pixmap,
        tab_x,
        tab_y,
        tab_w,
        tab_h,
        12.0,
        Color::new(255, 255, 255, 255),
    );

    fill_rounded_rect(
        pixmap,
        tab_x + 15.0,
        tab_y + 10.0,
        14.0,
        14.0,
        7.0,
        Color::new(66, 133, 244, 255),
    );
    fill_rounded_rect(
        pixmap,
        tab_x + 18.5,
        tab_y + 13.5,
        7.0,
        7.0,
        3.5,
        Color::new(255, 255, 255, 255),
    );

    let tab_title = fit_text(&chrome.title, tab_w - 72.0, 13.0);
    crate::render::draw_ui_text(
        pixmap,
        &tab_title,
        tab_x + 38.0,
        tab_y + 9.0,
        tab_w - 70.0,
        13.0,
        Color::new(32, 33, 36, 255),
        TextAlign::Left,
    );
    crate::render::draw_ui_text(
        pixmap,
        "x",
        tab_x + tab_w - 25.0,
        tab_y + 9.0,
        16.0,
        13.0,
        Color::new(95, 99, 104, 255),
        TextAlign::Center,
    );

    let plus_x = (tab_x + tab_w + 14.0).min(width - 44.0);
    fill_rounded_rect(
        pixmap,
        plus_x,
        9.0,
        28.0,
        28.0,
        14.0,
        Color::new(232, 234, 237, 255),
    );
    crate::render::draw_ui_text(
        pixmap,
        "+",
        plus_x,
        10.0,
        28.0,
        18.0,
        Color::new(60, 64, 67, 255),
        TextAlign::Center,
    );

    if chrome.tab_count > 1 {
        let badge = format!("{}", chrome.tab_count);
        crate::render::draw_ui_text(
            pixmap,
            &badge,
            plus_x + 34.0,
            14.0,
            28.0,
            11.0,
            Color::new(95, 99, 104, 255),
            TextAlign::Left,
        );
    }
}

fn draw_toolbar(pixmap: &mut Pixmap, width: f32, chrome: &ChromeSnapshot) {
    let y = TAB_STRIP_HEIGHT;
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        y,
        width,
        TOOLBAR_HEIGHT,
        Color::new(255, 255, 255, 255),
    );

    let button_y = y + 10.0;
    draw_nav_button(pixmap, 12.0, button_y, "<", chrome.can_back);
    draw_nav_button(pixmap, 48.0, button_y, ">", chrome.can_forward);
    draw_nav_button(
        pixmap,
        84.0,
        button_y,
        if chrome.loading { "x" } else { "o" },
        true,
    );

    let right_tools = 104.0;
    let address_x = 128.0;
    let address_w = (width - address_x - right_tools).max(140.0);
    fill_rounded_rect(
        pixmap,
        address_x,
        y + 8.0,
        address_w,
        36.0,
        18.0,
        Color::new(241, 243, 244, 255),
    );
    fill_rounded_rect(
        pixmap,
        address_x + 12.0,
        y + 19.0,
        14.0,
        14.0,
        7.0,
        Color::new(128, 134, 139, 255),
    );
    crate::render::draw_ui_text(
        pixmap,
        "i",
        address_x + 12.0,
        y + 18.0,
        14.0,
        12.0,
        Color::new(255, 255, 255, 255),
        TextAlign::Center,
    );

    let address_text = fit_text(&chrome.url, address_w - 78.0, 14.0);
    crate::render::draw_ui_text(
        pixmap,
        &address_text,
        address_x + 36.0,
        y + 18.0,
        address_w - 76.0,
        14.0,
        Color::new(60, 64, 67, 255),
        TextAlign::Left,
    );
    crate::render::draw_ui_text(
        pixmap,
        "*",
        address_x + address_w - 34.0,
        y + 17.0,
        24.0,
        16.0,
        Color::new(95, 99, 104, 255),
        TextAlign::Center,
    );

    let profile_x = width - 78.0;
    fill_rounded_rect(
        pixmap,
        profile_x,
        y + 12.0,
        28.0,
        28.0,
        14.0,
        Color::new(232, 240, 254, 255),
    );
    crate::render::draw_ui_text(
        pixmap,
        "O",
        profile_x,
        y + 16.0,
        28.0,
        13.0,
        Color::new(26, 115, 232, 255),
        TextAlign::Center,
    );
    draw_nav_button(pixmap, width - 42.0, button_y, "...", true);

    let status = if chrome.loading {
        "Loading..."
    } else {
        chrome.status.as_str()
    };
    if !status.trim().is_empty() {
        let label = fit_text(status, width - 28.0, 10.0);
        crate::render::draw_ui_text(
            pixmap,
            &label,
            14.0,
            CHROME_HEIGHT - 15.0,
            width - 28.0,
            10.0,
            Color::new(128, 134, 139, 255),
            TextAlign::Left,
        );
    }
}

fn draw_nav_button(pixmap: &mut Pixmap, x: f32, y: f32, label: &str, enabled: bool) {
    let bg = if enabled {
        Color::new(255, 255, 255, 255)
    } else {
        Color::new(255, 255, 255, 255)
    };
    let fg = if enabled {
        Color::new(60, 64, 67, 255)
    } else {
        Color::new(188, 192, 196, 255)
    };

    fill_rounded_rect(pixmap, x, y, 30.0, 30.0, 15.0, bg);
    crate::render::draw_ui_text(
        pixmap,
        label,
        x,
        y + 5.0,
        30.0,
        if label == "..." { 13.0 } else { 17.0 },
        fg,
        TextAlign::Center,
    );
}

fn fill_rounded_rect(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: Color,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let radius = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    if radius <= 0.0 {
        crate::render::fill_solid_rect(pixmap, x, y, width, height, color);
        return;
    }

    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );

    let mut path = PathBuilder::new();
    path.move_to(x + radius, y);
    path.line_to(x + width - radius, y);
    path.quad_to(x + width, y, x + width, y + radius);
    path.line_to(x + width, y + height - radius);
    path.quad_to(x + width, y + height, x + width - radius, y + height);
    path.line_to(x + radius, y + height);
    path.quad_to(x, y + height, x, y + height - radius);
    path.line_to(x, y + radius);
    path.quad_to(x, y, x + radius, y);
    path.close();

    if let Some(path) = path.finish() {
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
}

fn fit_text(text: &str, max_width: f32, font_size: f32) -> String {
    let max_chars = (max_width / (font_size * 0.55).max(1.0)).floor().max(4.0) as usize;
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(3).max(1);
    let mut fitted: String = text.chars().take(keep).collect();
    fitted.push_str("...");
    fitted
}
