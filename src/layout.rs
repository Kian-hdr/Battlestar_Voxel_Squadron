//! Stable UI sizes in logical display points. Native DPI is applied once by Bevy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudLayout {
    pub scale: f32,
    pub compact: bool,
    pub title_font: f32,
    pub body_font: f32,
    pub footer_font: f32,
}
pub fn hud_layout(pixel_width: u32, pixel_height: u32, native_scale: f32) -> HudLayout {
    let dpi = if native_scale.is_finite() && native_scale > 0. {
        native_scale
    } else {
        1.
    };
    let compact = pixel_width as f32 / dpi < 1000. || pixel_height as f32 / dpi < 700.;
    HudLayout {
        scale: 1.,
        compact,
        title_font: if compact { 48. } else { 76. },
        body_font: 20.,
        footer_font: 14.,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retina_equivalent_windows_have_the_same_layout() {
        for (w, h) in [(1280, 800), (800, 500)] {
            assert_eq!(hud_layout(w * 2, h * 2, 2.), hud_layout(w, h, 1.));
        }
    }
    #[test]
    fn resizing_preserves_body_and_footer_point_sizes() {
        for (w, h) in [(800, 500), (1280, 800), (1920, 1200), (7680, 4320)] {
            let layout = hud_layout(w, h, 1.);
            assert_eq!(
                (layout.scale, layout.body_font, layout.footer_font),
                (1., 20., 14.)
            );
        }
    }
    #[test]
    fn compact_breakpoint_only_changes_title_and_layout() {
        assert_eq!(hud_layout(1000, 700, 1.).title_font, 76.);
        for (w, h) in [(999, 800), (1280, 699), (0, 0)] {
            let layout = hud_layout(w, h, 1.);
            assert!(layout.compact);
            assert_eq!(layout.title_font, 48.);
        }
        assert_eq!(hud_layout(1280, 800, f32::NAN), hud_layout(1280, 800, 1.));
    }
}
