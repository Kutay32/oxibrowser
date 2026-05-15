# OxiBrowser

Rust shell ve native WebView motoru ile geliştirilmiş Chrome-benzeri web tarayıcısı.

## Özellikler

### Temel Gezinme
- **URL Bar** - Arama ve URL girişi (Google varsayılan arama motoru)
- **Geri / İleri / Yenile** - Tam navigasyon desteği
- **Sekme Yönetimi** - Çoklu sekme, sekme değiştirme, sekme kapatma
- **Zoom** - Chrome benzeri zoom seviyeleri (25% - 500%)
- **Session Restore** - Kapatılan sekmeler geri yüklenir

### Chrome Araçları
- **Yer İmleri** - Sayfaları kaydet ve yönet (`Cmd/Ctrl + D`)
- **Geçmiş** - Ziyaret edilen sayfalar (`Cmd/Ctrl + Y`)
- **İndirilenler** - İndirme takibi (`Cmd/Ctrl + J`)
- **Şifre Yöneticisi** - Form girişi otomatik kaydetme ve otomatik doldurma
- **Cookie Yönetimi** - Cookie görüntüleme ve temizleme
- **Sync** - Yerel JSON snapshot ile profil senkronizasyonu
- **DevTools** - Native WebView geliştirici araçları (`Cmd/Ctrl + Alt + I`)

### Gizli Mod
- **Gizli Sekme** - Geçmiş ve şifre kaydedilmez (`Cmd/Ctrl + Shift + N`)
- Gizli sekmelerde otomatik cookie temizleme

### Sayfa Motoru
- **Wry Native WebView** - macOS'ta WKWebView, Windows'ta WebView2, Linux'ta WebKitGTK
- **Tam JavaScript Desteği** - WebView üzerinden
- **Modern Web** - CSS, formlar, medya, canvas desteği
- **Özel HTML/CSS Parser** - `about:` sayfaları için dahili render pipeline

### Profil Yönetimi
- **JSON Profil** - Platform-veri dizininde saklanır
- **Session Persistence** - Sekmeler ve zoom seviyeleri kaydedilir
- **Sync Snapshot** - Dışa/içe aktarma ile profil taşıma

## Kurulum

### Gereksinimler
- **Rust** 1.70+ (Edition 2021)
- **macOS** 10.15+ / **Windows** 10+ / **Linux** (GTK/WebKit)

### Derleme

```bash
# Debug derleme
cargo build

# Release derleme (optimizasyonlu)
cargo build --release

# Çalıştırma
cargo run
cargo run --release

# Komut satırından URL ile başlatma
cargo run -- https://example.com
```

## Klavye Kısayolları

| Kısayol | Açıklama |
|---------|----------|
| `Cmd/Ctrl + T` | Yeni sekme |
| `Cmd/Ctrl + W` | Sekmeyi kapat |
| `Cmd/Ctrl + R` / `F5` | Sayfayı yenile |
| `Cmd/Ctrl + L` | URL bar'a odaklan |
| `Cmd/Ctrl + D` | Yer imi ekle/kaldır |
| `Cmd/Ctrl + B` | Yer imleri sayfası |
| `Cmd/Ctrl + Y` | Geçmiş sayfası |
| `Cmd/Ctrl + J` | İndirilenler sayfası |
| `Cmd/Ctrl + ,` | Ayarlar sayfası |
| `Cmd/Ctrl + Shift + P` | Şifreler sayfası |
| `Cmd/Ctrl + Shift + S` | Sync sayfası |
| `Cmd/Ctrl + Shift + N` | Gizli sekme |
| `Cmd/Ctrl + Shift + E` | Uzantılar sayfası |
| `Cmd/Ctrl + Alt + I` | DevTools aç |
| `Cmd/Ctrl + Alt + Delete` | Tüm tarama verilerini temizle |
| `Cmd/Ctrl + +` | Yakınlaştır |
| `Cmd/Ctrl + -` | Uzaklaştır |
| `Cmd/Ctrl + 0` | Zoom sıfırla |
| `Cmd/Ctrl + F` | Sayfada bul |
| `Tab` | Sonraki sekmeye geç |
| `Page Up/Down` | Sayfa kaydır |
| `Home/End` | En üst/alt |
| `Arrow Up/Down` | Satır kaydır |
| `Cmd/Ctrl + ←/→` | Geri/İleri |
| `Escape` | URL bar'dan çık / Uygulamayı kapat |

## İç Sayfalar

| URL | Açıklama |
|-----|----------|
| `about:welcome` | Karşılama sayfası |
| `about:blank` | Boş sayfa |
| `about:version` | Sürüm bilgisi |
| `about:history` | Tarama geçmişi |
| `about:downloads` | İndirilenler |
| `about:bookmarks` | Yer imleri |
| `about:settings` | Ayarlar |
| `about:passwords` | Kayıtlı şifreler |
| `about:cookies` | Cookie yönetimi |
| `about:sync` | Senkronizasyon |
| `about:extensions` | Uzantılar |
| `about:devtools` | Geliştirici araçları bilgisi |

## Mimari

```
┌─────────────────────────────────────────────────────┐
│                    OxiBrowser                        │
├─────────────────────────────────────────────────────┤
│  Browser Shell (winit + softbuffer + tiny-skia)     │
│  ┌─────────────────────────────────────────────┐   │
│  │  Tab Strip  │  Toolbar  │  URL Bar  │ Menu  │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  ┌─────────────────────────────────────────────┐   │
│  │                                             │   │
│  │          WebView (Wry)                       │   │
│  │    WKWebView / WebView2 / WebKitGTK          │   │
│  │                                             │   │
│  └─────────────────────────────────────────────┘   │
│                                                     │
│  Özel Pipeline (about: sayfaları için):             │
│  HTML Parser → CSS Parser → Style → Layout → Render │
│                                                     │
├─────────────────────────────────────────────────────┤
│  Profil: JSON (bookmarks, history, passwords, etc.) │
└─────────────────────────────────────────────────────┘
```

### Teknoloji Yığını

| Katman | Teknoloji |
|--------|-----------|
| Dil | Rust (Edition 2021) |
| Pencere | winit 0.30 |
| Pixel Buffer | softbuffer 0.4 |
| Browser Chrome UI | tiny-skia 0.11 |
| Web İçerik | Wry 0.55 (native WebView) |
| HTTP | reqwest 0.12 + tokio 1 |
| HTML/CSS Parser | Özel (hand-written) |
| Font | ab_glyph 0.2 + fontdb 0.16 |
| Serileştirme | serde + serde_json |

## Proje Yapısı

```
src/
├── main.rs          # Giriş noktası
├── browser.rs       # Browser durumu, sekmeler, profil, navigasyon
├── ui.rs            # winit event loop, pencere yönetimi, chrome UI
├── render.rs        # tiny-skia render engine, text rendering
├── layout.rs        # Box model layout engine
├── style.rs         # CSS style computation, inheritance
├── css.rs           # CSS parser, selectors, specificity
├── html.rs          # HTML parser
├── dom.rs           # DOM tree veri yapıları
├── types.rs         # Paylaşılan tipler: Color, Point, Size, Rect
├── net.rs           # HTTP networking, URL normalization
├── url_bar.rs       # URL parsing utilities
└── js.rs            # JS runtime placeholder
```

## Roadmap

- [ ] Cookie otomatik yönetimi (WebView ile entegrasyon)
- [ ] Tam incognito window desteği
- [ ] Context menu (sağ tık menüsü)
- [ ] Drag & drop ile sekme yeniden düzenleme
- [ ] Bookmark bar
- [ ] Find bar ile sayfada highlight
- [ ] Reklam engelleme
- [ ] Dark mode
- [ ] Extension API desteği

## Lisans

MIT

## Katkıda Bulunma

1. Fork edin
2. Feature branch oluşturun (`git checkout -b feature/yeni-ozellik`)
3. Değişikliklerinizi commit edin (`git commit -m 'Yeni özellik eklendi'`)
4. Branch'inizi push edin (`git push origin feature/yeni-ozellik`)
5. Pull Request açın
