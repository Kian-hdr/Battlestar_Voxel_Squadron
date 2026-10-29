//! Development-only native render of the exact in-game voxel models, for visual QA.
use battlestar_voxel::voxel;
use bevy::{
    asset::RenderAssetUsages,
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battlestar Voxel — Model Review".into(),
                resolution:
                    bevy::window::WindowResolution::new(1600, 1100).with_scale_factor_override(1.),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, capture)
        .run();
}
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Tonemapping::Reinhard,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 34.,
                min_height: 24.,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_xyz(0., -20., 40.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 220.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.,
            ..default()
        },
        Transform::from_xyz(-10., -8., 20.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let material = mats.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 0.15,
        perceptual_roughness: 0.65,
        ..default()
    });
    for (row, classic) in [true, false].into_iter().enumerate() {
        for (col, model) in [
            voxel::viper(classic),
            voxel::raider(classic),
            voxel::galactica(classic),
        ]
        .into_iter()
        .enumerate()
        {
            let mut g = model.mesh(voxel::UNIT);
            let min = g.positions.iter().fold([f32::INFINITY; 3], |mut a, p| {
                for i in 0..3 {
                    a[i] = a[i].min(p[i]);
                }
                a
            });
            let max = g.positions.iter().fold([f32::NEG_INFINITY; 3], |mut a, p| {
                for i in 0..3 {
                    a[i] = a[i].max(p[i]);
                }
                a
            });
            let size = (max[1] - min[1]).max(max[0] - min[0]);
            let scale = 7.5 / size;
            for p in &mut g.positions {
                for i in 0..3 {
                    p[i] = (p[i] - (min[i] + max[i]) * 0.5) * scale;
                }
            }
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, g.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, g.normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, g.colors);
            mesh.insert_indices(Indices::U32(g.indices));
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                Transform::from_xyz(
                    (col as f32 - 1.) * 11.,
                    if row == 0 { 5.3 } else { -6.3 },
                    0.,
                ),
            ));
            let name = match (col, classic) {
                (0, true) => "1978 / VIPER MK I",
                (0, false) => "2004 / VIPER MK II",
                (1, true) => "1978 / CYLON RAIDER",
                (1, false) => "2004 / CYLON RAIDER",
                (_, true) => "1978 / GALACTICA",
                (_, false) => "2004 / GALACTICA",
            };
            commands.spawn((
                Text::new(name),
                TextFont {
                    font_size: 18.,
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.71, 0.48)),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(col as f32 * 33.33),
                    top: percent(if row == 0 { 3. } else { 52. }),
                    width: percent(33.33),
                    ..default()
                },
            ));
        }
    }
}
fn capture(
    mut commands: Commands,
    time: Res<Time>,
    mut done: Local<bool>,
    mut exit: MessageWriter<AppExit>,
) {
    if time.elapsed_secs() > 5. && !*done {
        let dir = std::env::var("BSVX_EVIDENCE_DIR").unwrap_or_else(|_| "evidence".into());
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(
                std::path::Path::new(&dir).join("06_Model_Reference_Review.png"),
            ));
        *done = true;
    }
    if time.elapsed_secs() > 7. {
        exit.write(AppExit::Success);
    }
}
