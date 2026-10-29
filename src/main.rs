#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod audio;
mod input;
mod presentation;
use battlestar_voxel::simulation::{DT, Game};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, WindowMode, WindowResolution};

#[derive(Resource, Default)]
pub struct Session(pub Game, pub u64);
#[derive(Resource)]
pub struct ValidationMode(pub bool);
#[derive(Resource, Default)]
pub struct Controls(pub battlestar_voxel::simulation::Input);
// Record both sides of the runner boundary. A completed smoke scenario only
// requests exit; this message proves the event loop and its world teardown returned.
fn main() -> AppExit {
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Battlestar: Voxel Squadron".into(),
            resolution: WindowResolution::new(1280, 800),
            mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
            present_mode: PresentMode::AutoVsync,
            resizable: true,
            resize_constraints: bevy::window::WindowResizeConstraints {
                min_width: 800.,
                min_height: 500.,
                ..default()
            },
            ..default()
        }),
        ..default()
    });
    // On macOS, the captured shutdown stack deadlocked in RenderAppChannels::drop
    // waiting for a render schedule that itself awaited main-thread execution.
    // Keep rendering in the main app via the supported plugin switch, allowing
    // normal winit/world teardown. This trades CPU/render overlap for reliability.
    #[cfg(target_os = "macos")]
    let plugins = plugins.disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>();
    let exit = App::new()
        .insert_resource(ClearColor(Color::srgb(0.002, 0.005, 0.012)))
        .insert_resource(Time::<Fixed>::from_seconds(DT as f64))
        .insert_resource(ValidationMode(
            std::env::args().any(|a| a == "--smoke-test"),
        ))
        .init_resource::<Session>()
        .init_resource::<Controls>()
        .add_systems(Last, record_exit_request)
        .add_plugins(plugins)
        .add_plugins((
            input::InputPlugin,
            audio::SoundPlugin,
            presentation::PresentationPlugin,
        ))
        .run();
    eprintln!("LIFECYCLE runner returned: {exit:?}");
    exit
}

fn record_exit_request(mut exits: MessageReader<AppExit>) {
    for exit in exits.read() {
        eprintln!("LIFECYCLE AppExit observed: {exit:?}");
    }
}
