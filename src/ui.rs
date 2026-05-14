/// Browser UI - winit + softbuffer ile pencere yönetimi
use crate::browser::Browser;
use crate::types::{Color, TextAlign};
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

const TAB_STRIP_HEIGHT: f32 = 42.0;
const TOOLBAR_HEIGHT: f32 = 50.0;
const CHROME_HEIGHT: f32 = TAB_STRIP_HEIGHT + TOOLBAR_HEIGHT + 1.0;
const SCROLLBAR_WIDTH: f32 = 8.0;

// Chrome hit region IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ChromeHit {
    Back,
    Forward,
    Refresh,
    UrlBar,
    NewTab,
    CloseTab,
    Profile,
    Menu,
}

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
            cursor_pos: PhysicalPosition::new(0.0, 0.0),
            mouse_down: false,
            modifiers: winit::keyboard::ModifiersState::empty(),
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
    cursor_pos: PhysicalPosition<f64>,
    mouse_down: bool,
    modifiers: winit::keyboard::ModifiersState,
}

#[derive(Clone)]
struct ChromeSnapshot {
    title: String,
    url: String,
    url_bar_text: String,
    url_bar_focused: bool,
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

        let context = softbuffer::Context::new(window.clone()).ok();
        self.context = context;

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
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = position;
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_delta: f32 = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                };
                let mut browser = self.ui.lock().unwrap();
                browser.scroll_by(-scroll_delta);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button != MouseButton::Left {
                    return;
                }
                let pressed = state == ElementState::Pressed;

                if pressed {
                    self.mouse_down = true;

                    // Check chrome hits
                    let width = self
                        .window
                        .as_ref()
                        .map(|w| w.inner_size().width)
                        .unwrap_or(1024) as f32;
                    let hit = hit_test(self.cursor_pos.x as f32, self.cursor_pos.y as f32, width);

                    match hit {
                        Some(ChromeHit::Back) => {
                            let mut b = self.ui.lock().unwrap();
                            b.go_back();
                        }
                        Some(ChromeHit::Forward) => {
                            let mut b = self.ui.lock().unwrap();
                            b.go_forward();
                        }
                        Some(ChromeHit::Refresh) => {
                            let mut b = self.ui.lock().unwrap();
                            b.refresh();
                        }
                        Some(ChromeHit::UrlBar) => {
                            let mut b = self.ui.lock().unwrap();
                            b.focus_url_bar(true);
                        }
                        Some(ChromeHit::NewTab) => {
                            let mut b = self.ui.lock().unwrap();
                            b.open_new_tab(None);
                        }
                        Some(ChromeHit::CloseTab) => {
                            let mut b = self.ui.lock().unwrap();
                            let idx = b.active_tab;
                            if b.tabs.len() > 1 {
                                b.close_tab(idx);
                            }
                        }
                        _ => {
                            let page_click = (self.cursor_pos.y as f32) >= CHROME_HEIGHT;
                            let target = {
                                let mut b = self.ui.lock().unwrap();
                                if b.url_bar_focused {
                                    b.blur_url_bar();
                                }
                                if page_click {
                                    let page_x = self.cursor_pos.x as f32;
                                    let page_y =
                                        self.cursor_pos.y as f32 - CHROME_HEIGHT + b.scroll_offset;
                                    b.link_at(page_x, page_y)
                                } else {
                                    None
                                }
                            };
                            if let Some(url) = target {
                                let mut b = self.ui.lock().unwrap();
                                b.load_url_sync(&url);
                            }
                        }
                    }
                } else {
                    self.mouse_down = false;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    let mut browser = self.ui.lock().unwrap();
                    let is_cmd = self
                        .modifiers
                        .contains(winit::keyboard::ModifiersState::SUPER);
                    let is_ctrl = self
                        .modifiers
                        .contains(winit::keyboard::ModifiersState::CONTROL);
                    let is_alt = self
                        .modifiers
                        .contains(winit::keyboard::ModifiersState::ALT);
                    let primary = is_cmd || is_ctrl;

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            if browser.url_bar_focused {
                                browser.blur_url_bar();
                            } else {
                                event_loop.exit();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                        | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                            if browser.url_bar_focused {
                                browser.url_bar_focused = false;
                                let url = browser.url_bar_text.clone();
                                drop(browser);
                                // Navigate
                                let mut b = self.ui.lock().unwrap();
                                b.url_bar_text = url;
                                b.navigate_from_url_bar();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Backspace) => {
                            if browser.url_bar_focused {
                                browser.backspace_url_bar();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Space) => {
                            if browser.url_bar_focused && !primary {
                                browser.push_url_bar_text(" ");
                            }
                        }
                        PhysicalKey::Code(KeyCode::Tab) => {
                            // Tab switching
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            b.switch_to_next_tab();
                        }
                        PhysicalKey::Code(KeyCode::F5) => {
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            b.refresh();
                        }
                        PhysicalKey::Code(KeyCode::KeyR) if primary => {
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            b.refresh();
                        }
                        PhysicalKey::Code(KeyCode::KeyT) if primary => {
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            b.open_new_tab(None);
                        }
                        PhysicalKey::Code(KeyCode::KeyW) if primary => {
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            let idx = b.active_tab;
                            if b.tabs.len() > 1 {
                                b.close_tab(idx);
                            }
                        }
                        PhysicalKey::Code(KeyCode::PageUp) => {
                            let vh = browser.viewport_height;
                            browser.scroll_by(-vh * 0.8);
                        }
                        PhysicalKey::Code(KeyCode::PageDown) => {
                            let vh = browser.viewport_height;
                            browser.scroll_by(vh * 0.8);
                        }
                        PhysicalKey::Code(KeyCode::Home) => {
                            browser.scroll_offset = 0.0;
                        }
                        PhysicalKey::Code(KeyCode::End) => {
                            let max = browser.max_scroll_offset();
                            browser.scroll_offset = max;
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp) => {
                            browser.scroll_by(-40.0);
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown) => {
                            browser.scroll_by(40.0);
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft) if is_alt || is_cmd => {
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            b.go_back();
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) if is_alt || is_cmd => {
                            drop(browser);
                            let mut b = self.ui.lock().unwrap();
                            b.go_forward();
                        }
                        PhysicalKey::Code(KeyCode::KeyL) if primary => {
                            // Cmd+L / Ctrl+L → focus URL bar
                            browser.focus_url_bar(true);
                        }
                        _ => {
                            // Text input for URL bar
                            if browser.url_bar_focused && !primary {
                                if let Some(ref text) = event.text {
                                    for ch in text.chars() {
                                        if ch >= ' ' && ch != '\x7f' {
                                            browser.push_url_bar_text(&ch.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
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

        // Browser'dan render et (tek lock)
        let (page_pixmap, chrome, scroll_offset, max_scroll) = {
            let mut browser = self.ui.lock().unwrap();
            let chrome_y = CHROME_HEIGHT as u32;
            let page_height = height.saturating_sub(chrome_y).max(1);
            browser.relayout(width as f32, page_height as f32);
            let chrome = ChromeSnapshot::from_browser(&browser);
            let scroll_offset = browser.scroll_offset;
            let max_scroll = browser.max_scroll_offset();

            let page_pixmap = if let Some(layout_root) = &browser.layout_result {
                let mut commands = Vec::new();
                crate::render::build_display_list(layout_root, &mut commands);
                let render_height = (page_height as f32 + max_scroll).ceil() as u32;
                Some(crate::render::render_to_pixmap(
                    &commands,
                    width,
                    render_height.max(1),
                ))
            } else {
                None
            };

            (page_pixmap, chrome, scroll_offset, max_scroll)
        };

        // Page area background
        let page_area_top = CHROME_HEIGHT;
        let page_area_h = (height as f32 - page_area_top).max(0.0);
        // Fill page area with a light bg so gaps don't show white
        crate::render::fill_solid_rect(
            pixmap,
            0.0,
            page_area_top,
            width as f32,
            page_area_h,
            Color::new(255, 255, 255, 255),
        );

        if let Some(page_pixmap) = page_pixmap {
            blit_scrolled(
                pixmap,
                &page_pixmap,
                0,
                CHROME_HEIGHT as u32,
                scroll_offset,
                max_scroll,
            );
            draw_scrollbar(pixmap, width, height, scroll_offset, max_scroll);
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
            url_bar_text: if browser.url_bar_focused {
                browser.url_bar_text.clone()
            } else {
                String::new()
            },
            url_bar_focused: browser.url_bar_focused,
            status: browser.status_message.clone(),
            can_back,
            can_forward,
            loading: browser.loading,
            tab_count: browser.tabs.len().max(1),
        }
    }
}

/// Hit test for chrome UI elements
fn hit_test(mx: f32, my: f32, width: f32) -> Option<ChromeHit> {
    let toolbar_y = TAB_STRIP_HEIGHT;

    // Back button: x=12..42, y=toolbar_y+10..toolbar_y+40
    if mx >= 12.0 && mx < 42.0 && my >= toolbar_y + 10.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Back);
    }
    // Forward: x=48..78
    if mx >= 48.0 && mx < 78.0 && my >= toolbar_y + 10.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Forward);
    }
    // Refresh: x=84..114
    if mx >= 84.0 && mx < 114.0 && my >= toolbar_y + 10.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Refresh);
    }

    // URL bar: x=128..width-104, y=toolbar_y+8..toolbar_y+44
    let url_x = 128.0;
    let url_right = (width - 104.0).max(url_x + 100.0);
    if mx >= url_x && mx < url_right && my >= toolbar_y + 8.0 && my < toolbar_y + 44.0 {
        return Some(ChromeHit::UrlBar);
    }

    // Profile: x=width-78..width-50
    if mx >= width - 78.0 && mx < width - 50.0 && my >= toolbar_y + 12.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Profile);
    }
    // Menu: x=width-42..width-12
    if mx >= width - 42.0 && mx < width - 12.0 && my >= toolbar_y + 10.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Menu);
    }

    // New tab (+) button in tab strip: x after tab, width=28
    let tab_x = 12.0;
    let tab_w = (width * 0.34)
        .clamp(170.0, 260.0)
        .min((width - 86.0).max(120.0));
    let plus_x = (tab_x + tab_w + 14.0).min(width - 44.0);
    if mx >= plus_x && mx < plus_x + 28.0 && my >= 9.0 && my < 37.0 {
        return Some(ChromeHit::NewTab);
    }

    // Close tab (x) button on the tab
    let close_x = tab_x + tab_w - 25.0;
    if mx >= close_x && mx < close_x + 16.0 && my >= 12.0 && my < 34.0 {
        return Some(ChromeHit::CloseTab);
    }

    None
}

fn blit_scrolled(
    dest: &mut Pixmap,
    src: &Pixmap,
    x: u32,
    y: u32,
    scroll_offset: f32,
    _max_scroll: f32,
) {
    if x >= dest.width() || y >= dest.height() {
        return;
    }

    let src_start_row = scroll_offset as u32;
    if src_start_row >= src.height() {
        return; // Scrolled past end of content
    }

    let copy_height = src
        .height()
        .saturating_sub(src_start_row)
        .min(dest.height().saturating_sub(y)) as usize;
    let copy_width = src.width().min(dest.width().saturating_sub(x)) as usize;

    if copy_height == 0 || copy_width == 0 {
        return;
    }

    let dest_stride = dest.width() as usize * 4;
    let src_stride = src.width() as usize * 4;
    let dest_x = x as usize * 4;
    let dest_y = y as usize;
    let bytes = copy_width * 4;
    let src_data = src.data();
    let dest_data = dest.data_mut();

    for row in 0..copy_height {
        let src_row = src_start_row as usize + row;
        if src_row >= src.height() as usize {
            break;
        }
        let src_start = src_row * src_stride;
        let dest_start = (dest_y + row) * dest_stride + dest_x;
        let src_end = (src_start + bytes).min(src_data.len());
        let dest_end = (dest_start + bytes).min(dest_data.len());
        let copy_len = (src_end - src_start).min(dest_end - dest_start);
        if copy_len > 0 {
            dest_data[dest_start..dest_start + copy_len]
                .copy_from_slice(&src_data[src_start..src_start + copy_len]);
        }
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
    // Tab strip background - darker top bar like Chrome
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

    // Active tab - white with rounded top corners
    fill_rounded_rect_top(
        pixmap,
        tab_x,
        tab_y,
        tab_w,
        tab_h,
        10.0,
        Color::new(255, 255, 255, 255),
    );

    // Tab favicon (colored circle)
    fill_rounded_rect(
        pixmap,
        tab_x + 12.0,
        tab_y + 10.0,
        14.0,
        14.0,
        7.0,
        Color::new(66, 133, 244, 255),
    );
    // Inner dot
    fill_rounded_rect(
        pixmap,
        tab_x + 17.0,
        tab_y + 14.0,
        4.0,
        4.0,
        2.0,
        Color::new(255, 255, 255, 255),
    );

    // Tab title
    let tab_title = fit_text(&chrome.title, tab_w - 72.0, 13.0);
    crate::render::draw_ui_text(
        pixmap,
        &tab_title,
        tab_x + 34.0,
        tab_y + 10.0,
        tab_w - 72.0,
        13.0,
        Color::new(32, 33, 36, 255),
        TextAlign::Left,
    );

    // Close tab button
    let close_x = tab_x + tab_w - 24.0;
    let is_hover_close = false; // TODO: track hover state
    let close_bg = if is_hover_close {
        Color::new(220, 220, 220, 255)
    } else {
        Color::new(255, 255, 255, 255)
    };
    fill_rounded_rect(pixmap, close_x, tab_y + 8.0, 18.0, 18.0, 9.0, close_bg);
    crate::render::draw_ui_text(
        pixmap,
        "\u{2715}", // ✕
        close_x,
        tab_y + 9.0,
        18.0,
        11.0,
        Color::new(95, 99, 104, 255),
        TextAlign::Center,
    );

    // New tab button (+)
    let plus_x = (tab_x + tab_w + 14.0).min(width - 44.0);
    let plus_hover = false;
    let plus_bg = if plus_hover {
        Color::new(210, 213, 218, 255)
    } else {
        Color::new(232, 234, 237, 255)
    };
    fill_rounded_rect(pixmap, plus_x, 9.0, 28.0, 28.0, 14.0, plus_bg);
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

    // Tab count badge
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

    let btn_y = y + 10.0;

    // Back button ←
    draw_nav_button(pixmap, 12.0, btn_y, "\u{2190}", chrome.can_back);
    // Forward button →
    draw_nav_button(pixmap, 48.0, btn_y, "\u{2192}", chrome.can_forward);
    // Refresh button ↻
    draw_nav_button(
        pixmap,
        84.0,
        btn_y,
        if chrome.loading {
            "\u{2715}"
        } else {
            "\u{21BB}"
        },
        true,
    );

    // URL bar
    let address_x = 128.0;
    let address_w = (width - address_x - 104.0).max(140.0);

    // URL bar background
    let url_bg = Color::new(241, 243, 244, 255);
    fill_rounded_rect(pixmap, address_x, y + 8.0, address_w, 36.0, 20.0, url_bg);

    // Lock/security icon
    let is_https = chrome.url.starts_with("https://");
    let lock_color = if is_https {
        Color::new(52, 168, 83, 255)
    } else {
        Color::new(128, 134, 139, 255)
    };
    let lock_icon = if is_https { "\u{1F512}" } else { "\u{2139}" };
    crate::render::draw_ui_text(
        pixmap,
        lock_icon,
        address_x + 10.0,
        y + 16.0,
        16.0,
        14.0,
        lock_color,
        TextAlign::Center,
    );

    // URL text - show typed text when focused, URL when not
    let url_display = if chrome.url_bar_focused && !chrome.url_bar_text.is_empty() {
        chrome.url_bar_text.as_str()
    } else if chrome.url.is_empty() {
        "Aramak veya URL girin..."
    } else {
        chrome.url.as_str()
    };
    let url_color = if chrome.url_bar_focused {
        Color::new(32, 33, 36, 255)
    } else {
        Color::new(60, 64, 67, 255)
    };
    let url_text = fit_text(url_display, address_w - 80.0, 13.0);
    crate::render::draw_ui_text(
        pixmap,
        &url_text,
        address_x + 32.0,
        y + 18.0,
        address_w - 78.0,
        13.0,
        url_color,
        TextAlign::Left,
    );

    // Cursor indicator when focused
    if chrome.url_bar_focused {
        crate::render::fill_solid_rect(
            pixmap,
            address_x + 32.0 + (url_text.chars().count() as f32 * 7.0).min(address_w - 80.0),
            y + 18.0,
            1.5,
            14.0,
            Color::new(32, 33, 36, 255),
        );
    }

    // Star/bookmark button
    crate::render::draw_ui_text(
        pixmap,
        "\u{2606}",
        address_x + address_w - 34.0,
        y + 17.0,
        24.0,
        15.0,
        Color::new(95, 99, 104, 255),
        TextAlign::Center,
    );

    // Profile button
    let prof_x = width - 78.0;
    fill_rounded_rect(
        pixmap,
        prof_x,
        y + 12.0,
        28.0,
        28.0,
        14.0,
        Color::new(232, 240, 254, 255),
    );
    crate::render::draw_ui_text(
        pixmap,
        "\u{1F464}",
        prof_x,
        y + 15.0,
        28.0,
        13.0,
        Color::new(26, 115, 232, 255),
        TextAlign::Center,
    );

    // Menu button ⋮
    draw_nav_button(pixmap, width - 42.0, btn_y, "\u{22EE}", true);

    // Status bar text
    let status = if chrome.loading {
        "Yükleniyor..."
    } else {
        chrome.status.as_str()
    };
    if !status.trim().is_empty() {
        let label = fit_text(status, width - 28.0, 10.0);
        crate::render::draw_ui_text(
            pixmap,
            &label,
            14.0,
            CHROME_HEIGHT - 14.0,
            width - 28.0,
            10.0,
            Color::new(128, 134, 139, 255),
            TextAlign::Left,
        );
    }
}

fn draw_scrollbar(
    pixmap: &mut Pixmap,
    width: u32,
    height: u32,
    scroll_offset: f32,
    max_scroll: f32,
) {
    let page_height = height.saturating_sub(CHROME_HEIGHT as u32) as f32;
    if page_height <= 0.0 || max_scroll <= 0.0 {
        return;
    }

    let sb_x = width as f32 - SCROLLBAR_WIDTH - 2.0;
    let sb_y = CHROME_HEIGHT;
    let sb_h = page_height;

    // Track background
    crate::render::fill_solid_rect(
        pixmap,
        sb_x,
        sb_y,
        SCROLLBAR_WIDTH,
        sb_h,
        Color::new(241, 243, 244, 255),
    );

    // Thumb
    let total_height = page_height + max_scroll;
    let thumb_h = (page_height / total_height * sb_h).max(20.0).min(sb_h);
    let thumb_y = sb_y + (scroll_offset / max_scroll) * (sb_h - thumb_h);

    fill_rounded_rect(
        pixmap,
        sb_x + 1.0,
        thumb_y,
        SCROLLBAR_WIDTH - 2.0,
        thumb_h,
        3.0,
        Color::new(188, 192, 196, 255),
    );
}

fn draw_nav_button(pixmap: &mut Pixmap, x: f32, y: f32, label: &str, enabled: bool) {
    let bg = Color::new(255, 255, 255, 255);
    let fg = if enabled {
        Color::new(60, 64, 67, 255)
    } else {
        Color::new(200, 204, 208, 255)
    };

    fill_rounded_rect(pixmap, x, y, 30.0, 30.0, 15.0, bg);
    crate::render::draw_ui_text(pixmap, label, x, y + 4.0, 30.0, 17.0, fg, TextAlign::Center);
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

fn fill_rounded_rect_top(
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

    let radius = radius.min(width / 2.0).min(height).max(0.0);
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
    path.line_to(x + width, y + height);
    path.line_to(x, y + height);
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
