//! Exact occupied-voxel silhouettes on the orthographic arcade combat plane.
//! Cosmetic displacement along the camera's forward axis never changes aim.
use crate::voxel::{UNIT, Voxels};
use glam::{Quat, Vec2, Vec3};

/// The fixed camera looks from (0,-25,28) toward (0,-1.5,0).
/// Divide view-up coordinates by their Y component to retain logical world Y.
pub const VIEW_SLOPE: f32 = 23.5 / 28.;

pub fn view_forward() -> Vec3 {
    Vec3::new(0., 23.5, -28.).normalize()
}

/// Positive cosmetic depth brings geometry toward the viewer without parallax.
pub fn visual_position(pos: Vec2, cosmetic_depth: f32) -> Vec3 {
    pos.extend(0.) - view_forward() * cosmetic_depth
}

pub fn visual_velocity(velocity: Vec2, depth_velocity: f32) -> Vec3 {
    velocity.extend(0.) - view_forward() * depth_velocity
}

pub fn project_offset(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.y + VIEW_SLOPE * v.z)
}

/// First contact of a finite 2D point trajectory with projected occupied cells.
/// `start` and `end` are relative to the actor's logical XY position. Rotation
/// matches its rendered model; actor and projectile cosmetic depths are absent.
/// A projected cube is a zonotope with at most three independent edge normals.
/// Slabs on those normals are its exact silhouette, rather than an AABB proxy.
pub fn segment_hit(model: &Voxels, start: Vec2, end: Vec2, rotation: Quat) -> Option<Vec2> {
    if !start.is_finite()
        || !end.is_finite()
        || !(end - start).is_finite()
        || !rotation.is_finite()
        || !rotation.length_squared().is_finite()
        || rotation.length_squared() < 1e-12
    {
        return None;
    }
    let rotation = rotation.normalize();
    let columns = [Vec3::X, Vec3::Y, Vec3::Z].map(|v| project_offset(rotation * v));
    let half = UNIT * 0.5;
    let extents = (columns[0].abs() + columns[1].abs() + columns[2].abs()) * half;
    let axes = columns.map(|v| Vec2::new(-v.y, v.x).normalize_or_zero());
    let radii = axes.map(|axis| half * columns.iter().map(|v| axis.dot(*v).abs()).sum::<f32>());
    let delta = end - start;
    let motion = axes.map(|axis| axis.dot(delta));
    let low = start.min(end) - extents;
    let high = start.max(end) + extents;
    let mut closest = f32::INFINITY;
    for p in model.0.keys() {
        let center = columns[0] * ((p[0] as f32 + 0.5) * UNIT)
            + columns[1] * ((p[1] as f32 + 0.5) * UNIT)
            + columns[2] * ((p[2] as f32 + 0.5) * UNIT);
        if !center.cmpge(low).all() || !center.cmple(high).all() {
            continue;
        }
        let offset = start - center;
        let mut enter = 0_f32;
        let mut exit = closest.min(1.);
        for i in 0..3 {
            if axes[i] == Vec2::ZERO {
                continue;
            }
            let distance = axes[i].dot(offset);
            if motion[i].abs() < 1e-12 {
                if distance.abs() > radii[i] {
                    exit = -1.;
                    break;
                }
            } else {
                let a = (-radii[i] - distance) / motion[i];
                let b = (radii[i] - distance) / motion[i];
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
            }
        }
        if enter <= exit && enter < closest {
            closest = enter;
        }
    }
    closest.is_finite().then(|| start.lerp(end, closest))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cell() -> Voxels {
        let mut v = Voxels::default();
        v.0.insert([0; 3], [1.; 4]);
        v
    }
    #[test]
    fn camera_forward_has_no_combat_plane_displacement() {
        let forward = Vec3::new(0., 23.5, -28.).normalize();
        assert!(project_offset(forward).length() < 1e-6);
    }
    #[test]
    fn arbitrary_cosmetic_depth_and_velocity_preserve_logical_coordinates() {
        for depth in [-100., -2.8, 0., 6., 100.] {
            let pos = Vec2::new(-4., 7.);
            assert!((project_offset(visual_position(pos, depth)) - pos).length() < 1e-5);
            assert!((project_offset(visual_velocity(pos, depth)) - pos).length() < 1e-5);
        }
    }
    #[test]
    fn projected_occupied_cells_keep_holes_and_closed_grazing_edges() {
        let mut hull = Voxels::default();
        hull.0.insert([-10, 0, 0], [1.; 4]);
        hull.0.insert([10, 0, 0], [1.; 4]);
        assert!(
            segment_hit(&hull, Vec2::new(0., -1.), Vec2::new(0., 1.), Quat::IDENTITY).is_none()
        );
        let hull = cell();
        let hit = segment_hit(
            &hull,
            Vec2::new(UNIT, -1.),
            Vec2::new(UNIT, 1.),
            Quat::IDENTITY,
        )
        .unwrap();
        assert!((hit.y).abs() < 1e-6);
        assert!(
            segment_hit(
                &hull,
                Vec2::new(UNIT + 1e-4, -1.),
                Vec2::new(UNIT + 1e-4, 1.),
                Quat::IDENTITY
            )
            .is_none()
        );
    }
    #[test]
    fn rotated_cube_rejects_empty_corners_of_its_projected_bounding_box() {
        let rotation = Quat::from_rotation_z(0.6) * Quat::from_rotation_y(0.4);
        let corners: Vec<_> = (0..8)
            .map(|i| {
                project_offset(
                    rotation
                        * Vec3::new(
                            if i & 1 == 0 { 0. } else { UNIT },
                            if i & 2 == 0 { 0. } else { UNIT },
                            if i & 4 == 0 { 0. } else { UNIT },
                        ),
                )
            })
            .collect();
        let low = corners
            .iter()
            .copied()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min);
        let high = corners
            .iter()
            .copied()
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max);
        let empty = Vec2::new(low.x + 1e-4, high.y - 1e-4);
        assert!(segment_hit(&cell(), empty, empty, rotation).is_none());
        let center = project_offset(rotation * Vec3::splat(UNIT * 0.5));
        assert_eq!(segment_hit(&cell(), center, center, rotation), Some(center));
    }
}
