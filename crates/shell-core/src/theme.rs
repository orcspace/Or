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
            for surface in [
                palette.background,
                palette.surface,
                palette.surface_raised,
            ] {
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
}
