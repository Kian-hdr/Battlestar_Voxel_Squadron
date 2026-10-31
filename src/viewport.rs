//! Conservative viewport retirement for moving debris. Age is never a removal rule.
use crate::{
    voxel::UNIT,
    voxel_collision::{Collider, Contact},
};
use glam::{Mat4, Quat, Vec3};

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebrisMotion {
    pub position: Vec3,
    pub velocity: Vec3,
}

/// No force, steering, damping or delayed relaunch acts on unobstructed debris.
pub fn advance_debris(motion: DebrisMotion, dt: f32) -> DebrisMotion {
    DebrisMotion {
        position: motion.position + motion.velocity * dt,
        ..motion
    }
}

/// Subdivision changes the piece's pivot, never its translational momentum.
pub fn split_debris_motion(
    parent: DebrisMotion,
    rotation: Quat,
    local_offset: Vec3,
) -> DebrisMotion {
    DebrisMotion {
        position: parent.position + rotation * local_offset,
        velocity: parent.velocity,
    }
}

/// Choose the impact/hull impulse once. Any escape-cone correction also happens
/// here, at launch, so the resulting flight remains ballistic until a collision.
pub fn launch_debris_velocity(
    impact_velocity: Vec3,
    hull_velocity: Vec3,
    spread_velocity: Vec3,
    speed: f32,
    forward: Vec3,
    escape_tangent: f32,
) -> Vec3 {
    let forward = forward.normalize_or_zero();
    let raw = impact_velocity.normalize_or_zero() * speed + hull_velocity * 0.35 + spread_velocity;
    let velocity = receding_velocity(raw, forward, 8.);
    let depth = velocity.dot(forward);
    let lateral = velocity - forward * depth;
    let direction = if lateral.length_squared() > 1e-8 {
        lateral.normalize()
    } else {
        camera_plane_fallback(forward)
    };
    let lateral = direction
        * lateral
            .length()
            .max(depth * (escape_tangent.max(0.) + 0.35));
    (forward * depth + lateral).normalize_or_zero()
        * velocity.length().clamp(26., DEBRIS_SPEED_LIMIT)
}

fn camera_plane_fallback(forward: Vec3) -> Vec3 {
    let axis = if forward.x.abs() < 0.8 {
        Vec3::X
    } else {
        Vec3::Y
    };
    forward.cross(axis).normalize_or_zero()
}

fn departing_velocity(velocity: Vec3, forward: Vec3, escape_tangent: f32) -> bool {
    let depth = velocity.dot(forward);
    let lateral = velocity - forward * depth;
    velocity.length_squared() >= 1.
        && depth >= -1e-5
        && lateral.length() > depth.max(0.) * (escape_tangent.max(0.) + 0.02)
}

/// A genuine contact may choose a new departure once. Prefer the ordinary
/// bounded material deflection; if it would stall inside the camera cone,
/// choose a bounded slide in the camera/hull tangent instead. No later force
/// is needed to restart the fragment.
pub fn contact_departure_velocity(
    velocity: Vec3,
    target_velocity: Vec3,
    normal: Vec3,
    forward: Vec3,
    escape_tangent: f32,
) -> Vec3 {
    let forward = forward.normalize_or_zero();
    let normal = normal.normalize_or_zero();
    let ordinary = receding_contact_velocity(velocity, target_velocity, normal, forward);
    if departing_velocity(ordinary, forward, escape_tangent) {
        return ordinary;
    }
    let required = target_velocity.dot(normal);
    let tangent_normal = normal - forward * normal.dot(forward);
    let speed = velocity
        .length()
        .max(target_velocity.length())
        .clamp(8., DEBRIS_SPEED_LIMIT);
    let alignment = normal.dot(forward);
    if required > 0. && alignment > 1e-5 {
        // A rear hull moving into the background needs a positive depth speed;
        // a camera-plane-only slide would stall against it every frame.
        let depth = required / alignment;
        let slope = escape_tangent.max(0.) + 0.15;
        let departure_speed = speed.max(depth * (1. + slope * slope).sqrt());
        if departure_speed <= DEBRIS_SPEED_LIMIT {
            let cross = forward.cross(normal);
            let line = if cross.length_squared() > 1e-12 {
                let line = cross.normalize();
                let along = velocity.dot(line);
                let sign = if along.abs() > 1e-5 {
                    along.signum()
                } else if camera_plane_fallback(forward).dot(line) < 0. {
                    -1.
                } else {
                    1.
                };
                line * sign
            } else {
                camera_plane_fallback(forward)
            };
            let lateral = (departure_speed * departure_speed - depth * depth)
                .max(0.)
                .sqrt();
            let mut departure = forward * depth + line * lateral;
            let deficit = required - departure.dot(normal);
            if deficit > 0. {
                departure += forward * (deficit / alignment);
            }
            if departure.length_squared() <= DEBRIS_SPEED_LIMIT * DEBRIS_SPEED_LIMIT + 1e-4 {
                return departure.clamp_length_max(DEBRIS_SPEED_LIMIT);
            }
        }
    }
    if tangent_normal.length_squared() > 1e-6 {
        let base = tangent_normal * (required.max(0.) / tangent_normal.length_squared());
        if base.length_squared() <= DEBRIS_SPEED_LIMIT * DEBRIS_SPEED_LIMIT {
            let speed = speed.max(base.length());
            let line = forward.cross(normal).normalize_or_zero();
            let along = velocity.dot(line);
            let sign = if along.abs() > 1e-5 {
                along.signum()
            } else if camera_plane_fallback(forward).dot(line) < 0. {
                -1.
            } else {
                1.
            };
            let room = (speed * speed - base.length_squared()).max(0.).sqrt();
            let slide = base + line * (sign * room);
            if slide.length_squared() >= 1. {
                return receding_velocity(slide, forward, 0.);
            }
        }
    } else if required <= 0. {
        let lateral = velocity - forward * velocity.dot(forward);
        let direction = if lateral.length_squared() > 1e-8 {
            lateral.normalize()
        } else {
            camera_plane_fallback(forward)
        };
        return direction * speed;
    }
    // A moving frontal hull can make a bounded camera-away response impossible.
    // Retain a nonzero escape for the next step, but the caller must hold its
    // current safe contact rather than advance an inward material trajectory.
    let lateral = velocity - forward * velocity.dot(forward);
    let direction = if lateral.length_squared() > 1e-8 {
        lateral.normalize()
    } else {
        camera_plane_fallback(forward)
    };
    direction * speed
}

pub const DEBRIS_SPEED_LIMIT: f32 = 36.;

/// Preserve lateral fracture motion while preventing an incoming shot or
/// reflected velocity from carrying debris toward the viewer. The total speed
/// is bounded without discarding the downward escape direction at high speed.
pub fn receding_velocity(velocity: Vec3, forward: Vec3, minimum_speed: f32) -> Vec3 {
    let forward = forward.normalize_or_zero();
    let velocity = velocity.clamp_length_max(DEBRIS_SPEED_LIMIT);
    let minimum_speed = minimum_speed.clamp(0., DEBRIS_SPEED_LIMIT);
    let depth = velocity.dot(forward);
    if depth >= minimum_speed {
        velocity
    } else {
        let lateral = velocity - forward * depth;
        let room = (DEBRIS_SPEED_LIMIT * DEBRIS_SPEED_LIMIT - minimum_speed * minimum_speed).sqrt();
        forward * minimum_speed + lateral.clamp_length_max(room)
    }
}

/// A hull contact may stop depth motion, but must not reflect a fragment toward
/// the viewer. Keep the surface constraint by sliding along its camera tangent.
pub fn receding_contact_velocity(
    velocity: Vec3,
    target_velocity: Vec3,
    normal: Vec3,
    forward: Vec3,
) -> Vec3 {
    let forward = forward.normalize_or_zero();
    let normal = normal.normalize_or_zero();
    let velocity = receding_velocity(velocity, forward, 0.);
    let required = target_velocity.dot(normal);
    let deficit = required - velocity.dot(normal);
    if deficit <= 0. {
        return velocity;
    }

    // First try the direct surface projection. Trim its unconstrained tangent
    // component if necessary; do not scale away the required normal speed.
    let projected = velocity + normal * deficit;
    if projected.dot(forward) >= 0. && required.abs() <= DEBRIS_SPEED_LIMIT {
        let base = normal * required;
        let room = (DEBRIS_SPEED_LIMIT * DEBRIS_SPEED_LIMIT - base.length_squared())
            .max(0.)
            .sqrt();
        let candidate = base + (projected - base).clamp_length_max(room);
        if candidate.dot(forward) >= 0. {
            return receding_velocity(candidate, forward, 0.);
        }
    }

    // The camera-depth plane and hull plane intersect in a line. Only the
    // component along that line is free to spend the remaining speed budget.
    let tangent_normal = normal - forward * normal.dot(forward);
    let tangent_squared = tangent_normal.length_squared();
    if tangent_squared > 1e-6 {
        let base = tangent_normal * (required / tangent_squared);
        if base.length_squared() <= DEBRIS_SPEED_LIMIT * DEBRIS_SPEED_LIMIT {
            let line = forward.cross(normal).normalize_or_zero();
            let room = (DEBRIS_SPEED_LIMIT * DEBRIS_SPEED_LIMIT - base.length_squared())
                .max(0.)
                .sqrt();
            let candidate = base + line * velocity.dot(line).clamp(-room, room);
            return receding_velocity(candidate, forward, 0.);
        }
    }

    if required <= 0. {
        let line = forward.cross(normal).normalize_or_zero();
        return line * velocity.dot(line);
    }
    // A nearly frontal surface moving toward the camera may make the two
    // constraints infeasible below the speed limit. The caller holds the last
    // safe contact instead of advancing an inward or explosive response.
    Vec3::ZERO
}

/// Resolve an initial overlap through the occupied union in the background
/// direction rather than choosing a shorter exit toward the camera. This is
/// only depenetration; ordinary finite-step occupied-cell sweeps remain exact.
pub fn background_overlap_exit(
    local_min: Vec3,
    local_max: Vec3,
    start: Vec3,
    rotation: Quat,
    target: &Collider<'_>,
    forward: Vec3,
) -> Option<Contact> {
    if !local_min.is_finite()
        || !local_max.is_finite()
        || !start.is_finite()
        || !rotation.is_finite()
        || rotation.length_squared() < 1e-12
        || !target.position.is_finite()
        || !target.rotation.is_finite()
        || target.rotation.length_squared() < 1e-12
        || !target.scale.is_finite()
        || target.scale.abs().min_element() < 1e-6
        || !forward.is_finite()
        || forward.length_squared() < 1e-12
        || !local_min.cmple(local_max).all()
    {
        return None;
    }
    let forward = forward.normalize();
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
    let origin = (inverse * (start - target.position)) / target.scale;
    let direction = (inverse * forward) / target.scale;
    let mut intervals = Vec::new();
    for p in target.voxels.0.keys() {
        let low = Vec3::from_array(p.map(|v| v as f32 * UNIT)) - offset_max;
        let high = Vec3::from_array(p.map(|v| (v as f32 + 1.) * UNIT)) - offset_min;
        let mut near = f32::NEG_INFINITY;
        let mut far = f32::INFINITY;
        let mut normal = Vec3::ZERO;
        let mut miss = false;
        for axis in 0..3 {
            if direction[axis].abs() < 1e-12 {
                if origin[axis] < low[axis] || origin[axis] > high[axis] {
                    miss = true;
                    break;
                }
            } else {
                let a = (low[axis] - origin[axis]) / direction[axis];
                let b = (high[axis] - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                let end = a.max(b);
                if end < far {
                    far = end;
                    normal = Vec3::ZERO;
                    normal[axis] = direction[axis].signum();
                }
            }
        }
        if !miss && near <= far && far >= 0. && far.is_finite() {
            intervals.push((near, far, normal));
        }
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut reach = 0_f32;
    let mut normal = Vec3::ZERO;
    for (near, far, candidate) in intervals {
        if near > reach + 1e-6 {
            break;
        }
        if far >= reach {
            reach = far;
            normal = candidate;
        }
    }
    (normal != Vec3::ZERO).then(|| Contact {
        toi: 0.,
        point: start + forward * (reach + UNIT * 0.001),
        normal: target_rotation * (normal * target.scale.signum()),
    })
}

/// A broad, soft spatial shadow behind the combat plane. Retained hulls stay
/// opaque, with cool reflected light keeping their shape legible in shadow.
pub fn debris_shadow_tint(color: [f32; 4], depth: f32) -> [f32; 4] {
    let t = ((depth - 1.) / 8.).clamp(0., 1.);
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
    fn unobstructed_debris_keeps_velocity_and_a_collinear_path() {
        let start = DebrisMotion {
            position: Vec3::new(1., 2., -30.),
            velocity: Vec3::new(3., 22., -10.),
        };
        let mut motion = start;
        for _ in 0..16 {
            motion = advance_debris(motion, 0.125);
        }
        assert_eq!(
            motion.velocity, start.velocity,
            "an unforced fragment must not turn or accelerate during flight"
        );
        assert!((motion.position - Vec3::new(7., 46., -50.)).length() < 1e-5);
    }

    #[test]
    fn secondary_fracture_preserves_translation_without_a_delayed_impulse() {
        let parent = DebrisMotion {
            position: Vec3::new(1., 2., -30.),
            velocity: Vec3::new(3., 22., -10.),
        };
        let child = split_debris_motion(
            parent,
            Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            Vec3::X,
        );
        assert_eq!(
            child.velocity, parent.velocity,
            "subdivision must not redirect the child's translation"
        );
        assert!((child.position - Vec3::new(1., 3., -30.)).length() < 1e-5);
    }

    #[test]
    fn fast_ballistic_launch_recedes_and_leaves_tall_and_wide_views_without_steering() {
        let camera_position = Vec3::new(0., -25., 28.);
        let camera_forward = (Vec3::new(0., -1.5, 0.) - camera_position).normalize();
        for aspect in [0.4_f32, 1.6, 3.] {
            let fov = battle_fov(aspect);
            let escape_tangent = (fov * 0.5).tan() * (1. + aspect * aspect).sqrt();
            let view = Mat4::look_at_rh(camera_position, Vec3::new(0., -1.5, 0.), Vec3::Y);
            let clip = Mat4::perspective_infinite_reverse_rh(fov, aspect, 0.1) * view;
            for impact in [Vec3::Y, -Vec3::Y, camera_forward, Vec3::ZERO] {
                for hull in [Vec3::new(2., -5., 0.), Vec3::ZERO] {
                    for spread in [Vec3::X * 0.5, Vec3::ZERO] {
                        let velocity = launch_debris_velocity(
                            impact,
                            hull,
                            spread,
                            29.,
                            camera_forward,
                            escape_tangent,
                        );
                        assert!(velocity.dot(camera_forward) > 0.);
                        assert!((26. - 1e-4..=36.0001).contains(&velocity.length()));
                        let mut motion = DebrisMotion {
                            position: Vec3::ZERO,
                            velocity,
                        };
                        let mut exited = false;
                        for _ in 0..720 {
                            let previous = motion;
                            motion = advance_debris(motion, 1. / 120.);
                            assert_eq!(motion.velocity, velocity);
                            assert!(
                                (motion.position - previous.position).dot(camera_forward) >= -1e-5
                            );
                            if !sphere_visible(clip, motion.position, 1.) {
                                exited = true;
                                break;
                            }
                        }
                        assert!(
                            exited,
                            "launch should clear the view in under six seconds without a later force: aspect={aspect} impact={impact:?} hull={hull:?} spread={spread:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn ordinary_impacts_clear_the_combat_view_quickly_at_constant_velocity() {
        let camera = Vec3::new(0., -25., 28.);
        let forward = (Vec3::new(0., -1.5, 0.) - camera).normalize();
        let fov = battle_fov(1.6);
        let tangent = (fov * 0.5).tan() * (1. + 1.6_f32.powi(2)).sqrt();
        let clip = Mat4::perspective_infinite_reverse_rh(fov, 1.6, 0.1)
            * Mat4::look_at_rh(camera, Vec3::new(0., -1.5, 0.), Vec3::Y);
        for y in [-6., 0., 5.] {
            for impact in [Vec3::Y, -Vec3::Y] {
                let velocity = launch_debris_velocity(
                    impact,
                    Vec3::new(2., -5., 0.),
                    Vec3::ZERO,
                    29.,
                    forward,
                    tangent,
                );
                let mut motion = DebrisMotion {
                    position: Vec3::new(0., y, 0.),
                    velocity,
                };
                let mut exited = false;
                for _ in 0..144 {
                    motion = advance_debris(motion, 1. / 120.);
                    if !sphere_visible(clip, motion.position, 1.5) {
                        exited = true;
                        break;
                    }
                }
                assert!(
                    exited,
                    "ordinary unobstructed impacts should clear the current combat view within1.2s"
                );
            }
        }
    }

    #[test]
    fn a_moving_rear_hull_gets_a_material_safe_ballistic_departure() {
        let forward = -Vec3::Z;
        let target_velocity = forward * 10.;
        let velocity =
            contact_departure_velocity(forward * 10., target_velocity, forward, forward, 0.5);
        assert!(departing_velocity(velocity, forward, 0.5));
        assert!(
            (velocity - target_velocity).dot(forward) >= -1e-4,
            "do not store an inward slide that repeatedly stalls against a moving rear hull"
        );
        assert!(velocity.length() <= 36.0001);
    }

    #[test]
    fn frontal_contact_selects_a_nonzero_one_time_departure_instead_of_a_later_relaunch() {
        let forward = -Vec3::Z;
        for normal in [Vec3::Z, -Vec3::Z, Vec3::X] {
            let velocity =
                contact_departure_velocity(forward * 28., Vec3::ZERO, normal, forward, 0.5);
            assert!(departing_velocity(velocity, forward, 0.5));
            assert!((velocity - Vec3::ZERO).dot(normal) >= -1e-4);
            assert!(velocity.length() <= 36.0001);
            let motion = DebrisMotion {
                position: Vec3::new(0., 0., -30.),
                velocity,
            };
            assert_eq!(advance_debris(motion, 0.5).velocity, velocity);
        }
        let normal = (Vec3::Z + Vec3::Y * 0.02).normalize();
        let velocity =
            contact_departure_velocity(Vec3::new(0., 2., 12.), Vec3::Z * 4., normal, forward, 0.5);
        assert!(velocity.length() > 1. && velocity.length() <= 36.0001);
        assert!(velocity.dot(forward) >= -1e-5);
        assert!(
            (velocity - Vec3::Z * 4.).dot(normal) < -1e-4,
            "an infeasible moving frontal constraint must still trigger the caller's safe-contact hold"
        );
    }

    #[test]
    fn initial_overlap_exits_into_background_instead_of_toward_camera() {
        use crate::voxel::Voxels;
        let mut cells = Voxels::default();
        cells.0.insert([0; 3], [1.; 4]);
        let target = Collider {
            voxels: &cells,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            velocity: Vec3::ZERO,
        };
        let forward = -Vec3::Z;
        let start = Vec3::new(0.06, 0.06, 0.13);
        let bounds = Vec3::splat(0.02);
        let hit = background_overlap_exit(-bounds, bounds, start, Quat::IDENTITY, &target, forward)
            .unwrap();
        assert!(
            (hit.point - start).dot(forward) > 0.,
            "the old nearest-face exit moves this overlapping fragment toward the camera"
        );
        assert!(hit.normal.dot(forward) > 0.);
        assert!(
            crate::voxel_collision::sweep_chunk(
                -bounds,
                bounds,
                hit.point,
                hit.point,
                Quat::IDENTITY,
                &target,
                0.
            )
            .is_none()
        );
    }

    #[test]
    fn background_overlap_exit_clears_connected_cells_without_crossing_an_empty_gap() {
        use crate::voxel::Voxels;
        let mut cells = Voxels::default();
        for z in [-4, 0, 1] {
            cells.0.insert([0, 0, z], [1.; 4]);
        }
        let target = Collider {
            voxels: &cells,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            velocity: Vec3::ZERO,
        };
        let bounds = Vec3::splat(0.02);
        let hit = background_overlap_exit(
            -bounds,
            bounds,
            Vec3::new(0.06, 0.06, 0.25),
            Quat::IDENTITY,
            &target,
            -Vec3::Z,
        )
        .unwrap();
        assert!(
            hit.point.z < -0.02 && hit.point.z > -0.1,
            "exit the joined foreground cells into their first air gap"
        );
        assert!(
            crate::voxel_collision::sweep_chunk(
                -bounds,
                bounds,
                hit.point,
                hit.point,
                Quat::IDENTITY,
                &target,
                0.
            )
            .is_none()
        );
    }

    #[test]
    fn background_exit_uses_the_real_rotated_signed_scale_hull() {
        use crate::voxel::Voxels;
        let mut cells = Voxels::default();
        cells.block([0; 3], [2; 3], [1.; 4]);
        let forward = Vec3::new(0., 0.64, -0.77).normalize();
        for scale in [Vec3::ONE, Vec3::new(-1.2, 0.8, 1.6)] {
            let target = Collider {
                voxels: &cells,
                position: Vec3::new(2., -1., 0.4),
                rotation: Quat::from_rotation_x(0.2) * Quat::from_rotation_y(0.4),
                scale,
                velocity: Vec3::ZERO,
            };
            let start = target.position + target.rotation * (Vec3::splat(0.15) * scale);
            let rotation = Quat::from_rotation_z(0.3);
            let bounds = Vec3::splat(0.02);
            let hit = background_overlap_exit(-bounds, bounds, start, rotation, &target, forward)
                .unwrap();
            assert!((hit.point - start).dot(forward) > 0.);
            assert!(hit.normal.dot(forward) > 0.);
            assert!(
                crate::voxel_collision::sweep_chunk(
                    -bounds, bounds, hit.point, hit.point, rotation, &target, 0.
                )
                .is_none()
            );
        }
    }

    #[test]
    fn bounded_contact_response_does_not_amplify_a_near_frontal_collision() {
        let forward = -Vec3::Z;
        let normal = (Vec3::Z + Vec3::Y * 0.02).normalize();
        let response =
            receding_contact_velocity(Vec3::new(0., 2., 12.), Vec3::Z * 4., normal, forward);
        assert!(
            response.length() <= 36.0001,
            "near-parallel tangent division must not launch fragments at hundreds of units per second: {response:?}"
        );
        assert!(response.dot(forward) >= -1e-5);
    }

    #[test]
    fn bounded_feasible_contact_keeps_surface_clearance_and_lateral_slide() {
        let forward = -Vec3::Z;
        let normal = (Vec3::Z + Vec3::Y * 0.3).normalize();
        let target_velocity = Vec3::Z * 9.;
        let response =
            receding_contact_velocity(Vec3::new(30., 2., 12.), target_velocity, normal, forward);
        assert!(response.length() <= 36.0001);
        assert!(response.dot(forward) >= -1e-5);
        assert!((response - target_velocity).dot(normal) >= -1e-4);
        assert!(
            response.x > 0. && response.y >= 29.999,
            "keep the feasible surface component and trim only excess tangent speed"
        );
    }

    #[test]
    fn receding_contact_response_respects_rotated_surface_constraints() {
        let forward = Vec3::new(0., 0.64, -0.77).normalize();
        for bank in [-0.3_f32, 0., 0.3] {
            let rotation = Quat::from_rotation_y(bank) * Quat::from_rotation_z(0.2);
            for normal in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
                let normal = rotation * normal;
                let target_velocity = Vec3::new(1.5, -2., 0.4);
                let response = crate::voxel_collision::deflect_velocity(
                    forward * 12. + Vec3::X * 2.,
                    target_velocity,
                    normal,
                    Vec3::X,
                );
                let slid = receding_contact_velocity(response, target_velocity, normal, forward);
                assert!(slid.dot(forward) >= -1e-5);
                assert!((slid - target_velocity).dot(normal) >= -1e-5);
                assert!(slid.length() < 40.);
            }
        }
    }

    #[test]
    fn incoming_and_reflected_fragments_recede_without_growing_in_perspective() {
        let forward = -Vec3::Z;
        let velocity = receding_velocity(Vec3::new(3., 2., 14.), forward, 8.);
        assert_eq!(velocity.x, 3.);
        assert_eq!(velocity.y, 2.);
        assert!(velocity.dot(forward) >= 8.);
        let mut p = Vec3::new(0., 0., -30.);
        let mut previous_radius = 1. / p.dot(forward);
        for _ in 0..120 {
            p += velocity / 120.;
            let radius = 1. / p.dot(forward);
            assert!(radius <= previous_radius);
            previous_radius = radius;
        }
        let normal = Vec3::new(0., 0.8, 0.6);
        let slid = receding_contact_velocity(Vec3::new(2., 3., 12.), Vec3::ZERO, normal, forward);
        assert!(slid.dot(forward) >= -1e-5);
        assert!(slid.dot(normal) >= -1e-5);
    }

    #[test]
    fn spatial_shadow_preserves_opaque_visible_hulls() {
        let white = [1.; 4];
        assert_eq!(debris_shadow_tint(white, 0.), white);
        let middle = debris_shadow_tint(white, 5.);
        let shadow = debris_shadow_tint(white, 100.);
        assert!(middle[0] < 1. && middle[0] > shadow[0]);
        assert!(shadow[0] > 0.2 && shadow[2] > shadow[0]);
        assert_eq!(shadow[3], 1.);
        assert_eq!(shadow, debris_shadow_tint(white, 200.));
    }
}
