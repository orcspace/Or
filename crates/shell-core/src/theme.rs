//! Theme selection and contrast calculations. No UI, IO or JavaScript.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

impl ThemePreference {
    pub const fn resolve(self, system: ColorScheme) -> ColorScheme {
        match self {
            Self::Light => ColorScheme::Light,
            Self::Dark => ColorScheme::Dark,
            Self::System => system,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn relative_luminance(self) -> f64 {
        fn linear(value: u8) -> f64 {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linear(self.0) + 0.7152 * linear(self.1) + 0.0722 * linear(self.2)
    }

    pub fn contrast_with(self, other: Self) -> f64 {
        let first = self.relative_luminance();
        let second = other.relative_luminance();
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }
}

/// A native accessibility adapter must provide every system high-contrast color.
/// High contrast is not approximated by switching to the normal dark palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfacePalette {
    pub background: Rgb,
    pub surface: Rgb,
    pub surface_raised: Rgb,
    pub text_primary: Rgb,
    pub text_secondary: Rgb,
}

pub const LIGHT: SurfacePalette = SurfacePalette {
    background: Rgb(247, 248, 250),
    surface: Rgb(255, 255, 255),
    surface_raised: Rgb(255, 255, 255),
    text_primary: Rgb(23, 27, 34),
    text_secondary: Rgb(81, 90, 104),
};

pub const DARK: SurfacePalette = SurfacePalette {
    background: Rgb(17, 20, 26),
    surface: Rgb(25, 29, 37),
    surface_raised: Rgb(34, 39, 49),
    text_primary: Rgb(241, 243, 247),
    text_secondary: Rgb(171, 180, 195),
};

pub fn surface_palette(
    preference: ThemePreference,
    system: ColorScheme,
    high_contrast: Option<SurfacePalette>,
) -> SurfacePalette {
    if let Some(palette) = high_contrast {
        return palette;
    }
    match preference.resolve(system) {
        ColorScheme::Light => LIGHT,
        ColorScheme::Dark => DARK,
    }
}

pub const fn transition_duration_ms(reduced_motion: bool) -> u32 {
    if reduced_motion { 0 } else { 180 }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DocumentMode {
    #[default]
    Docs,
    Sheets,
    Slides,
}

/// Complete semantic palette for the future native Qt bridge.
/// The presentation adapter consumes these values; it does not choose colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeTokens {
    pub background: Rgb,
    pub surface: Rgb,
    pub surface_raised: Rgb,
    pub border: Rgb,
    pub text_primary: Rgb,
    pub text_secondary: Rgb,
    pub text_disabled: Rgb,
    pub button_text: Rgb,
    pub accent: Rgb,
    pub accent_docs: Rgb,
    pub accent_sheets: Rgb,
    pub accent_slides: Rgb,
    pub danger: Rgb,
    pub warning: Rgb,
    pub success: Rgb,
    pub selection: Rgb,
    pub selection_text: Rgb,
    pub ai_cursor: Rgb,
    pub remote_cursors: [Rgb; 8],
}

/// COLOR_WINDOW, COLOR_WINDOWTEXT, COLOR_3DFACE, COLOR_BTNTEXT,
/// COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT and COLOR_HOTLIGHT.
/// These values must come from the native platform adapter, not hardcoded themes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemContrastColors {
    pub window: Rgb,
    pub window_text: Rgb,
    pub button_face: Rgb,
    pub button_text: Rgb,
    pub disabled_text: Rgb,
    pub highlight: Rgb,
    pub highlight_text: Rgb,
    pub link: Rgb,
}

impl ThemeTokens {
    pub fn for_scheme(scheme: ColorScheme, mode: DocumentMode) -> Self {
        let mut tokens = match scheme {
            ColorScheme::Light => Self {
                background: LIGHT.background,
                surface: LIGHT.surface,
                surface_raised: LIGHT.surface_raised,
                border: Rgb(203, 213, 225),
                text_primary: LIGHT.text_primary,
                text_secondary: LIGHT.text_secondary,
                text_disabled: Rgb(128, 137, 150),
                button_text: LIGHT.text_primary,
                accent: Rgb(37, 99, 235),
                accent_docs: Rgb(37, 99, 235),
                accent_sheets: Rgb(21, 128, 61),
                accent_slides: Rgb(194, 65, 12),
                danger: Rgb(185, 28, 28),
                warning: Rgb(161, 98, 7),
                success: Rgb(21, 128, 61),
                selection: Rgb(219, 234, 254),
                selection_text: LIGHT.text_primary,
                ai_cursor: Rgb(109, 40, 217),
                remote_cursors: [
                    Rgb(37, 99, 235),
                    Rgb(21, 128, 61),
                    Rgb(194, 65, 12),
                    Rgb(109, 40, 217),
                    Rgb(190, 24, 93),
                    Rgb(14, 116, 144),
                    Rgb(161, 98, 7),
                    Rgb(81, 90, 104),
                ],
            },
            ColorScheme::Dark => Self {
                background: DARK.background,
                surface: DARK.surface,
                surface_raised: DARK.surface_raised,
                border: Rgb(67, 76, 92),
                text_primary: DARK.text_primary,
                text_secondary: DARK.text_secondary,
                text_disabled: Rgb(112, 123, 140),
                button_text: DARK.text_primary,
                accent: Rgb(96, 165, 250),
                accent_docs: Rgb(96, 165, 250),
                accent_sheets: Rgb(74, 222, 128),
                accent_slides: Rgb(251, 146, 60),
                danger: Rgb(248, 113, 113),
                warning: Rgb(250, 204, 21),
                success: Rgb(74, 222, 128),
                selection: Rgb(30, 58, 90),
                selection_text: DARK.text_primary,
                ai_cursor: Rgb(192, 132, 252),
                remote_cursors: [
                    Rgb(96, 165, 250),
                    Rgb(74, 222, 128),
                    Rgb(251, 146, 60),
                    Rgb(192, 132, 252),
                    Rgb(244, 114, 182),
                    Rgb(34, 211, 238),
                    Rgb(250, 204, 21),
                    Rgb(171, 180, 195),
                ],
            },
        };
        tokens.accent = tokens.mode_accent(mode);
        tokens
    }

    pub const fn mode_accent(self, mode: DocumentMode) -> Rgb {
        match mode {
            DocumentMode::Docs => self.accent_docs,
            DocumentMode::Sheets => self.accent_sheets,
            DocumentMode::Slides => self.accent_slides,
        }
    }

    pub const fn from_system_contrast(colors: SystemContrastColors) -> Self {
        Self {
            background: colors.window,
            surface: colors.window,
            surface_raised: colors.button_face,
            border: colors.window_text,
            text_primary: colors.window_text,
            text_secondary: colors.window_text,
            text_disabled: colors.disabled_text,
            button_text: colors.button_text,
            accent: colors.link,
            accent_docs: colors.link,
            accent_sheets: colors.link,
            accent_slides: colors.link,
            danger: colors.window_text,
            warning: colors.window_text,
            success: colors.window_text,
            selection: colors.highlight,
            selection_text: colors.highlight_text,
            ai_cursor: colors.link,
            remote_cursors: [colors.link; 8],
        }
    }
}

/// Native system events update this state before publishing one palette to QML.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeState {
    pub preference: ThemePreference,
    pub system_scheme: ColorScheme,
    pub document_mode: DocumentMode,
    pub high_contrast: Option<SystemContrastColors>,
    pub reduced_motion: bool,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            preference: ThemePreference::System,
            system_scheme: ColorScheme::Light,
            document_mode: DocumentMode::Docs,
            high_contrast: None,
            reduced_motion: false,
        }
    }
}

impl ThemeState {
    pub fn tokens(self) -> ThemeTokens {
        match self.high_contrast {
            Some(colors) => ThemeTokens::from_system_contrast(colors),
            None => ThemeTokens::for_scheme(
                self.preference.resolve(self.system_scheme),
                self.document_mode,
            ),
        }
    }

    pub const fn transition_duration_ms(self) -> u32 {
        transition_duration_ms(self.reduced_motion || self.high_contrast.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_live_system_theme_but_honors_explicit_preference() {
        assert_eq!(
            ThemePreference::System.resolve(ColorScheme::Light),
            ColorScheme::Light
        );
        assert_eq!(
            ThemePreference::System.resolve(ColorScheme::Dark),
            ColorScheme::Dark
        );
        assert_eq!(
            ThemePreference::Light.resolve(ColorScheme::Dark),
            ColorScheme::Light
        );
        assert_eq!(
            ThemePreference::Dark.resolve(ColorScheme::Light),
            ColorScheme::Dark
        );
    }

    #[test]
    fn readable_text_meets_wcag_aa_on_every_surface() {
        for palette in [LIGHT, DARK] {
            for surface in [palette.background, palette.surface, palette.surface_raised] {
                assert!(palette.text_primary.contrast_with(surface) >= 4.5);
                assert!(palette.text_secondary.contrast_with(surface) >= 4.5);
            }
        }
    }

    #[test]
    fn high_contrast_overrides_user_theme_without_modifying_system_colors() {
        let system = SurfacePalette {
            background: Rgb(0, 0, 0),
            surface: Rgb(0, 0, 0),
            surface_raised: Rgb(0, 0, 0),
            text_primary: Rgb(255, 255, 0),
            text_secondary: Rgb(0, 255, 255),
        };
        assert_eq!(
            surface_palette(ThemePreference::Light, ColorScheme::Light, Some(system)),
            system
        );
    }

    #[test]
    fn reduced_motion_removes_transition() {
        assert_eq!(transition_duration_ms(true), 0);
        assert!((150..=200).contains(&transition_duration_ms(false)));
    }

    #[test]
    fn contrast_is_symmetric_and_black_white_is_twenty_one() {
        let black = Rgb(0, 0, 0);
        let white = Rgb(255, 255, 255);
        assert!((black.contrast_with(white) - 21.0).abs() < 1e-9);
        assert_eq!(black.contrast_with(white), white.contrast_with(black));
    }

    #[test]
    fn all_semantic_foregrounds_are_readable_in_both_themes() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let tokens = ThemeTokens::for_scheme(scheme, DocumentMode::Docs);
            for background in [tokens.background, tokens.surface, tokens.surface_raised] {
                for text in [tokens.text_primary, tokens.text_secondary] {
                    assert!(text.contrast_with(background) >= 4.5);
                }
                for color in [
                    tokens.accent_docs,
                    tokens.accent_sheets,
                    tokens.accent_slides,
                    tokens.danger,
                    tokens.warning,
                    tokens.success,
                    tokens.ai_cursor,
                ] {
                    assert!(color.contrast_with(background) >= 3.0);
                }
                for cursor in tokens.remote_cursors {
                    assert!(cursor.contrast_with(background) >= 3.0);
                }
            }
            assert!(tokens.selection_text.contrast_with(tokens.selection) >= 4.5);
            assert!(tokens.button_text.contrast_with(tokens.surface_raised) >= 4.5);
            for first in 0..8 {
                for second in (first + 1)..8 {
                    assert_ne!(tokens.remote_cursors[first], tokens.remote_cursors[second]);
                }
            }
        }
    }

    #[test]
    fn document_switch_changes_the_accent_without_replacing_the_theme() {
        let mut state = ThemeState::default();
        let docs = state.tokens();
        state.document_mode = DocumentMode::Sheets;
        let sheets = state.tokens();
        state.document_mode = DocumentMode::Slides;
        let slides = state.tokens();
        assert_eq!(docs.background, sheets.background);
        assert_eq!(docs.background, slides.background);
        assert_ne!(docs.accent, sheets.accent);
        assert_ne!(sheets.accent, slides.accent);
    }

    #[test]
    fn system_events_apply_only_to_system_preference() {
        let mut state = ThemeState::default();
        let light = state.tokens();
        state.system_scheme = ColorScheme::Dark;
        assert_ne!(light.background, state.tokens().background);
        state.preference = ThemePreference::Light;
        assert_eq!(light, state.tokens());
    }

    #[test]
    fn system_high_contrast_is_exact_and_disables_color_crossfade() {
        let colors = SystemContrastColors {
            window: Rgb(0, 0, 0),
            window_text: Rgb(255, 255, 0),
            button_face: Rgb(12, 12, 12),
            button_text: Rgb(255, 255, 255),
            disabled_text: Rgb(160, 160, 160),
            highlight: Rgb(255, 255, 255),
            highlight_text: Rgb(0, 0, 0),
            link: Rgb(0, 255, 255),
        };
        let state = ThemeState {
            high_contrast: Some(colors),
            preference: ThemePreference::Light,
            document_mode: DocumentMode::Slides,
            ..ThemeState::default()
        };
        let tokens = state.tokens();
        assert_eq!(tokens.background, colors.window);
        assert_eq!(tokens.text_primary, colors.window_text);
        assert_eq!(tokens.button_text, colors.button_text);
        assert_eq!(tokens.selection, colors.highlight);
        assert_eq!(tokens.selection_text, colors.highlight_text);
        assert_eq!(tokens.accent, colors.link);
        assert_eq!(state.transition_duration_ms(), 0);
    }
}
