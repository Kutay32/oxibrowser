/// Browser - ana browser durumu ve işlemleri
use crate::css;
use crate::dom::Node;
use crate::html;
use crate::layout::{self, LayoutBox};
use crate::net;
use crate::style;
use crate::types::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_ZOOM_PERCENT: u16 = 100;
pub const ZOOM_PRESETS: &[u16] = &[
    25, 33, 50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200, 250, 300, 400, 500,
];

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationTarget {
    Url(String),
    Html { url: String, html: String },
}

/// Sekme bilgisi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub id: u32,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub history: Vec<String>,
    #[serde(default)]
    pub history_pos: usize,
    #[serde(default = "default_zoom_percent")]
    pub zoom_percent: u16,
    #[serde(default)]
    pub loading: bool,
    #[serde(default)]
    pub incognito: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bookmark {
    pub title: String,
    pub url: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HistoryEntry {
    pub title: String,
    pub url: String,
    pub visited_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DownloadState {
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadItem {
    pub url: String,
    pub path: Option<PathBuf>,
    pub state: DownloadState,
    pub started_at: u64,
    pub finished_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PasswordCredential {
    pub origin: String,
    pub username: String,
    pub password: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionItem {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub installed_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CookieItem {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub display_name: String,
    pub avatar_color: u32,
    pub created_at: u64,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            display_name: "Kullanıcı".to_string(),
            avatar_color: 0x4285F4,
            created_at: now_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSettings {
    pub default_search_url: String,
    pub restore_session: bool,
    pub home_url: String,
    #[serde(default)]
    pub sync_snapshot_path: Option<PathBuf>,
    #[serde(default = "default_extension_store_url")]
    pub extension_store_url: String,
    #[serde(default)]
    pub user_profile: Option<UserProfile>,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            default_search_url: "https://www.google.com/search?q={query}".to_string(),
            restore_session: true,
            home_url: "about:welcome".to_string(),
            sync_snapshot_path: None,
            extension_store_url: default_extension_store_url(),
            user_profile: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileData {
    #[serde(default)]
    pub settings: BrowserSettings,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    #[serde(default)]
    pub downloads: Vec<DownloadItem>,
    #[serde(default)]
    pub passwords: Vec<PasswordCredential>,
    #[serde(default)]
    pub cookies: Vec<CookieItem>,
    #[serde(default)]
    pub extensions: Vec<ExtensionItem>,
    #[serde(default)]
    pub session_tabs: Vec<Tab>,
    #[serde(default)]
    pub active_tab: usize,
}

#[derive(Debug, Clone)]
pub struct ProfileStore {
    pub path: PathBuf,
    pub data: ProfileData,
}

impl ProfileStore {
    pub fn load_default() -> Self {
        let path = default_profile_path();
        Self::load_from_path(path)
    }

    pub fn load_from_path(path: PathBuf) -> Self {
        let data = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<ProfileData>(&text) {
                Ok(data) => data,
                Err(_) => {
                    let backup = path.with_extension(format!("json.bad-{}", now_secs()));
                    let _ = std::fs::rename(&path, backup);
                    ProfileData::default()
                }
            },
            Err(_) => ProfileData::default(),
        };
        Self { path, data }
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Profil klasörü oluşturulamadı: {}", e))?;
        }

        let json = serde_json::to_string_pretty(&self.data)
            .map_err(|e| format!("Profil JSON yazılamadı: {}", e))?;
        std::fs::write(&self.path, json).map_err(|e| format!("Profil kaydedilemedi: {}", e))
    }
}

fn default_profile_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("OxiBrowser")
        .join("profile.json")
}

fn default_sync_snapshot_path() -> PathBuf {
    default_profile_path()
        .parent()
        .map(|path| path.join("profile-sync.json"))
        .unwrap_or_else(|| PathBuf::from("profile-sync.json"))
}

fn default_extension_store_url() -> String {
    "https://chromewebstore.google.com/".to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn title_for_url(url: &str) -> String {
    match url {
        "about:welcome" => "OxiBrowser".to_string(),
        "about:blank" => "Yeni Sekme".to_string(),
        "about:history" => "Geçmiş".to_string(),
        "about:downloads" => "İndirilenler".to_string(),
        "about:bookmarks" => "Yer İmleri".to_string(),
        "about:settings" => "Ayarlar".to_string(),
        "about:passwords" => "Şifreler".to_string(),
        "about:cookies" => "Cookie'ler".to_string(),
        "about:sync" => "Sync".to_string(),
        "about:extensions" => "Uzantılar".to_string(),
        "about:devtools" => "DevTools".to_string(),
        "about:profile" => "Profil".to_string(),
        "about:search-engines" => "Arama Motorları".to_string(),
        "about:import-bookmarks" => "İçe Aktar".to_string(),
        _ => url.to_string(),
    }
}

pub fn nearest_zoom_preset(value: u16) -> u16 {
    ZOOM_PRESETS
        .iter()
        .copied()
        .min_by_key(|preset| preset.abs_diff(value))
        .unwrap_or(DEFAULT_ZOOM_PERCENT)
}

fn default_zoom_percent() -> u16 {
    DEFAULT_ZOOM_PERCENT
}

/// Browser ana durumu
pub struct Browser {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub current_url: String,
    pub page_title: String,
    pub dom_tree: Option<Node>,
    pub stylesheet: css::Stylesheet,
    pub layout_result: Option<LayoutBox>,
    pub styles: HashMap<crate::dom::NodeId, style::ComputedStyle>,
    pub loading: bool,
    pub status_message: String,
    pub next_tab_id: u32,
    /// Scroll offset (pixels scrolled down from top)
    pub scroll_offset: f32,
    /// URL bar interaction state
    pub url_bar_focused: bool,
    pub url_bar_text: String,
    pub url_bar_select_all: bool,
    /// Viewport height for scroll calculations
    pub viewport_height: f32,
    pub profile: ProfileStore,
    pub pending_new_tabs: Vec<String>,
    pub last_find_query: String,
    pub find_bar_active: bool,
}

impl Browser {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            current_url: String::new(),
            page_title: String::new(),
            dom_tree: None,
            stylesheet: css::Stylesheet::empty(),
            layout_result: None,
            styles: HashMap::new(),
            loading: false,
            status_message: "OxiBrowser'a hoş geldiniz!".to_string(),
            next_tab_id: 1,
            scroll_offset: 0.0,
            url_bar_focused: false,
            url_bar_text: String::new(),
            url_bar_select_all: false,
            viewport_height: 600.0,
            profile: ProfileStore::load_default(),
            pending_new_tabs: Vec::new(),
            last_find_query: String::new(),
            find_bar_active: false,
        }
    }

    /// Yeni sekme aç
    pub fn new_tab(&mut self, url: &str) -> u32 {
        self.new_tab_with_options(url, false)
    }

    /// Yeni sekme aç (incognito seçeneği ile)
    pub fn new_tab_with_options(&mut self, url: &str, incognito: bool) -> u32 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let initial_url = if url.trim().is_empty() {
            "about:welcome".to_string()
        } else {
            net::normalize_url(url)
        };

        let tab = Tab {
            id,
            title: if initial_url == "about:welcome" {
                if incognito {
                    "Gizli Sekme".to_string()
                } else {
                    "Yeni Sekme".to_string()
                }
            } else {
                initial_url.clone()
            },
            url: initial_url.clone(),
            history: vec![initial_url],
            history_pos: 0,
            zoom_percent: DEFAULT_ZOOM_PERCENT,
            loading: false,
            incognito,
        };

        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        id
    }

    pub fn new_incognito_tab(&mut self, url: &str) -> u32 {
        self.new_tab_with_options(url, true)
    }

    /// Yeni sekme aç ve içeriğini hazırla.
    pub fn open_new_tab(&mut self, url: Option<&str>) -> u32 {
        self.new_tab(url.unwrap_or(""))
    }

    pub fn restore_session_or_welcome(&mut self, initial_url: Option<&str>) {
        if let Some(url) = initial_url {
            self.tabs.clear();
            self.active_tab = 0;
            self.new_tab(url);
            self.sync_active_tab_to_globals();
            return;
        }

        if self.profile.data.settings.restore_session && !self.profile.data.session_tabs.is_empty()
        {
            self.tabs = self.profile.data.session_tabs.clone();
            self.active_tab = self
                .profile
                .data
                .active_tab
                .min(self.tabs.len().saturating_sub(1));
            self.next_tab_id = self
                .tabs
                .iter()
                .map(|tab| tab.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
        }

        if self.tabs.is_empty() {
            let home = self.profile.data.settings.home_url.clone();
            self.new_tab(&home);
        }

        self.sync_active_tab_to_globals();
    }

    pub fn active_tab_id(&self) -> Option<u32> {
        self.tabs.get(self.active_tab).map(|tab| tab.id)
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn tab(&self, id: u32) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn switch_to_tab_id(&mut self, id: u32) -> bool {
        if let Some(index) = self.tabs.iter().position(|tab| tab.id == id) {
            self.active_tab = index;
            self.sync_active_tab_to_globals();
            true
        } else {
            false
        }
    }

    pub fn close_tab_by_id(&mut self, id: u32) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        self.tabs.remove(index);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len().saturating_sub(1);
        }
        self.sync_active_tab_to_globals();
        self.persist_session();
        true
    }

    pub fn plan_navigation(&mut self, input: &str) -> NavigationTarget {
        let url = net::normalize_url(input);
        if matches!(url.as_str(), "about:sync-export" | "about:sync-import") {
            let result = if url == "about:sync-export" {
                self.export_sync_snapshot()
            } else {
                self.import_sync_snapshot()
            };
            self.status_message = result.unwrap_or_else(|err| err);
            self.current_url = "about:sync".to_string();
            self.url_bar_text = self.current_url.clone();
            self.url_bar_focused = false;
            self.url_bar_select_all = false;
            self.find_bar_active = false;
            self.scroll_offset = 0.0;
            if let Some(tab) = self.active_tab_mut() {
                tab.loading = false;
                tab.url = "about:sync".to_string();
                tab.title = title_for_url("about:sync");
            }
            self.persist_session();
            return NavigationTarget::Html {
                html: self.about_page_html("about:sync"),
                url: "about:sync".to_string(),
            };
        }

        if url == "about:clear-passwords" {
            self.clear_passwords();
            self.current_url = "about:passwords".to_string();
            self.url_bar_text = self.current_url.clone();
            self.url_bar_focused = false;
            self.url_bar_select_all = false;
            self.find_bar_active = false;
            if let Some(tab) = self.active_tab_mut() {
                tab.loading = false;
                tab.url = "about:passwords".to_string();
                tab.title = title_for_url("about:passwords");
            }
            self.persist_session();
            return NavigationTarget::Html {
                html: self.about_page_html("about:passwords"),
                url: "about:passwords".to_string(),
            };
        }

        if url == "about:clear-cookies" {
            self.clear_cookies();
            self.current_url = "about:cookies".to_string();
            self.url_bar_text = self.current_url.clone();
            self.url_bar_focused = false;
            self.url_bar_select_all = false;
            self.find_bar_active = false;
            if let Some(tab) = self.active_tab_mut() {
                tab.loading = false;
                tab.url = "about:cookies".to_string();
                tab.title = title_for_url("about:cookies");
            }
            self.persist_session();
            return NavigationTarget::Html {
                html: self.about_page_html("about:cookies"),
                url: "about:cookies".to_string(),
            };
        }

        if url.starts_with("about:set-search-") {
            let engine = url.trim_start_matches("about:set-search-");
            self.set_default_search_engine(engine);
            self.current_url = "about:search-engines".to_string();
            self.url_bar_text = self.current_url.clone();
            self.url_bar_focused = false;
            self.url_bar_select_all = false;
            self.find_bar_active = false;
            if let Some(tab) = self.active_tab_mut() {
                tab.loading = false;
                tab.url = "about:search-engines".to_string();
                tab.title = title_for_url("about:search-engines");
            }
            self.persist_session();
            return NavigationTarget::Html {
                html: self.about_page_html("about:search-engines"),
                url: "about:search-engines".to_string(),
            };
        }

        if url.starts_with("about:set-avatar-") {
            let color_str = url.trim_start_matches("about:set-avatar-");
            let avatar_color = u32::from_str_radix(color_str, 16).unwrap_or(0x4285F4);
            let current_name = self.user_profile().display_name.clone();
            self.update_user_profile(&current_name, avatar_color);
            self.current_url = "about:profile".to_string();
            self.url_bar_text = self.current_url.clone();
            self.url_bar_focused = false;
            self.url_bar_select_all = false;
            self.find_bar_active = false;
            if let Some(tab) = self.active_tab_mut() {
                tab.loading = false;
                tab.url = "about:profile".to_string();
                tab.title = title_for_url("about:profile");
            }
            self.persist_session();
            return NavigationTarget::Html {
                html: self.about_page_html("about:profile"),
                url: "about:profile".to_string(),
            };
        }

        if url.starts_with("about:set-name-") {
            let name = url.trim_start_matches("about:set-name-");
            let current_color = self.user_profile().avatar_color;
            self.update_user_profile(name, current_color);
            self.current_url = "about:profile".to_string();
            self.url_bar_text = self.current_url.clone();
            self.url_bar_focused = false;
            self.url_bar_select_all = false;
            self.find_bar_active = false;
            if let Some(tab) = self.active_tab_mut() {
                tab.loading = false;
                tab.url = "about:profile".to_string();
                tab.title = title_for_url("about:profile");
            }
            self.persist_session();
            return NavigationTarget::Html {
                html: self.about_page_html("about:profile"),
                url: "about:profile".to_string(),
            };
        }

        self.loading = true;
        self.status_message = format!("{} yükleniyor...", url);
        self.current_url = url.clone();
        self.url_bar_text = url.clone();
        self.url_bar_focused = false;
        self.url_bar_select_all = false;
        self.find_bar_active = false;
        self.scroll_offset = 0.0;

        let title = title_for_url(&url);
        if let Some(tab) = self.active_tab_mut() {
            tab.loading = true;
            tab.url = url.clone();
            tab.title = title;
            let already_current = tab
                .history
                .get(tab.history_pos)
                .map(|current| current == &url)
                .unwrap_or(false);
            if !already_current {
                tab.history.truncate(tab.history_pos + 1);
                tab.history.push(url.clone());
                tab.history_pos = tab.history.len().saturating_sub(1);
            }
        }
        self.persist_session();

        if url.starts_with("about:") {
            NavigationTarget::Html {
                html: self.about_page_html(&url),
                url,
            }
        } else {
            NavigationTarget::Url(url)
        }
    }

    pub fn active_navigation_target(&self) -> NavigationTarget {
        let url = self
            .active_tab()
            .map(|tab| tab.url.clone())
            .unwrap_or_else(|| "about:welcome".to_string());
        self.navigation_target_for_url(&url)
    }

    pub fn navigation_target_for_url(&self, url: &str) -> NavigationTarget {
        if url.starts_with("about:") {
            NavigationTarget::Html {
                html: self.about_page_html(&url),
                url: url.to_string(),
            }
        } else {
            NavigationTarget::Url(url.to_string())
        }
    }

    pub fn mark_tab_loading(&mut self, tab_id: u32, url: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.loading = true;
            tab.url = url.to_string();
            let already_current = tab
                .history
                .get(tab.history_pos)
                .map(|current| current == url)
                .unwrap_or(false);
            if !already_current && !url.trim().is_empty() {
                tab.history.truncate(tab.history_pos + 1);
                tab.history.push(url.to_string());
                tab.history_pos = tab.history.len().saturating_sub(1);
            }
        }
        if self.active_tab_id() == Some(tab_id) {
            self.current_url = url.to_string();
            self.url_bar_text = url.to_string();
            self.loading = true;
            self.status_message = format!("{} yükleniyor...", url);
        }
    }

    pub fn finish_tab_load(&mut self, tab_id: u32, url: &str) {
        let mut title_for_history = String::new();
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.loading = false;
            if !url.trim().is_empty() {
                tab.url = url.to_string();
                if let Some(current) = tab.history.get_mut(tab.history_pos) {
                    *current = url.to_string();
                }
            }
            title_for_history = tab.title.clone();
        }

        if self.active_tab_id() == Some(tab_id) {
            self.current_url = url.to_string();
            self.url_bar_text = url.to_string();
            self.loading = false;
            self.status_message = "Hazır".to_string();
        }

        if !url.starts_with("about:") && !url.trim().is_empty() {
            self.push_history_entry(title_for_history, url.to_string());
        }
        self.persist_session();
    }

    pub fn update_tab_title(&mut self, tab_id: u32, title: &str) {
        let clean = title.trim();
        if clean.is_empty() {
            return;
        }
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            tab.title = clean.to_string();
        }
        if self.active_tab_id() == Some(tab_id) {
            self.page_title = clean.to_string();
        }
        self.persist_session();
    }

    pub fn queue_new_tab(&mut self, url: String) {
        self.pending_new_tabs.push(url);
    }

    pub fn take_pending_new_tabs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_new_tabs)
    }

    pub fn zoom_in(&mut self) -> u16 {
        let current = self
            .active_tab()
            .map(|tab| tab.zoom_percent)
            .unwrap_or(DEFAULT_ZOOM_PERCENT);
        let next = ZOOM_PRESETS
            .iter()
            .copied()
            .find(|preset| *preset > current)
            .unwrap_or(*ZOOM_PRESETS.last().unwrap_or(&DEFAULT_ZOOM_PERCENT));
        self.set_active_zoom(next)
    }

    pub fn zoom_out(&mut self) -> u16 {
        let current = self
            .active_tab()
            .map(|tab| tab.zoom_percent)
            .unwrap_or(DEFAULT_ZOOM_PERCENT);
        let next = ZOOM_PRESETS
            .iter()
            .rev()
            .copied()
            .find(|preset| *preset < current)
            .unwrap_or(*ZOOM_PRESETS.first().unwrap_or(&DEFAULT_ZOOM_PERCENT));
        self.set_active_zoom(next)
    }

    pub fn reset_zoom(&mut self) -> u16 {
        self.set_active_zoom(DEFAULT_ZOOM_PERCENT)
    }

    pub fn set_active_zoom(&mut self, zoom_percent: u16) -> u16 {
        let zoom_percent = nearest_zoom_preset(zoom_percent);
        if let Some(tab) = self.active_tab_mut() {
            tab.zoom_percent = zoom_percent;
        }
        self.status_message = format!("Yakınlaştırma: {}%", zoom_percent);
        self.persist_session();
        zoom_percent
    }

    pub fn toggle_bookmark_active(&mut self) -> bool {
        let Some(tab) = self.active_tab().cloned() else {
            return false;
        };
        if let Some(index) = self
            .profile
            .data
            .bookmarks
            .iter()
            .position(|bookmark| bookmark.url == tab.url)
        {
            self.profile.data.bookmarks.remove(index);
            self.status_message = "Yer imi kaldırıldı".to_string();
            let _ = self.profile.save();
            false
        } else {
            self.profile.data.bookmarks.push(Bookmark {
                title: tab.title,
                url: tab.url,
                created_at: now_secs(),
            });
            self.status_message = "Yer imi eklendi".to_string();
            let _ = self.profile.save();
            true
        }
    }

    pub fn is_active_bookmarked(&self) -> bool {
        self.active_tab()
            .map(|tab| {
                self.profile
                    .data
                    .bookmarks
                    .iter()
                    .any(|bookmark| bookmark.url == tab.url)
            })
            .unwrap_or(false)
    }

    pub fn record_download_started(&mut self, url: String, path: PathBuf) {
        self.profile.data.downloads.push(DownloadItem {
            url: url.clone(),
            path: Some(path),
            state: DownloadState::InProgress,
            started_at: now_secs(),
            finished_at: None,
        });
        self.status_message = format!("İndirme başladı: {}", url);
        let _ = self.profile.save();
    }

    pub fn record_download_finished(&mut self, url: String, path: Option<PathBuf>, success: bool) {
        if let Some(item) = self
            .profile
            .data
            .downloads
            .iter_mut()
            .rev()
            .find(|download| download.url == url && download.state == DownloadState::InProgress)
        {
            if path.is_some() {
                item.path = path;
            }
            item.state = if success {
                DownloadState::Completed
            } else {
                DownloadState::Failed
            };
            item.finished_at = Some(now_secs());
        }
        self.status_message = if success {
            "İndirme tamamlandı".to_string()
        } else {
            "İndirme başarısız".to_string()
        };
        let _ = self.profile.save();
    }

    pub fn clear_browsing_data(&mut self) {
        self.profile.data.history.clear();
        self.profile.data.downloads.clear();
        self.profile.data.cookies.clear();
        self.status_message = "Geçmiş, indirme ve cookie kayıtları temizlendi".to_string();
        let _ = self.profile.save();
    }

    pub fn add_cookie(&mut self, name: &str, value: &str, domain: &str, path: &str, expires: Option<u64>) {
        // Remove existing cookie with same name/domain/path
        self.profile.data.cookies.retain(|c| {
            !(c.name == name && c.domain == domain && c.path == path)
        });
        self.profile.data.cookies.push(CookieItem {
            name: name.to_string(),
            value: value.to_string(),
            domain: domain.to_string(),
            path: path.to_string(),
            created_at: now_secs(),
            expires_at: expires,
        });
        let _ = self.profile.save();
    }

    pub fn cookies_for_domain(&self, domain: &str) -> Vec<&CookieItem> {
        let now = now_secs();
        self.profile.data.cookies
            .iter()
            .filter(|c| {
                c.domain == domain || domain.ends_with(&c.domain)
            })
            .filter(|c| {
                c.expires_at.map(|exp| exp > now).unwrap_or(true)
            })
            .collect()
    }

    pub fn clear_cookies(&mut self) {
        self.profile.data.cookies.clear();
        self.status_message = "Cookie'ler temizlendi".to_string();
        let _ = self.profile.save();
    }

    pub fn expired_cookies_cleanup(&mut self) {
        let now = now_secs();
        self.profile.data.cookies.retain(|c| {
            c.expires_at.map(|exp| exp > now).unwrap_or(true)
        });
        let _ = self.profile.save();
    }

    pub fn user_profile(&self) -> &UserProfile {
        self.profile.data.settings.user_profile.as_ref().unwrap_or_else(|| {
            // Return a temporary default if not set
            static DEFAULT: std::sync::OnceLock<UserProfile> = std::sync::OnceLock::new();
            DEFAULT.get_or_init(UserProfile::default)
        })
    }

    pub fn update_user_profile(&mut self, display_name: &str, avatar_color: u32) {
        self.profile.data.settings.user_profile = Some(UserProfile {
            display_name: display_name.to_string(),
            avatar_color,
            created_at: self.profile.data.settings.user_profile
                .as_ref()
                .map(|p| p.created_at)
                .unwrap_or_else(now_secs),
        });
        self.status_message = format!("Profil güncellendi: {}", display_name);
        let _ = self.profile.save();
    }

    pub fn set_default_search_engine(&mut self, engine: &str) {
        let url = match engine {
            "google" => "https://www.google.com/search?q={query}",
            "bing" => "https://www.bing.com/search?q={query}",
            "duckduckgo" => "https://duckduckgo.com/?q={query}",
            "yahoo" => "https://search.yahoo.com/search?p={query}",
            _ => "https://www.google.com/search?q={query}",
        };
        self.profile.data.settings.default_search_url = url.to_string();
        self.status_message = format!("Arama motoru: {}", engine);
        let _ = self.profile.save();
    }

    pub fn save_password_for_url(&mut self, url: &str, username: &str, password: &str) -> bool {
        // Don't save passwords in incognito mode
        if let Some(tab) = self.active_tab() {
            if tab.incognito {
                return false;
            }
        }

        let Some(origin) = Self::origin_for_url(url) else {
            return false;
        };
        let username = username.trim();
        if username.is_empty() || password.is_empty() {
            return false;
        }

        if let Some(existing) = self
            .profile
            .data
            .passwords
            .iter_mut()
            .find(|item| item.origin == origin && item.username == username)
        {
            existing.password = password.to_string();
            existing.updated_at = now_secs();
        } else {
            self.profile.data.passwords.push(PasswordCredential {
                origin,
                username: username.to_string(),
                password: password.to_string(),
                updated_at: now_secs(),
            });
        }
        self.status_message = "Şifre kasasına kaydedildi".to_string();
        let _ = self.profile.save();
        true
    }

    pub fn credentials_for_url(&self, url: &str) -> Vec<PasswordCredential> {
        let Some(origin) = Self::origin_for_url(url) else {
            return Vec::new();
        };
        self.profile
            .data
            .passwords
            .iter()
            .filter(|item| item.origin == origin)
            .cloned()
            .collect()
    }

    pub fn clear_passwords(&mut self) {
        self.profile.data.passwords.clear();
        self.status_message = "Kayıtlı şifreler temizlendi".to_string();
        let _ = self.profile.save();
    }

    pub fn export_sync_snapshot(&mut self) -> Result<String, String> {
        let path = self
            .profile
            .data
            .settings
            .sync_snapshot_path
            .clone()
            .unwrap_or_else(default_sync_snapshot_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Sync klasörü oluşturulamadı: {}", e))?;
        }
        let json = serde_json::to_string_pretty(&self.profile.data)
            .map_err(|e| format!("Sync JSON hazırlanamadı: {}", e))?;
        std::fs::write(&path, json).map_err(|e| format!("Sync yazılamadı: {}", e))?;
        self.profile.data.settings.sync_snapshot_path = Some(path.clone());
        let _ = self.profile.save();
        Ok(format!("Sync snapshot yazıldı: {}", path.display()))
    }

    pub fn import_sync_snapshot(&mut self) -> Result<String, String> {
        let path = self
            .profile
            .data
            .settings
            .sync_snapshot_path
            .clone()
            .unwrap_or_else(default_sync_snapshot_path);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("Sync snapshot okunamadı: {}", e))?;
        let imported: ProfileData =
            serde_json::from_str(&text).map_err(|e| format!("Sync JSON bozuk: {}", e))?;
        self.merge_profile_data(imported);
        self.profile.data.settings.sync_snapshot_path = Some(path.clone());
        let _ = self.profile.save();
        Ok(format!("Sync snapshot içe aktarıldı: {}", path.display()))
    }

    pub fn set_find_query(&mut self, query: String) {
        self.last_find_query = query;
    }

    pub fn persist_session(&mut self) {
        self.profile.data.session_tabs = self.tabs.clone();
        self.profile.data.active_tab = self.active_tab;
        let _ = self.profile.save();
    }

    pub fn origin_for_url(url: &str) -> Option<String> {
        let parsed = url::Url::parse(url).ok()?;
        match parsed.scheme() {
            "http" | "https" => {
                let host = parsed.host_str()?;
                let port = parsed.port().map(|p| format!(":{}", p)).unwrap_or_default();
                Some(format!("{}://{}{}", parsed.scheme(), host, port))
            }
            _ => None,
        }
    }

    fn sync_active_tab_to_globals(&mut self) {
        if let Some(tab) = self.active_tab() {
            let url = tab.url.clone();
            let title = tab.title.clone();
            let loading = tab.loading;
            self.current_url = url.clone();
            self.url_bar_text = url;
            self.page_title = title;
            self.loading = loading;
        }
    }

    fn push_history_entry(&mut self, title: String, url: String) {
        if url.trim().is_empty() {
            return;
        }

        // Don't save history for incognito tabs
        if let Some(tab) = self.active_tab() {
            if tab.incognito {
                return;
            }
        }

        if self
            .profile
            .data
            .history
            .last()
            .map(|entry| entry.url == url)
            .unwrap_or(false)
        {
            return;
        }
        self.profile.data.history.push(HistoryEntry {
            title: if title.trim().is_empty() {
                url.clone()
            } else {
                title
            },
            url,
            visited_at: now_secs(),
        });
        if self.profile.data.history.len() > 2000 {
            let extra = self.profile.data.history.len() - 2000;
            self.profile.data.history.drain(0..extra);
        }
        let _ = self.profile.save();
    }

    fn merge_profile_data(&mut self, imported: ProfileData) {
        self.profile.data.settings = imported.settings;
        for bookmark in imported.bookmarks {
            if !self
                .profile
                .data
                .bookmarks
                .iter()
                .any(|existing| existing.url == bookmark.url)
            {
                self.profile.data.bookmarks.push(bookmark);
            }
        }
        for history in imported.history {
            if !self.profile.data.history.iter().any(|existing| {
                existing.url == history.url && existing.visited_at == history.visited_at
            }) {
                self.profile.data.history.push(history);
            }
        }
        for password in imported.passwords {
            if let Some(existing) =
                self.profile.data.passwords.iter_mut().find(|item| {
                    item.origin == password.origin && item.username == password.username
                })
            {
                if password.updated_at >= existing.updated_at {
                    *existing = password;
                }
            } else {
                self.profile.data.passwords.push(password);
            }
        }
        for extension in imported.extensions {
            if !self
                .profile
                .data
                .extensions
                .iter()
                .any(|existing| existing.path == extension.path)
            {
                self.profile.data.extensions.push(extension);
            }
        }
        if !imported.session_tabs.is_empty() {
            self.profile.data.session_tabs = imported.session_tabs;
            self.profile.data.active_tab = imported.active_tab;
        }
    }

    fn about_page_html(&self, url: &str) -> String {
        match url {
            "about:blank" => {
                "<html><head><title>Yeni Sekme</title></head><body></body></html>".to_string()
            }
            "about:version" => self.about_version_html(),
            "about:history" => self.history_html(),
            "about:downloads" => self.downloads_html(),
            "about:bookmarks" => self.bookmarks_html(),
            "about:settings" => self.settings_html(),
            "about:passwords" => self.passwords_html(),
            "about:cookies" => self.cookies_html(),
            "about:sync" => self.sync_html(),
            "about:extensions" => self.extensions_html(),
            "about:devtools" => self.devtools_html(),
            "about:welcome" => self.welcome_html(),
            "about:profile" => self.profile_html(),
            "about:search-engines" => self.search_engines_html(),
            "about:import-bookmarks" => self.import_bookmarks_html(),
            _ => self.error_page_html("Bilinmeyen about sayfası", url, "Bu iç sayfa bulunamadı."),
        }
    }

    /// URL'ye git
    pub fn navigate(&mut self, url_str: &str) {
        let normalized = net::normalize_url(url_str);
        self.current_url = normalized.clone();
        self.url_bar_text = normalized.clone();
        self.loading = true;
        self.status_message = format!("{} yükleniyor...", normalized);
    }

    /// Sayfayı yükle (senkron)
    pub fn load_url_sync(&mut self, url_str: &str) {
        self.load_url_sync_with_history(url_str, true);
    }

    fn load_url_sync_without_history(&mut self, url_str: &str) {
        self.load_url_sync_with_history(url_str, false);
    }

    fn load_url_sync_with_history(&mut self, url_str: &str, update_history: bool) {
        self.navigate(url_str);

        let url = net::normalize_url(url_str);
        self.scroll_offset = 0.0;
        self.status_message = format!("{} yükleniyor...", url);

        if url.is_empty() || url == "about:welcome" {
            self.render_about_page("about:welcome", update_history);
            return;
        }

        if url.starts_with("about:") {
            self.render_about_page(&url, update_history);
            return;
        }

        if url.starts_with("file://") {
            self.load_response_result(net::load_file_url(&url), &url, update_history);
            return;
        }

        // Blocking HTTP isteği
        let response = match Self::blocking_fetch(&url) {
            Ok(resp) => resp,
            Err(e) => {
                self.status_message = format!("Hata: {}", e);
                self.loading = false;
                // Hata sayfası oluştur
                let error_html = self.error_page_html("Bağlantı Hatası", &url, &e);
                self.parse_and_render(&error_html, &url);
                self.finish_navigation(&url, update_history);
                return;
            }
        };

        self.load_response_result(Ok(response), &url, update_history);
    }

    fn load_response_result(
        &mut self,
        result: Result<net::HttpResponse, String>,
        requested_url: &str,
        update_history: bool,
    ) {
        let response = match result {
            Ok(response) => response,
            Err(e) => {
                self.status_message = format!("Hata: {}", e);
                self.loading = false;
                let error_html = self.error_page_html("Sayfa Açılamadı", requested_url, &e);
                self.parse_and_render(&error_html, requested_url);
                self.finish_navigation(requested_url, update_history);
                return;
            }
        };

        self.status_message = format!("Durum: {}", response.status);
        let final_url = response.url.clone();

        // Content-Type kontrol et
        let is_html = response
            .headers
            .get("content-type")
            .map(|ct| ct.contains("text/html") || ct.contains("text/plain"))
            .unwrap_or(true);

        if is_html || response.body.trim().starts_with('<') {
            self.parse_and_render(&response.body, &final_url);
        } else {
            // HTML değil - metin olarak göster
            let wrapped = format!(
                "<html><body><pre>{}</pre></body></html>",
                Self::escape_html(&response.body.chars().take(10000).collect::<String>())
            );
            self.parse_and_render(&wrapped, &final_url);
        }

        self.current_url = final_url;
        self.url_bar_text = self.current_url.clone();
        self.loading = false;
        let current_url = self.current_url.clone();
        self.finish_navigation(&current_url, update_history);
    }

    /// Varsayılan hoş geldin sayfasını yükle.
    pub fn load_welcome_page(&mut self) {
        self.render_about_page("about:welcome", false);
    }

    /// Blocking HTTP isteği
    fn blocking_fetch(url: &str) -> Result<net::HttpResponse, String> {
        // Tokio runtime ile blocking fetch
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime: {}", e))?;
        rt.block_on(net::fetch_url(url))
    }

    /// HTML'i parse et, stilleri uygula, layout ve render
    pub fn parse_and_render(&mut self, html_text: &str, base_url: &str) {
        self.current_url = base_url.to_string();
        self.url_bar_text = base_url.to_string();
        self.url_bar_select_all = false;

        // HTML parse et
        self.dom_tree = Some(html::parse_html_simple(html_text));

        let dom = self.dom_tree.as_ref().unwrap();

        // Sayfa başlığını bul
        self.page_title = Self::extract_title(dom);

        // Satır içi CSS'i bul
        let inline_css = Self::extract_styles(dom);
        self.stylesheet = css::parse_css(&inline_css);

        // Stilleri uygula
        self.styles = style::apply_stylesheet(dom, &self.stylesheet);

        // Layout ağacı oluştur
        self.layout_result = layout::build_layout_tree(dom, &self.styles);

        // Layout hesapla
        if let Some(ref mut layout_root) = self.layout_result {
            layout::layout_document(layout_root, 1024.0, 2000.0);
        }

        // Sekme başlığını güncelle
        if !self.tabs.is_empty() {
            let tab = &mut self.tabs[self.active_tab];
            tab.title = self.page_title.clone();
            tab.url = base_url.to_string();
        }
    }

    /// Mevcut layout'u yeni viewport boyutuna gore tekrar hesapla.
    pub fn relayout(&mut self, viewport_width: f32, viewport_height: f32) {
        self.viewport_height = viewport_height;
        if let Some(ref mut layout_root) = self.layout_result {
            layout::layout_document(layout_root, viewport_width, viewport_height);
        }
        self.clamp_scroll_offset();
    }

    /// Sayfayı kaydır (delta pixels, pozitif = aşağı)
    pub fn scroll_by(&mut self, delta: f32) {
        self.scroll_offset += delta;
        self.clamp_scroll_offset();
    }

    /// Mevcut kaydırma konumunu sayfanın geçerli sınırlarına sabitle.
    pub fn clamp_scroll_offset(&mut self) {
        let max_scroll = self.max_scroll_offset();
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
    }

    /// Maksimum kaydırma mesafesi
    pub fn max_scroll_offset(&self) -> f32 {
        if let Some(ref layout_root) = self.layout_result {
            let content_h = layout_root.dimensions.margin_box().bottom();
            let viewport_h = self.viewport_height.max(100.0);
            (content_h - viewport_h).max(0.0)
        } else {
            0.0
        }
    }

    /// Viewport'un tamamına uygulanacak canvas arka plan rengini bul.
    pub fn canvas_background_color(&self) -> Color {
        self.first_element_background("body")
            .or_else(|| self.first_element_background("html"))
            .unwrap_or(Color::WHITE)
    }

    fn first_element_background(&self, tag: &str) -> Option<Color> {
        let dom = self.dom_tree.as_ref()?;
        for node in dom.elements_by_tag(tag) {
            if let Some(style) = self.styles.get(&node.id) {
                let color = style.background_color;
                if color.a > 0.0 {
                    return Some(color);
                }
            }
        }
        None
    }

    /// URL bar metnini güncelle
    pub fn set_url_bar_text(&mut self, text: &str) {
        self.url_bar_text = text.to_string();
        self.url_bar_select_all = false;
    }

    /// URL bar'ı odakla.
    pub fn focus_url_bar(&mut self, select_all: bool) {
        self.url_bar_focused = true;
        self.url_bar_select_all = select_all;
        self.find_bar_active = false;
        self.url_bar_text = self.current_url.clone();
    }

    /// URL bar odaktan çıksın.
    pub fn blur_url_bar(&mut self) {
        self.url_bar_focused = false;
        self.url_bar_select_all = false;
        self.find_bar_active = false;
        self.url_bar_text = self.current_url.clone();
    }

    /// URL bar'a yazı ekle.
    pub fn push_url_bar_text(&mut self, text: &str) {
        if self.url_bar_select_all {
            self.url_bar_text.clear();
            self.url_bar_select_all = false;
        }
        self.url_bar_text.push_str(text);
    }

    /// URL bar'dan karakter sil.
    pub fn backspace_url_bar(&mut self) {
        if self.url_bar_select_all {
            self.url_bar_text.clear();
            self.url_bar_select_all = false;
        } else {
            self.url_bar_text.pop();
        }
    }

    /// URL bar'dan navigasyon yap
    pub fn navigate_from_url_bar(&mut self) {
        let url = self.url_bar_text.clone();
        if !url.is_empty() {
            self.scroll_offset = 0.0;
            self.load_url_sync(&url);
        }
    }

    /// Sayfa koordinatındaki link hedefini bul.
    pub fn link_at(&self, x: f32, y: f32) -> Option<String> {
        let dom = self.dom_tree.as_ref()?;
        let layout_root = self.layout_result.as_ref()?;
        let mut link_targets = HashMap::new();
        Self::collect_link_targets(dom, None, &mut link_targets);
        Self::hit_test_link(layout_root, x, y, &link_targets)
            .map(|href| net::resolve_url(&self.current_url, &href))
    }

    /// HTML'den başlık çıkar
    fn extract_title(dom: &Node) -> String {
        if let Some(title_node) = dom.get_element_by_id("title") {
            let text = title_node.text_content().trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }

        // title tag'ini ara
        let titles: Vec<&Node> = dom.elements_by_tag("title");
        if let Some(title) = titles.first() {
            let text = title.text_content().trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }

        // h1 tag'ini ara
        let h1s: Vec<&Node> = dom.elements_by_tag("h1");
        if let Some(h1) = h1s.first() {
            let text = h1.text_content().trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }

        "OxiBrowser".to_string()
    }

    /// Satır içi CSS stillerini çıkar
    fn extract_styles(dom: &Node) -> String {
        let mut css_text = String::new();
        let styles: Vec<&Node> = dom.elements_by_tag("style");
        for style_node in &styles {
            css_text.push_str(&style_node.text_content());
            css_text.push('\n');
        }
        css_text
    }

    /// Sayfayı yenile
    pub fn refresh(&mut self) {
        let url = self.current_url.clone();
        if !url.is_empty() {
            self.load_url_sync_without_history(&url);
        }
    }

    /// Geri git
    pub fn go_back(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.history_pos > 0 {
            tab.history_pos -= 1;
            let url = tab.history[tab.history_pos].clone();
            self.load_url_sync_without_history(&url);
            true
        } else {
            false
        }
    }

    /// İleri git
    pub fn go_forward(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let tab = &mut self.tabs[self.active_tab];
        if tab.history_pos < tab.history.len().saturating_sub(1) {
            tab.history_pos += 1;
            let url = tab.history[tab.history_pos].clone();
            self.load_url_sync_without_history(&url);
            true
        } else {
            false
        }
    }

    /// Sekmeyi kapat
    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() && self.tabs.len() > 1 {
            self.tabs.remove(index);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            self.reload_active_tab_without_history();
        }
    }

    /// Sıradaki sekmeye geç.
    pub fn switch_to_next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.reload_active_tab_without_history();
        }
    }

    fn reload_active_tab_without_history(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let url = tab.url.clone();
            self.load_url_sync_without_history(&url);
        }
    }

    fn finish_navigation(&mut self, url: &str, update_history: bool) {
        self.current_url = url.to_string();
        self.url_bar_text = url.to_string();
        self.url_bar_select_all = false;
        self.loading = false;

        if self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.active_tab];
        tab.url = url.to_string();
        tab.loading = false;
        if !self.page_title.trim().is_empty() {
            tab.title = self.page_title.clone();
        }

        if update_history {
            let already_current = tab
                .history
                .get(tab.history_pos)
                .map(|current| current == url)
                .unwrap_or(false);
            if !already_current {
                tab.history.truncate(tab.history_pos + 1);
                tab.history.push(url.to_string());
                tab.history_pos = tab.history.len().saturating_sub(1);
            }
        }
    }

    fn render_about_page(&mut self, url: &str, update_history: bool) {
        let html = self.about_page_html(url);

        self.status_message = "Hazır".to_string();
        self.parse_and_render(&html, url);
        self.finish_navigation(url, update_history);
    }

    fn collect_link_targets(
        node: &Node,
        current_href: Option<String>,
        targets: &mut HashMap<crate::dom::NodeId, String>,
    ) {
        let href = match node.as_element() {
            Some(el) if el.tag_name == "a" => el
                .attributes
                .get("href")
                .filter(|href| !href.trim().is_empty())
                .cloned()
                .or(current_href),
            _ => current_href,
        };

        if let Some(ref href) = href {
            targets.insert(node.id, href.clone());
        }

        for child in &node.children {
            Self::collect_link_targets(child, href.clone(), targets);
        }
    }

    fn hit_test_link(
        layout_box: &LayoutBox,
        x: f32,
        y: f32,
        targets: &HashMap<crate::dom::NodeId, String>,
    ) -> Option<String> {
        for child in layout_box.children.iter().rev() {
            if let Some(url) = Self::hit_test_link(child, x, y, targets) {
                return Some(url);
            }
        }

        let rect = layout_box.dimensions.border_box();
        if rect.contains(x, y) {
            return targets.get(&layout_box.node_id).cloned();
        }

        None
    }

    fn welcome_html(&self) -> String {
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>OxiBrowser</title>
    <style>
        body { font-family: Helvetica, Arial, sans-serif; margin: 36px; background: #f8fafc; color: #1f2937; }
        .container { max-width: 760px; margin: 0 auto; background: white; padding: 28px; border: 1px solid #d7dee8; }
        h1 { margin: 0 0 8px 0; color: #0f766e; font-size: 34px; }
        p { font-size: 16px; line-height: 1.55; color: #334155; }
        a { color: #2563eb; }
        .feature { margin: 12px 0; padding: 12px 14px; border-left: 4px solid #0f766e; background: #f0fdfa; }
        .feature h3 { margin: 0 0 5px 0; color: #115e59; font-size: 17px; }
        code { color: #7c2d12; }
        .shortcut { display: inline-block; background: #f1f3f4; padding: 2px 8px; border-radius: 4px; font-size: 13px; font-family: monospace; }
    </style>
</head>
<body>
    <div class="container">
        <h1>OxiBrowser</h1>
        <p>Rust shell ve native WebView motoru ile Chrome-benzeri web tarayıcısı.</p>
        <div class="feature"><h3>Gezinme</h3><p>URL bar, geri/ileri, yenileme, sekme, zoom, history ve session restore çalışır.</p></div>
        <div class="feature"><h3>Sayfa motoru</h3><p>Modern web, JavaScript, formlar, cookies ve medya desteği Wry native WebView üzerinden gelir.</p></div>
        <div class="feature"><h3>Gizli Mod</h3><p><span class="shortcut">Cmd/Ctrl + Shift + N</span> ile gizli sekme açın. Gizli sekmelerde geçmiş ve şifre kaydedilmez.</p></div>
        <div class="feature"><h3>Chrome araçları</h3><p><a href="about:bookmarks">Yer imleri</a>, <a href="about:downloads">indirilenler</a>, <a href="about:passwords">şifreler</a>, <a href="about:sync">sync</a>, <a href="about:extensions">uzantılar</a> ve <a href="about:devtools">DevTools</a>.</p></div>
        <p><a href="about:version">Sürüm bilgisi</a> · <a href="about:settings">Ayarlar</a></p>
    </div>
</body>
</html>
"#
        .to_string()
    }

    fn history_html(&self) -> String {
        let mut items = String::new();
        for entry in self.profile.data.history.iter().rev().take(200) {
            items.push_str(&format!(
                r#"<li><a href="{url}">{title}</a><small>{url}</small></li>"#,
                title = Self::escape_html(&entry.title),
                url = Self::escape_html(&entry.url)
            ));
        }
        if items.is_empty() {
            items.push_str("<li>Henüz geçmiş yok.</li>");
        }
        self.list_page_html("Geçmiş", "Ziyaret edilen sayfalar", &items)
    }

    fn downloads_html(&self) -> String {
        let mut items = String::new();
        for item in self.profile.data.downloads.iter().rev().take(200) {
            let state = match item.state {
                DownloadState::InProgress => "Sürüyor",
                DownloadState::Completed => "Tamamlandı",
                DownloadState::Failed => "Başarısız",
            };
            let path = item
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Konum bilinmiyor".to_string());
            items.push_str(&format!(
                r#"<li><a href="{url}">{url}</a><small>{state} · {path}</small></li>"#,
                url = Self::escape_html(&item.url),
                state = state,
                path = Self::escape_html(&path)
            ));
        }
        if items.is_empty() {
            items.push_str("<li>Henüz indirme yok.</li>");
        }
        self.list_page_html("İndirilenler", "İndirme kayıtları", &items)
    }

    fn bookmarks_html(&self) -> String {
        let mut items = String::new();
        for bookmark in self.profile.data.bookmarks.iter().rev() {
            items.push_str(&format!(
                r#"<li><a href="{url}">{title}</a><small>{url}</small></li>"#,
                title = Self::escape_html(&bookmark.title),
                url = Self::escape_html(&bookmark.url)
            ));
        }
        if items.is_empty() {
            items.push_str("<li>Henüz yer imi yok.</li>");
        }
        self.list_page_html("Yer İmleri", "Kaydedilen sayfalar", &items)
    }

    fn settings_html(&self) -> String {
        let settings = &self.profile.data.settings;
        let sync_path = settings
            .sync_snapshot_path
            .clone()
            .unwrap_or_else(default_sync_snapshot_path);
        let profile = self.user_profile();
        let avatar_color = format!("#{:06X}", profile.avatar_color);
        let items = format!(
            r#"
            <li><strong>Kullanıcı</strong><small><span style="display:inline-block;width:16px;height:16px;border-radius:50%;background:{avatar_color};vertical-align:middle;margin-right:6px;"></span>{name}</small></li>
            <li><a href="about:profile">Profil ayarları</a><small>İsim ve avatar rengini değiştir</small></li>
            <li><strong>Ana sayfa</strong><small>{home}</small></li>
            <li><strong>Arama motoru</strong><small>{search}</small></li>
            <li><a href="about:search-engines">Arama motorunu değiştir</a><small>Google, Bing, DuckDuckGo, Yahoo</small></li>
            <li><strong>Session restore</strong><small>{restore}</small></li>
            <li><a href="about:bookmarks">Yer imleri</a><small>{bookmark_count} kayıtlı yer imi</small></li>
            <li><a href="about:import-bookmarks">Chrome'dan yer imi içe aktar</a><small>Chrome bookmark JSON dosyasını yükle</small></li>
            <li><a href="about:passwords">Şifre yöneticisi</a><small>{password_count} kayıtlı giriş</small></li>
            <li><a href="about:cookies">Cookie yönetimi</a><small>{cookie_count} kayıtlı cookie</small></li>
            <li><a href="about:sync">Sync snapshot</a><small>{sync_path}</small></li>
            <li><a href="about:extensions">Uzantılar</a><small>Chrome Web Store linki ve yerel uzantı klasörü</small></li>
            <li><a href="about:devtools">DevTools</a><small>Cmd/Ctrl + Alt + I ile açılır</small></li>
            <li><strong>Profil dosyası</strong><small>{profile}</small></li>
            "#,
            name = Self::escape_html(&profile.display_name),
            avatar_color = avatar_color,
            home = Self::escape_html(&settings.home_url),
            search = Self::escape_html(&settings.default_search_url),
            restore = if settings.restore_session {
                "Açık"
            } else {
                "Kapalı"
            },
            bookmark_count = self.profile.data.bookmarks.len(),
            password_count = self.profile.data.passwords.len(),
            cookie_count = self.profile.data.cookies.len(),
            sync_path = Self::escape_html(&sync_path.display().to_string()),
            profile = Self::escape_html(&self.profile.path.display().to_string())
        );
        self.list_page_html("Ayarlar", "Browser ve profil ayarları", &items)
    }

    fn passwords_html(&self) -> String {
        let mut items = String::new();
        for credential in self.profile.data.passwords.iter().rev() {
            let masked = "•".repeat(credential.password.chars().count().clamp(8, 24));
            items.push_str(&format!(
                r#"<li><strong>{origin}</strong><small>{username} · {masked}</small></li>"#,
                origin = Self::escape_html(&credential.origin),
                username = Self::escape_html(&credential.username),
                masked = masked
            ));
        }
        if items.is_empty() {
            items.push_str("<li>Henüz kayıtlı şifre yok. Bir web formunda giriş yaptığında OxiBrowser yerel kasaya kaydeder.</li>");
        }
        items.push_str(
            r#"<li><a href="about:clear-passwords">Tüm kayıtlı şifreleri temizle</a><small>Bu işlem yerel profil JSON kasasını boşaltır.</small></li>"#,
        );
        self.list_page_html(
            "Şifreler",
            "Yerel profil kasasına kaydedilen giriş bilgileri",
            &items,
        )
    }

    fn cookies_html(&self) -> String {
        let mut items = String::new();
        for cookie in self.profile.data.cookies.iter().rev().take(200) {
            let expires = cookie
                .expires_at
                .map(|exp| {
                    if exp == 0 {
                        "Oturum sonunda".to_string()
                    } else {
                        format!("Süre: {}sn", exp.saturating_sub(now_secs()))
                    }
                })
                .unwrap_or("Kalıcı".to_string());
            items.push_str(&format!(
                r#"<li><strong>{name}</strong><small>{domain}{path} · {expires}</small></li>"#,
                name = Self::escape_html(&cookie.name),
                domain = Self::escape_html(&cookie.domain),
                path = Self::escape_html(&cookie.path),
                expires = expires
            ));
        }
        if items.is_empty() {
            items.push_str("<li>Henüz kayıtlı cookie yok. Web siteleri ziyaret edildiğinde cookie'ler yerel olarak saklanır.</li>");
        }
        items.push_str(
            r#"<li><a href="about:clear-cookies">Tüm cookie'leri temizle</a><small>Bu işlem yerel cookie deposunu boşaltır.</small></li>"#,
        );
        self.list_page_html(
            "Cookie'ler",
            "Yerel olarak saklanan cookie bilgileri",
            &items,
        )
    }

    fn sync_html(&self) -> String {
        let path = self
            .profile
            .data
            .settings
            .sync_snapshot_path
            .clone()
            .unwrap_or_else(default_sync_snapshot_path);
        let items = format!(
            r#"
            <li><a href="about:sync-export">Snapshot dışa aktar</a><small>{path}</small></li>
            <li><a href="about:sync-import">Snapshot içe aktar</a><small>Yer imleri, geçmiş, şifreler, uzantı kayıtları ve session verisi birleştirilir.</small></li>
            <li><strong>Durum</strong><small>{status}</small></li>
            "#,
            path = Self::escape_html(&path.display().to_string()),
            status = Self::escape_html(&self.status_message)
        );
        self.list_page_html(
            "Sync",
            "Google hesabı yerine taşınabilir yerel JSON snapshot senkronu",
            &items,
        )
    }

    fn extensions_html(&self) -> String {
        let mut items = String::new();
        items.push_str(&format!(
            r#"<li><a href="{store}">Chrome Web Store'u aç</a><small>Wry WebView Chrome Web Store uzantı kurulumunu macOS'ta doğrudan desteklemez; mağaza sayfası normal web sayfası olarak açılır.</small></li>"#,
            store = Self::escape_html(&self.profile.data.settings.extension_store_url)
        ));
        for extension in &self.profile.data.extensions {
            let state = if extension.enabled {
                "Etkin"
            } else {
                "Kapalı"
            };
            items.push_str(&format!(
                r#"<li><strong>{name}</strong><small>{state} · {path}</small></li>"#,
                name = Self::escape_html(&extension.name),
                state = state,
                path = Self::escape_html(&extension.path.display().to_string())
            ));
        }
        if self.profile.data.extensions.is_empty() {
            items.push_str("<li>Yerel unpacked uzantı kaydı yok.</li>");
        }
        self.list_page_html(
            "Uzantılar",
            "Chrome Web Store erişimi ve WebView'in izin verdiği yerel uzantı yönetimi",
            &items,
        )
    }

    fn devtools_html(&self) -> String {
        let items = r#"
            <li><strong>DevTools kısayolu</strong><small>Cmd/Ctrl + Alt + I aktif sekmenin WebView DevTools penceresini açar.</small></li>
            <li><strong>Motor</strong><small>macOS'ta WKWebView inspector, Windows'ta WebView2 DevTools, Linux'ta WebKitGTK inspector kullanılır.</small></li>
        "#;
        self.list_page_html(
            "DevTools",
            "Native WebView motorunun geliştirici araçları",
            items,
        )
    }

    fn profile_html(&self) -> String {
        let profile = self.user_profile();
        let avatar_color = format!("#{:06X}", profile.avatar_color);
        let colors = [
            ("0x4285F4", "Mavi"), ("0xEA4335", "Kırmızı"), ("0xFBBC04", "Sarı"),
            ("0x34A853", "Yeşil"), ("0x8E44AD", "Mor"), ("0xE67E22", "Turuncu"),
            ("0x1ABC9C", "Turkuaz"), ("0xE91E63", "Pembe"), ("0x607D8B", "Gri"),
        ];
        let mut color_buttons = String::new();
        for (color, name) in &colors {
            let hex = format!("#{}", &color[2..]);
            color_buttons.push_str(&format!(
                r#"<a href="about:set-avatar-{}" style="display:inline-block;width:32px;height:32px;border-radius:50%;background:{};margin:4px;border:2px solid {};" title="{}"></a>"#,
                color, hex, if *color == avatar_color { "#000" } else { "transparent" }, name
            ));
        }
        let items = format!(
            r#"
            <li><strong>Mevcut profil</strong><small><span style="display:inline-block;width:24px;height:24px;border-radius:50%;background:{avatar_color};vertical-align:middle;margin-right:8px;"></span>{name}</small></li>
            <li><strong>Avatar rengi seç</strong><small>{colors}</small></li>
            <li><a href="about:set-name-Kullanıcı">Varsayılan isme dön</a><small>İsmi "Kullanıcı" olarak sıfırla</small></li>
            "#,
            name = Self::escape_html(&profile.display_name),
            avatar_color = avatar_color,
            colors = color_buttons
        );
        self.list_page_html(
            "Profil",
            "Kullanıcı profil ayarları",
            &items,
        )
    }

    fn search_engines_html(&self) -> String {
        let current = &self.profile.data.settings.default_search_url;
        let engines = [
            ("google", "Google", "https://www.google.com"),
            ("bing", "Bing", "https://www.bing.com"),
            ("duckduckgo", "DuckDuckGo", "https://duckduckgo.com"),
            ("yahoo", "Yahoo", "https://search.yahoo.com"),
        ];
        let mut items = String::new();
        for (key, name, url) in &engines {
            let is_active = current.contains(key);
            let badge = if is_active { " ✓ Aktif" } else { "" };
            items.push_str(&format!(
                r#"<li><a href="about:set-search-{}">{}</a><small>{} {}</small></li>"#,
                key, name, url, badge
            ));
        }
        self.list_page_html(
            "Arama Motorları",
            "Varsayılan arama motorunu seçin",
            &items,
        )
    }

    fn import_bookmarks_html(&self) -> String {
        let items = r#"
            <li><strong>Chrome'dan içe aktarma</strong><small>Chrome'da Yer İmleri Yöneticisi'ni açın → Yer imlerini dışa aktar → HTML dosyasını kaydedin.</small></li>
            <li><strong>Desteklenen format</strong><small>Chrome bookmark HTML dosyası (bookmarks_*.html)</small></li>
            <li><a href="about:import-trigger">İçe aktarmayı başlat</a><small>Dosya seçici açılır (macOS Finder)</small></li>
            <li><strong>Not</strong><small>OxiBrowser şu an için yerel JSON sync dosyasından da içe aktarabilir. about:sync-import sayfasını kullanın.</small></li>
        "#;
        self.list_page_html(
            "Chrome'dan İçe Aktar",
            "Chrome yer imlerini OxiBrowser'a aktarın",
            items,
        )
    }

    fn list_page_html(&self, title: &str, subtitle: &str, items: &str) -> String {
        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 0; background: #f8fafc; color: #1f2937; }}
        main {{ max-width: 920px; margin: 0 auto; padding: 36px 28px; }}
        h1 {{ margin: 0 0 6px; color: #111827; font-size: 32px; }}
        p {{ margin: 0 0 22px; color: #64748b; }}
        ul {{ list-style: none; margin: 0; padding: 0; background: white; border: 1px solid #d7dee8; }}
        li {{ padding: 14px 16px; border-bottom: 1px solid #e5e7eb; }}
        li:last-child {{ border-bottom: 0; }}
        a, strong {{ display: block; color: #0f766e; text-decoration: none; font-size: 15px; }}
        small {{ display: block; margin-top: 5px; color: #64748b; word-break: break-all; }}
    </style>
</head>
<body>
    <main>
        <h1>{title}</h1>
        <p>{subtitle}</p>
        <ul>{items}</ul>
    </main>
</body>
</html>
"#,
            title = Self::escape_html(title),
            subtitle = Self::escape_html(subtitle),
            items = items
        )
    }

    fn about_version_html(&self) -> String {
        format!(
            r#"
<html>
<head>
    <title>OxiBrowser Sürüm</title>
    <style>
        body {{ font-family: Helvetica, Arial, sans-serif; margin: 32px; color: #1f2937; }}
        .box {{ max-width: 680px; border: 1px solid #d7dee8; padding: 24px; }}
        h1 {{ margin-top: 0; color: #0f766e; }}
        p {{ line-height: 1.55; }}
    </style>
</head>
<body>
    <div class="box">
        <h1>OxiBrowser {}</h1>
        <p>Rust, winit, softbuffer, tiny-skia shell ve Wry native WebView ile çalışan Chrome-benzeri tarayıcı.</p>
        <p><a href="about:welcome">Başlangıç sayfasına dön</a></p>
    </div>
</body>
</html>
"#,
            env!("CARGO_PKG_VERSION")
        )
    }

    fn error_page_html(&self, title: &str, url: &str, message: &str) -> String {
        format!(
            r#"
<html>
<head>
    <title>{title}</title>
    <style>
        body {{ font-family: Helvetica, Arial, sans-serif; margin: 32px; background: #fff7ed; color: #1f2937; }}
        .box {{ max-width: 760px; background: white; border: 1px solid #fed7aa; padding: 24px; }}
        h1 {{ margin-top: 0; color: #c2410c; }}
        code {{ color: #7c2d12; }}
    </style>
</head>
<body>
    <div class="box">
        <h1>{title}</h1>
        <p><code>{url}</code></p>
        <p>{message}</p>
    </div>
</body>
</html>
"#,
            title = Self::escape_html(title),
            url = Self::escape_html(url),
            message = Self::escape_html(message)
        )
    }

    fn escape_html(text: &str) -> String {
        let mut escaped = String::with_capacity(text.len());
        for ch in text.chars() {
            match ch {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&#39;"),
                _ => escaped.push(ch),
            }
        }
        escaped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_navigation_updates_history() {
        let mut browser = Browser::new();
        browser.new_tab("");

        browser.load_url_sync("about:welcome");
        browser.load_url_sync("about:version");

        assert_eq!(browser.current_url, "about:version");
        assert!(browser.go_back());
        assert_eq!(browser.current_url, "about:welcome");
        assert!(browser.go_forward());
        assert_eq!(browser.current_url, "about:version");
    }

    #[test]
    fn link_hit_testing_resolves_relative_href() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            r#"<html><body><p><a href="/docs">Docs</a></p></body></html>"#,
            "https://example.com/start",
        );

        let mut href = None;
        for y in 0..120 {
            for x in 0..220 {
                href = browser.link_at(x as f32, y as f32);
                if href.is_some() {
                    break;
                }
            }
            if href.is_some() {
                break;
            }
        }

        assert_eq!(href.as_deref(), Some("https://example.com/docs"));
    }

    #[test]
    fn stylesheet_is_reset_between_pages() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            "<html><head><style>body { background: red; }</style></head><body>One</body></html>",
            "about:first",
        );
        assert!(!browser.stylesheet.rules.is_empty());

        browser.parse_and_render("<html><body>Two</body></html>", "about:second");
        assert!(browser.stylesheet.rules.is_empty());
    }

    #[test]
    fn canvas_background_prefers_body_over_html() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            r#"
            <html>
                <head><style>html { background: #010203; } body { background: #f8fafc; }</style></head>
                <body>Welcome</body>
            </html>
            "#,
            "about:test",
        );

        assert_eq!(
            browser.canvas_background_color(),
            Color::new(248, 250, 252, 255)
        );
    }

    #[test]
    fn canvas_background_uses_html_when_body_is_transparent() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            r#"
            <html>
                <head><style>html { background: #0a141e; } body { background: transparent; }</style></head>
                <body>Welcome</body>
            </html>
            "#,
            "about:test",
        );

        assert_eq!(
            browser.canvas_background_color(),
            Color::new(10, 20, 30, 255)
        );
    }

    #[test]
    fn canvas_background_falls_back_to_white() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render("<html><body>Welcome</body></html>", "about:test");

        assert_eq!(browser.canvas_background_color(), Color::WHITE);
    }

    #[test]
    fn relayout_clamps_scroll_offset_to_new_viewport() {
        let mut browser = Browser::new();
        browser.new_tab("");
        browser.parse_and_render(
            r#"<html><body style="height: 1000px;">Tall page</body></html>"#,
            "about:test",
        );

        browser.relayout(400.0, 200.0);
        browser.scroll_by(900.0);
        assert!(browser.scroll_offset > 0.0);

        browser.relayout(400.0, 2000.0);
        assert_eq!(browser.scroll_offset, 0.0);
    }

    #[test]
    fn zoom_presets_step_like_chrome() {
        let mut browser = Browser::new();
        browser.new_tab("about:welcome");

        assert_eq!(browser.zoom_in(), 110);
        assert_eq!(browser.zoom_out(), 100);
        assert_eq!(browser.set_active_zoom(126), 125);
        assert_eq!(browser.reset_zoom(), DEFAULT_ZOOM_PERCENT);
    }

    #[test]
    fn bookmark_toggle_dedupes_by_url() {
        let mut browser = Browser::new();
        browser.profile = temp_profile_store("bookmark_toggle");
        browser.new_tab("https://example.com");

        assert!(browser.toggle_bookmark_active());
        assert_eq!(browser.profile.data.bookmarks.len(), 1);
        assert!(!browser.toggle_bookmark_active());
        assert!(browser.profile.data.bookmarks.is_empty());
    }

    #[test]
    fn profile_store_loads_saved_json_and_falls_back_on_bad_json() {
        let path = temp_profile_path("profile_store");
        let store = ProfileStore {
            path: path.clone(),
            data: ProfileData {
                bookmarks: vec![Bookmark {
                    title: "Example".to_string(),
                    url: "https://example.com".to_string(),
                    created_at: 1,
                }],
                ..ProfileData::default()
            },
        };
        store.save().unwrap();

        let loaded = ProfileStore::load_from_path(path.clone());
        assert_eq!(loaded.data.bookmarks.len(), 1);

        std::fs::write(&path, "{not json").unwrap();
        let fallback = ProfileStore::load_from_path(path);
        assert!(fallback.data.bookmarks.is_empty());
    }

    #[test]
    fn restore_session_rehydrates_tabs_and_active_index() {
        let mut browser = Browser::new();
        browser.profile = temp_profile_store("session_restore");
        browser.profile.data.session_tabs = vec![
            test_tab(7, "https://one.example"),
            test_tab(8, "https://two.example"),
        ];
        browser.profile.data.active_tab = 1;

        browser.restore_session_or_welcome(None);

        assert_eq!(browser.tabs.len(), 2);
        assert_eq!(browser.active_tab_id(), Some(8));
        assert_eq!(browser.current_url, "https://two.example");
        assert_eq!(browser.next_tab_id, 9);
    }

    #[test]
    fn password_manager_dedupes_by_origin_and_username() {
        let mut browser = Browser::new();
        browser.profile = temp_profile_store("password_manager");

        assert!(browser.save_password_for_url(
            "https://example.com/login",
            "kutay@example.com",
            "one"
        ));
        assert!(browser.save_password_for_url(
            "https://example.com/account",
            "kutay@example.com",
            "two"
        ));

        let credentials = browser.credentials_for_url("https://example.com/dashboard");
        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].origin, "https://example.com");
        assert_eq!(credentials[0].password, "two");
        assert!(browser
            .credentials_for_url("https://other.example")
            .is_empty());
    }

    #[test]
    fn sync_snapshot_round_trips_profile_data() {
        let sync_path = temp_profile_path("sync_snapshot");
        let mut source = Browser::new();
        source.profile = temp_profile_store("sync_source");
        source.profile.data.settings.sync_snapshot_path = Some(sync_path.clone());
        source.profile.data.bookmarks.push(Bookmark {
            title: "Example".to_string(),
            url: "https://example.com".to_string(),
            created_at: 1,
        });
        source.save_password_for_url("https://example.com/login", "user", "secret");

        source.export_sync_snapshot().unwrap();

        let mut target = Browser::new();
        target.profile = temp_profile_store("sync_target");
        target.profile.data.settings.sync_snapshot_path = Some(sync_path);
        target.import_sync_snapshot().unwrap();

        assert_eq!(target.profile.data.bookmarks.len(), 1);
        assert_eq!(target.credentials_for_url("https://example.com").len(), 1);
    }

    #[test]
    fn chrome_tools_about_pages_are_available() {
        let browser = Browser::new();

        for url in [
            "about:passwords",
            "about:sync",
            "about:extensions",
            "about:devtools",
        ] {
            match browser.navigation_target_for_url(url) {
                NavigationTarget::Html { html, .. } => {
                    assert!(html.contains("<html>") || html.contains("<!DOCTYPE html>"));
                }
                NavigationTarget::Url(_) => panic!("{url} should be an internal page"),
            }
        }
    }

    fn temp_profile_store(name: &str) -> ProfileStore {
        ProfileStore {
            path: temp_profile_path(name),
            data: ProfileData::default(),
        }
    }

    fn temp_profile_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "oxibrowser-{name}-{}-profile.json",
            std::process::id()
        ))
    }

    fn test_tab(id: u32, url: &str) -> Tab {
        Tab {
            id,
            title: url.to_string(),
            url: url.to_string(),
            history: vec![url.to_string()],
            history_pos: 0,
            zoom_percent: DEFAULT_ZOOM_PERCENT,
            loading: false,
            incognito: false,
        }
    }
}
