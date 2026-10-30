//! Conservative viewport retirement for moving debris. Age is never a removal rule.
use glam::{Mat4, Vec3};

pub fn battle_fov(aspect: f32) -> f32 {
    let height = 17.5_f32.max(26. / aspect);
    2. * (height / (2. * 38.)).atan()
}

/// Keep the entire bounding sphere until it has crossed a camera clip plane.
/// Testing the pivot alone would pop large, partly visible pieces at the edges.
pub fn sphere_visible(clip_from_world: Mat4, center: Vec3, radius: f32) -> bool {
    let rows = clip_from_world
        .transpose()
        .to_cols_array_2d()
        .map(glam::Vec4::from_array);
    [
        rows[3] + rows[0],
        rows[3] - rows[0],
        rows[3] + rows[1],
        rows[3] - rows[1],
        rows[2],
        rows[3] - rows[2],
    ]
    .iter()
    .all(|p| p.dot(center.extend(1.)) >= -radius * p.truncate().length())
}

/// A broad, soft spatial shadow behind the combat plane. Retained hulls stay
/// opaque, with cool reflected light keeping their shape legible in shadow.
pub fn debris_shadow_tint(color: [f32; 4], depth: f32) -> [f32; 4] {
    let t = ((-depth - 1.) / 8.).clamp(0., 1.);
    let shadow = t * t * (3. - 2. * t);
    [
        color[0] * (1. - 0.76 * shadow),
        color[1] * (1. - 0.68 * shadow),
        color[2] * (1. - 0.55 * shadow),
        color[3],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_partly_visible_pieces_and_retires_only_beyond_edges() {
        let clip = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1., 0.1, 100.);
        assert!(sphere_visible(clip, Vec3::new(10.4, 0., -10.), 1.));
        assert!(!sphere_visible(clip, Vec3::new(12., 0., -10.), 1.));
        assert!(sphere_visible(clip, Vec3::new(0., 0., -10.), 1.));
        assert!(!sphere_visible(clip, Vec3::new(0., 0., 10.), 1.));
        assert!(!sphere_visible(clip, Vec3::new(0., 0., -102.), 1.));
    }
    #[test]
    fn spatial_shadow_preserves_opaque_visible_hulls() {
        let white = [1.; 4];
        assert_eq!(debris_shadow_tint(white, 0.), white);
        let middle = debris_shadow_tint(white, -5.);
        let shadow = debris_shadow_tint(white, -100.);
        assert!(middle[0] < 1. && middle[0] > shadow[0]);
        assert!(shadow[0] > 0.2 && shadow[2] > shadow[0]);
        assert_eq!(shadow[3], 1.);
        assert_eq!(shadow, debris_shadow_tint(white, -200.));
    }
}
