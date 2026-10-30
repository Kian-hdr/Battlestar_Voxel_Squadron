//! Differential coverage against the v0.4.0 occupied-cell collision algorithm.
use battlestar_voxel::{
    voxel::{self, UNIT, Voxels},
    voxel_collision::{Collider, Contact, sweep_chunk},
};
use glam::{Quat, Vec3};

fn expanded(p: [i32; 3], offset_min: Vec3, offset_max: Vec3) -> (Vec3, Vec3) {
    let low = Vec3::from_array(p.map(|v| v as f32 * UNIT));
    (low - offset_max, low + Vec3::splat(UNIT) - offset_min)
}

fn contains(p: Vec3, low: Vec3, high: Vec3) -> bool {
    p.cmpge(low).all() && p.cmple(high).all()
}

/// Sweep a conservative debris box against the target's actual occupied boxes.
/// Debris bounds are relative to its pivot. Both pivot endpoints are world-space;
/// target velocity is integrated across `dt`. Rotations remain fixed in the step.
/// Initial overlap returns a pivot outside the connected union of expanded cells;
/// overlap already moving outward is ignored to prevent repeated reflections.
#[allow(clippy::too_many_arguments)]
fn reference_sweep(
    local_min: Vec3,
    local_max: Vec3,
    start: Vec3,
    end: Vec3,
    rotation: Quat,
    target: &Collider<'_>,
    dt: f32,
) -> Option<Contact> {
    if !local_min.is_finite()
        || !local_max.is_finite()
        || !start.is_finite()
        || !end.is_finite()
        || !(end - start).is_finite()
        || !target.position.is_finite()
        || !target.velocity.is_finite()
        || !target.scale.is_finite()
        || !rotation.is_finite()
        || !target.rotation.is_finite()
        || !rotation.length_squared().is_finite()
        || !target.rotation.length_squared().is_finite()
        || rotation.length_squared() < 1e-12
        || target.rotation.length_squared() < 1e-12
        || target.scale.abs().min_element() < 1e-6
        || !dt.is_finite()
        || dt < 0.
        || !local_min.cmple(local_max).all()
        || target.voxels.0.is_empty()
    {
        return None;
    }
    let target_rotation = target.rotation.normalize();
    let inverse = target_rotation.conjugate();
    let rotation = rotation.normalize();
    let mut offset_min = Vec3::splat(f32::INFINITY);
    let mut offset_max = Vec3::splat(f32::NEG_INFINITY);
    for corner in 0..8 {
        let p = Vec3::new(
            if corner & 1 == 0 {
                local_min.x
            } else {
                local_max.x
            },
            if corner & 2 == 0 {
                local_min.y
            } else {
                local_max.y
            },
            if corner & 4 == 0 {
                local_min.z
            } else {
                local_max.z
            },
        );
        let offset = (inverse * (rotation * p)) / target.scale;
        offset_min = offset_min.min(offset);
        offset_max = offset_max.max(offset);
    }
    let a = (inverse * (start - target.position)) / target.scale;
    let b = (inverse * (end - target.position - target.velocity * dt)) / target.scale;
    if !a.is_finite() || !b.is_finite() || !offset_min.is_finite() || !offset_max.is_finite() {
        return None;
    }
    let delta = b - a;
    if !delta.is_finite() {
        return None;
    }
    // All slab normals are single local axes. Their inverse-transpose transform
    // only needs the scale sign, avoiding underflow for very large valid scales.
    let world_normal = |normal: Vec3| target_rotation * (normal * target.scale.signum());
    let overlaps = target.voxels.0.keys().any(|p| {
        let (low, high) = expanded(*p, offset_min, offset_max);
        contains(a, low, high)
    });
    if overlaps {
        // Find the nearest exit of the UNION, not just the nearest individual
        // voxel. Adjacent occupied cells must not depenetrate into one another.
        let mut best_distance = f32::INFINITY;
        let mut best_normal = Vec3::ZERO;
        for axis in 0..3 {
            for sign in [-1., 1.] {
                let mut reach = 0_f32;
                for _ in 0..=target.voxels.0.len() {
                    let previous = reach;
                    for p in target.voxels.0.keys() {
                        let (low, high) = expanded(*p, offset_min, offset_max);
                        if (0..3).any(|i| i != axis && (a[i] < low[i] || a[i] > high[i])) {
                            continue;
                        }
                        let (near, far) = if sign > 0. {
                            (low[axis] - a[axis], high[axis] - a[axis])
                        } else {
                            (a[axis] - high[axis], a[axis] - low[axis])
                        };
                        if near <= reach + 1e-6 && far >= 0. {
                            reach = reach.max(far);
                        }
                    }
                    if reach == previous {
                        break;
                    }
                }
                let distance = reach * target.scale[axis].abs();
                if distance < best_distance {
                    best_distance = distance;
                    best_normal = Vec3::ZERO;
                    best_normal[axis] = sign;
                }
            }
        }
        let normal = world_normal(best_normal);
        if !best_distance.is_finite() || !normal.is_finite() {
            return None;
        }
        let relative_motion = end - start - target.velocity * dt;
        if relative_motion.dot(normal) <= 1e-6 {
            return Some(Contact {
                toi: 0.,
                point: start + normal * (best_distance + UNIT * 0.001),
                normal,
            });
        }
        // Leaving this overlap must not hide a different occupied section
        // farther along the finite step. Initial boxes have no entering slab.
    }
    let mut closest = f32::INFINITY;
    let mut normal = Vec3::ZERO;
    for p in target.voxels.0.keys() {
        let (low, high) = expanded(*p, offset_min, offset_max);
        let mut enter = 0_f32;
        let mut exit = 1_f32;
        let mut candidate_normal = Vec3::ZERO;
        for axis in 0..3 {
            if delta[axis].abs() < 1e-12 {
                if a[axis] < low[axis] || a[axis] > high[axis] {
                    exit = -1.;
                    break;
                }
            } else {
                let near = (low[axis] - a[axis]) / delta[axis];
                let far = (high[axis] - a[axis]) / delta[axis];
                let entry = near.min(far);
                if entry > enter {
                    enter = entry;
                    candidate_normal = Vec3::ZERO;
                    candidate_normal[axis] = if delta[axis] > 0. { -1. } else { 1. };
                }
                exit = exit.min(near.max(far));
            }
        }
        if enter <= exit && enter < closest && candidate_normal != Vec3::ZERO {
            closest = enter;
            normal = candidate_normal;
        }
    }
    closest.is_finite().then(|| Contact {
        toi: closest,
        point: start.lerp(end, closest),
        normal: world_normal(normal),
    })
}

#[derive(Clone, Copy)]
struct Sweep {
    min: Vec3,
    max: Vec3,
    start: Vec3,
    end: Vec3,
    rotation: Quat,
    dt: f32,
}
fn check(s: Sweep, target: &Collider<'_>) {
    let a = reference_sweep(s.min, s.max, s.start, s.end, s.rotation, target, s.dt);
    let b = sweep_chunk(s.min, s.max, s.start, s.end, s.rotation, target, s.dt);
    match (a, b) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            assert_eq!(a.toi, b.toi, "time of impact");
            assert_eq!(a.point, b.point, "contact point");
            assert_eq!(a.normal, b.normal, "normal");
        }
        pair => panic!("contact existence differs: {pair:?}"),
    }
}
fn random(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 40) as f32 / 16_777_216.
}
fn vector(seed: &mut u64, magnitude: f32) -> Vec3 {
    Vec3::new(random(seed), random(seed), random(seed)) * magnitude * 2. - Vec3::splat(magnitude)
}
#[test]
fn seeded_sweeps_match_original_with_rotations_signed_scales_and_disconnected_hulls() {
    let mut disconnected = Voxels::default();
    disconnected.block([-4, -2, -1], [-1, 2, 1], [1.; 4]);
    disconnected.block([3, -1, -2], [5, 1, 2], [1.; 4]);
    let mut seed = 0x511a_1024;
    for hull in [
        disconnected,
        voxel::raider(true),
        voxel::raider(false),
        voxel::viper(true),
    ] {
        for case in 0..500 {
            let position = vector(&mut seed, 3.);
            let rotation = Quat::from_scaled_axis(vector(&mut seed, 2.));
            let scale =
                vector(&mut seed, 2.).map(|v| if v < 0. { v.min(-0.2) } else { v.max(0.2) });
            let velocity = vector(&mut seed, 2.);
            let target = Collider {
                voxels: &hull,
                position,
                rotation,
                scale,
                velocity,
            };
            let a = vector(&mut seed, if case % 2 == 0 { 0.3 } else { 4. });
            let b = a + vector(&mut seed, 2.);
            let dt = 1. / 120.;
            check(
                Sweep {
                    min: -Vec3::splat(0.01 + random(&mut seed) * 0.2),
                    max: Vec3::splat(0.01 + random(&mut seed) * 0.2),
                    start: position + rotation * (a * scale),
                    end: position + rotation * (b * scale) + velocity * dt,
                    rotation: Quat::from_scaled_axis(vector(&mut seed, 1.)),
                    dt,
                },
                &target,
            );
        }
    }
}
#[test]
fn overlap_exit_and_reentry_into_a_separate_section_match_original() {
    let mut hull = Voxels::default();
    hull.block([-3, -1, -1], [1, 1, 1], [1.; 4]);
    hull.block([4, -1, -1], [5, 1, 1], [1.; 4]);
    let target = Collider {
        voxels: &hull,
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        velocity: Vec3::ZERO,
    };
    for x in [-0.3, -0.12, 0., 0.12, 0.23999] {
        for end in [-2., -0.4, 0.1, 0.5, 2.] {
            check(
                Sweep {
                    min: Vec3::splat(-0.02),
                    max: Vec3::splat(0.02),
                    start: Vec3::new(x, 0.06, 0.06),
                    end: Vec3::new(end, 0.06, 0.06),
                    rotation: Quat::IDENTITY,
                    dt: 1. / 120.,
                },
                &target,
            );
        }
    }
}
#[test]
#[ignore = "explicit release-mode CPU benchmark; not a frame-rate claim"]
fn benchmark_galactica_short_sweeps() {
    use std::{hint::black_box, time::Instant};
    let hull = voxel::galactica(true);
    let target = Collider {
        voxels: &hull,
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        velocity: Vec3::ZERO,
    };
    let cases: Vec<_> = (0..1024)
        .map(|i| {
            let x = (i % 32) as f32 * 0.25 - 4.;
            let y = (i / 32) as f32 * 0.35 - 5.6;
            Sweep {
                min: Vec3::splat(-0.025),
                max: Vec3::splat(0.025),
                start: Vec3::new(x, y, 0.8),
                end: Vec3::new(x + 0.08, y + 0.04, 0.73),
                rotation: Quat::from_rotation_y(0.2),
                dt: 1. / 120.,
            }
        })
        .collect();
    for s in &cases {
        check(*s, &target);
    }
    let start = Instant::now();
    for s in &cases {
        black_box(reference_sweep(
            s.min,
            s.max,
            s.start,
            s.end,
            s.rotation,
            black_box(&target),
            s.dt,
        ));
    }
    let original = start.elapsed();
    let start = Instant::now();
    for s in &cases {
        black_box(sweep_chunk(
            s.min,
            s.max,
            s.start,
            s.end,
            s.rotation,
            black_box(&target),
            s.dt,
        ));
    }
    let optimized = start.elapsed();
    let ratio = original.as_secs_f64() / optimized.as_secs_f64();
    println!(
        "Galactica {} cells / {} sweeps: original {original:?}, optimized {optimized:?}, speedup {ratio:.2}x",
        hull.0.len(),
        cases.len()
    );
    assert!(
        ratio > 2.,
        "target at least 2x faster for short Galactica sweeps"
    );
}

#[test]
fn exact_voxel_boundaries_and_extreme_finite_positions_preserve_reference_behavior() {
    let mut hull = Voxels::default();
    hull.block([-2, -2, -2], [2, 2, 2], [1.; 4]);
    let target = Collider {
        voxels: &hull,
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
        velocity: Vec3::ZERO,
    };
    for x in [-1e30, -10001., -0.24, -0.12, 0., 0.12, 0.24, 10001., 1e30] {
        for y in [-0.24, 0., 0.36] {
            check(
                Sweep {
                    min: Vec3::ZERO,
                    max: Vec3::ZERO,
                    start: Vec3::new(x, y, 0.),
                    end: Vec3::new(-x, y, 0.),
                    rotation: Quat::IDENTITY,
                    dt: 1. / 120.,
                },
                &target,
            );
        }
    }
}
