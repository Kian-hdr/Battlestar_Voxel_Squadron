// Bevy ECS systems expose disjoint queries as parameters.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
use crate::{
    Controls, Session,
    audio::{self, AudioSettings, Sounds},
};
use battlestar_voxel::{
    simulation::{Era, Event, Input, Phase},
    voxel,
};
use bevy::{
    asset::RenderAssetUsages,
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    mesh::{Indices, PrimitiveTopology},
    post_process::bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "menu.rs"]
mod menu;
#[path = "shatter_render.rs"]
mod shatter;
#[path = "soak.rs"]
mod soak;

pub(super) const PLAYER_LAYER: f32 = 4.;
const SHOT_LAYER: f32 = 6.;

pub struct PresentationPlugin;
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<soak::Soak>()
            .init_resource::<FleetVisuals>()
            .init_resource::<FrameStats>()
            .init_resource::<Smoke>()
            .init_resource::<shatter::ShatterQueue>()
            .init_resource::<shatter::ShatterEffects>()
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, (tick, shatter::update).chain())
            .add_systems(
                Update,
                (
                    apply_pixel_fonts,
                    responsive_hud,
                    fit_camera,
                    sync_era,
                    sync_visuals,
                    shatter::render,
                    exercise_menu_quit,
                    menu::interact,
                    menu::update,
                    update_hud,
                    update_reserves,
                    animate_background,
                    audit_projectile_colors,
                    smoke_test,
                    soak::run,
                )
                    .chain(),
            );
    }
}
#[derive(Resource)]
struct Art {
    eye: Handle<Mesh>,
    red_glow: Handle<StandardMaterial>,
    viper_classic: Handle<Mesh>,
    viper_modern: Handle<Mesh>,
    galactica_classic: Handle<Mesh>,
    galactica_modern: Handle<Mesh>,
    classic: Handle<Mesh>,
    modern: Handle<Mesh>,
    hull: Handle<StandardMaterial>,
    cyan: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    shot: Handle<Mesh>,
}
#[derive(Resource, Default)]
struct FleetVisuals {
    enemies: BTreeMap<u64, Entity>,
    shots: BTreeMap<u64, Entity>,
}
#[derive(Component)]
struct Player;
#[derive(Component)]
struct Galactica;
#[derive(Component)]
struct Engine(u8);
#[derive(Component)]
struct Backdrop;
#[derive(Component)]
struct Hud;
#[derive(Component)]
struct CombatHud;
#[derive(Resource)]
struct BodyPixelFont(Handle<Font>);
#[derive(Component)]
struct ReserveIcon {
    slot: u8,
    classic: bool,
}
#[derive(Component)]
struct WaveLabel;
#[derive(Resource, Default)]
struct FrameStats {
    samples_1080: Vec<f32>,
    samples: Vec<f32>,
    frames: u64,
    exhaust_min_length: Option<f32>,
    exhaust_max_length: f32,
    exhaust_updates: u64,
    projectile_color_checks: u64,
}
#[derive(Resource, Default)]
struct Smoke {
    stage: u8,
    started: f32,
    enabled: bool,
    last_lives: u8,
    wave_reached: bool,
    resize_stable: u8,
    resize_last: f32,
    paused_shatter_time: f32,
    pause_probed: bool,
    restart_probed: bool,
    expected_cleanup_run: u64,
    pose_probed: bool,
    hit_probe_x: f32,
    hit_probe_time: f32,
    backdrop_audited: Option<(u32, u32, bool)>,
    backdrop_checks: u32,
}
fn mesh(v: voxel::Voxels, unit: f32) -> Mesh {
    let g = v.mesh(unit);
    let mut m = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    m.insert_attribute(Mesh::ATTRIBUTE_POSITION, g.positions);
    m.insert_attribute(Mesh::ATTRIBUTE_NORMAL, g.normals);
    m.insert_attribute(Mesh::ATTRIBUTE_COLOR, g.colors);
    m.insert_indices(Indices::U32(g.indices));
    m
}
fn glowing(materials: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: c,
        unlit: true,
        ..default()
    })
}
fn emitting(materials: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    let linear = c.to_linear();
    materials.add(StandardMaterial {
        base_color: c,
        emissive: LinearRgba::rgb(linear.red * 8., linear.green * 8., linear.blue * 8.),
        ..default()
    })
}

fn label(commands: &mut Commands, text: &str, size: f32, color: Color, node: Node) -> Entity {
    commands
        .spawn((
            Text::new(text),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            node,
        ))
        .id()
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut fonts: ResMut<Assets<Font>>,
    mut smoke: ResMut<Smoke>,
) {
    smoke.enabled = std::env::args().any(|a| a == "--smoke-test");
    let body_font = fonts.add(
        Font::try_from_bytes(include_bytes!("../assets/Fonts/Silkscreen_Regular.ttf").to_vec())
            .expect("embedded SIL OFL pixel font"),
    );
    commands.insert_resource(BodyPixelFont(body_font));
    let white = Color::srgb(0.84, 0.88, 0.90);
    commands.spawn((
        Camera3d::default(),
        Tonemapping::Reinhard,
        Bloom {
            intensity: 0.08,
            low_frequency_boost: 0.1,
            max_mip_dimension: 256,
            prefilter: BloomPrefilter {
                threshold: 1.5,
                threshold_softness: 0.5,
            },
            composite_mode: BloomCompositeMode::Additive,
            ..Bloom::NATURAL
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 17.5,
            },
            near: 0.1,
            far: 200.,
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0., -25., 28.).looking_at(Vec3::new(0., -1.5, 0.), Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 220.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-10., -8., 20.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let hull = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 0.15,
        perceptual_roughness: 0.65,
        ..default()
    });
    let cyan = emitting(&mut materials, Color::srgb(0.32, 0.88, 1.));
    let red = emitting(&mut materials, Color::srgb(1., 0.22, 0.11));
    let mut shot_mesh = mesh(voxel::projectile(), voxel::UNIT);
    if let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        shot_mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for p in positions {
            p[0] -= voxel::UNIT * 0.5;
            p[1] -= voxel::UNIT * 3.;
            p[2] -= voxel::UNIT * 0.5;
        }
    }
    let shot = meshes.add(shot_mesh);
    let engine_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        ..default()
    });
    let viper = meshes.add(mesh(voxel::viper(true), voxel::UNIT));
    let viper_modern = meshes.add(mesh(voxel::viper(false), voxel::UNIT));
    let classic = meshes.add(mesh(voxel::raider(true), voxel::UNIT));
    let modern = meshes.add(mesh(voxel::raider(false), voxel::UNIT));
    let mut eye_voxels = voxel::raider(false);
    eye_voxels
        .0
        .retain(|_, c| c[0] > c[1] * 2. && c[0] > c[2] * 2.);
    let eye = meshes.add(mesh(eye_voxels, voxel::UNIT));
    let red_glow = materials.add(StandardMaterial {
        base_color: Color::srgb(1., 0.015, 0.005),
        emissive: LinearRgba::new(5., 0.015, 0.005, 1.),
        ..default()
    });

    let player = commands
        .spawn((
            Player,
            Mesh3d(viper.clone()),
            MeshMaterial3d(hull.clone()),
            Transform::from_xyz(0., -9., 0.),
        ))
        .id();
    for (nozzle, x) in [-0.36f32, 0., 0.36].into_iter().enumerate() {
        let plume = battlestar_voxel::exhaust::sample(0., nozzle as u8, true, 0.);
        let mut exhaust = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        exhaust.insert_attribute(Mesh::ATTRIBUTE_POSITION, plume.positions);
        exhaust.insert_attribute(Mesh::ATTRIBUTE_COLOR, plume.colors);
        exhaust.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0., 0., 1.]; battlestar_voxel::exhaust::VERTICES],
        );
        exhaust.insert_indices(Indices::U32(battlestar_voxel::exhaust::indices()));
        let engine = commands
            .spawn((
                Engine(nozzle as u8),
                Mesh3d(meshes.add(exhaust)),
                MeshMaterial3d(engine_material.clone()),
                bevy::camera::visibility::NoFrustumCulling,
                Transform::from_xyz(
                    x + voxel::UNIT * 0.5,
                    battlestar_voxel::exhaust::NOZZLE_Y,
                    (if x == 0. { 0.24 } else { 0. }) + voxel::UNIT * 0.5,
                ),
            ))
            .id();
        commands.entity(player).add_child(engine);
    }
    let star_mesh = meshes.add(Cuboid::new(0.028, 0.028, 0.028));
    let star_bright = glowing(&mut materials, Color::srgb(0.45, 0.58, 0.75));
    let star_dim = glowing(&mut materials, Color::srgb(0.13, 0.23, 0.35));
    let mut rng = 834723u32;
    for i in 0..360 {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let x = (rng >> 8) as f32 / 16777216. * 68. - 34.;
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let y = (rng >> 8) as f32 / 16777216. * 42. - 21.;
        commands.spawn((
            Backdrop,
            Mesh3d(star_mesh.clone()),
            MeshMaterial3d(if i % 5 == 0 {
                star_bright.clone()
            } else {
                star_dim.clone()
            }),
            Transform::from_xyz(x, y, -8. - (i % 7) as f32)
                .with_scale(Vec3::splat(if i % 9 == 0 { 2. } else { 1. })),
        ));
    }
    let galactica = meshes.add(mesh(voxel::galactica(true), voxel::UNIT));
    let galactica_modern = meshes.add(mesh(voxel::galactica(false), voxel::UNIT));
    let distant = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 0.3,
        perceptual_roughness: 0.9,
        ..default()
    });
    let (position, rotation, scale) = battlestar_voxel::backdrop::galactica_transform(1.6);
    commands.spawn((
        Galactica,
        Mesh3d(galactica.clone()),
        MeshMaterial3d(distant),
        Transform::from_translation(position)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(scale)),
    ));
    commands.insert_resource(Art {
        eye,
        red_glow,
        viper_classic: viper,
        viper_modern,
        galactica_classic: galactica,
        galactica_modern,
        classic,
        modern,
        hull,
        cyan,
        red,
        shot,
    });
    let hud = label(
        &mut commands,
        "SCORE  000000\n\nRESERVES  02",
        20.,
        white,
        Node {
            position_type: PositionType::Absolute,
            left: px(30),
            top: px(24),
            ..default()
        },
    );
    commands.entity(hud).insert((Hud, CombatHud));
    for classic in [true, false] {
        let model = voxel::viper(classic);
        let mut top = BTreeMap::<[i32; 2], (i32, voxel::Tint)>::new();
        for (p, tint) in model.0 {
            let entry = top.entry([p[0], p[1]]).or_insert((p[2], tint));
            if p[2] > entry.0 {
                *entry = (p[2], tint);
            }
        }
        let min_x = top.keys().map(|p| p[0]).min().unwrap_or(0);
        let max_x = top.keys().map(|p| p[0]).max().unwrap_or(0);
        let min_y = top.keys().map(|p| p[1]).min().unwrap_or(0);
        let max_y = top.keys().map(|p| p[1]).max().unwrap_or(0);
        let cell = 2.;
        for slot in 0..2u8 {
            let icon = commands
                .spawn((
                    ReserveIcon { slot, classic },
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(30. + slot as f32 * 44.),
                        top: px(120),
                        width: px((max_x - min_x + 1) as f32 * cell),
                        height: px((max_y - min_y + 1) as f32 * cell),
                        ..default()
                    },
                ))
                .id();
            for (p, (_, c)) in &top {
                let block = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px((p[0] - min_x) as f32 * cell),
                            top: px((max_y - p[1]) as f32 * cell),
                            width: px(cell),
                            height: px(cell),
                            ..default()
                        },
                        BackgroundColor(Color::linear_rgba(c[0], c[1], c[2], c[3])),
                    ))
                    .id();
                commands.entity(icon).add_child(block);
            }
        }
    }
    let wave = label(
        &mut commands,
        "SECTOR  01",
        16.,
        Color::srgb(0.70, 0.84, 0.80),
        Node {
            position_type: PositionType::Absolute,
            right: px(30),
            top: px(24),
            ..default()
        },
    );
    commands.entity(wave).insert((WaveLabel, CombatHud));
    menu::spawn(&mut commands, &mut fonts);
}

fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut session: ResMut<Session>,
    controls: Res<Controls>,
    sounds: Res<Sounds>,
    audio_settings: Res<AudioSettings>,
    smoke: Res<Smoke>,
    soak: Res<soak::Soak>,
    mut shatters: ResMut<shatter::ShatterQueue>,
) {
    let active = session.0.phase == Phase::Playing;
    let input =
        if (smoke.enabled && matches!(smoke.stage, 2 | 4)) || (soak.enabled && !soak.draining) {
            let g = &session.0;
            let target = g
                .enemies
                .iter()
                .filter(|e| e.pos.y > g.player.y)
                .min_by(|a, b| a.pos.y.total_cmp(&b.pos.y));
            Input {
                axis: target.map_or(0., |e| ((e.pos.x - g.player.x) * 8.).clamp(-1., 1.)),
                fire: true,
            }
        } else {
            controls.0
        };
    session.0.step(input, time.delta_secs());
    if !active {
        return;
    }
    audio::play_events(&mut commands, &session, &sounds, &audio_settings);
    for event in &session.0.events {
        if let Event::EnemyDestroyed {
            id,
            pos,
            classic,
            velocity,
            impact,
            shot_velocity,
            rotation,
            depth,
            velocity_z,
        } = event
        {
            shatters.0.push(shatter::Request {
                id: *id,
                pos: *pos,
                classic: *classic,
                velocity: *velocity,
                impact: *impact,
                shot_velocity: *shot_velocity,
                rotation: *rotation,
                player: false,
                depth: *depth,
                velocity_z: *velocity_z,
            });
        }
        if let Event::PlayerDestroyed {
            pos,
            classic,
            velocity,
            impact,
            shot_velocity,
            rotation,
        } = event
        {
            shatters.0.push(shatter::Request {
                id: u64::MAX - 1,
                pos: *pos,
                classic: *classic,
                velocity: *velocity,
                impact: *impact,
                shot_velocity: *shot_velocity,
                rotation: *rotation,
                player: true,
                depth: 0.,
                velocity_z: 0.,
            });
        }
    }
}

fn projectile_transform(shot: &battlestar_voxel::simulation::Shot) -> Transform {
    let forward = battlestar_voxel::combat_plane::view_forward();
    let velocity = Vec3::new(shot.velocity.x, shot.velocity.y, 0.);
    let velocity = if velocity.length_squared() > 1e-10 {
        velocity
    } else {
        Vec3::Y
    };
    let screen_velocity = velocity - forward * velocity.dot(forward);
    let y = screen_velocity.normalize();
    let z = -forward;
    let x = y.cross(z).normalize();
    Transform::from_translation(battlestar_voxel::combat_plane::visual_position(
        Vec2::new(shot.pos.x, shot.pos.y),
        SHOT_LAYER,
    ))
    .with_rotation(Quat::from_mat3(&Mat3::from_cols(x, y, z)))
    .with_scale(Vec3::new(
        1.,
        screen_velocity.length() / velocity.length(),
        1.,
    ))
}

fn sync_visuals(
    mut commands: Commands,
    session: Res<Session>,
    art: Res<Art>,
    mut fleet: ResMut<FleetVisuals>,
    mut player: Single<(&mut Transform, &mut Visibility), With<Player>>,
    mut transforms: Query<&mut Transform, Without<Player>>,
) {
    let g = &session.0;
    let (t, v) = &mut *player;
    t.translation = battlestar_voxel::combat_plane::visual_position(
        Vec2::new(g.player.x, g.player.y),
        PLAYER_LAYER,
    );
    t.rotation = Quat::from_array(g.player_rotation);
    let visibility = if g.phase == Phase::GameOver {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    v.set_if_neq(visibility);
    let living: BTreeSet<_> = g.enemies.iter().map(|e| e.id).collect();
    fleet.enemies.retain(|id, entity| {
        if living.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for e in &g.enemies {
        let rotation = Quat::from_array(e.rotation);
        let transform = Transform::from_translation(
            battlestar_voxel::combat_plane::visual_position(Vec2::new(e.pos.x, e.pos.y), e.depth),
        )
        .with_rotation(rotation);
        if let Some(entity) = fleet.enemies.get(&e.id) {
            if let Ok(mut t) = transforms.get_mut(*entity) {
                t.translation = transform.translation;
                t.rotation = rotation;
            }
        } else {
            let entity = commands
                .spawn((
                    Mesh3d(if e.classic {
                        art.classic.clone()
                    } else {
                        art.modern.clone()
                    }),
                    MeshMaterial3d(art.hull.clone()),
                    transform,
                ))
                .id();
            if !e.classic {
                let eye = commands
                    .spawn((
                        Mesh3d(art.eye.clone()),
                        MeshMaterial3d(art.red_glow.clone()),
                        Transform::from_scale(Vec3::splat(1.01)),
                    ))
                    .id();
                let light = commands
                    .spawn((
                        PointLight {
                            color: Color::srgb(1., 0.015, 0.005),
                            intensity: 45.,
                            range: 1.5,
                            radius: 0.08,
                            shadows_enabled: false,
                            ..default()
                        },
                        Transform::from_xyz(voxel::UNIT * 0.5, -0.58, 0.40),
                    ))
                    .id();
                commands.entity(entity).add_children(&[eye, light]);
            }
            fleet.enemies.insert(e.id, entity);
        }
    }
    let living: BTreeSet<_> = g.shots.iter().map(|s| s.id).collect();
    fleet.shots.retain(|id, entity| {
        if living.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
    for s in &g.shots {
        // The leading face is anchored to the exact 2D collision point, and
        // thickness lies along the viewing axis rather than above/below a hull.
        let transform = projectile_transform(s);
        if let Some(entity) = fleet.shots.get(&s.id) {
            if let Ok(mut t) = transforms.get_mut(*entity) {
                *t = transform;
            }
        } else {
            let entity = commands
                .spawn((
                    Mesh3d(art.shot.clone()),
                    MeshMaterial3d(if s.friendly {
                        art.cyan.clone()
                    } else {
                        art.red.clone()
                    }),
                    transform,
                ))
                .id();
            fleet.shots.insert(s.id, entity);
        }
    }
}
fn update_hud(
    session: Res<Session>,
    mut texts: ParamSet<(
        Query<&mut Text, With<Hud>>,
        Query<&mut Text, With<WaveLabel>>,
    )>,
    mut nodes: Query<&mut Node, With<CombatHud>>,
) {
    let g = &session.0;
    for mut text in &mut texts.p0() {
        let value = format!(
            "SCORE  {:06}\n\nRESERVES  {:02}",
            g.score,
            g.lives.saturating_sub(1)
        );
        if text.0 != value {
            text.0 = value;
        }
    }
    for mut text in &mut texts.p1() {
        let value = format!("SECTOR {:02}", g.wave.max(1));
        if text.0 != value {
            text.0 = value;
        }
    }
    for mut node in &mut nodes {
        let display = if g.phase == Phase::Playing {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}
fn animate_background(
    time: Res<Time>,
    session: Res<Session>,
    mut stars: Query<&mut Transform, With<Backdrop>>,
    engines: Query<(&Engine, &Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut exhaust_clock: Local<f32>,
    mut stats: ResMut<FrameStats>,
) {
    if session.0.phase == Phase::Paused {
        return;
    }
    *exhaust_clock += time.delta_secs();
    for (engine, mesh) in &engines {
        let plume = battlestar_voxel::exhaust::sample(
            *exhaust_clock,
            engine.0,
            session.0.wave < 4,
            (session.0.velocity_x.abs() / 16.).min(1.),
        );
        if let Some(mesh) = meshes.get_mut(&mesh.0) {
            let length = plume.positions.iter().map(|p| -p[1]).fold(0., f32::max);
            stats.exhaust_min_length = Some(
                stats
                    .exhaust_min_length
                    .map_or(length, |old| old.min(length)),
            );
            stats.exhaust_max_length = stats.exhaust_max_length.max(length);
            stats.exhaust_updates += 1;
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, plume.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, plume.colors);
        }
    }
    for mut t in &mut stars {
        t.translation.y -= time.delta_secs() * 0.18;
        if t.translation.y < -21. {
            t.translation.y = 21.;
        }
    }
}
// The v0.4.1 impact flash reused the bullet mesh and looked like a red
// friendly projectile. Audit the actual scene graph, including hit frames.
fn audit_projectile_colors(
    smoke: Res<Smoke>,
    session: Res<Session>,
    art: Res<Art>,
    fleet: Res<FleetVisuals>,
    meshes: Query<(
        Entity,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
    )>,
    mut stats: ResMut<FrameStats>,
) {
    if !smoke.enabled {
        return;
    }
    for enemy in &session.0.enemies {
        if let Some(entity) = fleet.enemies.get(&enemy.id)
            && let Ok((_, _, _, transform)) = meshes.get(*entity)
        {
            let projected = battlestar_voxel::combat_plane::project_offset(transform.translation);
            assert!(
                projected.distance(Vec2::new(enemy.pos.x, enemy.pos.y)) < 0.0001,
                "cosmetic ship depth must not move its 2D target"
            );
        }
    }
    for (entity, mesh, material, transform) in &meshes {
        if mesh.0 != art.shot {
            continue;
        }
        let shot = session
            .0
            .shots
            .iter()
            .find(|shot| fleet.shots.get(&shot.id) == Some(&entity))
            .expect("projectile-shaped visuals must correspond to an actual live shot");
        let expected = if shot.friendly { &art.cyan } else { &art.red };
        assert_eq!(
            &material.0, expected,
            "projectiles must keep their side's color through contact"
        );
        let projected = battlestar_voxel::combat_plane::project_offset(transform.translation);
        assert!(
            projected.distance(Vec2::new(shot.pos.x, shot.pos.y)) < 0.0001,
            "drawn projectile must share the logical 2D hit plane"
        );
        for z in [-voxel::UNIT * 0.5, voxel::UNIT * 0.5] {
            let tip = transform.transform_point(Vec3::new(0., 0., z));
            assert!(
                battlestar_voxel::combat_plane::project_offset(tip)
                    .distance(Vec2::new(shot.pos.x, shot.pos.y))
                    < 0.0001,
                "visible beam tip/thickness must align with its 2D contact point"
            );
        }
        stats.projectile_color_checks += 1;
    }
}

// Inject after Bevy's UI focus pass, immediately before the real menu handler.
// Host cursor position cannot erase this deterministic shutdown probe.
fn exercise_menu_quit(
    mut smoke: ResMut<Smoke>,
    mut buttons: Query<(&menu::Action, &mut Interaction)>,
) {
    if !smoke.enabled || smoke.stage != 9 {
        return;
    }
    for (action, mut interaction) in &mut buttons {
        if matches!(action, menu::Action::Quit) {
            *interaction = Interaction::Pressed;
        }
    }
    smoke.stage = 10;
    info!("SMOKE menu quit button pressed through normal interaction path");
}

fn capture(commands: &mut Commands, name: &str) {
    let dir = std::env::var("BSVX_EVIDENCE_DIR").unwrap_or_else(|_| "evidence".into());
    let path = std::path::Path::new(&dir).join(name);
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}
fn smoke_test(
    mut commands: Commands,
    time: Res<Time>,
    real_time: Res<Time<Real>>,
    mut smoke: ResMut<Smoke>,
    mut session: ResMut<Session>,
    mut controls: ResMut<Controls>,
    mut stats: ResMut<FrameStats>,
    mut shatters: ResMut<shatter::ShatterEffects>,
    mut shatter_queue: ResMut<shatter::ShatterQueue>,
    mut window: Single<&mut Window>,
    player: Single<&Visibility, With<Player>>,
    mut menu_buttons: Query<(&menu::Action, &mut Interaction, &ComputedNode)>,
    menu_root: Single<&Visibility, With<menu::MenuRoot>>,
    camera_projection: Single<&Projection, With<Camera3d>>,
    galactica: Single<&Transform, With<Galactica>>,
) {
    stats.frames += 1;
    if time.elapsed_secs() > 5. && stats.samples.len() < 10000 {
        stats.samples.push(real_time.delta_secs());
        if smoke.stage == 4
            && time.elapsed_secs() - smoke.started > 1.5
            && window.physical_width() == 1920
            && window.physical_height() == 1080
        {
            stats.samples_1080.push(real_time.delta_secs());
        }
    }
    if !smoke.enabled {
        return;
    }
    let now = time.elapsed_secs();
    assert!(
        matches!(&**camera_projection, Projection::Orthographic(_)),
        "arcade hit projection must not have depth parallax"
    );
    let size = (
        window.physical_width(),
        window.physical_height(),
        session.0.wave < 4,
    );
    if smoke.backdrop_audited != Some(size) {
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let height = 17.5_f32.max(26. / aspect);
        let half_view = Vec2::new(height * aspect, height) * 0.5;
        let (low, high) = battlestar_voxel::backdrop::projected_galactica_bounds(
            size.2,
            galactica.translation,
            galactica.rotation,
            galactica.scale.x,
        );
        assert!(
            low.cmpgt(-half_view).all() && high.cmplt(half_view).all(),
            "full Galactica hull must stay inside current viewport: {low:?}..{high:?} view={half_view:?}"
        );
        smoke.backdrop_audited = Some(size);
        smoke.backdrop_checks += 1;
        info!(
            "SMOKE full Galactica framing verified {}x{} classic={}",
            size.0, size.1, size.2
        );
    }
    if session.0.phase == Phase::Playing {
        assert_eq!(
            **player,
            Visibility::Visible,
            "player must never blink during protection"
        );
    }
    if session.0.wave < 4
        && shatters.bursts >= 3
        && shatters.review_burst(true)
        && shatters.captures & 1 == 0
    {
        capture(&mut commands, "06_Shatter_Classic.png");
        shatters.captures |= 1;
    }
    if session.0.wave >= 4 && shatters.review_burst(false) && shatters.captures & 2 == 0 {
        capture(&mut commands, "07_Shatter_Modern.png");
        shatters.captures |= 2;
    }

    if shatters.review_cooling(true) && shatters.captures & 4 == 0 {
        capture(&mut commands, "08_Cooling_Classic.png");
        shatters.captures |= 4;
    }
    if shatters.review_cooling(false) && shatters.captures & 8 == 0 {
        capture(&mut commands, "09_Cooling_Modern.png");
        shatters.captures |= 8;
    }
    if shatters.review_bounce(session.0.elapsed) && shatters.captures & 16 == 0 {
        capture(&mut commands, "10_Debris_Bounce.png");
        shatters.captures |= 16;
    }
    let Session(g, run) = &mut *session;
    match smoke.stage {
        0 if now > 1. => {
            capture(&mut commands, "01_Title.png");
            smoke.stage = 1;
        }
        1 if now > 2. => {
            g.start();
            *run = run.wrapping_add(1);
            smoke.started = now;
            smoke.stage = 2;
            info!("SMOKE started actual renderer gameplay");
        }
        2 => {
            g.invulnerable = 10.;
            if !smoke.pose_probed && g.elapsed > 0.2 && g.player.x.abs() > 0.5 {
                smoke.hit_probe_x = g.player.x;
                smoke.hit_probe_time = now;
                g.invulnerable = 0.;
                g.lives = 3;
                g.inject_shot(g.player, false);
                smoke.stage = 12;
                return;
            }
            if !smoke.pause_probed && shatters.active_cubes() > 0 {
                smoke.paused_shatter_time = shatters.motion_time();
                g.toggle_pause();
                smoke.stage = 20;
                smoke.started = now;
                return;
            }
            let target = g
                .enemies
                .iter()
                .filter(|e| e.pos.y > g.player.y)
                .min_by(|a, b| a.pos.y.total_cmp(&b.pos.y));
            controls.0 = Input {
                axis: target.map_or(0., |e| ((e.pos.x - g.player.x) * 8.).clamp(-1., 1.)),
                fire: true,
            };
            if now - smoke.started > 3. && smoke.last_lives == 0 {
                capture(&mut commands, "02_Gameplay.png");
                smoke.last_lives = 3;
            }
            if g.wave >= 4 || now - smoke.started > 80. {
                smoke.wave_reached = g.wave >= 4;
                assert!(
                    smoke.wave_reached,
                    "Smoke combat failed to reach the modern era within 80 seconds"
                );
                info!(
                    "SMOKE combat score={} wave={} remaining={} wave_advanced={}",
                    g.score,
                    g.wave,
                    g.enemies.len(),
                    smoke.wave_reached
                );
                g.toggle_pause();
                smoke.paused_shatter_time = shatters.motion_time();
                smoke.started = now;
                smoke.stage = 3;
            }
        }
        12 => {
            assert!(
                now - smoke.hit_probe_time < 0.5,
                "nonfatal hit probe must contact the Viper"
            );
            if g.lives == 2 {
                assert!(
                    (g.player.x - smoke.hit_probe_x).abs()
                        <= 13. * (now - smoke.hit_probe_time) + 0.02,
                    "life loss must not teleport the visible Viper"
                );
                smoke.pose_probed = true;
                g.invulnerable = 10.;
                smoke.stage = 2;
                info!("SMOKE nonfatal life-loss position continuity verified");
            }
        }
        20 if now - smoke.started > 0.4 => {
            assert!(
                shatters.active_cubes() > 0,
                "pause fixture must contain real fragments"
            );
            assert_eq!(
                shatters.motion_time(),
                smoke.paused_shatter_time,
                "pause must freeze active fragments"
            );
            assert_eq!(g.phase, Phase::Paused);
            g.toggle_pause();
            smoke.pause_probed = true;
            smoke.stage = 2;
            smoke.started = now;
            info!("SMOKE active-shard pause verified");
        }
        3 if now - smoke.started > 1. => {
            capture(&mut commands, "03_Paused.png");
            assert_eq!(g.phase, Phase::Paused);
            assert_eq!(
                shatters.motion_time(),
                smoke.paused_shatter_time,
                "pause must freeze shattering"
            );
            g.toggle_pause();
            g.muted = true;
            window.mode = bevy::window::WindowMode::Windowed;
            info!("SMOKE pause/resume, mute; waiting for native fullscreen transition");
            smoke.stage = 30;
            smoke.resize_stable = 0;
            smoke.resize_last = now;
            smoke.started = now;
        }
        30 if now - smoke.started > 0.5 && now - smoke.resize_last > 0.2 => {
            smoke.resize_last = now;
            if window.physical_width() == 1920 && window.physical_height() == 1080 {
                smoke.resize_stable += 1;
            } else {
                window.resolution.set_physical_resolution(1920, 1080);
                smoke.resize_stable = 0;
            }
            if smoke.resize_stable >= 3 || now - smoke.started > 8. {
                info!(
                    "SMOKE resize settled actual={}x{}",
                    window.physical_width(),
                    window.physical_height()
                );
                smoke.stage = 4;
                smoke.started = now;
            }
        }
        4 if now - smoke.started > 10. => {
            capture(&mut commands, "04_1080p_Modern_Era.png");
            g.invulnerable = 0.;
            g.lives = 1;
            g.inject_shot(g.player, false);
            controls.0 = Input::default();
            smoke.stage = 5;
            smoke.started = now;
        }
        5 if now - smoke.started > 1. => {
            assert_eq!(g.phase, Phase::GameOver);
            capture(&mut commands, "05_Game_Over.png");
            info!("SMOKE game over verified");
            smoke.stage = 6;
            smoke.started = now;
        }
        6 if now - smoke.started > 0.3 => {
            let scale = window.resolution.scale_factor();
            window
                .resolution
                .set_physical_resolution((800. * scale) as u32, (500. * scale) as u32);
            smoke.stage = 60;
            smoke.started = now;
        }
        60 if now - smoke.started > 0.6 => {
            assert!(
                (window.width() - 800.).abs() < 2. && (window.height() - 500.).abs() < 2.,
                "minimum menu viewport must settle"
            );
            capture(&mut commands, "11_Minimum_Menu.png");
            info!(
                "SMOKE minimum_menu logical={}x{}",
                window.width(),
                window.height()
            );
            // A renderer fixture guarantees restart is tested while fragments are alive.
            shatter_queue.0.push(shatter::Request {
                id: u64::MAX,
                pos: battlestar_voxel::simulation::Point { x: 0., y: 3. },
                classic: false,
                velocity: default(),
                impact: battlestar_voxel::simulation::Point { x: 0., y: 2.3 },
                shot_velocity: battlestar_voxel::simulation::Point { x: 0., y: 25. },
                rotation: Quat::IDENTITY.to_array(),
                player: false,
                depth: 0.,
                velocity_z: 0.,
            });
            smoke.stage = 61;
            smoke.started = now;
        }
        61 if shatters.active_cubes() > 0 => {
            assert!(
                now - smoke.started < 1.,
                "restart fixture must still be alive"
            );
            g.start();
            *run = run.wrapping_add(1);
            smoke.expected_cleanup_run = *run;
            assert_eq!((g.score, g.lives, g.wave, g.enemies.len()), (0, 3, 1, 24));
            assert!(g.muted);
            smoke.stage = 62;
            smoke.started = now;
        }
        62 if shatters.run_id() == smoke.expected_cleanup_run => {
            assert!(
                now - smoke.started < 0.5,
                "check cleanup before natural fragment expiry"
            );
            assert_eq!(
                shatters.active_cubes(),
                0,
                "first fixed update after restart must clear fragments"
            );
            smoke.restart_probed = true;
            smoke.stage = 7;
            smoke.started = now;
            info!("SMOKE active-shard restart verified");
        }
        7 if now - smoke.started > 2. => {
            assert_eq!(
                shatters.active_cubes(),
                0,
                "restart must clear old fragments"
            );
            assert!(
                shatters.bursts > 0
                    && shatters.captures & 3 == 3
                    && smoke.pause_probed
                    && smoke.restart_probed,
                "both eras' shattering must be exercised"
            );
            info!(
                "SMOKE voxel_shatter bursts={} peak_cubes={} captures={}",
                shatters.bursts, shatters.peak_cubes, shatters.captures
            );
            assert!(
                shatters.secondary_splits > 0 && shatters.cooling_seen,
                "actual connected chunks must fracture again and cool during flight"
            );
            info!(
                "SMOKE voxel_bodies secondary_splits={} peak_bodies={} cooling_seen={}",
                shatters.secondary_splits, shatters.peak_bodies, shatters.cooling_seen
            );
            assert!(
                shatters.bounces > 0,
                "native scenario must exercise debris/ship bounce"
            );
            info!(
                "SMOKE debris_collision bounces={} peak_speed={:.2}",
                shatters.bounces, shatters.peak_speed
            );
            assert!(
                shatters.offscreen_retired > 0,
                "debris must retire after leaving the viewport"
            );
            assert!(
                smoke.pose_probed,
                "nonfatal pose continuity must be exercised"
            );
            assert!(
                stats.projectile_color_checks > 0,
                "actual projectile colors must be exercised"
            );
            assert!(
                shatters.peak_speed <= battlestar_voxel::viewport::DEBRIS_SPEED_LIMIT + 0.01,
                "debris contacts must not amplify into explosive speeds"
            );
            assert!(
                shatters.depth_motion_steps > 0 && shatters.camera_approach_steps == 0,
                "native debris must recede rather than approach the viewer"
            );
            info!(
                "SMOKE debris_depth steps={} approach_steps={} max_approach={:.6}",
                shatters.depth_motion_steps,
                shatters.camera_approach_steps,
                shatters.max_camera_approach
            );
            assert!(
                shatters.unforced_flight_steps > 0
                    && shatters.unforced_course_changes == 0
                    && shatters.max_unforced_path_error < 0.0001,
                "unobstructed debris must preserve its launch velocity and straight path"
            );
            assert!(
                shatters.split_translation_samples > 0
                    && shatters.split_impulse_events == 0
                    && shatters.max_split_velocity_delta < 0.00001,
                "subdivision must not add a delayed momentum kick"
            );
            info!(
                "SMOKE ballistic steps={} course_changes={} max_velocity_delta={:.6} path_error={:.6} split_samples={} split_impulses={}",
                shatters.unforced_flight_steps,
                shatters.unforced_course_changes,
                shatters.max_unforced_velocity_delta,
                shatters.max_unforced_path_error,
                shatters.split_translation_samples,
                shatters.split_impulse_events
            );
            assert!(
                stats.exhaust_updates > 100
                    && stats.exhaust_max_length - stats.exhaust_min_length.unwrap_or(0.) > 0.06,
                "actual exhaust meshes must animate instead of staying rigid rods"
            );
            stats.samples.sort_by(|a, b| a.total_cmp(b));
            let n = stats.samples.len();
            if n > 0 {
                let mean = stats.samples.iter().sum::<f32>() / n as f32;
                let p95 = stats.samples[(n as f32 * 0.95) as usize];
                info!(
                    "SMOKE frames={} mean_ms={:.2} p95_ms={:.2} wave_advanced={}",
                    n,
                    mean * 1000.,
                    p95 * 1000.,
                    smoke.wave_reached
                );
            }
            let n = stats.samples_1080.len();
            if n > 0 {
                stats.samples_1080.sort_by(|a, b| a.total_cmp(b));
                info!(
                    "SMOKE physical_1920x1080 frames={} mean_ms={:.2} p95_ms={:.2}",
                    n,
                    stats.samples_1080.iter().sum::<f32>() / n as f32 * 1000.,
                    stats.samples_1080[(n as f32 * 0.95) as usize] * 1000.
                );
            } else {
                warn!("SMOKE physical 1920x1080 not achieved; performance unqualified");
            }
            let dir = std::env::var("BSVX_EVIDENCE_DIR").unwrap_or_else(|_| "evidence".into());
            let n = stats.samples_1080.len();
            let mean = if n > 0 {
                stats.samples_1080.iter().sum::<f32>() / n as f32 * 1000.
            } else {
                0.
            };
            let p95 = if n > 0 {
                stats.samples_1080[(n as f32 * 0.95) as usize] * 1000.
            } else {
                0.
            };
            let report = format!(
                "{{\"complete\":true,\"reached_modern_sector_4\":{},\"pause_resume\":true,\"mute\":true,\"game_over\":true,\"restart\":true,\"physical_1080p_frames\":{},\"mean_ms\":{:.3},\"p95_ms\":{:.3}}}\n",
                smoke.wave_reached, n, mean, p95
            );
            std::fs::write(
                std::path::Path::new(&dir).join("Frame_Times_1080p.csv"),
                "frame_ms\n".to_owned()
                    + &stats
                        .samples_1080
                        .iter()
                        .map(|s| format!("{:.6}\n", s * 1000.))
                        .collect::<String>(),
            )
            .expect("write frame timings");
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"shatter_bursts\":{},\"peak_shatter_cubes\":{},\"shatter_both_eras\":{},\"shatter_pause_restart\":true,\"secondary_splits\":{},\"peak_voxel_bodies\":{},\"cooling_seen\":{},\"peak_damage_events\":{},\"peak_hot_vertices\":{},\"debris_bounces\":{},\"peak_debris_speed\":{:.3},\"offscreen_retired\":{},\"retained_after_old_timeout\":{},\"player_no_blink\":true}}\n",
                    shatters.bursts,
                    shatters.peak_cubes,
                    shatters.captures & 3 == 3,
                    shatters.secondary_splits,
                    shatters.peak_bodies,
                    shatters.cooling_seen,
                    shatters.peak_effects,
                    shatters.peak_hot_vertices,
                    shatters.bounces,
                    shatters.peak_speed,
                    shatters.offscreen_retired,
                    shatters.retained_after_timeout
                );
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"exhaust_mesh_updates\":{},\"exhaust_min_length\":{:.4},\"exhaust_max_length\":{:.4}}}\n",
                    stats.exhaust_updates,
                    stats.exhaust_min_length.unwrap_or(0.),
                    stats.exhaust_max_length
                );
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"camera_approach_steps\":{},\"max_camera_approach\":{:.6},\"depth_motion_steps\":{}}}\n",
                    shatters.camera_approach_steps,
                    shatters.max_camera_approach,
                    shatters.depth_motion_steps
                );
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"projectile_color_checks\":{}}}\n",
                    stats.projectile_color_checks
                );
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"unforced_flight_steps\":{},\"unforced_course_changes\":{},\"max_unforced_velocity_delta\":{:.6},\"max_unforced_path_error\":{:.6},\"split_translation_samples\":{},\"split_impulse_events\":{},\"max_split_velocity_delta\":{:.6}}}\n",
                    shatters.unforced_flight_steps,
                    shatters.unforced_course_changes,
                    shatters.max_unforced_velocity_delta,
                    shatters.max_unforced_path_error,
                    shatters.split_translation_samples,
                    shatters.split_impulse_events,
                    shatters.max_split_velocity_delta
                );
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"strict_2d_projection\":true,\"nonfatal_pose_continuity\":true,\"galactica_framing_checks\":{}}}\n",
                    smoke.backdrop_checks
                );
            std::fs::write(std::path::Path::new(&dir).join("Smoke_Result.json"), report)
                .expect("write smoke result");
            info!("SMOKE complete");
            if g.phase == Phase::Playing {
                g.toggle_pause();
            }
            smoke.stage = 8;
            smoke.started = now;
        }
        8 if now - smoke.started > 0.25 => {
            assert_eq!(
                **menu_root,
                Visibility::Visible,
                "quit must be available in a visible menu"
            );
            let mut found = false;
            for (action, _interaction, node) in &mut menu_buttons {
                if matches!(action, menu::Action::Quit) {
                    assert!(
                        node.size().min_element() >= 20.,
                        "quit must have a visible clickable layout"
                    );
                    found = true;
                }
            }
            assert!(found, "menu must include Quit Game");
            smoke.stage = 9;
        }
        _ => {}
    }
}

fn sync_era(
    session: Res<Session>,
    art: Res<Art>,
    mut last: Local<Option<Era>>,
    mut ships: Query<(&mut Mesh3d, Option<&Player>), Or<(With<Player>, With<Galactica>)>>,
) {
    let era = Era::for_wave(session.0.wave);
    if *last == Some(era) {
        return;
    }
    let classic = era == Era::Classic;
    for (mut mesh, player) in &mut ships {
        mesh.0 = if player.is_some() {
            if classic {
                art.viper_classic.clone()
            } else {
                art.viper_modern.clone()
            }
        } else if classic {
            art.galactica_classic.clone()
        } else {
            art.galactica_modern.clone()
        };
    }
    *last = Some(era);
}

fn fit_camera(
    window: Single<&Window>,
    mut projection: Single<&mut Projection, With<Camera3d>>,
    mut galactica: Single<&mut Transform, With<Galactica>>,
) {
    let aspect = window.physical_width().max(1) as f32 / window.physical_height().max(1) as f32;
    let height = 17.5_f32.max(26. / aspect);
    let (position, rotation, scale) = battlestar_voxel::backdrop::galactica_transform(aspect);
    let transform = Transform::from_translation(position)
        .with_rotation(rotation)
        .with_scale(Vec3::splat(scale));
    galactica.set_if_neq(transform);
    let needs_update = match &**projection {
        Projection::Orthographic(p) => !matches!(p.scaling_mode,
            ScalingMode::FixedVertical { viewport_height } if viewport_height == height),
        _ => false,
    };
    if needs_update && let Projection::Orthographic(p) = &mut **projection {
        p.scaling_mode = ScalingMode::FixedVertical {
            viewport_height: height,
        };
    }
}

fn apply_pixel_fonts(font: Res<BodyPixelFont>, mut labels: Query<&mut TextFont, Added<TextFont>>) {
    for mut label in &mut labels {
        if label.font == Handle::<Font>::default() {
            label.font = font.0.clone();
        }
    }
}

fn responsive_hud(
    window: Single<&Window>,
    mut ui: ResMut<UiScale>,
    mut fonts: Query<&mut TextFont, With<Hud>>,
) {
    let layout = battlestar_voxel::layout::hud_layout(
        window.physical_width(),
        window.physical_height(),
        window.resolution.scale_factor(),
    );
    if ui.0 != layout.scale {
        ui.0 = layout.scale;
    }
    for mut font in &mut fonts {
        if font.font_size != layout.body_font {
            font.font_size = layout.body_font;
        }
    }
}

fn update_reserves(session: Res<Session>, mut icons: Query<(&ReserveIcon, &mut Node)>) {
    let reserves = session.0.lives.saturating_sub(1);
    for (icon, mut node) in &mut icons {
        let display = if session.0.phase == Phase::Playing
            && icon.slot < reserves
            && icon.classic == (session.0.wave < 4)
        {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}
