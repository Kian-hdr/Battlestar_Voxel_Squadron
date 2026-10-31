use battlestar_voxel::viewport::{
    DebrisMotion, advance_debris, battle_fov, box_visible, launch_debris_velocity, sphere_visible,
};
use glam::{Mat3, Mat4, Quat, Vec3};

fn camera(aspect: f32, reverse_z: bool) -> (Mat4, Mat4, f32, f32) {
    let height = 17.5_f32.max(26. / aspect);
    let width = height * aspect;
    let view = Mat4::look_at_rh(Vec3::new(0., -25., 28.), Vec3::new(0., -1.5, 0.), Vec3::Y);
    let (near, far) = if reverse_z { (200., 0.1) } else { (0.1, 200.) };
    let projection = Mat4::orthographic_rh(
        -width * 0.5,
        width * 0.5,
        -height * 0.5,
        height * 0.5,
        near,
        far,
    );
    (projection * view, view.inverse(), width, height)
}

// Independent oracle: transform all eight actual box corners into normalized
// clip coordinates. Reject only if every corner is beyond the same clip plane.
// Requiring one corner inside all planes would incorrectly reject crossing boxes.
fn corner_support(clip: Mat4, position: Vec3, rotation: Quat, low: Vec3, high: Vec3) -> [f32; 6] {
    let mut support = [f32::NEG_INFINITY; 6];
    for mask in 0..8 {
        let local = Vec3::from_array(std::array::from_fn(|axis| {
            if mask & (1 << axis) == 0 {
                low[axis]
            } else {
                high[axis]
            }
        }));
        let p = clip * (position + rotation * local).extend(1.);
        let q = p.truncate() / p.w;
        for (maximum, distance) in
            support
                .iter_mut()
                .zip([1. + q.x, 1. - q.x, 1. + q.y, 1. - q.y, q.z, 1. - q.z])
        {
            *maximum = maximum.max(distance);
        }
    }
    support
}

#[test]
fn thin_rotated_shards_retire_after_the_box_leaves_even_when_its_sphere_overlaps() {
    let (clip, world_from_view, width, _) = camera(16. / 9., true);
    let orientation =
        Quat::from_mat3(&Mat3::from_mat4(world_from_view)) * Quat::from_rotation_z(0.05);
    let low = Vec3::new(-0.04, -2., -0.04);
    let high = -low;
    let position = world_from_view.transform_point3(Vec3::new(width * 0.5 + 0.4, 0., -40.));
    assert!(
        sphere_visible(clip, position, high.length()),
        "old sphere retains empty space beside a long shard"
    );
    assert!(!box_visible(clip, position, orientation, low, high));
    assert!(
        corner_support(clip, position, orientation, low, high)
            .iter()
            .any(|distance| *distance < 0.)
    );

    let partly_visible = world_from_view.transform_point3(Vec3::new(width * 0.5 + 0.1, 0., -40.));
    assert!(
        box_visible(clip, partly_visible, orientation, low, high),
        "rotated tip remains visible even though pivot is outside"
    );
}

#[test]
fn crossing_box_is_kept_when_no_corner_is_inside_the_whole_viewport() {
    let (clip, world_from_view, width, _) = camera(1.6, true);
    let orientation = Quat::from_mat3(&Mat3::from_mat4(world_from_view));
    let position = world_from_view.transform_point3(Vec3::new(0., 0., -40.));
    let high = Vec3::new(width, 0.1, 0.1);
    let low = -high;
    // Every corner is left or right of the image, but the center crosses it.
    for mask in 0..8 {
        let local = Vec3::from_array(std::array::from_fn(|axis| {
            if mask & (1 << axis) == 0 {
                low[axis]
            } else {
                high[axis]
            }
        }));
        let p = clip * (position + orientation * local).extend(1.);
        assert!((p.x / p.w).abs() > 1.);
    }
    assert!(box_visible(clip, position, orientation, low, high));
}

#[test]
fn near_and_far_planes_keep_partial_geometry_and_reject_complete_departures() {
    for reverse_z in [false, true] {
        let (clip, world_from_view, _, _) = camera(1.6, reverse_z);
        let orientation = Quat::from_mat3(&Mat3::from_mat4(world_from_view));
        for (z, half, expected) in [
            (-0.12, 0.05, true),
            (-0.01, 0.02, false),
            (1., 0.02, false),
            (-40., 0.2, true),
            (-200.1, 0.2, true),
            (-201., 0.2, false),
        ] {
            let position = world_from_view.transform_point3(Vec3::new(0., 0., z));
            let high = Vec3::splat(half);
            assert_eq!(
                box_visible(clip, position, orientation, -high, high),
                expected,
                "z={z} reverse={reverse_z}"
            );
            let oracle = corner_support(clip, position, orientation, -high, high);
            assert_eq!(oracle.iter().all(|v| *v >= 0.), expected);
        }
    }
}

#[test]
fn oriented_bounds_match_normalized_eight_corner_support_across_actual_camera_aspects() {
    let mut seed = 0xc001_2026_u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 40) as f32 / 16777216.
    };
    for aspect in [0.4, 1.6, 16. / 9., 3.] {
        for reverse_z in [false, true] {
            let (clip, world_from_view, width, height) = camera(aspect, reverse_z);
            for _ in 0..128 {
                let position = world_from_view.transform_point3(Vec3::new(
                    (random() - 0.5) * width * 1.4,
                    (random() - 0.5) * height * 1.4,
                    -random() * 220.,
                ));
                let rotation = Quat::from_euler(
                    glam::EulerRot::XYZ,
                    random() * 6.,
                    random() * 6.,
                    random() * 6.,
                );
                let low = Vec3::new(-0.02 - random() * 0.1, -random() * 3., -0.1);
                let high = Vec3::new(0.3, random() * 0.2 + 0.01, 0.2);
                let oracle = corner_support(clip, position, rotation, low, high);
                assert_eq!(
                    box_visible(clip, position, rotation, low, high),
                    oracle.iter().all(|v| *v >= 0.),
                    "corner support mismatch: {oracle:?}"
                );
            }
        }
    }
}

#[test]
fn unforced_launches_leave_the_actual_orthographic_view_without_later_acceleration() {
    for aspect in [0.4_f32, 1.6, 3.] {
        let (clip, world_from_view, _, _) = camera(aspect, true);
        let forward = world_from_view.transform_vector3(-Vec3::Z).normalize();
        let fov = battle_fov(aspect);
        let tangent = (fov * 0.5).tan() * (1. + aspect * aspect).sqrt();
        for impact in [Vec3::Y, -Vec3::Y, forward, Vec3::ZERO] {
            let velocity =
                launch_debris_velocity(impact, Vec3::ZERO, Vec3::ZERO, 29., forward, tangent);
            let mut motion = DebrisMotion {
                position: world_from_view.transform_point3(Vec3::new(0., 0., -40.)),
                velocity,
            };
            let low = Vec3::new(-0.04, -1., -0.04);
            let high = -low;
            let mut left = false;
            for _ in 0..1200 {
                motion = advance_debris(motion, 1. / 120.);
                assert_eq!(motion.velocity, velocity);
                let visible = box_visible(clip, motion.position, Quat::IDENTITY, low, high);
                if left {
                    assert!(
                        !visible,
                        "constant ballistic path must not reenter after leaving"
                    );
                }
                left |= !visible;
            }
            assert!(
                left,
                "ordinary launch must drain the actual ortho viewport within this ten-second observation"
            );
        }
    }
}
