/// Browser UI - winit + softbuffer ile pencere yönetimi
use crate::browser::Browser;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use tiny_skia::Pixmap;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

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

        // Browser'dan render et
        {
            let mut browser = self.ui.lock().unwrap();
            browser.relayout(width as f32, height as f32);

            if let Some(layout_root) = &browser.layout_result {
                let mut commands = Vec::new();
                crate::render::build_display_list(layout_root, &mut commands);
                let page_pixmap = crate::render::render_to_pixmap(&commands, width, height);
                let data = pixmap.data_mut();
                let page_data = page_pixmap.data();
                let copy_len = data.len().min(page_data.len());
                data[..copy_len].copy_from_slice(&page_data[..copy_len]);
            } else {
                pixmap.fill(tiny_skia::Color::from_rgba8(245, 245, 245, 255));
                let mut pm = pixmap.as_mut();

                for y in 0..height.min(100) {
                    let t = y as f32 / 100.0;
                    let r = (245.0 * (1.0 - t) + 230.0 * t) as u8;
                    let g = (245.0 * (1.0 - t) + 126.0 * t) as u8;
                    let b = (245.0 * (1.0 - t) + 34.0 * t) as u8;
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color_rgba8(r, g, b, 255);
                    if let Some(rect) = tiny_skia::Rect::from_xywh(0.0, y as f32, width as f32, 1.0)
                    {
                        pm.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                    }
                }

                let bx = (width as f32 - 300.0) / 2.0;
                let by = (height as f32 - 200.0) / 2.0;
                let mut paint = tiny_skia::Paint::default();
                paint.set_color_rgba8(255, 255, 255, 230);
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx, by, 300.0, 200.0) {
                    pm.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }
                paint.set_color_rgba8(230, 126, 34, 255);
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx + 30.0, by + 30.0, 180.0, 24.0) {
                    pm.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }
                paint.set_color_rgba8(100, 100, 100, 255);
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx + 30.0, by + 70.0, 240.0, 3.0) {
                    pm.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }
                paint.set_color_rgba8(60, 60, 60, 255);
                if let Some(rect) = tiny_skia::Rect::from_xywh(bx + 30.0, by + 100.0, 120.0, 16.0) {
                    pm.fill_rect(rect, &paint, tiny_skia::Transform::identity(), None);
                }
            }
        }

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
