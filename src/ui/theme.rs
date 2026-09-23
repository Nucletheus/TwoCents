use eframe::egui::{self, Color32, Stroke, Visuals};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariantMode {
    Dark,
    Light,
    System,
}

impl VariantMode {
    /// stable storage key for app_settings persistence.
    pub fn as_key(&self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::System => "system",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "dark" => Self::Dark,
            "light" => Self::Light,
            _ => Self::System,
        }
    }
}

pub fn system_is_dark() -> bool {
    // Cache the result since the OS theme rarely changes mid-session
    use std::sync::OnceLock;
    static IS_DARK: OnceLock<bool> = OnceLock::new();
    *IS_DARK.get_or_init(|| {
        // Try Windows registry
        let out = std::process::Command::new("reg")
            .args([
                "query",
                "HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output()
            .ok();
        if let Some(output) = out {
            let s = String::from_utf8_lossy(&output.stdout);
            // "0x1" = light mode, "0x0" = dark mode
            if s.contains("0x0") {
                return true;
            }
            if s.contains("0x1") {
                return false;
            }
        }
        true // default to dark if detection fails
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    OneDark,
    Nord,
    Dracula,
    Catppuccin,
    CatppuccinMacchiato,
    GitHub,
    Solarized,
    Tokyonight,
    Everforest,
    Gruvbox,
    Kanagawa,
    Ayu,
    Matrix,
}

impl ThemePreset {
    pub fn all() -> &'static [(Self, &'static str)] {
        &[
            (Self::OneDark, "One Dark"),
            (Self::Nord, "Nord"),
            (Self::Dracula, "Dracula"),
            (Self::Catppuccin, "Catppuccin"),
            (Self::CatppuccinMacchiato, "Catppuccin Macchiato"),
            (Self::GitHub, "GitHub"),
            (Self::Solarized, "Solarized"),
            (Self::Tokyonight, "Tokyonight"),
            (Self::Everforest, "Everforest"),
            (Self::Gruvbox, "Gruvbox"),
            (Self::Kanagawa, "Kanagawa"),
            (Self::Ayu, "Ayu"),
            (Self::Matrix, "Matrix"),
        ]
    }

    /// Display name — also the stable app_settings storage key.
    pub fn name(&self) -> &'static str {
        Self::all()
            .iter()
            .find(|(t, _)| t == self)
            .map(|(_, n)| *n)
            .unwrap_or("One Dark")
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(t, _)| *t)
    }
}

#[derive(Clone)]
struct ThemeColors {
    panel_fill: Color32,
    window_fill: Color32,
    extreme_bg: Color32,
    faint_bg: Color32,
    hovered_bg: Color32,
    text_primary: Color32,
    border: Color32,
    accent: Color32,
    success: Color32,
    warning: Color32,
    error: Color32,
}

const fn hex(c: u32) -> Color32 {
    Color32::from_rgb(
        ((c >> 16) & 0xFF) as u8,
        ((c >> 8) & 0xFF) as u8,
        (c & 0xFF) as u8,
    )
}

struct ThemePair {
    dark: ThemeColors,
    light: ThemeColors,
}

// --- One Dark ---
fn one_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x0e0f11),
        window_fill: hex(0x1a1c20),
        extreme_bg: hex(0x0e0f11),
        faint_bg: hex(0x1a1c20),
        hovered_bg: hex(0x2c2e33),
        text_primary: hex(0xf3f4f6),
        border: hex(0x2c2e33),
        accent: hex(0x3b82f6),
        success: hex(0x98c379),
        warning: hex(0xd19a66),
        error: hex(0xe06c75),
    }
}
fn one_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xf3f4f6),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xf3f4f6),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xe5e7eb),
        text_primary: hex(0x111827),
        border: hex(0xd1d5db),
        accent: hex(0x2563eb),
        success: hex(0x16a34a),
        warning: hex(0xd97706),
        error: hex(0xdc2626),
    }
}

// --- Nord ---
fn nord_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x2e3440),
        window_fill: hex(0x3b4252),
        extreme_bg: hex(0x2e3440),
        faint_bg: hex(0x3b4252),
        hovered_bg: hex(0x434c5e),
        text_primary: hex(0xeceff4),
        border: hex(0x4c566a),
        accent: hex(0x88c0d0),
        success: hex(0xa3be8c),
        warning: hex(0xebcb8b),
        error: hex(0xbf616a),
    }
}
fn nord_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xeceff4),
        window_fill: hex(0xf5f7fa),
        extreme_bg: hex(0xeceff4),
        faint_bg: hex(0xf5f7fa),
        hovered_bg: hex(0xd8dee9),
        text_primary: hex(0x2e3440),
        border: hex(0xd8dee9),
        accent: hex(0x5e81ac),
        success: hex(0x8fbcbb),
        warning: hex(0xd08770),
        error: hex(0xbf616a),
    }
}

// --- Dracula ---
fn dracula_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x1e1f29),
        window_fill: hex(0x282a36),
        extreme_bg: hex(0x1e1f29),
        faint_bg: hex(0x282a36),
        hovered_bg: hex(0x3a3c4e),
        text_primary: hex(0xf8f8f2),
        border: hex(0x3a3c4e),
        accent: hex(0xbd93f9),
        success: hex(0x50fa7b),
        warning: hex(0xf1fa8c),
        error: hex(0xff5555),
    }
}
fn dracula_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xfaf8f5),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xfaf8f5),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xf0ece6),
        text_primary: hex(0x282a36),
        border: hex(0xe0dcd6),
        accent: hex(0xbd93f9),
        success: hex(0x50fa7b),
        warning: hex(0xf1fa8c),
        error: hex(0xff5555),
    }
}

// --- Catppuccin Mocha ---
fn catppuccin_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x11111b),
        window_fill: hex(0x1e1e2e),
        extreme_bg: hex(0x11111b),
        faint_bg: hex(0x1e1e2e),
        hovered_bg: hex(0x313244),
        text_primary: hex(0xcdd6f4),
        border: hex(0x313244),
        accent: hex(0x89b4fa),
        success: hex(0xa6e3a1),
        warning: hex(0xf9e2af),
        error: hex(0xf38ba8),
    }
}
fn catppuccin_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xeff1f5),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xeff1f5),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xccd0da),
        text_primary: hex(0x1e1e2e),
        border: hex(0xccd0da),
        accent: hex(0x1e66f5),
        success: hex(0x40a02b),
        warning: hex(0xdf8e1d),
        error: hex(0xd20f39),
    }
}

// --- Catppuccin Macchiato ---
fn macchiato_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x181926),
        window_fill: hex(0x24273a),
        extreme_bg: hex(0x181926),
        faint_bg: hex(0x24273a),
        hovered_bg: hex(0x363a4f),
        text_primary: hex(0xcad3f5),
        border: hex(0x363a4f),
        accent: hex(0x8aadf4),
        success: hex(0xa6da95),
        warning: hex(0xeed49f),
        error: hex(0xed8796),
    }
}
fn macchiato_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xeff1f5),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xeff1f5),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xccd0da),
        text_primary: hex(0x24273a),
        border: hex(0xccd0da),
        accent: hex(0x8aadf4),
        success: hex(0x40a02b),
        warning: hex(0xdf8e1d),
        error: hex(0xd20f39),
    }
}

// --- GitHub ---
fn github_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x0d1117),
        window_fill: hex(0x161b22),
        extreme_bg: hex(0x0d1117),
        faint_bg: hex(0x161b22),
        hovered_bg: hex(0x21262d),
        text_primary: hex(0xe6edf3),
        border: hex(0x30363d),
        accent: hex(0x58a6ff),
        success: hex(0x2da44e),
        warning: hex(0xbf8700),
        error: hex(0xcf222e),
    }
}
fn github_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xf6f8fa),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xf6f8fa),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xeaeef2),
        text_primary: hex(0x1f2328),
        border: hex(0xd0d7de),
        accent: hex(0x0969da),
        success: hex(0x1a7f37),
        warning: hex(0x9a6700),
        error: hex(0xcf222e),
    }
}

// --- Solarized ---
fn solarized_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x002b36),
        window_fill: hex(0x073642),
        extreme_bg: hex(0x002b36),
        faint_bg: hex(0x073642),
        hovered_bg: hex(0x184a54),
        text_primary: hex(0x93a1a1),
        border: hex(0x184a54),
        accent: hex(0x268bd2),
        success: hex(0x859900),
        warning: hex(0xb58900),
        error: hex(0xdc322f),
    }
}
fn solarized_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xfdf6e3),
        window_fill: hex(0xfffce5),
        extreme_bg: hex(0xfdf6e3),
        faint_bg: hex(0xfffce5),
        hovered_bg: hex(0xe9e2c7),
        text_primary: hex(0x657b83),
        border: hex(0xd4cfa6),
        accent: hex(0x268bd2),
        success: hex(0x859900),
        warning: hex(0xb58900),
        error: hex(0xdc322f),
    }
}

// --- Tokyonight ---
fn tokyonight_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x1a1b26),
        window_fill: hex(0x24283b),
        extreme_bg: hex(0x1a1b26),
        faint_bg: hex(0x24283b),
        hovered_bg: hex(0x33467a),
        text_primary: hex(0xc0caf5),
        border: hex(0x33467a),
        accent: hex(0x7aa2f7),
        success: hex(0x9ece6a),
        warning: hex(0xe0af68),
        error: hex(0xf7768e),
    }
}
fn tokyonight_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xe1e2e7),
        window_fill: hex(0xf5f6fa),
        extreme_bg: hex(0xe1e2e7),
        faint_bg: hex(0xf5f6fa),
        hovered_bg: hex(0xc9cbd1),
        text_primary: hex(0x1a1b26),
        border: hex(0xc9cbd1),
        accent: hex(0x565f89),
        success: hex(0x9ece6a),
        warning: hex(0xe0af68),
        error: hex(0xf7768e),
    }
}

// --- Everforest ---
fn everforest_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x2b3339),
        window_fill: hex(0x343f44),
        extreme_bg: hex(0x2b3339),
        faint_bg: hex(0x343f44),
        hovered_bg: hex(0x47565c),
        text_primary: hex(0xd3c6aa),
        border: hex(0x47565c),
        accent: hex(0xa7c080),
        success: hex(0xa7c080),
        warning: hex(0xdbbc7f),
        error: hex(0xe67e80),
    }
}
fn everforest_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xfff9e8),
        window_fill: hex(0xfef6e3),
        extreme_bg: hex(0xfff9e8),
        faint_bg: hex(0xfef6e3),
        hovered_bg: hex(0xe8dfc6),
        text_primary: hex(0x5c6a72),
        border: hex(0xe8dfc6),
        accent: hex(0x83c092),
        success: hex(0x83c092),
        warning: hex(0xe6985e),
        error: hex(0xe67e80),
    }
}

// --- Gruvbox ---
fn gruvbox_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x282828),
        window_fill: hex(0x32302f),
        extreme_bg: hex(0x282828),
        faint_bg: hex(0x32302f),
        hovered_bg: hex(0x504945),
        text_primary: hex(0xebdbb2),
        border: hex(0x504945),
        accent: hex(0xb8bb26),
        success: hex(0x98971a),
        warning: hex(0xd79921),
        error: hex(0xcc241d),
    }
}
fn gruvbox_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xfbf1c7),
        window_fill: hex(0xf9f5d7),
        extreme_bg: hex(0xfbf1c7),
        faint_bg: hex(0xf9f5d7),
        hovered_bg: hex(0xebdbb2),
        text_primary: hex(0x3c3836),
        border: hex(0xebdbb2),
        accent: hex(0x98971a),
        success: hex(0x79740e),
        warning: hex(0xb57614),
        error: hex(0x9d0006),
    }
}

// --- Kanagawa ---
fn kanagawa_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x1f1f28),
        window_fill: hex(0x2a2a37),
        extreme_bg: hex(0x1f1f28),
        faint_bg: hex(0x2a2a37),
        hovered_bg: hex(0x363646),
        text_primary: hex(0xdcd7ba),
        border: hex(0x363646),
        accent: hex(0x7e9cd8),
        success: hex(0x76946a),
        warning: hex(0xc0a36e),
        error: hex(0xc34043),
    }
}
fn kanagawa_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xf2ecbc),
        window_fill: hex(0xfaf3d1),
        extreme_bg: hex(0xf2ecbc),
        faint_bg: hex(0xfaf3d1),
        hovered_bg: hex(0xe4d9a0),
        text_primary: hex(0x2d2b2a),
        border: hex(0xe4d9a0),
        accent: hex(0x2d4f67),
        success: hex(0x76946a),
        warning: hex(0xc0a36e),
        error: hex(0xc34043),
    }
}

// --- Ayu ---
fn ayu_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x0b0e14),
        window_fill: hex(0x131721),
        extreme_bg: hex(0x0b0e14),
        faint_bg: hex(0x131721),
        hovered_bg: hex(0x1f2430),
        text_primary: hex(0xbfc8d5),
        border: hex(0x1f2430),
        accent: hex(0x39bae6),
        success: hex(0x7fd962),
        warning: hex(0xffd580),
        error: hex(0xf07178),
    }
}
fn ayu_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xfafafa),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xfafafa),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xeaeaea),
        text_primary: hex(0x1a1f29),
        border: hex(0xeaeaea),
        accent: hex(0x399ee6),
        success: hex(0x6bb75b),
        warning: hex(0xe6b450),
        error: hex(0xef6b68),
    }
}

// --- Matrix ---
fn matrix_dark() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0x000000),
        window_fill: hex(0x0a0a0a),
        extreme_bg: hex(0x000000),
        faint_bg: hex(0x0a0a0a),
        hovered_bg: hex(0x001a00),
        text_primary: hex(0x00ff41),
        border: hex(0x003b00),
        accent: hex(0x00ff41),
        success: hex(0x00ff41),
        warning: hex(0xffff00),
        error: hex(0xff0000),
    }
}
fn matrix_light() -> ThemeColors {
    ThemeColors {
        panel_fill: hex(0xf0fff0),
        window_fill: hex(0xffffff),
        extreme_bg: hex(0xf0fff0),
        faint_bg: hex(0xffffff),
        hovered_bg: hex(0xd0ffd0),
        text_primary: hex(0x003b00),
        border: hex(0x00aa41),
        accent: hex(0x00cc44),
        success: hex(0x00cc44),
        warning: hex(0xaaaa00),
        error: hex(0xcc0000),
    }
}

fn theme_pair(t: ThemePreset) -> ThemePair {
    match t {
        ThemePreset::OneDark => ThemePair {
            dark: one_dark(),
            light: one_light(),
        },
        ThemePreset::Nord => ThemePair {
            dark: nord_dark(),
            light: nord_light(),
        },
        ThemePreset::Dracula => ThemePair {
            dark: dracula_dark(),
            light: dracula_light(),
        },
        ThemePreset::Catppuccin => ThemePair {
            dark: catppuccin_dark(),
            light: catppuccin_light(),
        },
        ThemePreset::CatppuccinMacchiato => ThemePair {
            dark: macchiato_dark(),
            light: macchiato_light(),
        },
        ThemePreset::GitHub => ThemePair {
            dark: github_dark(),
            light: github_light(),
        },
        ThemePreset::Solarized => ThemePair {
            dark: solarized_dark(),
            light: solarized_light(),
        },
        ThemePreset::Tokyonight => ThemePair {
            dark: tokyonight_dark(),
            light: tokyonight_light(),
        },
        ThemePreset::Everforest => ThemePair {
            dark: everforest_dark(),
            light: everforest_light(),
        },
        ThemePreset::Gruvbox => ThemePair {
            dark: gruvbox_dark(),
            light: gruvbox_light(),
        },
        ThemePreset::Kanagawa => ThemePair {
            dark: kanagawa_dark(),
            light: kanagawa_light(),
        },
        ThemePreset::Ayu => ThemePair {
            dark: ayu_dark(),
            light: ayu_light(),
        },
        ThemePreset::Matrix => ThemePair {
            dark: matrix_dark(),
            light: matrix_light(),
        },
    }
}

// ---- The palette (the one color source) ------------------------------------
//
// The only place the app gets colors. Raw preset fields feed a
// derive() step that produces every token the UI is allowed to touch:
// primary/secondary text, primary/secondary background, hover/active
// surfaces, border, accent, and the three status colors. No other module
// may pick a color: no `ui.visuals()` reads, no fallback hex, no literals.
// (Alpha/lerp variants of a palette token at the point of use are fine,
// they are still derived from the palette.)

/// The full token set, derived once per theme change from a preset's raw
/// fields. `pub` fields for chrome builders that need several at once.
#[derive(Clone, Copy)]
pub struct Palette {
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_faint: Color32,
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_hover: Color32,
    pub bg_active: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub selection_fg: Color32,
    pub is_dark: bool,
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8;
    let g = (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8;
    let bl = (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8;
    Color32::from_rgb(r, g, bl)
}

/// THE derivation — raw preset fields in, every UI token out.
/// Secondary text/bg tokens are mix-derived here (the 13 presets only
/// provide one text + two surfaces each); this is the single documented
/// tradeoff, and it lives here and nowhere else.
fn derive(c: &ThemeColors, is_dark: bool) -> Palette {
    let toward = if is_dark {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    Palette {
        text_primary: c.text_primary,
        text_secondary: mix(c.text_primary, toward, 0.35),
        text_faint: mix(c.text_primary, toward, 0.55),
        bg_primary: c.panel_fill,
        bg_secondary: c.window_fill,
        bg_hover: c.hovered_bg,
        bg_active: if is_dark {
            c.hovered_bg
        } else {
            mix(c.hovered_bg, c.accent, 0.12)
        },
        border: c.border,
        border_strong: mix(c.border, c.accent, 0.35),
        accent: c.accent,
        accent_hover: mix(c.accent, toward, 0.10),
        success: c.success,
        warning: c.warning,
        error: c.error,
        selection_fg: contrast_text(c.accent),
        is_dark,
    }
}

/// one global, replaced wholesale by `configure_theme` on theme
/// change (its LAST_APPLIED gate makes that cheap). First access before the
/// first frame derives the default preset through the same derive() — never
/// a scattered fallback hex.
static PALETTE: Mutex<Option<Palette>> = Mutex::new(None);

pub fn palette() -> Palette {
    let mut guard = PALETTE.lock().expect("theme palette mutex poisoned");
    if guard.is_none() {
        *guard = Some(derive(&theme_pair(ThemePreset::OneDark).dark, true));
    }
    *guard.as_ref().unwrap()
}

/// Single-field accessors — the API every UI feature calls.
pub fn fg_primary() -> Color32 {
    palette().text_primary
}
pub fn fg_secondary() -> Color32 {
    palette().text_secondary
}
pub fn fg_faint() -> Color32 {
    palette().text_faint
}
pub fn bg_primary() -> Color32 {
    palette().bg_primary
}
pub fn bg_secondary() -> Color32 {
    palette().bg_secondary
}
pub fn bg_hover() -> Color32 {
    palette().bg_hover
}
pub fn bg_active() -> Color32 {
    palette().bg_active
}
pub fn border() -> Color32 {
    palette().border
}
pub fn border_strong() -> Color32 {
    palette().border_strong
}
pub fn accent() -> Color32 {
    palette().accent
}
pub fn success() -> Color32 {
    palette().success
}
pub fn warning() -> Color32 {
    palette().warning
}
pub fn error() -> Color32 {
    palette().error
}

pub fn active_is_dark() -> bool {
    palette().is_dark
}

/// Text-on-fill contrast helper: picks black or white text for legibility
/// on a given background. All swatch, picker, and tint labels and
/// accent-filled buttons route through this so legibility can't diverge
/// per call site. Uses WCAG-style sRGB relative luminance (gamma-linearized
/// channels) with a 0.25 threshold; the previous rec.601/120 formula scored
/// mid-dark accents like One Dark's blue #3b82f6 at 121.9 and picked black
/// text on accent-blue buttons. Linearized luminance separates cleanly:
/// blue-500 (≈0.24) gets white text; pale accents, yellows, oranges, and
/// teals (≥0.37) get black.
pub fn contrast_text(bg: Color32) -> Color32 {
    let lin = |c: u8| -> f32 {
        let v = c as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let lum = 0.2126 * lin(bg.r()) + 0.7152 * lin(bg.g()) + 0.0722 * lin(bg.b());
    if lum > 0.25 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

fn apply_visuals(visuals: &mut Visuals, p: &Palette) {
    visuals.panel_fill = p.bg_primary;
    visuals.window_fill = p.bg_secondary;
    // All 26 preset definitions satisfy extreme_bg == panel_fill and
    // faint_bg == window_fill, so these two map onto the primary/secondary
    // backgrounds with zero visual change.
    visuals.extreme_bg_color = p.bg_primary;
    visuals.faint_bg_color = p.bg_secondary;
    visuals.text_edit_bg_color = Some(p.bg_primary);
    visuals.widgets.noninteractive.bg_fill = p.bg_secondary;
    visuals.widgets.inactive.bg_fill = p.bg_secondary;
    visuals.widgets.inactive.weak_bg_fill = p.bg_hover;
    visuals.widgets.hovered.bg_fill = p.bg_hover;
    visuals.widgets.active.bg_fill = p.accent;
    visuals.widgets.open.bg_fill = p.accent;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.border);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.accent);
    visuals.widgets.noninteractive.fg_stroke.color = p.text_primary;
    visuals.widgets.inactive.fg_stroke.color = p.text_primary;
    visuals.widgets.hovered.fg_stroke.color = p.text_primary;
    // active/open widget states sit on accent backgrounds — their
    // text must contrast with the accent, not text_primary (which is near-black
    // in light themes → the black-on-blue button bug class).
    visuals.widgets.active.fg_stroke.color = p.selection_fg;
    visuals.widgets.open.fg_stroke.color = p.selection_fg;
    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke = Stroke::new(1.0_f32, p.selection_fg);
    visuals.text_cursor.stroke = Stroke::new(2.5_f32, p.text_primary);
    visuals.hyperlink_color = p.accent;
    visuals.warn_fg_color = p.warning;
    visuals.error_fg_color = p.error;
    // the wrong-font-color bug class — unstyled RichText used to
    // inherit egui's default text color, which could diverge from the theme.
    // Forcing the override means EVERY label without an explicit color gets
    // the palette's primary text.
    visuals.override_text_color = Some(p.text_primary);
    visuals.window_stroke = Stroke::new(1.0_f32, p.border);
}

pub fn configure_theme(ctx: &egui::Context, preset: ThemePreset, use_dark: bool) {
    // skip the rebuild unless the theme actually changed — this ran
    // every frame (Visuals alloc + set_visuals Arc swap + palette store) for
    // values that only change on user action.
    static LAST_APPLIED: Mutex<Option<(ThemePreset, bool)>> = Mutex::new(None);
    let unchanged = LAST_APPLIED
        .lock()
        .ok()
        .map(|guard| {
            guard
                .as_ref()
                .is_some_and(|(p, d)| *p == preset && *d == use_dark)
        })
        .unwrap_or(false);
    if unchanged {
        return;
    }

    let pair = theme_pair(preset);
    let c = if use_dark { &pair.dark } else { &pair.light };
    let p = derive(c, use_dark);

    let mut visuals = if use_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    visuals.dark_mode = use_dark;
    apply_visuals(&mut visuals, &p);

    visuals.text_cursor.on_duration = 0.65;
    visuals.text_cursor.off_duration = 0.35;
    visuals.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(150),
    };
    visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(200),
    };

    // egui keeps TWO style slots (dark_style/light_style) and picks
    // the active one via theme_preference, which defaults to System (follows
    // the OS) — independent of this app's variant logic. set_visuals writes
    // only the active slot, so the other slot kept STOCK egui visuals and
    // surfaced them whenever egui's slot choice disagreed with ours (OS theme
    // flip, variant != OS) — "the theme switch only applies to parts of the
    // app". Fix: write BOTH slots, and pin egui's preference to this app's
    // variant so slot choice is deterministic (and the native title bar
    // follows the in-app variant).
    let theme = if use_dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(egui::ThemePreference::from(theme));
    ctx.all_styles_mut(|s| s.visuals = visuals.clone());
    *PALETTE.lock().expect("theme palette mutex poisoned") = Some(p);
    // style writes go through Context::write, which does NOT request
    // a repaint — without this, a theme change with the mouse idle kept
    // showing the last-painted frame until the next input event.
    ctx.request_repaint();
    if let Ok(mut guard) = LAST_APPLIED.lock() {
        *guard = Some((preset, use_dark));
    }
}

pub fn sel_text(_ui: &egui::Ui, selected: bool, label: &str) -> egui::RichText {
    if selected {
        egui::RichText::new(label).color(contrast_text(palette().accent))
    } else {
        egui::RichText::new(label).color(palette().text_primary)
    }
}

/// Consistent horizontal margins applied to the main content area.
/// Left/right margins keep content from touching the window edges.
/// Top/bottom margins provide vertical breathing room.

pub fn theme_selector_ui(preset: &mut ThemePreset, ui: &mut egui::Ui) {
    let current_name = ThemePreset::all()
        .iter()
        .find(|(t, _)| t == preset)
        .map(|(_, n)| *n)
        .unwrap_or("One Dark");
    let use_dark = active_is_dark();
    ui.menu_button(current_name, |ui| {
        for &(t, name) in ThemePreset::all() {
            let resp = ui.selectable_label(*preset == t, sel_text(ui, *preset == t, name));
            // Live preview: repaint the whole app in the hovered preset. On
            // un-hover / menu-dismiss the selected preset comes back automatically
            // (main re-applies it every frame; configure_theme's cache makes that
            // a no-op unless it actually changed).
            if resp.hovered() {
                configure_theme(ui.ctx(), t, use_dark);
            }
            if resp.clicked() {
                *preset = t;
                ui.close();
            }
        }
    });
}

#[cfg(test)]
mod contrast_tests {
    use super::*;

    #[test]
    fn contrast_picks_white_on_mid_dark_accents() {
        // One Dark accent blue #3b82f6 — the old formula scored it 121.9 > 120
        // and put black text on the accent-filled buttons.
        assert_eq!(contrast_text(hex(0x3b82f6)), Color32::WHITE);
        // Nord pale cyan accent and One Dark warning orange stay black.
        assert_eq!(contrast_text(hex(0x88c0d0)), Color32::BLACK);
        assert_eq!(contrast_text(hex(0xd19a66)), Color32::BLACK);
        // Salmon-red error fill keeps its current black text (no regression).
        assert_eq!(contrast_text(hex(0xe06c75)), Color32::BLACK);
    }
}
