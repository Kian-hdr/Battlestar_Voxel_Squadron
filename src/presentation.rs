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
    core_pipeline::tonemapping::Tonemapping,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "menu.rs"]
mod menu;
#[path = "shatter_render.rs"]
mod shatter;

pub struct PresentationPlugin;
impl Plugin for PresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FleetVisuals>()
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
                    menu::interact,
                    menu::update,
                    update_hud,
                    update_reserves,
                    animate_background,
                    smoke_test,
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
    gold: Handle<StandardMaterial>,
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
struct Engine;
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
    let gold = Color::srgb(0.91, 0.66, 0.32);
    commands.spawn((
        Camera3d::default(),
        Tonemapping::Reinhard,
        Projection::Perspective(PerspectiveProjection {
            fov: 0.72,
            near: 0.1,
            far: 200.,
            ..default()
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
    let cyan = glowing(&mut materials, Color::srgb(0.32, 0.88, 1.));
    let red = glowing(&mut materials, Color::srgb(1., 0.22, 0.11));
    let gold_mat = glowing(&mut materials, gold);
    let shot = meshes.add(mesh(voxel::projectile(), voxel::UNIT));
    let engine_mesh = shot.clone();
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
        unlit: true,
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
    for x in [-0.36f32, 0., 0.36] {
        let engine = commands
            .spawn((
                Engine,
                Mesh3d(engine_mesh.clone()),
                MeshMaterial3d(cyan.clone()),
                Transform::from_xyz(x, -1.15, if x == 0. { 0.24 } else { 0. }),
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
    commands.spawn((
        Galactica,
        Mesh3d(galactica.clone()),
        MeshMaterial3d(distant),
        Transform::from_xyz(13., 8., -16.)
            .with_rotation(Quat::from_rotation_z(-0.38) * Quat::from_rotation_x(0.15))
            .with_scale(Vec3::splat(0.65)),
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
        gold: gold_mat,
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
    mut shatters: ResMut<shatter::ShatterQueue>,
) {
    let active = session.0.phase == Phase::Playing;
    let input = if smoke.enabled && matches!(smoke.stage, 2 | 4) {
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
    t.translation = Vec3::new(g.player.x, g.player.y, 0.);
    t.rotation = Quat::from_array(g.player_rotation);
    **v = if g.phase == Phase::GameOver
        || (g.invulnerable > 0. && (g.elapsed * 12.) as i32 % 2 == 0)
    {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
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
        let transform = Transform::from_xyz(e.pos.x, e.pos.y, e.depth).with_rotation(rotation);
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
        let direction = Vec3::new(s.velocity.x, s.velocity.y, s.velocity_z);
        let rotation = if direction.length_squared() > 1e-10 {
            Quat::from_rotation_arc(Vec3::Y, direction.normalize())
        } else {
            Quat::IDENTITY
        };
        let transform = Transform::from_xyz(s.pos.x, s.pos.y, s.depth).with_rotation(rotation);
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
        text.0 = format!(
            "SCORE  {:06}\n\nRESERVES  {:02}",
            g.score,
            g.lives.saturating_sub(1)
        );
    }
    for mut text in &mut texts.p1() {
        text.0 = format!("SECTOR {:02}", g.wave.max(1));
    }
    for mut node in &mut nodes {
        node.display = if g.phase == Phase::Playing {
            Display::Flex
        } else {
            Display::None
        };
    }
}
fn animate_background(
    time: Res<Time>,
    session: Res<Session>,
    mut stars: Query<&mut Transform, With<Backdrop>>,
    mut engines: Query<&mut Transform, (With<Engine>, Without<Backdrop>)>,
) {
    if session.0.phase == Phase::Paused {
        return;
    }
    for mut t in &mut stars {
        t.translation.y -= time.delta_secs() * 0.18;
        if t.translation.y < -21. {
            t.translation.y = 21.;
        }
    }
    for mut t in &mut engines {
        t.translation.y = -1.15 + (time.elapsed_secs() * 27.).sin() * 0.035;
    }
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
    mut smoke: ResMut<Smoke>,
    mut session: ResMut<Session>,
    mut controls: ResMut<Controls>,
    mut stats: ResMut<FrameStats>,
    mut shatters: ResMut<shatter::ShatterEffects>,
    mut shatter_queue: ResMut<shatter::ShatterQueue>,
    mut exit: MessageWriter<AppExit>,
    mut window: Single<&mut Window>,
) {
    stats.frames += 1;
    if time.elapsed_secs() > 5. && stats.samples.len() < 10000 {
        stats.samples.push(time.delta_secs());
        if smoke.stage == 4
            && time.elapsed_secs() - smoke.started > 1.5
            && window.physical_width() == 1920
            && window.physical_height() == 1080
        {
            stats.samples_1080.push(time.delta_secs());
        }
    }
    if !smoke.enabled {
        return;
    }
    let now = time.elapsed_secs();
    assert!(
        shatters.active_cubes() <= shatter::WORLD_BUDGET,
        "shatter resource budget exceeded"
    );
    assert!(
        shatters.peak_effects <= shatter::MAX_EFFECTS,
        "debris event limit exceeded"
    );
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
            let report = report.trim_end().trim_end_matches('}').to_owned()
                + &format!(
                    ",\"shatter_bursts\":{},\"peak_shatter_cubes\":{},\"shatter_both_eras\":{},\"shatter_pause_restart\":true,\"secondary_splits\":{},\"peak_voxel_bodies\":{},\"cooling_seen\":{},\"peak_damage_events\":{},\"peak_hot_vertices\":{},\"debris_bounces\":{},\"peak_debris_speed\":{:.3}}}\n",
                    shatters.bursts,
                    shatters.peak_cubes,
                    shatters.captures & 3 == 3,
                    shatters.secondary_splits,
                    shatters.peak_bodies,
                    shatters.cooling_seen,
                    shatters.peak_effects,
                    shatters.peak_hot_vertices,
                    shatters.bounces,
                    shatters.peak_speed
                );
            std::fs::write(std::path::Path::new(&dir).join("Smoke_Result.json"), report)
                .expect("write smoke result");
            info!("SMOKE complete");
            exit.write(AppExit::Success);
            smoke.stage = 8;
        }
        _ => {}
    }
}

fn sync_era(
    session: Res<Session>,
    art: Res<Art>,
    mut last: Local<Option<Era>>,
    mut ships: Query<(&mut Mesh3d, Option<&Player>), Or<(With<Player>, With<Galactica>)>>,
    mut engines: Query<&mut MeshMaterial3d<StandardMaterial>, With<Engine>>,
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
    for mut material in &mut engines {
        material.0 = if classic {
            art.gold.clone()
        } else {
            art.cyan.clone()
        };
    }
    *last = Some(era);
}

fn fit_camera(window: Single<&Window>, mut projection: Single<&mut Projection, With<Camera3d>>) {
    let aspect = window.physical_width() as f32 / window.physical_height().max(1) as f32;
    let height = 17.5_f32.max(26. / aspect);
    if let Projection::Perspective(p) = &mut **projection {
        p.fov = 2. * (height / (2. * 38.)).atan();
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
    ui.0 = layout.scale;
    for mut font in &mut fonts {
        font.font_size = layout.body_font;
    }
}

fn update_reserves(session: Res<Session>, mut icons: Query<(&ReserveIcon, &mut Node)>) {
    let reserves = session.0.lives.saturating_sub(1);
    for (icon, mut node) in &mut icons {
        node.display = if session.0.phase == Phase::Playing
            && icon.slot < reserves
            && icon.classic == (session.0.wave < 4)
        {
            Display::Flex
        } else {
            Display::None
        };
    }
}
