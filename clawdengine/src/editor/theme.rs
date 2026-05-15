use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

// ---- Catppuccin Mocha Palette ----

// Backgrounds (blue-tinted grays, darkest to lightest)
pub const BG_CRUST:    Color32 = Color32::from_rgb(0x11, 0x11, 0x1B);
pub const BG_MANTLE:   Color32 = Color32::from_rgb(0x18, 0x18, 0x25);
pub const BG_BASE:     Color32 = Color32::from_rgb(0x1E, 0x1E, 0x2E);
pub const BG_SURFACE0: Color32 = Color32::from_rgb(0x31, 0x32, 0x44);
pub const BG_SURFACE1: Color32 = Color32::from_rgb(0x45, 0x47, 0x5A);

// Text
pub const TEXT_PRIMARY:  Color32 = Color32::from_rgb(0xCD, 0xD6, 0xF4);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xA6, 0xAD, 0xC8);
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x6C, 0x70, 0x86);

// Accent (sapphire blue)
pub const ACCENT:         Color32 = Color32::from_rgb(0x89, 0xB4, 0xFA);
pub const ACCENT_HOVER:   Color32 = Color32::from_rgb(0xB4, 0xBE, 0xFE);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x74, 0xC7, 0xEC);

// Semantic
pub const SUCCESS: Color32 = Color32::from_rgb(0xA6, 0xE3, 0xA1);
pub const ERROR:   Color32 = Color32::from_rgb(0xF3, 0x8B, 0xA8);
pub const WARNING: Color32 = Color32::from_rgb(0xF9, 0xE2, 0xAF);

// Axis colors (for gizmos & inspector)
pub const AXIS_X: Color32 = Color32::from_rgb(0xDC, 0x50, 0x50);
pub const AXIS_Y: Color32 = Color32::from_rgb(0x50, 0xBE, 0x50);
pub const AXIS_Z: Color32 = Color32::from_rgb(0x50, 0x78, 0xDC);

// Additional Catppuccin Mocha accents (used by inspector & viewport icons).
pub const MAUVE: Color32 = Color32::from_rgb(0xCB, 0xA6, 0xF7);
pub const TEAL:  Color32 = Color32::from_rgb(0x94, 0xE2, 0xD5);
pub const PEACH: Color32 = Color32::from_rgb(0xFA, 0xB3, 0x87);
pub const SKY:   Color32 = Color32::from_rgb(0x89, 0xDC, 0xEB);

// Component-section accents referenced by the inspector panel.
pub const COMPONENT_MESH:      Color32 = TEAL;   // pink-ish mesh accent
pub const COMPONENT_MATERIAL:  Color32 = Color32::from_rgb(0xF5, 0xC2, 0xE7); // pink
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
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    // ---- Widgets — hovered ----
    style.visuals.widgets.hovered.bg_fill = BG_SURFACE1;
    style.visuals.widgets.hovered.weak_bg_fill = BG_SURFACE1;
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    // ---- Widgets — active (being interacted with) ----
    style.visuals.widgets.active.bg_fill = ACCENT_PRESSED;
    style.visuals.widgets.active.weak_bg_fill = ACCENT_PRESSED;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(6);

    // ---- Widgets — open (combo boxes, menus) ----
    style.visuals.widgets.open.bg_fill = BG_SURFACE0;
    style.visuals.widgets.open.weak_bg_fill = BG_SURFACE0;
    style.visuals.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    style.visuals.widgets.open.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    style.visuals.widgets.open.corner_radius = CornerRadius::same(6);

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
    style.visuals.window_corner_radius = CornerRadius::same(8);
    style.visuals.menu_corner_radius = CornerRadius::same(8);
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
    style.text_styles.insert(TextStyle::Heading, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace));
    style.text_styles.insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));

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
