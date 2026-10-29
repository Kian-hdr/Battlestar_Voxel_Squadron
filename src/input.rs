use crate::{Controls, Session, ValidationMode};
use battlestar_voxel::simulation::{Input, Phase};
use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode};
pub struct InputPlugin;
#[derive(Resource)]
struct NativeTrace(bool);
impl Default for NativeTrace {
    fn default() -> Self {
        Self(std::env::var_os("BSVX_INPUT_TRACE").is_some())
    }
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeTrace>()
            .add_systems(PreUpdate, keyboard);
    }
}
fn keyboard(
    validation: Res<ValidationMode>,
    trace: Res<NativeTrace>,
    mut previous_focus: Local<Option<bool>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<Session>,
    mut controls: ResMut<Controls>,
    mut window: Single<&mut Window>,
) {
    if validation.0 {
        return;
    } // Scripted validation owns input; ordinary play retains focus safety.
    let Session(g, run) = &mut *session;
    if trace.0 {
        if *previous_focus != Some(window.focused) {
            eprintln!(
                "NATIVE_INPUT focus={} phase={:?} x={:.3}",
                window.focused, g.phase, g.player.x
            );
            *previous_focus = Some(window.focused);
        }
        for key in keys.get_just_pressed() {
            eprintln!(
                "NATIVE_INPUT pressed={key:?} focused={} phase={:?} x={:.3}",
                window.focused, g.phase, g.player.x
            );
        }
        for key in keys.get_just_released() {
            eprintln!(
                "NATIVE_INPUT released={key:?} focused={} phase={:?} x={:.3}",
                window.focused, g.phase, g.player.x
            );
        }
    }

    if !window.focused {
        if g.phase == Phase::Playing {
            g.toggle_pause();
            if trace.0 {
                eprintln!("NATIVE_INPUT focus-loss paused combat");
            }
        }
        controls.0 = Input::default();
        return;
    }
    if keys.just_pressed(KeyCode::Enter) && matches!(g.phase, Phase::Title | Phase::GameOver) {
        g.start();
        *run = run.wrapping_add(1);
    }
    if keys.just_pressed(KeyCode::Escape) {
        g.toggle_pause();
    }
    if keys.just_pressed(KeyCode::KeyM) {
        g.muted = !g.muted;
        if trace.0 {
            eprintln!("NATIVE_INPUT muted={}", g.muted);
        }
    }
    if keys.just_pressed(KeyCode::F11) {
        window.mode = match window.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            _ => WindowMode::Windowed,
        };
        if trace.0 {
            eprintln!("NATIVE_INPUT fullscreen mode={:?}", window.mode);
        }
    }
    controls.0 = if g.phase == Phase::Playing {
        Input {
            axis: (keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD)) as i8 as f32
                - (keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA)) as i8 as f32,
            fire: keys.pressed(KeyCode::Space),
        }
    } else {
        Input::default()
    };
}
