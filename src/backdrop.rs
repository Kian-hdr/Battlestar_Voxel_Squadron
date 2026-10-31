//! Orthographic framing for the background Galactica, independent of combat.
use crate::{combat_plane, voxel};
use glam::{Quat, Vec2, Vec3};
use std::sync::LazyLock;

const BACKGROUND_Z: f32 = -16.;
const SCALE: f32 = 0.48;

fn rotation() -> Quat {
    Quat::from_rotation_z(-0.38) * Quat::from_rotation_x(0.15)
}
fn view_up() -> Vec3 {
    Vec3::new(0., 28., 23.5).normalize()
}
fn project(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.dot(view_up()))
}
fn exact_local_bounds(classic: bool, rotation: Quat) -> (Vec2, Vec2) {
    let model = voxel::galactica(classic);
    let mut low = Vec2::splat(f32::INFINITY);
    let mut high = Vec2::splat(f32::NEG_INFINITY);
    for cell in model.0.keys() {
        for mask in 0..8 {
            let corner = Vec3::from_array(std::array::from_fn(|axis| {
                (cell[axis] as f32 + (if mask & (1 << axis) != 0 { 1. } else { 0. })) * voxel::UNIT
            }));
            let point = project(rotation * corner);
            low = low.min(point);
            high = high.max(point);
        }
    }
    (low, high)
}
fn cached_bounds() -> &'static [(Vec2, Vec2); 2] {
    static BOUNDS: LazyLock<[(Vec2, Vec2); 2]> =
        LazyLock::new(|| [false, true].map(|classic| exact_local_bounds(classic, rotation())));
    &BOUNDS
}

/// Retain the earlier right-side composition: hull center near 82% across and
/// 63% down the viewport. Keep the background depth and original rigid rotation.
/// The fixed scale retains roughly one-quarter viewport height in normal desktop
/// layouts without enlarging the hull until it clips in a tall window.
pub fn galactica_transform(aspect: f32) -> (Vec3, Quat, f32) {
    let aspect = if aspect.is_finite() && aspect > 0. {
        aspect
    } else {
        16. / 10.
    };
    let height = 17.5_f32.max(26. / aspect);
    let width = height * aspect;
    let bounds = cached_bounds();
    let low = bounds[0].0.min(bounds[1].0);
    let high = bounds[0].1.max(bounds[1].1);
    let center = (low + high) * (SCALE * 0.5);
    let desired = Vec2::new(width * 0.32, -height * 0.13) - center;
    let position = Vec3::new(
        desired.x,
        desired.y / view_up().y - combat_plane::VIEW_SLOPE * BACKGROUND_Z - 1.5,
        BACKGROUND_Z,
    );
    (position, rotation(), SCALE)
}

/// Exact bounds in physical camera-view units, with screen center at (0,0).
/// Compare X against +/-viewport_width/2 and Y against +/-viewport_height/2.
/// The normal fixed rotation uses cached all-cell-corner bounds, so checking a
/// resized or switched-era transform does not rescan the model every frame.
pub fn projected_galactica_bounds(
    classic: bool,
    position: Vec3,
    orientation: Quat,
    scale: f32,
) -> (Vec2, Vec2) {
    let (low, high) = if orientation == rotation() {
        cached_bounds()[usize::from(classic)]
    } else {
        exact_local_bounds(classic, orientation)
    };
    let offset = project(position - Vec3::new(0., -1.5, 0.));
    let a = low * scale + offset;
    let b = high * scale + offset;
    (a.min(b), a.max(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_actual_hulls_fit_desktop_retina_tall_and_wide_viewports() {
        for (width, height) in [
            (800., 500.),
            (1920., 1080.),
            (3024., 1964.),
            (800., 1600.),
            (3440., 1440.),
        ] {
            let aspect = width / height;
            let view_height = 17.5_f32.max(26. / aspect);
            let view_width = view_height * aspect;
            let (position, orientation, scale) = galactica_transform(aspect);
            assert_eq!(position.z, BACKGROUND_Z);
            assert_eq!(orientation, rotation());
            assert_eq!(scale, SCALE);
            for classic in [false, true] {
                let (low, high) = projected_galactica_bounds(classic, position, orientation, scale);
                // Independently check every source cube corner using the actual
                // camera view matrix, rather than only the origin or a radius.
                let view = glam::Mat4::look_at_rh(
                    Vec3::new(0., -25., 28.),
                    Vec3::new(0., -1.5, 0.),
                    Vec3::Y,
                );
                for cell in voxel::galactica(classic).0.keys() {
                    for mask in 0..8 {
                        let corner = Vec3::from_array(std::array::from_fn(|axis| {
                            (cell[axis] as f32 + (if mask & (1 << axis) != 0 { 1. } else { 0. }))
                                * voxel::UNIT
                        }));
                        let point =
                            view.transform_point3(position + orientation * (corner * scale));
                        assert!(
                            point.x.abs() < view_width * 0.5,
                            "horizontal clip at {width}x{height}"
                        );
                        assert!(
                            point.y.abs() < view_height * 0.5,
                            "vertical clip at {width}x{height}"
                        );
                        assert!(point.x >= low.x - 1e-5 && point.x <= high.x + 1e-5);
                        assert!(point.y >= low.y - 1e-5 && point.y <= high.y + 1e-5);
                        assert!(point.z < -0.1 && point.z > -200.);
                    }
                }
            }
        }
    }

    #[test]
    fn ordinary_desktop_framing_matches_the_previous_background_composition() {
        for aspect in [16. / 10., 16. / 9., 3024. / 1964.] {
            let view_height = 17.5_f32.max(26. / aspect);
            let view_width = view_height * aspect;
            let (position, orientation, scale) = galactica_transform(aspect);
            for classic in [false, true] {
                let (low, high) = projected_galactica_bounds(classic, position, orientation, scale);
                let center = (low + high) * 0.5;
                let fraction_x = 0.5 + center.x / view_width;
                let fraction_y = 0.5 - center.y / view_height;
                let fraction_height = (high.y - low.y) / view_height;
                assert!((fraction_x - 0.82).abs() < 0.015);
                assert!((fraction_y - 0.63).abs() < 0.015);
                assert!(
                    (0.20..0.28).contains(&fraction_height),
                    "hull height fraction {fraction_height}"
                );
            }
        }
    }
}
