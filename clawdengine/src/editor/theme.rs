use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

// ---- ClawdEngine "Rust" palette (ported from Claude Design mockup) ----
// Tokens are the source of truth in design_extract/clawdengine/project/styles.css.
// Constant NAMES are kept identical to the previous Catppuccin theme so the
// whole editor re-skins automatically — only RGB values change.

// Backgrounds, darkest -> lightest (mockup --line, --bg-0..--bg-3)
pub const BG_CRUST:    Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0C); // --line: gutters, extreme insets
pub const BG_MANTLE:   Color32 = Color32::from_rgb(0x13, 0x13, 0x16); // --bg-0: menubar, tab bar, status bar
pub const BG_BASE:     Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1E); // --bg-1: panel surface
pub const BG_SURFACE0: Color32 = Color32::from_rgb(0x20, 0x20, 0x24); // --bg-2: inputs, section headers
pub const BG_SURFACE1: Color32 = Color32::from_rgb(0x26, 0x26, 0x2B); // --bg-3: hover

// Text (mockup --text / --text-2 / --text-3)
pub const TEXT_PRIMARY:   Color32 = Color32::from_rgb(0xE6, 0xE6, 0xE8);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xA4, 0xA4, 0xAC);
// Mockup --text-3 is #6E6E76 (~3:1 on BG_BASE — fails WCAG AA). The merged
// contrast fix mandates >=4.5:1 for real labels, so disabled text is lifted to
// #8A8A93 (~4.6:1 on #1A1A1E) — close to the design intent, still accessible.
pub const TEXT_DISABLED:  Color32 = Color32::from_rgb(0x8A, 0x8A, 0x93);

// Accent — Rust orange. Text placed ON accent must use ON_ACCENT, never white.
pub const ACCENT:         Color32 = Color32::from_rgb(0xFF, 0x7A, 0x3D); // --accent
pub const ACCENT_HOVER:   Color32 = Color32::from_rgb(0xFF, 0x9A, 0x66); // --accent-2
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0xE8, 0x6A, 0x2D);
pub const ACCENT_SOFT:    Color32 = Color32::from_rgba_premultiplied(0x2C, 0x18, 0x0C, 0x24); // selection fill ~rgba(255,122,61,.14)
pub const ACCENT_RING:    Color32 = Color32::from_rgba_premultiplied(0x52, 0x27, 0x14, 0x52); // focus ring ~rgba(255,122,61,.32)
pub const ON_ACCENT:      Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1E); // text/icon on an accent fill

// Semantic (mockup --green / --red / --yellow)
pub const SUCCESS: Color32 = Color32::from_rgb(0x5D, 0xD3, 0x9E);
pub const ERROR:   Color32 = Color32::from_rgb(0xFF, 0x5D, 0x5D);
pub const WARNING: Color32 = Color32::from_rgb(0xF5, 0xC3, 0x4B);

// Axis colors (mockup --x / --y / --z)
pub const AXIS_X: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
pub const AXIS_Y: Color32 = Color32::from_rgb(0x7E, 0xD9, 0x57);
pub const AXIS_Z: Color32 = Color32::from_rgb(0x5E, 0xB1, 0xFF);

// Secondary accents (mockup --purple / cyan / --blue) for inspector & icons.
pub const MAUVE: Color32 = Color32::from_rgb(0xB0, 0x8C, 0xFF); // --purple
pub const TEAL:  Color32 = Color32::from_rgb(0x3D, 0xDB, 0xD9); // cyan
pub const PEACH: Color32 = Color32::from_rgb(0xFF, 0xB2, 0x7A);
pub const SKY:   Color32 = Color32::from_rgb(0x5E, 0xB1, 0xFF); // --blue

// Component-section accents referenced by the inspector panel.
pub const COMPONENT_MESH:      Color32 = TEAL;
pub const COMPONENT_MATERIAL:  Color32 = Color32::from_rgb(0xB0, 0x8C, 0xFF); // material = purple
pub const COMPONENT_RIGIDBODY: Color32 = PEACH;

pub fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    // ---- Dark mode base ----
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = BG_BASE;
    style.visuals.window_fill = BG_BASE;
    style.visuals.extreme_bg_color = BG_CRUST;
    style.visuals.faint_bg_color = BG_SURFACE0;
    style.visuals.code_bg_color = BG_SURFACE0;
    style.visuals.override_text_color = Some(TEXT_PRIMARY);

    // ---- Selection ----
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, ACCENT_HOVER);

    // ---- Widgets — noninteractive (labels, separators) ----
    style.visuals.widgets.noninteractive.bg_fill = BG_BASE;
    style.visuals.widgets.noninteractive.weak_bg_fill = BG_BASE;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BG_SURFACE0);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    // ---- Widgets — inactive (buttons, inputs at rest) ----
    style.visuals.widgets.inactive.bg_fill = BG_SURFACE0;
    style.visuals.widgets.inactive.weak_bg_fill = BG_SURFACE0;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BG_SURFACE1);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(4);

    // ---- Widgets — hovered ----
    style.visuals.widgets.hovered.bg_fill = BG_SURFACE1;
    style.visuals.widgets.hovered.weak_bg_fill = BG_SURFACE1;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(4);

    // ---- Widgets — active (being interacted with) ----
    style.visuals.widgets.active.bg_fill = ACCENT;
    style.visuals.widgets.active.weak_bg_fill = ACCENT;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.5, ON_ACCENT);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(4);

    // ---- Widgets — open (combo boxes, menus) ----
    style.visuals.widgets.open.bg_fill = BG_SURFACE0;
    style.visuals.widgets.open.weak_bg_fill = BG_SURFACE0;
    style.visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(4);

    // ---- Shadows & windows ----
    style.visuals.window_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    style.visuals.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 2,
        color: Color32::from_black_alpha(100),
    };
    style.visuals.window_corner_radius = CornerRadius::same(6);
    style.visuals.menu_corner_radius = CornerRadius::same(6);
    style.visuals.window_stroke = Stroke::new(1.0, BG_SURFACE1);

    // ---- Visual polish ----
    style.visuals.collapsing_header_frame = true;
    style.visuals.indent_has_left_vline = true;
    style.visuals.slider_trailing_fill = true;
    style.visuals.handle_shape = egui::style::HandleShape::Circle;
    style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    style.visuals.button_frame = true;
    style.visuals.striped = true;

    // ---- Animations & interactions ----
    style.animation_time = 0.12;
    style.interaction.tooltip_delay = 0.3;

    // ---- Semantic colors ----
    style.visuals.warn_fg_color = WARNING;
    style.visuals.error_fg_color = ERROR;
    style.visuals.hyperlink_color = ACCENT;

    // ---- Text styles ----
    // Mockup scale: base 12, small 11, mono ~11. Tighter than Catppuccin.
    style.text_styles.insert(TextStyle::Heading, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(11.0, FontFamily::Monospace));
    style.text_styles.insert(TextStyle::Button, FontId::new(12.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(10.5, FontFamily::Proportional));

    // ---- Spacing ----
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.indent = 18.0;
    style.spacing.scroll = egui::style::ScrollStyle::thin();
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_width = 4.0;
    style.spacing.scroll.bar_inner_margin = 2.0;
    style.spacing.scroll.bar_outer_margin = 2.0;
    style.spacing.window_margin = Margin::same(12);

    ctx.set_style(style);
}

/// Custom dock style matching the Catppuccin Mocha theme.
pub fn dock_style(egui_style: &egui::Style) -> egui_dock::Style {
    let mut style = egui_dock::Style::from_egui(egui_style);

    // Tab bar
    style.tab_bar.bg_fill = BG_MANTLE;
    style.tab_bar.height = 28.0;
    style.tab_bar.corner_radius = CornerRadius::ZERO;
    style.tab_bar.hline_color = BG_SURFACE0;

    let tab_rounding = CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };

    // Active tab
    style.tab.active.bg_fill = BG_BASE;
    style.tab.active.text_color = TEXT_PRIMARY;
    style.tab.active.outline_color = ACCENT;
    style.tab.active.corner_radius = tab_rounding;

    // Inactive tab
    style.tab.inactive.bg_fill = BG_MANTLE;
    style.tab.inactive.text_color = TEXT_DISABLED;
    style.tab.inactive.outline_color = Color32::TRANSPARENT;
    style.tab.inactive.corner_radius = tab_rounding;

    // Hovered tab
    style.tab.hovered.bg_fill = BG_SURFACE0;
    style.tab.hovered.text_color = TEXT_SECONDARY;
    style.tab.hovered.outline_color = Color32::TRANSPARENT;
    style.tab.hovered.corner_radius = tab_rounding;

    // Focused tab
    style.tab.focused.bg_fill = BG_BASE;
    style.tab.focused.text_color = TEXT_PRIMARY;
    style.tab.focused.outline_color = ACCENT;
    style.tab.focused.corner_radius = tab_rounding;

    // Keyboard focus variants
    style.tab.active_with_kb_focus = style.tab.active.clone();
    style.tab.inactive_with_kb_focus = style.tab.inactive.clone();
    style.tab.focused_with_kb_focus = style.tab.focused.clone();

    // Tab spacing & line
    style.tab.spacing = 2.0;
    style.tab.hline_below_active_tab_name = false;

    // Tab body (panel content)
    style.tab.tab_body.bg_fill = BG_BASE;
    style.tab.tab_body.inner_margin = Margin::same(6);
    style.tab.tab_body.stroke = Stroke::new(1.0, BG_SURFACE0);
    style.tab.tab_body.corner_radius = CornerRadius::ZERO;

    // Separators
    style.separator.width = 1.0;
    style.separator.extra_interact_width = 4.0;
    style.separator.color_idle = BG_SURFACE0;
    style.separator.color_hovered = ACCENT;
    style.separator.color_dragged = ACCENT_PRESSED;

    // Border
    style.main_surface_border_stroke = Stroke::new(1.0, BG_CRUST);
    style.dock_area_padding = Some(Margin::same(0));

    style
}
