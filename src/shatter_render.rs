//! Destructible occupied voxel bodies. Meshes are derived surfaces, never the damage source.
use crate::Session;
use battlestar_voxel::{
    combat_plane::{visual_position, visual_velocity},
    simulation::{Phase, Point},
    viewport::{
        DebrisMotion, advance_debris, background_overlap_exit, contact_departure_velocity,
        launch_debris_velocity, split_debris_motion,
    },
    voxel::{self, Geometry, UNIT},
    voxel_collision::{Collider, deflect_velocity, sweep_chunk},
    voxel_damage::{self, VoxelChunk, cooled_heat, fracture, split_chunk, thermal_tint},
};
use bevy::{
    asset::RenderAssetUsages,
    camera::{ScalingMode, visibility::NoFrustumCulling},
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

#[derive(Clone)]
pub(super) struct Request {
    pub id: u64,
    pub pos: Point,
    pub classic: bool,
    pub velocity: Point,
    pub impact: Point,
    pub shot_velocity: Point,
    pub rotation: [f32; 4],
    pub player: bool,
    pub depth: f32,
    pub velocity_z: f32,
}
#[derive(Resource, Default)]
pub(super) struct ShatterQueue(pub Vec<Request>);
struct Body {
    chunk: VoxelChunk,
    surface: Geometry,
    face_heat: Vec<f32>,
    bounds_min: Vec3,
    bounds_max: Vec3,
    radius: f32,
    position: Vec3,
    rotation: Quat,
    velocity: Vec3,
    angular_velocity: Vec3,
    release: f32,
    split_at: f32,
    generation: u8,
}
impl Body {
    fn new(
        chunk: VoxelChunk,
        position: Vec3,
        rotation: Quat,
        velocity: Vec3,
        angular_velocity: Vec3,
        release: f32,
        split_at: f32,
        generation: u8,
    ) -> Self {
        let surface = chunk.cells.mesh(UNIT);
        let face_heat = surface
            .cells
            .as_chunks::<4>()
            .0
            .iter()
            .map(|face| chunk.heat.get(&face[0]).copied().unwrap_or(0.))
            .collect();
        let pivot = Vec3::from_array(chunk.pivot);
        let (bounds_min, bounds_max) = surface.positions.iter().fold(
            (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
            |(min, max), v| {
                let p = Vec3::from_array(*v) - pivot;
                (min.min(p), max.max(p))
            },
        );
        let radius = bounds_min.abs().max(bounds_max.abs()).length();
        Self {
            bounds_min,
            bounds_max,
            radius,
            chunk,
            surface,
            face_heat,
            position,
            rotation,
            velocity,
            angular_velocity,
            release,
            split_at,
            generation,
        }
    }
}
struct Active {
    classic: bool,
    core: Entity,
    coating: Entity,
    solid: Handle<Mesh>,
    hot: Handle<Mesh>,
    bodies: Vec<Body>,
    cubes: usize,
    age: f32,
    origin: Vec3,
}
#[derive(Resource, Default)]
pub(super) struct ShatterEffects {
    active: Vec<Active>,
    run: u64,
    core_material: Option<Handle<StandardMaterial>>,
    hot_material: Option<Handle<StandardMaterial>>,
    pub bursts: u32,
    pub peak_cubes: usize,
    pub captures: u32,
    pub secondary_splits: u32,
    pub peak_bodies: usize,
    pub cooling_seen: bool,
    pub peak_effects: usize,
    pub peak_hot_vertices: usize,
    pub bounces: u32,
    pub peak_speed: f32,
    pub offscreen_retired: usize,
    pub retained_after_timeout: usize,
    /// Motion-only camera-depth probes. Up to 1 cm of collision skin is ignored.
    pub camera_approach_steps: u32,
    pub max_camera_approach: f32,
    pub depth_motion_steps: u32,
    pub unforced_flight_steps: u32,
    pub unforced_course_changes: u32,
    pub max_unforced_velocity_delta: f32,
    pub max_unforced_path_error: f32,
    pub split_translation_samples: u32,
    pub split_impulse_events: u32,
    pub max_split_velocity_delta: f32,
    last_bounce: Option<(Vec3, f32)>,
}
impl ShatterEffects {
    pub fn review_burst(&self, classic: bool) -> bool {
        self.active.iter().any(|e| {
            e.classic == classic
                && e.age > 0.12
                && e.age < 0.35
                && e.origin.x.abs() < 9.3
                && e.origin.y > -6.
                && e.origin.y < 9.5
        })
    }
    pub fn review_cooling(&self, classic: bool) -> bool {
        self.active.iter().any(|e| {
            e.classic == classic
                && cooled_heat(1., e.age) < 0.65
                && cooled_heat(1., e.age) > 0.2
                && e.origin.x.abs() < 8.
                && e.origin.y > -6.
                && e.origin.y < 5.
        })
    }
    pub fn review_bounce(&self, now: f32) -> bool {
        self.last_bounce
            .is_some_and(|(p, time)| now - time < 0.12 && p.x.abs() < 9.5 && p.y > -8. && p.y < 9.)
    }
    pub fn run_id(&self) -> u64 {
        self.run
    }
    pub fn motion_time(&self) -> f32 {
        self.active.iter().map(|e| e.age).sum()
    }
    pub fn active_cubes(&self) -> usize {
        self.active.iter().map(|e| e.cubes).sum()
    }
}
fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}
fn take_vec3(mesh: &mut Mesh, attribute: bevy::mesh::MeshVertexAttribute) -> Vec<[f32; 3]> {
    match mesh.remove_attribute(attribute).unwrap() {
        VertexAttributeValues::Float32x3(v) => v,
        _ => unreachable!(),
    }
}
fn take_colors(mesh: &mut Mesh) -> Vec<[f32; 4]> {
    match mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR).unwrap() {
        VertexAttributeValues::Float32x4(v) => v,
        _ => unreachable!(),
    }
}
fn take_indices(mesh: &mut Mesh) -> Vec<u32> {
    match mesh.remove_indices().unwrap() {
        Indices::U32(v) => v,
        _ => unreachable!(),
    }
}
// Build visual buffers once per display frame. Fixed simulation never uploads meshes.
fn draw(effect: &Active, meshes: &mut Assets<Mesh>, camera_forward: Vec3) -> usize {
    let solid = meshes.get_mut(&effect.solid).unwrap();
    let mut positions = take_vec3(solid, Mesh::ATTRIBUTE_POSITION);
    let mut normals = take_vec3(solid, Mesh::ATTRIBUTE_NORMAL);
    let mut colors = take_colors(solid);
    let mut indices = take_indices(solid);
    positions.clear();
    normals.clear();
    colors.clear();
    indices.clear();
    let hot = meshes.get_mut(&effect.hot).unwrap();
    let mut hot_positions = take_vec3(hot, Mesh::ATTRIBUTE_POSITION);
    let mut hot_normals = take_vec3(hot, Mesh::ATTRIBUTE_NORMAL);
    let mut hot_colors = take_colors(hot);
    let mut hot_indices = take_indices(hot);
    hot_positions.clear();
    hot_normals.clear();
    hot_colors.clear();
    hot_indices.clear();
    let cooling = cooled_heat(1., effect.age);
    for body in &effect.bodies {
        let base = positions.len() as u32;
        let pivot = Vec3::from_array(body.chunk.pivot);
        for (face, quad) in body.surface.positions.as_chunks::<4>().0.iter().enumerate() {
            let start = face * 4;
            let normal = (body.rotation * Vec3::from_array(body.surface.normals[start])).to_array();
            let heat = body.face_heat[face] * cooling;
            let original = body.surface.colors[start];
            let mut color = if heat > 0. {
                thermal_tint(original, heat)
            } else {
                original
            };
            color = battlestar_voxel::viewport::debris_shadow_tint(
                color,
                camera_forward.dot(body.position - effect.origin),
            );
            let posed: [[f32; 3]; 4] = std::array::from_fn(|corner| {
                let p = quad[corner];
                (body.position + body.rotation * (Vec3::from_array(p) - pivot)).to_array()
            });
            positions.extend(posed);
            normals.extend([normal; 4]);
            colors.extend([color; 4]);
            // Only hot source-cell faces receive a coating. Cold hull is not duplicated.
            if heat > 0.02 {
                let at = hot_positions.len() as u32;
                let thermal = thermal_tint([0., 0., 0., 1.], heat);
                let energy = heat * heat.sqrt() * 2.;
                hot_positions.extend(posed);
                hot_normals.extend([normal; 4]);
                hot_colors.extend(
                    [[
                        thermal[0] * energy,
                        thermal[1] * energy,
                        thermal[2] * energy,
                        1.,
                    ]; 4],
                );
                hot_indices.extend([at, at + 1, at + 2, at, at + 2, at + 3]);
            }
        }
        indices.extend(body.surface.indices.iter().map(|i| base + i));
    }
    let hot_vertices = hot_positions.len();
    let hot = meshes.get_mut(&effect.hot).unwrap();
    hot.insert_attribute(Mesh::ATTRIBUTE_POSITION, hot_positions);
    hot.insert_attribute(Mesh::ATTRIBUTE_NORMAL, hot_normals);
    hot.insert_attribute(Mesh::ATTRIBUTE_COLOR, hot_colors);
    hot.insert_indices(Indices::U32(hot_indices));
    let solid = meshes.get_mut(&effect.solid).unwrap();
    solid.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    solid.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    solid.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    solid.insert_indices(Indices::U32(indices));
    hot_vertices
}
pub(super) fn render(
    session: Res<Session>,
    mut state: ResMut<ShatterEffects>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera: Single<&Transform, With<Camera3d>>,
) {
    if session.0.phase == Phase::Paused {
        return;
    }
    let hot_vertices = state
        .active
        .iter()
        .map(|effect| draw(effect, &mut meshes, *camera.forward()))
        .sum();
    state.peak_hot_vertices = state.peak_hot_vertices.max(hot_vertices);
}
fn retire(commands: &mut Commands, meshes: &mut Assets<Mesh>, effect: Active) {
    commands.entity(effect.core).try_despawn();
    commands.entity(effect.coating).try_despawn();
    meshes.remove(effect.solid.id());
    meshes.remove(effect.hot.id());
}
fn noise(seed: u64) -> f32 {
    let mut x = seed.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) >> 40) as f32 / 16_777_216.
}
struct HullModel {
    cells: voxel::Voxels,
    radius: f32,
}
fn hull_models() -> &'static [HullModel; 6] {
    use std::sync::LazyLock;
    static MODELS: LazyLock<[HullModel; 6]> = LazyLock::new(|| {
        [
            voxel::viper(false),
            voxel::viper(true),
            voxel::raider(false),
            voxel::raider(true),
            voxel::galactica(false),
            voxel::galactica(true),
        ]
        .map(|cells| {
            let radius = cells
                .0
                .keys()
                .map(|p| {
                    Vec3::from_array(p.map(|v| (v.abs().max((v + 1).abs()) as f32) * UNIT)).length()
                })
                .fold(0., f32::max);
            HullModel { cells, radius }
        })
    });
    &MODELS
}
struct Target {
    model: &'static HullModel,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    velocity: Vec3,
}
fn targets(session: &Session, galactica: &Transform, dt: f32) -> Vec<Target> {
    let models = hull_models();
    let mut result = Vec::with_capacity(session.0.enemies.len() + 2);
    for enemy in &session.0.enemies {
        let velocity = visual_velocity(
            Vec2::new(enemy.velocity.x, enemy.velocity.y),
            enemy.velocity_z,
        );
        result.push(Target {
            model: &models[2 + usize::from(enemy.classic)],
            position: visual_position(Vec2::new(enemy.pos.x, enemy.pos.y), enemy.depth)
                - velocity * dt,
            rotation: Quat::from_array(enemy.rotation),
            scale: Vec3::ONE,
            velocity,
        });
    }
    if session.0.phase != Phase::GameOver {
        let velocity = Vec3::new(session.0.velocity_x, 0., 0.);
        result.push(Target {
            model: &models[usize::from(session.0.wave < 4)],
            position: visual_position(
                Vec2::new(session.0.player.x, session.0.player.y),
                super::PLAYER_LAYER,
            ) - velocity * dt,
            rotation: Quat::from_array(session.0.player_rotation),
            scale: Vec3::ONE,
            velocity,
        });
    }
    result.push(Target {
        model: &models[4 + usize::from(session.0.wave < 4)],
        position: galactica.translation,
        rotation: galactica.rotation,
        scale: galactica.scale,
        velocity: Vec3::ZERO,
    });
    result
}
fn move_body(
    body: &mut Body,
    dt: f32,
    start_elapsed: f32,
    targets: &[Target],
    camera_forward: Vec3,
    escape_tangent: f32,
) -> (u32, bool) {
    let mut remaining = dt;
    let mut elapsed = start_elapsed;
    let mut bounces = 0;
    let mut had_contact = false;
    for _ in 0..2 {
        let motion = advance_debris(
            DebrisMotion {
                position: body.position,
                velocity: body.velocity,
            },
            remaining,
        );
        let end = motion.position;
        body.velocity = motion.velocity;
        let mut earliest = None;
        for target in targets {
            let position = target.position + target.velocity * elapsed;
            let relative_end = end - target.velocity * remaining;
            let delta = relative_end - body.position;
            let t = ((position - body.position).dot(delta) / delta.length_squared().max(1e-12))
                .clamp(0., 1.);
            let closest = body.position + delta * t;
            let radius =
                body.radius + target.model.radius * target.scale.abs().max_element() + 0.01;
            if closest.distance_squared(position) > radius * radius {
                continue;
            }
            let collider = Collider {
                voxels: &target.model.cells,
                position,
                rotation: target.rotation,
                scale: target.scale,
                velocity: target.velocity,
            };
            if let Some(mut contact) = sweep_chunk(
                body.bounds_min,
                body.bounds_max,
                body.position,
                end,
                body.rotation,
                &collider,
                remaining,
            ) {
                if contact.toi == 0.
                    && (contact.point - body.position).dot(camera_forward) < 0.
                    && let Some(exit) = background_overlap_exit(
                        body.bounds_min,
                        body.bounds_max,
                        body.position,
                        body.rotation,
                        &collider,
                        camera_forward,
                    )
                {
                    contact = exit;
                }
                if earliest.as_ref().is_none_or(
                    |(old, _, _): &(battlestar_voxel::voxel_collision::Contact, Vec3, Vec3)| {
                        contact.toi < old.toi
                    },
                ) {
                    earliest = Some((
                        contact,
                        target.velocity,
                        position + target.velocity * (remaining * contact.toi),
                    ));
                }
            }
        }
        if let Some((contact, target_velocity, target_position)) = earliest {
            had_contact = true;
            let old = body.velocity;
            body.position = contact.point + contact.normal * 0.004;
            body.velocity = contact_departure_velocity(
                deflect_velocity(
                    old,
                    target_velocity,
                    contact.normal,
                    contact.point - target_position,
                )
                .clamp_length_max(36.),
                target_velocity,
                contact.normal,
                camera_forward,
                escape_tangent,
            );
            if (body.velocity - old).length_squared() > 0.01 {
                body.angular_velocity = (body.angular_velocity
                    + contact.normal.cross(old - target_velocity) * 0.16)
                    .clamp_length_max(12.);
                bounces += 1;
            }
            if (body.velocity - target_velocity).dot(contact.normal) < -0.0001 {
                // No bounded camera-away response can outrun this frontal hull.
                // As with the two-contact budget below, hold its safe contact
                // for this step instead of moving through occupied material.
                break;
            }
            let spent = remaining * contact.toi;
            elapsed += spent;
            remaining -= spent;
            if remaining <= 1e-6 {
                break;
            }
        } else {
            body.position = end;
            break;
        }
    }
    // Leave at the last safe contact if more than two hits happen within one120Hz step.
    body.rotation =
        (Quat::from_scaled_axis(body.angular_velocity * dt) * body.rotation).normalize();
    (bounces, had_contact)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn update(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    session: Res<Session>,
    galactica: Single<&Transform, With<super::Galactica>>,
    camera: Single<(&Transform, &Projection), With<Camera3d>>,
    window: Single<&Window>,
    mut queue: ResMut<ShatterQueue>,
    mut state: ResMut<ShatterEffects>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if state.run != session.1 {
        for e in state.active.drain(..) {
            retire(&mut commands, &mut meshes, e);
        }
        queue.0.clear();
        state.run = session.1;
    }
    if session.0.phase == Phase::Paused {
        return;
    }
    if state.core_material.is_none() {
        state.core_material = Some(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.15,
            perceptual_roughness: 0.65,
            ..default()
        }));
        state.hot_material = Some(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: true,
            alpha_mode: AlphaMode::Add,
            depth_bias: 1.,
            ..default()
        }));
    }
    let dt = time.delta_secs();
    let (camera_transform, projection) = *camera;
    let aspect = window.physical_width().max(1) as f32 / window.physical_height().max(1) as f32;
    let fov = battlestar_voxel::viewport::battle_fov(aspect);
    // Recreate the current window's actual projection before fixed-step retirement,
    // including a resize that the camera system will apply later this frame.
    let mut actual = (*projection).clone();
    if let Projection::Orthographic(p) = &mut actual {
        p.scaling_mode = ScalingMode::FixedVertical {
            viewport_height: 17.5_f32.max(26. / aspect),
        };
    }
    actual.update(
        window.physical_width().max(1) as f32,
        window.physical_height().max(1) as f32,
    );
    let clip = actual.get_clip_from_view() * camera_transform.to_matrix().inverse();
    let camera_forward = *camera_transform.forward();
    // A conservative corner cone makes a launch/contact departure clear the
    // current view without any later acceleration or steering.
    let escape_tangent = (fov * 0.5).tan() * (1. + aspect * aspect).sqrt();
    let targets = targets(&session, &galactica, dt);
    let mut bounces = 0;
    let mut peak_speed = 0_f32;
    let mut last_bounce = None;
    let mut split_count = 0;
    let mut cooling_seen = false;
    let mut retired = 0;
    let mut retained = 0;
    let mut camera_approach_steps = 0;
    let mut max_camera_approach = 0_f32;
    let mut depth_motion_steps = 0;
    let mut unforced_flight_steps = 0;
    let mut unforced_course_changes = 0;
    let mut max_unforced_velocity_delta = 0_f32;
    let mut max_unforced_path_error = 0_f32;
    let mut split_translation_samples = 0;
    let mut split_impulse_events = 0;
    let mut max_split_velocity_delta = 0_f32;
    let mut i = 0;
    while i < state.active.len() {
        let effect = &mut state.active[i];
        let previous_age = effect.age;
        effect.age += dt;
        let mut children = Vec::new();
        let mut j = 0;
        while j < effect.bodies.len() {
            let b = &mut effect.bodies[j];
            let flight_dt = (effect.age - b.release).max(0.) - (previous_age - b.release).max(0.);
            if flight_dt > 0. {
                let previous_position = b.position;
                let previous_velocity = b.velocity;
                let (contacts, had_contact) = move_body(
                    b,
                    flight_dt,
                    dt - flight_dt,
                    &targets,
                    camera_forward,
                    escape_tangent,
                );
                if !had_contact {
                    unforced_flight_steps += 1;
                    let delta = (b.velocity - previous_velocity).length();
                    max_unforced_velocity_delta = max_unforced_velocity_delta.max(delta);
                    unforced_course_changes += u32::from(delta > 1e-5);
                    max_unforced_path_error = max_unforced_path_error.max(
                        (b.position - (previous_position + previous_velocity * flight_dt)).length(),
                    );
                }
                let approach = -(b.position - previous_position).dot(camera_forward);
                max_camera_approach = max_camera_approach.max(approach);
                camera_approach_steps += u32::from(approach > 0.01);
                depth_motion_steps += 1;
                bounces += contacts;
                if contacts > 0 {
                    last_bounce = Some((b.position, session.0.elapsed));
                }
                peak_speed = peak_speed.max(b.velocity.length());
            }
            if b.generation == 0 && effect.age >= b.split_at && b.chunk.cells.0.len() > 72 {
                let parent = effect.bodies.remove(j);
                let seed = parent
                    .chunk
                    .cells
                    .0
                    .keys()
                    .fold(13_u64, |s, p| s.wrapping_add(p[0] as u64).wrapping_mul(37));
                let pieces = split_chunk(&parent.chunk, seed);
                debug_assert_eq!(
                    pieces.iter().map(|c| c.cells.0.len()).sum::<usize>(),
                    parent.chunk.cells.0.len()
                );
                for chunk in pieces {
                    let local_offset = Vec3::from_array(std::array::from_fn(|axis| {
                        chunk.pivot[axis] - parent.chunk.pivot[axis]
                    }));
                    let motion = split_debris_motion(
                        DebrisMotion {
                            position: parent.position,
                            velocity: parent.velocity,
                        },
                        parent.rotation,
                        local_offset,
                    );
                    let delta = (motion.velocity - parent.velocity).length();
                    max_split_velocity_delta = max_split_velocity_delta.max(delta);
                    split_impulse_events += u32::from(delta > 1e-5);
                    split_translation_samples += 1;
                    children.push(Body::new(
                        chunk,
                        motion.position,
                        parent.rotation,
                        motion.velocity,
                        parent.angular_velocity,
                        effect.age,
                        f32::INFINITY,
                        1,
                    ));
                }
                split_count += 1;
            } else {
                j += 1;
            }
        }
        effect.bodies.extend(children);
        let before = effect.bodies.len();
        effect
            .bodies
            .retain(|b| battlestar_voxel::viewport::sphere_visible(clip, b.position, b.radius));
        retired += before - effect.bodies.len();
        effect.cubes = effect.bodies.iter().map(|b| b.chunk.cells.0.len()).sum();
        if effect.bodies.is_empty() {
            let e = state.active.remove(i);
            retire(&mut commands, &mut meshes, e);
            continue;
        }
        if effect.age > 1.05 {
            retained += effect.bodies.len();
        }
        if cooled_heat(1., effect.age) < cooled_heat(1., previous_age)
            && effect.bodies.iter().any(|b| !b.chunk.heat.is_empty())
        {
            cooling_seen = true;
        }
        i += 1;
    }
    state.unforced_flight_steps += unforced_flight_steps;
    state.unforced_course_changes += unforced_course_changes;
    state.max_unforced_velocity_delta = state
        .max_unforced_velocity_delta
        .max(max_unforced_velocity_delta);
    state.max_unforced_path_error = state.max_unforced_path_error.max(max_unforced_path_error);
    state.split_translation_samples += split_translation_samples;
    state.split_impulse_events += split_impulse_events;
    state.max_split_velocity_delta = state.max_split_velocity_delta.max(max_split_velocity_delta);
    state.camera_approach_steps += camera_approach_steps;
    state.max_camera_approach = state.max_camera_approach.max(max_camera_approach);
    state.depth_motion_steps += depth_motion_steps;
    state.offscreen_retired += retired;
    state.retained_after_timeout += retained;
    if last_bounce.is_some() {
        state.last_bounce = last_bounce;
    }
    state.bounces += bounces;
    state.peak_speed = state.peak_speed.max(peak_speed);
    state.secondary_splits += split_count;
    state.cooling_seen |= cooling_seen;
    for request in queue.0.drain(..) {
        let rotation = Quat::from_array(request.rotation);
        let inverse = rotation.inverse();
        let model = if request.player {
            voxel::viper(request.classic)
        } else {
            voxel::raider(request.classic)
        };
        let origin = visual_position(Vec2::new(request.pos.x, request.pos.y), request.depth);
        let logical_contact =
            visual_position(Vec2::new(request.impact.x, request.impact.y), request.depth);
        let local_contact = inverse * (logical_contact - origin);
        let forward =
            Vec3::new(request.shot_velocity.x, request.shot_velocity.y, 0.).normalize_or_zero();
        // A 2D hit identifies the visible silhouette. Recover its actual occupied
        // surface along the viewing ray for local heat/fracture placement.
        let local_ray = inverse * camera_forward;
        let ray_origin = local_contact - local_ray * 8.;
        let contact = voxel_damage::ray_hit(&model, ray_origin.to_array(), local_ray.to_array())
            .map(Vec3::from_array)
            .unwrap_or(local_contact);
        let chunks = fracture(
            &model,
            contact.to_array(),
            request.id ^ ((session.0.wave as u64) << 32),
        );
        let cubes = chunks.iter().map(|c| c.cells.0.len()).sum::<usize>();
        if cubes == 0 {
            continue;
        }
        debug_assert_eq!(cubes, model.0.len());
        let mut bodies = Vec::with_capacity(chunks.len());
        for (k, chunk) in chunks.into_iter().enumerate() {
            let seed = request.id.wrapping_mul(131).wrapping_add(k as u64);
            let offset = rotation * (Vec3::from_array(chunk.pivot) - contact);
            let mass = chunk.cells.0.len() as f32;
            let speed = 26. + noise(seed) * 4. + (1. / mass.sqrt());
            let velocity = launch_debris_velocity(
                forward,
                visual_velocity(
                    Vec2::new(request.velocity.x, request.velocity.y),
                    request.velocity_z,
                ),
                offset.normalize_or_zero() * (0.8 + noise(seed.wrapping_add(1)) * 1.2),
                speed,
                camera_forward,
                escape_tangent,
            );
            let omega = offset.cross(forward) * 1.2
                + Vec3::new(
                    noise(seed.wrapping_add(2)) * 5. - 2.5,
                    noise(seed.wrapping_add(3)) * 5. - 2.5,
                    noise(seed.wrapping_add(4)) * 5. - 2.5,
                );
            let position = origin + rotation * Vec3::from_array(chunk.pivot);
            let release = (offset.length() * 0.035).min(0.09);
            let split_at = if noise(seed.wrapping_add(6)) < 0.33 {
                release + 0.25 + noise(seed.wrapping_add(5)) * 0.15
            } else {
                f32::INFINITY
            };
            bodies.push(Body::new(
                chunk, position, rotation, velocity, omega, release, split_at, 0,
            ));
        }
        let solid = meshes.add(empty_mesh());
        let hot = meshes.add(empty_mesh());
        let core = commands
            .spawn((
                Mesh3d(solid.clone()),
                MeshMaterial3d(state.core_material.as_ref().unwrap().clone()),
                Transform::IDENTITY,
                NoFrustumCulling,
            ))
            .id();
        let coating = commands
            .spawn((
                Mesh3d(hot.clone()),
                MeshMaterial3d(state.hot_material.as_ref().unwrap().clone()),
                Transform::IDENTITY,
                NoFrustumCulling,
            ))
            .id();
        state.active.push(Active {
            classic: request.classic,
            core,
            coating,
            solid,
            hot,
            bodies,
            cubes,
            age: 0.,
            origin,
        });
        state.bursts += 1;
        state.peak_cubes = state.peak_cubes.max(state.active_cubes());
        state.peak_effects = state.peak_effects.max(state.active.len());
    }
    let body_count = state.active.iter().map(|e| e.bodies.len()).sum();
    state.peak_bodies = state.peak_bodies.max(body_count);
}
