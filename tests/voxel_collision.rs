use battlestar_voxel::{
    voxel::{UNIT, Voxels},
    voxel_collision::{Collider, bounce_velocity, deflect_velocity, sweep_chunk},
};
use glam::{Quat, Vec3};

fn collider(voxels: &Voxels) -> Collider<'_> {
    Collider {
        voxels,
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        velocity: Vec3::ZERO,
    }
}
fn single() -> Voxels {
    let mut hull = Voxels::default();
    hull.0.insert([0; 3], [1.; 4]);
    hull
}
fn sweep(
    start: Vec3,
    end: Vec3,
    target: &Collider<'_>,
) -> Option<battlestar_voxel::voxel_collision::Contact> {
    sweep_chunk(
        Vec3::splat(-0.02),
        Vec3::splat(0.02),
        start,
        end,
        Quat::IDENTITY,
        target,
        1.,
    )
}

#[test]
fn stationary_ship_blocks_fast_debris_and_reflects_relative_velocity() {
    let hull = single();
    let target = collider(&hull);
    let contact = sweep(
        Vec3::new(-100., 0.06, 0.06),
        Vec3::new(100., 0.06, 0.06),
        &target,
    )
    .unwrap();
    assert!((contact.point.x + 0.02).abs() < 0.00001);
    assert_eq!(contact.normal, -Vec3::X);
    assert!((0.0..1.).contains(&contact.toi));
    let bounced = bounce_velocity(Vec3::new(20., 3., 0.), Vec3::ZERO, contact.normal, 0.7);
    assert!((bounced - Vec3::new(-14., 2.7, 0.)).length() < 1e-5);
    assert_eq!(
        bounce_velocity(-Vec3::X, Vec3::ZERO, -Vec3::X, 0.7),
        -Vec3::X
    );
}

#[test]
fn moving_ship_sweeps_into_stationary_debris_and_transfers_momentum() {
    let hull = single();
    let mut target = collider(&hull);
    target.velocity = Vec3::new(-2., 0., 0.);
    let p = Vec3::new(-1., 0.06, 0.06);
    let contact = sweep(p, p, &target).unwrap();
    assert!(contact.toi > 0. && contact.toi < 1.);
    assert_eq!(contact.normal, -Vec3::X);
    let slow = bounce_velocity(Vec3::ZERO, target.velocity, contact.normal, 0.7);
    let fast = bounce_velocity(Vec3::ZERO, target.velocity * 2., contact.normal, 0.7);
    assert!(slow.x < target.velocity.x);
    assert!(fast.x < slow.x * 1.9);
}

#[test]
fn rotated_scaled_actual_hull_uses_world_normals_and_keeps_empty_gap() {
    let mut hull = Voxels::default();
    hull.0.insert([0, 0, 0], [1.; 4]);
    hull.0.insert([0, 4, 0], [1.; 4]);
    let mut target = collider(&hull);
    target.position = Vec3::new(2., 3., -1.);
    target.scale = Vec3::new(2., 0.5, 1.5);
    target.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    let world = |p: Vec3| target.position + target.rotation * (p * target.scale);
    let hit = sweep(
        world(Vec3::new(-1., 0.06, 0.06)),
        world(Vec3::new(1., 0.06, 0.06)),
        &target,
    )
    .unwrap();
    assert!(hit.normal.dot(-Vec3::Y) > 0.999);
    assert!(
        sweep(
            world(Vec3::new(-1., 0.30, 0.06)),
            world(Vec3::new(1., 0.30, 0.06)),
            &target
        )
        .is_none()
    );
}

#[test]
fn initial_overlap_depenetrates_past_adjacent_occupied_cells_and_ignores_exit_motion() {
    let mut hull = single();
    hull.0.insert([1, 0, 0], [1.; 4]);
    let target = collider(&hull);
    let start = Vec3::new(0.06, 0.06, 0.06);
    let contact = sweep(start, start, &target).unwrap();
    assert_eq!(contact.toi, 0.);
    assert!(contact.point.distance(start) > 0.079);
    assert!(sweep(contact.point, contact.point, &target).is_none());
    assert!(sweep(start, start + contact.normal, &target).is_none());
}

#[test]
fn corner_contact_and_invalid_inputs_are_handled() {
    let hull = single();
    let target = collider(&hull);
    let hit = sweep(Vec3::splat(-1.), Vec3::splat(UNIT), &target).unwrap();
    assert!(hit.normal.is_normalized());
    assert!(sweep(Vec3::splat(f32::NAN), Vec3::ZERO, &target).is_none());
    let mut invalid = collider(&hull);
    invalid.scale = Vec3::ZERO;
    assert!(sweep(-Vec3::ONE, Vec3::ONE, &invalid).is_none());
    assert!(
        sweep_chunk(
            Vec3::ONE,
            Vec3::ZERO,
            -Vec3::ONE,
            Vec3::ONE,
            Quat::IDENTITY,
            &target,
            1.
        )
        .is_none()
    );
}

#[test]
fn outward_overlap_still_detects_a_separate_section_ahead() {
    let mut hull = single();
    hull.0.insert([-5, 0, 0], [1.; 4]);
    let target = collider(&hull);
    let contact = sweep(
        Vec3::new(0.01, 0.06, 0.06),
        Vec3::new(-2., 0.06, 0.06),
        &target,
    )
    .unwrap();
    assert!(contact.toi > 0.);
    assert_eq!(contact.normal, Vec3::X);
    assert!((contact.point.x - (-4. * UNIT + 0.02)).abs() < 1e-5);
}

#[test]
fn rotated_debris_bounds_and_mirrored_target_give_finite_surface_normals() {
    let hull = single();
    let mut target = collider(&hull);
    target.scale = Vec3::new(-2., 1., 1.);
    let contact = sweep_chunk(
        Vec3::new(-0.05, -0.01, -0.02),
        Vec3::new(0.05, 0.01, 0.02),
        Vec3::new(1., 0.06, 0.06),
        Vec3::new(-1., 0.06, 0.06),
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        &target,
        1.,
    )
    .unwrap();
    assert!(contact.normal.is_finite() && contact.normal.is_normalized());
    assert!(contact.normal.dot(Vec3::X) > 0.999);
    assert!((contact.point.x - 0.01).abs() < 1e-5);
    assert!(
        sweep(
            Vec3::new(1., 0.06, 0.06),
            Vec3::new(0.021, 0.06, 0.06),
            &target
        )
        .is_none()
    );
}

#[test]
fn head_on_deflection_goes_around_hull_instead_of_retracing_incoming_path() {
    let incoming = Vec3::new(0., 15., 0.);
    let normal = -Vec3::Y;
    let result = deflect_velocity(incoming, Vec3::ZERO, normal, Vec3::new(0.4, -1., 0.));
    assert!(
        result.x > 14.,
        "off-center hit follows its hull-side offset"
    );
    assert!(
        result.dot(normal) > 0.,
        "small outward separation prevents re-entry"
    );
    assert!(
        result.normalize().dot(incoming.normalize()) > -0.1,
        "no reverse reflection"
    );
    assert!((result.length() / incoming.length() - 0.96).abs() < 1e-5);
    let centered = deflect_velocity(incoming, Vec3::ZERO, normal, Vec3::ZERO);
    assert!(centered.is_finite() && centered.cross(normal).length() > 14.);
    assert_eq!(
        centered,
        deflect_velocity(incoming, Vec3::ZERO, normal, Vec3::ZERO)
    );
}

#[test]
fn oblique_deflection_keeps_tangent_and_most_relative_speed() {
    let incoming = Vec3::new(12., 4., 3.);
    let result = deflect_velocity(incoming, Vec3::ZERO, -Vec3::X, -Vec3::X);
    assert!(result.y > 0. && result.z > 0.);
    assert!((result.y / result.z - incoming.y / incoming.z).abs() < 1e-5);
    assert!(result.x < 0. && result.x.abs() < 0.05 * incoming.length());
    assert!(
        result.dot(incoming) > 0.,
        "glancing debris continues with forward momentum"
    );
    assert!((result.length() - incoming.length() * 0.96).abs() < 1e-5);
}

#[test]
fn moving_hulls_transfer_motion_during_deflection_and_outward_debris_is_unchanged() {
    let normal = Vec3::X;
    let target = Vec3::new(8., 0., 0.);
    let slow = deflect_velocity(Vec3::ZERO, target, normal, Vec3::Y);
    let fast = deflect_velocity(Vec3::ZERO, target * 2., normal, Vec3::Y);
    assert!(slow.x > 8. && slow.y > 7.);
    assert!((fast - slow * 2.).length() < 1e-5);
    let outward = target + normal * 5. + Vec3::Y;
    assert_eq!(
        deflect_velocity(outward, target, normal, Vec3::ZERO),
        outward
    );
    assert_eq!(
        deflect_velocity(Vec3::ZERO, Vec3::ZERO, normal, Vec3::ZERO),
        Vec3::ZERO
    );
    assert_eq!(
        deflect_velocity(outward, target, Vec3::ZERO, Vec3::ZERO),
        outward
    );
    assert_eq!(
        deflect_velocity(outward, target, Vec3::splat(f32::NAN), Vec3::ZERO),
        outward
    );
}
