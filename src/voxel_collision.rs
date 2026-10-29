//! Debris motion against occupied ship cells. No ship damage or scoring changes.
use glam::{Quat, Vec3};

use crate::voxel::{UNIT, Voxels};

pub struct Collider<'a> {
    pub voxels: &'a Voxels,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub velocity: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Contact {
    /// Fraction of the finite step, in 0..=1.
    pub toi: f32,
    /// World-space debris pivot at impact, or its corrected pivot for overlap.
    pub point: Vec3,
    /// Outward world-space target surface normal.
    pub normal: Vec3,
}

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
pub fn sweep_chunk(
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

/// Reflect inward relative velocity, with 90% tangent retention. An outward
/// body is unchanged. Target motion is added back after the relative response.
pub fn bounce_velocity(
    incoming: Vec3,
    target_velocity: Vec3,
    normal: Vec3,
    restitution: f32,
) -> Vec3 {
    if !incoming.is_finite()
        || !target_velocity.is_finite()
        || !normal.is_finite()
        || normal.length_squared() < 1e-12
        || !normal.length_squared().is_finite()
        || !restitution.is_finite()
    {
        return incoming;
    }
    let normal = normal.normalize();
    let relative = incoming - target_velocity;
    if !relative.is_finite() {
        return incoming;
    }
    let inward = relative.dot(normal);
    if inward >= 0. {
        return incoming;
    }
    let tangent = relative - normal * inward;
    let result = target_velocity + tangent * 0.9 - normal * inward * restitution.clamp(0., 1.);
    if result.is_finite() { result } else { incoming }
}

/// Redirect inward momentum along the hull instead of reflecting it back along
/// the incoming path. Existing tangent direction takes precedence; head-on hits
/// use the projected impact offset, with a deterministic centered-hit fallback.
/// Retains 96% of relative speed and adds a small outward separation component.
pub fn deflect_velocity(
    incoming: Vec3,
    target_velocity: Vec3,
    normal: Vec3,
    impact_offset: Vec3,
) -> Vec3 {
    if !incoming.is_finite()
        || !target_velocity.is_finite()
        || !normal.is_finite()
        || !normal.length_squared().is_finite()
        || normal.length_squared() < 1e-12
    {
        return incoming;
    }
    let normal = normal.normalize();
    let relative = incoming - target_velocity;
    let speed = relative.length();
    if !relative.is_finite() || !speed.is_finite() || speed < 1e-6 {
        return incoming;
    }
    let inward = relative.dot(normal);
    if inward >= 0. {
        return incoming;
    }
    let tangent = relative - normal * inward;
    let direction = if tangent.length_squared() > speed * speed * 1e-8 {
        tangent.normalize()
    } else {
        let offset = if impact_offset.is_finite() {
            impact_offset
        } else {
            Vec3::ZERO
        };
        let projected = offset - normal * offset.dot(normal);
        if projected.length_squared().is_finite() && projected.length_squared() > 1e-12 {
            projected.normalize()
        } else {
            // The least aligned world axis gives a stable nonzero cross product.
            let absolute = normal.abs();
            let axis = if absolute.x <= absolute.y && absolute.x <= absolute.z {
                Vec3::X
            } else if absolute.y <= absolute.z {
                Vec3::Y
            } else {
                Vec3::Z
            };
            normal.cross(axis).normalize()
        }
    };
    let outward_speed = (-inward * 0.05).min(speed * 0.04);
    let retained = speed * 0.96;
    let tangent_speed = (retained * retained - outward_speed * outward_speed)
        .max(0.)
        .sqrt();
    let result = target_velocity + direction * tangent_speed + normal * outward_speed;
    if result.is_finite() { result } else { incoming }
}
