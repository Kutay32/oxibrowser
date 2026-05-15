/// Browser UI - winit + softbuffer ile pencere yönetimi
use crate::browser::{Browser, NavigationTarget, DEFAULT_ZOOM_PERCENT};
use crate::icons;
use crate::types::{Color, TextAlign};
use serde::Deserialize;
use softbuffer::Surface;
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Transform};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};
use wry::dpi::{LogicalPosition as WryLogicalPosition, LogicalSize as WryLogicalSize};
use wry::{NewWindowResponse, PageLoadEvent, Rect as WryRect, WebView, WebViewBuilder};

const TAB_STRIP_HEIGHT: f32 = 42.0;
const TOOLBAR_HEIGHT: f32 = 50.0;
const CHROME_HEIGHT: f32 = TAB_STRIP_HEIGHT + TOOLBAR_HEIGHT + 1.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLL_EASE_SECONDS: f32 = 0.10;

// Chrome hit region IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ChromeHit {
    Back,
    Forward,
    Refresh,
    UrlBar,
    Bookmark,
    NewTab,
    CloseTab,
    Profile,
    Menu,
}

enum ZoomChange {
    In,
    Out,
    Reset,
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
            webviews: HashMap::new(),
            last_autofill_keys: HashMap::new(),
            scroll_target: 0.0,
            scroll_animating: false,
            last_scroll_frame: None,
            needs_redraw: true,
            hovered_tab_index: None,
            hovered_toolbar_btn: None,
            hovered_link_url: None,
            window_width: 1024.0,
            scale_factor: 1.0,
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
    webviews: HashMap<u32, WebView>,
    last_autofill_keys: HashMap<u32, String>,
    scroll_target: f32,
    scroll_animating: bool,
    last_scroll_frame: Option<Instant>,
    needs_redraw: bool,
    hovered_tab_index: Option<usize>,
    hovered_toolbar_btn: Option<ChromeHit>,
    hovered_link_url: Option<String>,
    window_width: f32,
    scale_factor: f64,
}

#[derive(Debug, Deserialize)]
struct BrowserIpcMessage {
    kind: String,
    url: Option<String>,
    username: Option<String>,
    password: Option<String>,
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
    zoom_percent: u16,
    bookmarked: bool,
    tabs: Vec<TabInfo>,
    active_tab_index: usize,
    incognito: bool,
    user_display_name: String,
    user_avatar_color: u32,
}

#[derive(Clone)]
struct TabInfo {
    title: String,
    loading: bool,
    incognito: bool,
}

impl ApplicationHandler for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("OxiBrowser")
            .with_inner_size(LogicalSize::new(1200.0, 800.0))
            .with_position(winit::dpi::LogicalPosition::new(100.0, 50.0))
            .with_visible(true)
            .with_resizable(true)
            .with_decorations(true);

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
        self.scale_factor = window.scale_factor();
        self.pixmap = Pixmap::new(size.width.max(1), size.height.max(1));
        if let Some(ref mut p) = self.pixmap {
            p.fill(tiny_skia::Color::WHITE);
        }
        self.needs_redraw = true;
        self.sync_webviews();
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Ok(mut browser) = self.ui.lock() {
                    browser.persist_session();
                }
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
                    self.update_webview_bounds();
                    self.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let prev_hovered_tab = self.hovered_tab_index;
                let prev_hovered_btn = self.hovered_toolbar_btn;
                self.cursor_pos = position;
                let logical_width = self
                    .window
                    .as_ref()
                    .map(|w| w.inner_size().width)
                    .unwrap_or(1024) as f32 / self.scale_factor as f32;
                self.window_width = logical_width;

                // Convert physical cursor position to logical coordinates
                let logical_mx = self.cursor_pos.x as f32 / self.scale_factor as f32;
                let logical_my = self.cursor_pos.y as f32 / self.scale_factor as f32;

                // Update tab hover
                let (tab_count, active_idx) = {
                    let browser = self.ui.lock().unwrap();
                    (browser.tabs.len(), browser.active_tab)
                };
                self.hovered_tab_index = hit_test_tab(
                    logical_mx,
                    logical_my,
                    tab_count,
                    active_idx,
                    logical_width,
                );

                // Update toolbar button hover
                let hit = hit_test(
                    logical_mx,
                    logical_my,
                    logical_width,
                );
                self.hovered_toolbar_btn = hit;

                if self.hovered_tab_index != prev_hovered_tab
                    || self.hovered_toolbar_btn != prev_hovered_btn
                {
                    self.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let logical_my = self.cursor_pos.y as f32 / self.scale_factor as f32;
                if logical_my < CHROME_HEIGHT {
                    return;
                }
                let scroll_delta: f32 = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 40.0,
                    MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / self.scale_factor as f32,
                };
                self.smooth_scroll_by(-scroll_delta);
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
                            self.go_back_active();
                        }
                        Some(ChromeHit::Forward) => {
                            self.go_forward_active();
                        }
                        Some(ChromeHit::Refresh) => {
                            self.reload_active();
                        }
                        Some(ChromeHit::UrlBar) => {
                            {
                                let mut b = self.ui.lock().unwrap();
                                b.focus_url_bar(true);
                            }
                            self.request_redraw();
                        }
                        Some(ChromeHit::Bookmark) => {
                            let mut b = self.ui.lock().unwrap();
                            b.toggle_bookmark_active();
                            drop(b);
                            self.request_redraw();
                        }
                        Some(ChromeHit::NewTab) => {
                            self.create_tab("about:welcome");
                        }
                        Some(ChromeHit::CloseTab) => {
                            let active_id = self.ui.lock().unwrap().active_tab_id();
                            if let Some(id) = active_id {
                                self.close_tab(id);
                            }
                        }
                        Some(ChromeHit::Profile) => {
                            self.navigate_active("about:downloads");
                        }
                        Some(ChromeHit::Menu) => {
                            self.navigate_active("about:settings");
                        }
                        _ => {
                            // Check if clicking on a tab for switching
                            if let Some(tab_idx) = self.hovered_tab_index {
                                let tab_id = {
                                    let browser = self.ui.lock().unwrap();
                                    browser.tabs.get(tab_idx).map(|t| t.id)
                                };
                                if let Some(id) = tab_id {
                                    self.switch_to_tab_by_id(id);
                                }
                            } else {
                                let logical_chrome_height = CHROME_HEIGHT;
                                let logical_my = self.cursor_pos.y as f32 / self.scale_factor as f32;
                                let page_click = logical_my >= logical_chrome_height;
                                {
                                    let mut b = self.ui.lock().unwrap();
                                    if b.url_bar_focused {
                                        b.blur_url_bar();
                                    }
                                }
                                if page_click {
                                    self.focus_active_webview();
                                }
                            }
                            self.request_redraw();
                        }
                    }
                } else {
                    self.mouse_down = false;
                    self.request_redraw();
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
                    let is_shift = self
                        .modifiers
                        .contains(winit::keyboard::ModifiersState::SHIFT);
                    let primary = is_cmd || is_ctrl;

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Escape) => {
                            if browser.url_bar_focused {
                                browser.blur_url_bar();
                                drop(browser);
                                self.request_redraw();
                            } else {
                                event_loop.exit();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Enter)
                        | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                            if browser.url_bar_focused {
                                browser.url_bar_focused = false;
                                let text = browser.url_bar_text.clone();
                                let find = browser.find_bar_active;
                                browser.find_bar_active = false;
                                if find {
                                    browser.set_find_query(text.clone());
                                }
                                drop(browser);
                                if find {
                                    self.find_in_active_page(&text);
                                } else {
                                    self.navigate_active(&text);
                                }
                            }
                        }
                        PhysicalKey::Code(KeyCode::Backspace) => {
                            if browser.url_bar_focused {
                                browser.backspace_url_bar();
                                drop(browser);
                                self.request_redraw();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Space) => {
                            if browser.url_bar_focused && !primary {
                                browser.push_url_bar_text(" ");
                                drop(browser);
                                self.request_redraw();
                            }
                        }
                        PhysicalKey::Code(KeyCode::Tab) => {
                            // Tab switching
                            drop(browser);
                            self.switch_to_next_tab();
                        }
                        PhysicalKey::Code(KeyCode::F5) => {
                            drop(browser);
                            self.reload_active();
                        }
                        PhysicalKey::Code(KeyCode::KeyR) if primary => {
                            drop(browser);
                            self.reload_active();
                        }
                        PhysicalKey::Code(KeyCode::KeyN) if primary && is_shift => {
                            drop(browser);
                            self.create_incognito_tab();
                        }
                        PhysicalKey::Code(KeyCode::KeyT) if primary => {
                            drop(browser);
                            self.create_tab("about:welcome");
                        }
                        PhysicalKey::Code(KeyCode::KeyW) if primary => {
                            drop(browser);
                            let active_id = self.ui.lock().unwrap().active_tab_id();
                            if let Some(id) = active_id {
                                self.close_tab(id);
                            }
                        }
                        PhysicalKey::Code(KeyCode::Equal) if primary => {
                            drop(browser);
                            self.zoom_active(ZoomChange::In);
                        }
                        PhysicalKey::Code(KeyCode::Minus) if primary => {
                            drop(browser);
                            self.zoom_active(ZoomChange::Out);
                        }
                        PhysicalKey::Code(KeyCode::Digit0) if primary => {
                            drop(browser);
                            self.zoom_active(ZoomChange::Reset);
                        }
                        PhysicalKey::Code(KeyCode::KeyD) if primary => {
                            browser.toggle_bookmark_active();
                            drop(browser);
                            self.request_redraw();
                        }
                        PhysicalKey::Code(KeyCode::KeyY) if primary => {
                            drop(browser);
                            self.navigate_active("about:history");
                        }
                        PhysicalKey::Code(KeyCode::KeyJ) if primary => {
                            drop(browser);
                            self.navigate_active("about:downloads");
                        }
                        PhysicalKey::Code(KeyCode::Comma) if primary => {
                            drop(browser);
                            self.navigate_active("about:settings");
                        }
                        PhysicalKey::Code(KeyCode::KeyP) if primary && is_shift => {
                            drop(browser);
                            self.navigate_active("about:passwords");
                        }
                        PhysicalKey::Code(KeyCode::KeyS) if primary && is_shift => {
                            drop(browser);
                            self.navigate_active("about:sync");
                        }
                        PhysicalKey::Code(KeyCode::KeyE) if primary && is_shift => {
                            drop(browser);
                            self.navigate_active("about:extensions");
                        }
                        PhysicalKey::Code(KeyCode::KeyB) if primary => {
                            drop(browser);
                            self.navigate_active("about:bookmarks");
                        }
                        PhysicalKey::Code(KeyCode::KeyF) if primary => {
                            let previous = browser.last_find_query.clone();
                            browser.url_bar_focused = true;
                            browser.url_bar_select_all = true;
                            browser.find_bar_active = true;
                            browser.url_bar_text = previous;
                            browser.status_message =
                                "Find: sorguyu yazıp Enter'a basınca sayfada aranır".to_string();
                            drop(browser);
                            self.request_redraw();
                        }
                        PhysicalKey::Code(KeyCode::KeyI) if primary && is_alt => {
                            drop(browser);
                            self.open_devtools();
                        }
                        PhysicalKey::Code(KeyCode::Delete) if primary && is_alt => {
                            browser.clear_browsing_data();
                            drop(browser);
                            self.clear_webview_data();
                        }
                        PhysicalKey::Code(KeyCode::PageUp) => {
                            let vh = browser.viewport_height;
                            drop(browser);
                            self.smooth_scroll_by(-vh * 0.8);
                        }
                        PhysicalKey::Code(KeyCode::PageDown) => {
                            let vh = browser.viewport_height;
                            drop(browser);
                            self.smooth_scroll_by(vh * 0.8);
                        }
                        PhysicalKey::Code(KeyCode::Home) => {
                            drop(browser);
                            self.smooth_scroll_to(0.0);
                        }
                        PhysicalKey::Code(KeyCode::End) => {
                            let max = browser.max_scroll_offset();
                            drop(browser);
                            self.smooth_scroll_to(max);
                        }
                        PhysicalKey::Code(KeyCode::ArrowUp) => {
                            drop(browser);
                            self.smooth_scroll_by(-40.0);
                        }
                        PhysicalKey::Code(KeyCode::ArrowDown) => {
                            drop(browser);
                            self.smooth_scroll_by(40.0);
                        }
                        PhysicalKey::Code(KeyCode::ArrowLeft) if is_alt || is_cmd => {
                            drop(browser);
                            self.go_back_active();
                        }
                        PhysicalKey::Code(KeyCode::ArrowRight) if is_alt || is_cmd => {
                            drop(browser);
                            self.go_forward_active();
                        }
                        PhysicalKey::Code(KeyCode::KeyL) if primary => {
                            // Cmd+L / Ctrl+L → focus URL bar
                            browser.focus_url_bar(true);
                            drop(browser);
                            self.request_redraw();
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
                                    drop(browser);
                                    self.request_redraw();
                                }
                            }
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                self.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.drain_pending_new_tabs();
        self.apply_password_autofill();
        let browser_loading = self
            .ui
            .lock()
            .map(|browser| browser.loading)
            .unwrap_or(false);
        if self.needs_redraw || self.scroll_animating || browser_loading {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

impl BrowserApp {
    fn webview_bounds_for(physical_width: u32, physical_height: u32, scale: f64) -> WryRect {
        let logical_chrome = CHROME_HEIGHT as f64;
        WryRect {
            position: WryLogicalPosition::new(0.0, logical_chrome).into(),
            size: WryLogicalSize::new(
                (physical_width as f64 / scale).max(1.0),
                ((physical_height as f64 / scale) - logical_chrome).max(1.0),
            )
            .into(),
        }
    }

    fn current_webview_bounds(&self) -> WryRect {
        let size = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .unwrap_or_else(|| winit::dpi::PhysicalSize::new(1024, 768));
        Self::webview_bounds_for(size.width, size.height, self.scale_factor)
    }

    fn sync_webviews(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };

        let (tabs, active_id) = {
            let browser = self.ui.lock().unwrap();
            (browser.tabs.clone(), browser.active_tab_id())
        };

        self.webviews
            .retain(|id, _| tabs.iter().any(|tab| tab.id == *id));

        for tab in &tabs {
            if !self.webviews.contains_key(&tab.id) {
                match self.create_webview_for_tab(&window, tab.id) {
                    Ok(webview) => {
                        self.webviews.insert(tab.id, webview);
                    }
                    Err(err) => {
                        let mut browser = self.ui.lock().unwrap();
                        browser.status_message = format!("WebView oluşturulamadı: {}", err);
                    }
                }
            }
        }

        let bounds = self.current_webview_bounds();
        for tab in &tabs {
            if let Some(webview) = self.webviews.get(&tab.id) {
                let visible = active_id == Some(tab.id);
                let _ = webview.set_visible(visible);
                let _ = webview.set_bounds(bounds);
                let _ = webview.zoom(tab.zoom_percent as f64 / 100.0);
                if visible {
                    let _ = webview.focus();
                }
            }
        }
    }

    fn create_webview_for_tab(&self, window: &Arc<Window>, tab_id: u32) -> Result<WebView, String> {
        let target = {
            let browser = self.ui.lock().unwrap();
            browser
                .tab(tab_id)
                .map(|tab| browser.navigation_target_for_url(&tab.url))
                .unwrap_or_else(|| NavigationTarget::Url("https://example.com".to_string()))
        };

        let browser_for_nav = self.ui.clone();
        let browser_for_title = self.ui.clone();
        let browser_for_load = self.ui.clone();
        let browser_for_new_window = self.ui.clone();
        let browser_for_download_started = self.ui.clone();
        let browser_for_download_finished = self.ui.clone();
        let browser_for_ipc = self.ui.clone();

        let mut builder = WebViewBuilder::new()
            .with_bounds(self.current_webview_bounds())
            .with_background_color((255, 255, 255, 255))
            .with_devtools(true)
            .with_clipboard(true)
            .with_back_forward_navigation_gestures(true)
            .with_hotkeys_zoom(false)
            .with_initialization_script(password_capture_script())
            .with_ipc_handler(move |request| {
                if let Ok(message) = serde_json::from_str::<BrowserIpcMessage>(request.body()) {
                    if message.kind == "password-submit" {
                        if let (Some(url), Some(username), Some(password)) =
                            (message.url, message.username, message.password)
                        {
                            if let Ok(mut browser) = browser_for_ipc.lock() {
                                browser.save_password_for_url(&url, &username, &password);
                            }
                        }
                    }
                }
            })
            .with_navigation_handler(move |url| {
                if let Ok(mut browser) = browser_for_nav.lock() {
                    browser.mark_tab_loading(tab_id, &url);
                }
                true
            })
            .with_document_title_changed_handler(move |title| {
                if let Ok(mut browser) = browser_for_title.lock() {
                    browser.update_tab_title(tab_id, &title);
                }
            })
            .with_on_page_load_handler(move |event, url| {
                if let Ok(mut browser) = browser_for_load.lock() {
                    match event {
                        PageLoadEvent::Started => browser.mark_tab_loading(tab_id, &url),
                        PageLoadEvent::Finished => browser.finish_tab_load(tab_id, &url),
                    }
                }
            })
            .with_new_window_req_handler(move |url, _features| {
                if let Ok(mut browser) = browser_for_new_window.lock() {
                    browser.queue_new_tab(url);
                }
                NewWindowResponse::Deny
            })
            .with_download_started_handler(move |url, path| {
                let target = download_target_for_url(&url, path);
                *path = target.clone();
                if let Ok(mut browser) = browser_for_download_started.lock() {
                    browser.record_download_started(url, target);
                }
                true
            })
            .with_download_completed_handler(move |url, path, success| {
                if let Ok(mut browser) = browser_for_download_finished.lock() {
                    browser.record_download_finished(url, path, success);
                }
            });

        builder = match target {
            NavigationTarget::Url(url) => builder.with_url(url),
            NavigationTarget::Html { html, .. } => builder.with_html(html),
        };

        builder
            .build_as_child(window.as_ref())
            .map_err(|err| err.to_string())
    }

    fn update_webview_bounds(&mut self) {
        let bounds = self.current_webview_bounds();
        for webview in self.webviews.values() {
            let _ = webview.set_bounds(bounds);
        }
    }

    fn navigate_active(&mut self, input: &str) {
        let (tab_id, target) = {
            let mut browser = self.ui.lock().unwrap();
            let target = browser.plan_navigation(input);
            (browser.active_tab_id(), target)
        };

        self.sync_webviews();
        if let Some(tab_id) = tab_id {
            if let Some(webview) = self.webviews.get(&tab_id) {
                apply_navigation_target(webview, target);
                let _ = webview.focus();
            }
        }
        self.request_redraw();
    }

    fn create_tab(&mut self, url: &str) {
        {
            let mut browser = self.ui.lock().unwrap();
            browser.open_new_tab(Some(url));
            browser.persist_session();
        }
        self.sync_webviews();
        self.request_redraw();
    }

    fn create_incognito_tab(&mut self) {
        {
            let mut browser = self.ui.lock().unwrap();
            browser.new_incognito_tab("about:welcome");
            browser.persist_session();
        }
        self.sync_webviews();
        self.request_redraw();
    }

    fn close_tab(&mut self, tab_id: u32) {
        self.webviews.remove(&tab_id);
        {
            let mut browser = self.ui.lock().unwrap();
            browser.close_tab_by_id(tab_id);
        }
        self.sync_webviews();
        self.request_redraw();
    }

    fn switch_to_next_tab(&mut self) {
        let next_id = {
            let browser = self.ui.lock().unwrap();
            if browser.tabs.is_empty() {
                None
            } else {
                let next = (browser.active_tab + 1) % browser.tabs.len();
                browser.tabs.get(next).map(|tab| tab.id)
            }
        };
        if let Some(id) = next_id {
            let mut browser = self.ui.lock().unwrap();
            browser.switch_to_tab_id(id);
            drop(browser);
            self.sync_webviews();
            self.request_redraw();
        }
    }

    fn switch_to_tab_by_id(&mut self, tab_id: u32) {
        let mut browser = self.ui.lock().unwrap();
        if browser.switch_to_tab_id(tab_id) {
            drop(browser);
            self.sync_webviews();
            self.request_redraw();
        }
    }

    fn active_webview(&self) -> Option<&WebView> {
        let id = self.ui.lock().ok()?.active_tab_id()?;
        self.webviews.get(&id)
    }

    fn focus_active_webview(&self) {
        if let Some(webview) = self.active_webview() {
            let _ = webview.focus();
        }
    }

    fn go_back_active(&mut self) {
        if let Some(webview) = self.active_webview() {
            let _ = webview.evaluate_script("history.back()");
        }
        self.request_redraw();
    }

    fn go_forward_active(&mut self) {
        if let Some(webview) = self.active_webview() {
            let _ = webview.evaluate_script("history.forward()");
        }
        self.request_redraw();
    }

    fn reload_active(&mut self) {
        if let Some(webview) = self.active_webview() {
            let _ = webview.reload();
        }
        self.request_redraw();
    }

    fn zoom_active(&mut self, change: ZoomChange) {
        let zoom = {
            let mut browser = self.ui.lock().unwrap();
            match change {
                ZoomChange::In => browser.zoom_in(),
                ZoomChange::Out => browser.zoom_out(),
                ZoomChange::Reset => browser.reset_zoom(),
            }
        };
        if let Some(webview) = self.active_webview() {
            if webview.zoom(zoom as f64 / 100.0).is_err() {
                let _ = webview.evaluate_script(&format!(
                    "document.documentElement.style.zoom = '{}%'",
                    zoom
                ));
            }
        }
        self.request_redraw();
    }

    fn find_in_active_page(&mut self, query: &str) {
        if query.trim().is_empty() {
            return;
        }
        let escaped = js_string(query);
        if let Some(webview) = self.active_webview() {
            let _ = webview.evaluate_script(&format!(
                "window.find({}, false, false, true, false, false, false);",
                escaped
            ));
        }
        self.request_redraw();
    }

    fn open_devtools(&self) {
        if let Some(webview) = self.active_webview() {
            webview.open_devtools();
        }
    }

    fn clear_webview_data(&mut self) {
        for webview in self.webviews.values() {
            let _ = webview.clear_all_browsing_data();
        }
        self.last_autofill_keys.clear();
        self.request_redraw();
    }

    fn drain_pending_new_tabs(&mut self) {
        let pending = {
            let mut browser = self.ui.lock().unwrap();
            browser.take_pending_new_tabs()
        };
        for url in pending {
            self.create_tab(&url);
        }
    }

    fn apply_password_autofill(&mut self) {
        let Some(tab_id) = self
            .ui
            .lock()
            .ok()
            .and_then(|browser| browser.active_tab_id())
        else {
            return;
        };
        let (url, credentials) = {
            let browser = self.ui.lock().unwrap();
            let Some(tab) = browser.tab(tab_id) else {
                return;
            };
            (tab.url.clone(), browser.credentials_for_url(&tab.url))
        };
        let Some(credential) = credentials.last() else {
            return;
        };
        let key = format!(
            "{}:{}:{}:{}",
            tab_id, url, credential.username, credential.updated_at
        );
        if self.last_autofill_keys.get(&tab_id) == Some(&key) {
            return;
        }
        if let Some(webview) = self.webviews.get(&tab_id) {
            let script = password_autofill_script(&credential.username, &credential.password);
            if webview.evaluate_script(&script).is_ok() {
                self.last_autofill_keys.insert(tab_id, key);
            }
        }
    }

    fn request_redraw(&mut self) {
        self.needs_redraw = true;
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn sync_scroll_target_to_browser(&mut self) {
        let offset = {
            let mut browser = self.ui.lock().unwrap();
            browser.clamp_scroll_offset();
            browser.scroll_offset
        };
        self.scroll_target = offset;
        self.scroll_animating = false;
        self.last_scroll_frame = None;
        self.request_redraw();
    }

    fn smooth_scroll_by(&mut self, delta: f32) {
        let target = {
            let mut browser = self.ui.lock().unwrap();
            browser.clamp_scroll_offset();
            let max_scroll = browser.max_scroll_offset();
            let base = if self.scroll_animating {
                self.scroll_target
            } else {
                browser.scroll_offset
            };
            (base + delta).clamp(0.0, max_scroll)
        };
        self.smooth_scroll_to(target);
    }

    fn smooth_scroll_to(&mut self, target: f32) {
        {
            let mut browser = self.ui.lock().unwrap();
            browser.clamp_scroll_offset();
            let max_scroll = browser.max_scroll_offset();
            self.scroll_target = target.clamp(0.0, max_scroll);

            if (self.scroll_target - browser.scroll_offset).abs() <= 0.5 {
                browser.scroll_offset = self.scroll_target;
                self.scroll_animating = false;
                self.last_scroll_frame = None;
            } else {
                self.scroll_animating = true;
                self.last_scroll_frame = Some(Instant::now());
            }
        }
        self.request_redraw();
    }

    fn advance_scroll_animation(&mut self, browser: &mut Browser) {
        browser.clamp_scroll_offset();
        let max_scroll = browser.max_scroll_offset();
        self.scroll_target = self.scroll_target.clamp(0.0, max_scroll);

        if !self.scroll_animating {
            self.scroll_target = browser.scroll_offset;
            return;
        }

        let now = Instant::now();
        let dt = self
            .last_scroll_frame
            .map(|last| now.duration_since(last).as_secs_f32())
            .unwrap_or(1.0 / 60.0)
            .clamp(0.001, 0.050);
        self.last_scroll_frame = Some(now);

        let diff = self.scroll_target - browser.scroll_offset;
        if diff.abs() <= 0.5 {
            browser.scroll_offset = self.scroll_target;
            self.scroll_animating = false;
            self.last_scroll_frame = None;
            return;
        }

        let ease = 1.0 - (-dt / SCROLL_EASE_SECONDS).exp();
        browser.scroll_offset = (browser.scroll_offset + diff * ease).clamp(0.0, max_scroll);
    }

    fn render(&mut self) {
        let (width, height) = match &self.pixmap {
            Some(p) => (p.width(), p.height()),
            None => return,
        };

        // Browser'dan render et (tek lock)
        let (page_pixmap, chrome, scroll_offset, max_scroll, page_background) = {
            let ui = self.ui.clone();
            let mut browser = ui.lock().unwrap();
            let chrome_y = CHROME_HEIGHT as u32;
            let page_height = height.saturating_sub(chrome_y).max(1);
            browser.relayout(width as f32, page_height as f32);
            self.advance_scroll_animation(&mut browser);
            let chrome = ChromeSnapshot::from_browser(&browser);
            let scroll_offset = browser.scroll_offset;
            let max_scroll = browser.max_scroll_offset();
            let page_background = browser.canvas_background_color();

            let page_pixmap = if let Some(layout_root) = &browser.layout_result {
                let mut commands = Vec::new();
                crate::render::build_display_list(layout_root, &mut commands);
                let render_height = (page_height as f32 + max_scroll).ceil() as u32;
                Some(crate::render::render_to_pixmap_with_background(
                    &commands,
                    width,
                    render_height.max(1),
                    page_background,
                ))
            } else {
                None
            };

            (
                page_pixmap,
                chrome,
                scroll_offset,
                max_scroll,
                page_background,
            )
        };

        let Some(pixmap) = &mut self.pixmap else {
            return;
        };
        pixmap.fill(page_background.to_tiny_skia());

        let scale = self.scale_factor as f32;
        let logical_width = width as f32 / scale;

        // Page area background
        let page_area_top = (CHROME_HEIGHT * scale) as u32;
        let page_area_h = (height as f32 - page_area_top as f32).max(0.0);
        crate::render::fill_solid_rect(
            pixmap,
            0.0,
            page_area_top as f32,
            width as f32,
            page_area_h,
            page_background,
        );

        if let Some(page_pixmap) = page_pixmap {
            blit_scrolled(
                pixmap,
                &page_pixmap,
                0,
                (CHROME_HEIGHT * scale) as u32,
                scroll_offset * scale,
                max_scroll * scale,
            );
            draw_scrollbar(pixmap, width, height, scroll_offset * scale, max_scroll * scale, scale);
        } else {
            draw_empty_page(pixmap, width, height, page_background, scale);
        }
        draw_chrome_shell(
            pixmap,
            width,
            &chrome,
            self.hovered_tab_index,
            self.hovered_toolbar_btn,
            &self.hovered_link_url,
            scale,
            logical_width,
        );

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
        self.needs_redraw = false;
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
        let incognito = active_tab.map(|tab| tab.incognito).unwrap_or(false);

        let user_profile = browser.user_profile();
        let user_display_name = user_profile.display_name.clone();
        let user_avatar_color = user_profile.avatar_color;

        let tabs = browser
            .tabs
            .iter()
            .map(|tab| TabInfo {
                title: if tab.title.trim().is_empty() {
                    "Yeni Sekme".to_string()
                } else {
                    tab.title.clone()
                },
                loading: tab.loading,
                incognito: tab.incognito,
            })
            .collect();

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
            zoom_percent: active_tab
                .map(|tab| tab.zoom_percent)
                .unwrap_or(DEFAULT_ZOOM_PERCENT),
            bookmarked: browser.is_active_bookmarked(),
            tabs,
            active_tab_index: browser.active_tab,
            incognito,
            user_display_name,
            user_avatar_color,
        }
    }
}

fn apply_navigation_target(webview: &WebView, target: NavigationTarget) {
    match target {
        NavigationTarget::Url(url) => {
            let _ = webview.load_url(&url);
        }
        NavigationTarget::Html { html, .. } => {
            let _ = webview.load_html(&html);
        }
    }
}

fn download_target_for_url(url: &str, suggested: &Path) -> PathBuf {
    let downloads_dir = dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let name = suggested
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| name.to_string())
        .or_else(|| {
            url::Url::parse(url)
                .ok()
                .and_then(|parsed| {
                    parsed
                        .path_segments()
                        .and_then(|mut segments| segments.next_back().map(|s| s.to_string()))
                })
                .filter(|name| !name.trim().is_empty())
        })
        .unwrap_or_else(|| "download".to_string());

    downloads_dir.join(sanitize_filename(&name))
}

fn sanitize_filename(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect();
    if clean.trim().is_empty() {
        "download".to_string()
    } else {
        clean
    }
}

fn js_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn password_capture_script() -> &'static str {
    r#"
(function () {
  if (window.__oxiPasswordCaptureInstalled) return;
  window.__oxiPasswordCaptureInstalled = true;
  function bestUsername(form, passwordInput) {
    var inputs = Array.prototype.slice.call(form.querySelectorAll('input'));
    var candidates = inputs.filter(function (input) {
      var type = (input.getAttribute('type') || 'text').toLowerCase();
      return input !== passwordInput && (type === 'text' || type === 'email' || type === 'tel' || type === 'username' || type === '');
    });
    var beforePassword = candidates.filter(function (input) {
      return inputs.indexOf(input) < inputs.indexOf(passwordInput);
    });
    var selected = beforePassword.pop() || candidates[0];
    return selected ? selected.value : '';
  }
  document.addEventListener('submit', function (event) {
    var form = event.target;
    if (!form || !form.querySelector) return;
    var passwordInput = form.querySelector('input[type="password"]');
    if (!passwordInput || !passwordInput.value) return;
    var username = bestUsername(form, passwordInput);
    if (!username) return;
    try {
      window.ipc.postMessage(JSON.stringify({
        kind: 'password-submit',
        url: window.location.href,
        username: username,
        password: passwordInput.value
      }));
    } catch (_) {}
  }, true);
})();
"#
}

fn password_autofill_script(username: &str, password: &str) -> String {
    format!(
        r#"
(function () {{
  var username = {username};
  var password = {password};
  function fill() {{
    var passwordInput = document.querySelector('input[type="password"]');
    if (!passwordInput || passwordInput.value) return;
    var inputs = Array.prototype.slice.call(document.querySelectorAll('input'));
    var candidates = inputs.filter(function (input) {{
      var type = (input.getAttribute('type') || 'text').toLowerCase();
      return input !== passwordInput && (type === 'text' || type === 'email' || type === 'tel' || type === 'username' || type === '');
    }});
    var beforePassword = candidates.filter(function (input) {{
      return inputs.indexOf(input) < inputs.indexOf(passwordInput);
    }});
    var userInput = beforePassword.pop() || candidates[0];
    if (userInput && !userInput.value) {{
      userInput.value = username;
      userInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
      userInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
    }}
    passwordInput.value = password;
    passwordInput.dispatchEvent(new Event('input', {{ bubbles: true }}));
    passwordInput.dispatchEvent(new Event('change', {{ bubbles: true }}));
  }}
  if (document.readyState === 'loading') {{
    document.addEventListener('DOMContentLoaded', fill, {{ once: true }});
  }} else {{
    fill();
  }}
}})();
"#,
        username = js_string(username),
        password = js_string(password)
    )
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

fn draw_chrome_shell(
    pixmap: &mut Pixmap,
    width: u32,
    chrome: &ChromeSnapshot,
    hovered_tab_index: Option<usize>,
    hovered_toolbar_btn: Option<ChromeHit>,
    hovered_link_url: &Option<String>,
    scale: f32,
    logical_width: f32,
) {
    let width = width as f32;
    draw_tab_strip(pixmap, width, chrome, hovered_tab_index, scale, logical_width);
    draw_toolbar(pixmap, width, chrome, hovered_toolbar_btn, scale, logical_width);
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        (CHROME_HEIGHT - 1.0) * scale,
        width,
        1.0 * scale,
        Color::new(218, 220, 224, 255),
    );

    // Show hovered link URL at bottom
    if let Some(link_url) = hovered_link_url {
        if !link_url.is_empty() {
            let label = fit_text(link_url, logical_width - 28.0, 10.0);
            crate::render::draw_ui_text(
                pixmap,
                &label,
                14.0 * scale,
                (CHROME_HEIGHT - 14.0) * scale,
                width - 28.0 * scale,
                10.0 * scale,
                Color::new(95, 99, 104, 255),
                TextAlign::Left,
            );
        }
    }
}

fn draw_tab_strip(
    pixmap: &mut Pixmap,
    width: f32,
    chrome: &ChromeSnapshot,
    hovered_tab_index: Option<usize>,
    scale: f32,
    logical_width: f32,
) {
    // Tab strip background - darker top bar like Chrome
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        0.0,
        width,
        TAB_STRIP_HEIGHT * scale,
        Color::new(222, 225, 230, 255),
    );

    let tab_x_start = 12.0 * scale;
    let tab_y = 6.0 * scale;
    let tab_h = 34.0 * scale;
    let tab_gap = 2.0 * scale;
    let max_tab_w = (logical_width * 0.25)
        .clamp(120.0, 220.0)
        .min((logical_width - 86.0).max(80.0)) * scale;

    let mut current_x = tab_x_start;
    let max_x = width - 56.0 * scale;

    for (idx, tab_info) in chrome.tabs.iter().enumerate() {
        if current_x + max_tab_w > max_x && idx != chrome.active_tab_index {
            continue;
        }

        let tab_w = max_tab_w.min(max_x - current_x).max(60.0 * scale);
        let is_active = idx == chrome.active_tab_index;
        let is_hovered = hovered_tab_index == Some(idx);

        // Tab background
        let tab_bg = if is_active {
            Color::new(255, 255, 255, 255)
        } else if is_hovered {
            Color::new(238, 240, 244, 255)
        } else {
            Color::new(215, 218, 224, 255)
        };

        if is_active {
            fill_rounded_rect_top(pixmap, current_x, tab_y, tab_w, tab_h, 10.0 * scale, tab_bg);
        } else {
            fill_rounded_rect(pixmap, current_x, tab_y + 4.0 * scale, tab_w, tab_h - 4.0 * scale, 8.0 * scale, tab_bg);
        }

        // Tab favicon (colored circle)
        let favicon_color = if tab_info.incognito {
            Color::new(128, 128, 128, 255)
        } else if tab_info.loading {
            Color::new(251, 188, 4, 255)
        } else {
            Color::new(66, 133, 244, 255)
        };
        fill_rounded_rect(
            pixmap,
            current_x + 10.0 * scale,
            tab_y + 10.0 * scale,
            14.0 * scale,
            14.0 * scale,
            7.0 * scale,
            favicon_color,
        );

        // Loading spinner indicator
        if tab_info.loading {
            fill_rounded_rect(
                pixmap,
                current_x + 14.0 * scale,
                tab_y + 14.0 * scale,
                6.0 * scale,
                6.0 * scale,
                3.0 * scale,
                Color::new(255, 255, 255, 255),
            );
        }

        // Tab title
        let title_max_w = tab_w - 52.0 * scale;
        let tab_title = fit_text(&tab_info.title, title_max_w, 12.0);
        let title_color = if is_active {
            Color::new(32, 33, 36, 255)
        } else {
            Color::new(95, 99, 104, 255)
        };
        crate::render::draw_ui_text(
            pixmap,
            &tab_title,
            current_x + 28.0 * scale,
            tab_y + 10.0 * scale,
            title_max_w,
            12.0 * scale,
            title_color,
            TextAlign::Left,
        );

        // Close tab button
        let close_x = current_x + tab_w - 22.0 * scale;
        let close_y = tab_y + 8.0 * scale;
        let is_close_hovered = false;
        let close_bg = if is_close_hovered {
            Color::new(220, 220, 220, 255)
        } else if is_active {
            Color::new(240, 240, 240, 255)
        } else {
            Color::new(215, 218, 224, 255)
        };
        fill_rounded_rect(pixmap, close_x, close_y, 16.0 * scale, 16.0 * scale, 8.0 * scale, close_bg);
        icons::draw_icon(
            pixmap,
            "close",
            close_x + 4.0 * scale,
            close_y + 4.0 * scale,
            8.0 * scale,
            Color::new(95, 99, 104, 255),
        );

        current_x += tab_w + tab_gap;
    }

    // New tab button (+)
    let plus_x = current_x.min(width - 44.0 * scale);
    let plus_hover = false;
    let plus_bg = if plus_hover {
        Color::new(210, 213, 218, 255)
    } else {
        Color::new(232, 234, 237, 255)
    };
    fill_rounded_rect(pixmap, plus_x, 9.0 * scale, 28.0 * scale, 28.0 * scale, 14.0 * scale, plus_bg);
    icons::draw_icon(
        pixmap,
        "plus",
        plus_x + 6.0 * scale,
        9.0 * scale,
        16.0 * scale,
        Color::new(60, 64, 67, 255),
    );

    // Incognito indicator
    if chrome.incognito {
        icons::draw_icon(
            pixmap,
            "incognito",
            width - 100.0 * scale,
            12.0 * scale,
            18.0 * scale,
            Color::new(128, 128, 128, 255),
        );
    }
}

fn draw_toolbar(
    pixmap: &mut Pixmap,
    width: f32,
    chrome: &ChromeSnapshot,
    hovered_toolbar_btn: Option<ChromeHit>,
    scale: f32,
    logical_width: f32,
) {
    let y = TAB_STRIP_HEIGHT * scale;
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        y,
        width,
        TOOLBAR_HEIGHT * scale,
        Color::new(255, 255, 255, 255),
    );

    let btn_y = y + 10.0 * scale;

    // Back button
    let back_hover = hovered_toolbar_btn == Some(ChromeHit::Back);
    draw_nav_button(pixmap, 12.0 * scale, btn_y, "back", chrome.can_back, back_hover, scale);
    // Forward button
    let fwd_hover = hovered_toolbar_btn == Some(ChromeHit::Forward);
    draw_nav_button(pixmap, 48.0 * scale, btn_y, "forward", chrome.can_forward, fwd_hover, scale);
    // Refresh button
    let refresh_hover = hovered_toolbar_btn == Some(ChromeHit::Refresh);
    draw_nav_button(
        pixmap,
        84.0 * scale,
        btn_y,
        if chrome.loading { "close" } else { "refresh" },
        true,
        refresh_hover,
        scale,
    );

    // URL bar
    let address_x = 128.0 * scale;
    let address_w = (logical_width - 128.0 - 104.0).max(140.0) * scale;

    // URL bar background
    let url_bg = if chrome.url_bar_focused {
        Color::new(255, 255, 255, 255)
    } else {
        Color::new(241, 243, 244, 255)
    };
    fill_rounded_rect(pixmap, address_x, y + 8.0 * scale, address_w, 36.0 * scale, 20.0 * scale, url_bg);

    // Focus border
    if chrome.url_bar_focused {
        draw_rounded_rect_border(
            pixmap,
            address_x,
            y + 8.0 * scale,
            address_w,
            36.0 * scale,
            20.0 * scale,
            Color::new(66, 133, 244, 255),
            1.5 * scale,
        );
    }

    // Lock/security icon
    let is_https = chrome.url.starts_with("https://");
    let lock_color = if is_https {
        Color::new(52, 168, 83, 255)
    } else {
        Color::new(128, 134, 139, 255)
    };
    let lock_icon_name = if is_https { "lock" } else { "search" };
    icons::draw_icon(
        pixmap,
        lock_icon_name,
        address_x + 10.0 * scale,
        y + 15.0 * scale,
        16.0 * scale,
        lock_color,
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
    let url_text = fit_text(url_display, address_w - 80.0 * scale, 13.0);
    crate::render::draw_ui_text(
        pixmap,
        &url_text,
        address_x + 32.0 * scale,
        y + 18.0 * scale,
        address_w - 78.0 * scale,
        13.0 * scale,
        url_color,
        TextAlign::Left,
    );

    // Cursor indicator when focused
    if chrome.url_bar_focused {
        crate::render::fill_solid_rect(
            pixmap,
            address_x + 32.0 * scale + (url_text.chars().count() as f32 * 7.0 * scale).min(address_w - 80.0 * scale),
            y + 18.0 * scale,
            1.5 * scale,
            14.0 * scale,
            Color::new(32, 33, 36, 255),
        );
    }

    // Star/bookmark button
    let star_hover = hovered_toolbar_btn == Some(ChromeHit::Bookmark);
    let star_color = if chrome.bookmarked {
        Color::new(251, 188, 4, 255)
    } else if star_hover {
        Color::new(180, 180, 180, 255)
    } else {
        Color::new(95, 99, 104, 255)
    };
    icons::draw_icon(
        pixmap,
        if chrome.bookmarked { "star" } else { "star_outline" },
        address_x + address_w - 34.0 * scale,
        y + 15.0 * scale,
        20.0 * scale,
        star_color,
    );

    // Profile button
    let prof_x = width - 78.0 * scale;
    let prof_hover = hovered_toolbar_btn == Some(ChromeHit::Profile);
    let avatar_r = ((chrome.user_avatar_color >> 16) & 0xFF) as u8;
    let avatar_g = ((chrome.user_avatar_color >> 8) & 0xFF) as u8;
    let avatar_b = (chrome.user_avatar_color & 0xFF) as u8;
    let prof_bg = if prof_hover {
        Color::new(avatar_r.saturating_sub(20), avatar_g.saturating_sub(20), avatar_b.saturating_sub(20), 255)
    } else {
        Color::new(avatar_r, avatar_g, avatar_b, 255)
    };
    fill_rounded_rect(pixmap, prof_x, y + 12.0 * scale, 28.0 * scale, 28.0 * scale, 14.0 * scale, prof_bg);
    // Show first letter of display name as avatar
    let initial = chrome.user_display_name.chars().next().unwrap_or('U').to_uppercase().to_string();
    crate::render::draw_ui_text(
        pixmap,
        &initial,
        prof_x,
        y + 15.0 * scale,
        28.0 * scale,
        14.0 * scale,
        Color::new(255, 255, 255, 255),
        TextAlign::Center,
    );

    // Menu button
    let menu_hover = hovered_toolbar_btn == Some(ChromeHit::Menu);
    draw_nav_button(pixmap, width - 42.0 * scale, btn_y, "menu", true, menu_hover, scale);

    // Status bar / link hover display
    let status_display = if chrome.loading {
        "Yükleniyor...".to_string()
    } else if !chrome.status.trim().is_empty() {
        chrome.status.clone()
    } else {
        String::new()
    };
    if !status_display.trim().is_empty() {
        let status_with_zoom = format!("{} · {}%", status_display, chrome.zoom_percent);
        let label = fit_text(&status_with_zoom, logical_width - 28.0, 10.0);
        crate::render::draw_ui_text(
            pixmap,
            &label,
            14.0 * scale,
            (CHROME_HEIGHT - 14.0) * scale,
            width - 28.0 * scale,
            10.0 * scale,
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
    scale: f32,
) {
    let page_height = (height as f32 - (CHROME_HEIGHT * scale)) as f32;
    if page_height <= 0.0 || max_scroll <= 0.0 {
        return;
    }

    let sb_x = width as f32 - (SCROLLBAR_WIDTH + 2.0) * scale;
    let sb_y = CHROME_HEIGHT * scale;
    let sb_h = page_height;

    // Track background
    crate::render::fill_solid_rect(
        pixmap,
        sb_x,
        sb_y,
        SCROLLBAR_WIDTH * scale,
        sb_h,
        Color::new(241, 243, 244, 255),
    );

    // Thumb
    let total_height = page_height + max_scroll;
    let thumb_h = (page_height / total_height * sb_h).max(20.0 * scale).min(sb_h);
    let thumb_y = sb_y + (scroll_offset / max_scroll) * (sb_h - thumb_h);

    fill_rounded_rect(
        pixmap,
        sb_x + 1.0 * scale,
        thumb_y,
        (SCROLLBAR_WIDTH - 2.0) * scale,
        thumb_h,
        3.0 * scale,
        Color::new(188, 192, 196, 255),
    );
}

fn draw_empty_page(pixmap: &mut Pixmap, width: u32, height: u32, background: Color, scale: f32) {
    crate::render::fill_solid_rect(
        pixmap,
        0.0,
        CHROME_HEIGHT * scale,
        width as f32,
        (height as f32 - CHROME_HEIGHT * scale).max(0.0),
        background,
    );
}

fn draw_nav_button(pixmap: &mut Pixmap, x: f32, y: f32, icon: &str, enabled: bool, hover: bool, scale: f32) {
    let bg = if hover {
        Color::new(241, 243, 244, 255)
    } else {
        Color::new(255, 255, 255, 255)
    };
    let fg = if enabled {
        Color::new(60, 64, 67, 255)
    } else {
        Color::new(200, 204, 208, 255)
    };

    fill_rounded_rect(pixmap, x, y, 30.0 * scale, 30.0 * scale, 15.0 * scale, bg);
    icons::draw_icon(pixmap, icon, x + 7.0 * scale, y + 7.0 * scale, 16.0 * scale, fg);
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

fn draw_rounded_rect_border(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
    color: Color,
    stroke_width: f32,
) {
    if width <= 0.0 || height <= 0.0 {
        return;
    }

    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let sw = stroke_width;

    // Draw outer rounded rect (border color)
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    );

    let mut outer = PathBuilder::new();
    outer.move_to(x - sw + r, y - sw);
    outer.line_to(x + width - r + sw, y - sw);
    outer.quad_to(x + width + sw, y - sw, x + width + sw, y - sw + r);
    outer.line_to(x + width + sw, y + height - r + sw);
    outer.quad_to(x + width + sw, y + height + sw, x + width - r + sw, y + height + sw);
    outer.line_to(x - sw + r, y + height + sw);
    outer.quad_to(x - sw, y + height + sw, x - sw, y + height - r + sw);
    outer.line_to(x - sw, y - sw + r);
    outer.quad_to(x - sw, y - sw, x - sw + r, y - sw);
    outer.close();

    if let Some(path) = outer.finish() {
        pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
    }

    // Draw inner rounded rect (white to create hollow effect)
    let mut inner_paint = Paint::default();
    inner_paint.set_color_rgba8(255, 255, 255, 255);

    let mut inner = PathBuilder::new();
    inner.move_to(x + r, y);
    inner.line_to(x + width - r, y);
    inner.quad_to(x + width, y, x + width, y + r);
    inner.line_to(x + width, y + height - r);
    inner.quad_to(x + width, y + height, x + width - r, y + height);
    inner.line_to(x + r, y + height);
    inner.quad_to(x, y + height, x, y + height - r);
    inner.line_to(x, y + r);
    inner.quad_to(x, y, x + r, y);
    inner.close();

    if let Some(path) = inner.finish() {
        pixmap.fill_path(&path, &inner_paint, FillRule::Winding, Transform::identity(), None);
    }
}

/// Hit test for tab strip - returns tab index if hovering over a tab
fn hit_test_tab(mx: f32, my: f32, tab_count: usize, active_tab: usize, logical_width: f32) -> Option<usize> {
    if my < 6.0 || my > TAB_STRIP_HEIGHT {
        return None;
    }

    if tab_count == 0 {
        return None;
    }

    let tab_x_start = 12.0;
    let tab_y = 6.0;
    let tab_h = 34.0;
    let tab_gap = 2.0;
    let max_tab_w = (logical_width * 0.25)
        .clamp(120.0, 220.0)
        .min((logical_width - 86.0).max(80.0));

    let mut current_x = tab_x_start;
    let max_x = logical_width - 56.0;

    for idx in 0..tab_count {
        if current_x + max_tab_w > max_x && idx != active_tab {
            continue;
        }

        let tab_w = max_tab_w.min(max_x - current_x).max(60.0);

        if mx >= current_x && mx < current_x + tab_w && my >= tab_y && my < tab_y + tab_h {
            return Some(idx);
        }

        current_x += tab_w + tab_gap;
    }

    None
}

/// Hit test for chrome UI elements
fn hit_test(mx: f32, my: f32, logical_width: f32) -> Option<ChromeHit> {
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
    let url_right = (logical_width - 104.0).max(url_x + 100.0);
    // Bookmark star at the right side of URL bar
    if mx >= url_right - 42.0
        && mx < url_right - 10.0
        && my >= toolbar_y + 10.0
        && my < toolbar_y + 42.0
    {
        return Some(ChromeHit::Bookmark);
    }
    if mx >= url_x && mx < url_right && my >= toolbar_y + 8.0 && my < toolbar_y + 44.0 {
        return Some(ChromeHit::UrlBar);
    }

    // Profile: x=width-78..width-50
    if mx >= logical_width - 78.0 && mx < logical_width - 50.0 && my >= toolbar_y + 12.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Profile);
    }
    // Menu: x=width-42..width-12
    if mx >= logical_width - 42.0 && mx < logical_width - 12.0 && my >= toolbar_y + 10.0 && my < toolbar_y + 40.0 {
        return Some(ChromeHit::Menu);
    }

    // New tab (+) button in tab strip: x after tab, width=28
    let tab_x = 12.0;
    let tab_w = (logical_width * 0.25)
        .clamp(120.0, 220.0)
        .min((logical_width - 86.0).max(80.0));
    let tab_gap = 2.0;
    let mut current_x = tab_x + tab_w + tab_gap;
    let max_x = logical_width - 56.0;
    // Skip to end of tabs
    for _ in 1..10 {
        if current_x + tab_w > max_x {
            break;
        }
        current_x += tab_w + tab_gap;
    }
    let plus_x = current_x.min(logical_width - 44.0);
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
