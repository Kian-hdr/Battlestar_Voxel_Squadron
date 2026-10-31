use crate::Session;
use battlestar_voxel::simulation::Event;
use bevy::audio::Volume;
use bevy::prelude::*;
pub struct SoundPlugin;
#[derive(Resource)]
pub struct AudioSettings {
    pub effects_volume: f32,
    silent_validation: bool,
}
impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            effects_volume: 0.20,
            silent_validation: std::env::args()
                .any(|arg| arg == "--smoke-test" || arg == "--soak-test")
                || std::env::var_os("BSVX_SILENT_CHECK").is_some(),
        }
    }
}
impl AudioSettings {
    fn level(&self) -> f32 {
        if self.silent_validation {
            0.0
        } else if self.effects_volume.is_finite() {
            self.effects_volume.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}
#[derive(Component)]
struct SoundLifetime(f32);
#[derive(Resource)]
pub struct Sounds {
    pub shot: Handle<AudioSource>,
    pub enemy: Handle<AudioSource>,
    pub boom: Handle<AudioSource>,
}
impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioSettings>()
            .add_systems(Startup, setup)
            .add_systems(Update, expire_sounds);
    }
}
fn setup(mut commands: Commands, mut assets: ResMut<Assets<AudioSource>>) {
    commands.insert_resource(Sounds {
        shot: assets.add(AudioSource {
            bytes: include_bytes!(concat!(env!("OUT_DIR"), "/Player_Fire.wav"))
                .as_slice()
                .into(),
        }),
        enemy: assets.add(AudioSource {
            bytes: include_bytes!(concat!(env!("OUT_DIR"), "/Enemy_Fire.wav"))
                .as_slice()
                .into(),
        }),
        boom: assets.add(AudioSource {
            bytes: include_bytes!(concat!(env!("OUT_DIR"), "/Hull_Impact.wav"))
                .as_slice()
                .into(),
        }),
    });
}
pub fn play_events(
    commands: &mut Commands,
    session: &Session,
    sounds: &Sounds,
    settings: &AudioSettings,
) {
    if session.0.muted || settings.level() == 0.0 {
        return;
    }
    for e in &session.0.events {
        let source = match e {
            Event::Shot(true) => &sounds.shot,
            Event::Shot(false) => &sounds.enemy,
            Event::PlayerDestroyed { .. } | Event::EnemyDestroyed { .. } => &sounds.boom,
            Event::Wave => continue,
        };
        commands.spawn((
            AudioPlayer::new(source.clone()),
            PlaybackSettings {
                volume: Volume::Linear(settings.level()),
                ..PlaybackSettings::DESPAWN
            },
            SoundLifetime(
                if matches!(
                    e,
                    Event::PlayerDestroyed { .. } | Event::EnemyDestroyed { .. }
                ) {
                    1.15
                } else {
                    0.75
                },
            ),
        ));
    }
}
/// Menu audition uses the same electronic impact and effects bus as normal play.
pub fn preview(
    commands: &mut Commands,
    session: &Session,
    sounds: &Sounds,
    settings: &AudioSettings,
) {
    if session.0.muted || settings.level() == 0. {
        return;
    }
    commands.spawn((
        AudioPlayer::new(sounds.boom.clone()),
        PlaybackSettings {
            volume: Volume::Linear(settings.level()),
            ..PlaybackSettings::DESPAWN
        },
        SoundLifetime(1.15),
    ));
}

// Bevy cannot finish a sink when no audio output device exists; bound that case too.
fn expire_sounds(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<Session>,
    settings: Res<AudioSettings>,
    mut sounds: Query<(
        Entity,
        &mut SoundLifetime,
        &mut PlaybackSettings,
        Option<&mut AudioSink>,
    )>,
) {
    for (entity, mut life, mut playback, sink) in &mut sounds {
        life.0 -= time.delta_secs();
        if session.0.muted || settings.level() == 0.0 {
            if let Some(sink) = sink {
                sink.stop();
            }
            // Remove queued players too, so unmuting cannot revive an old sound.
            commands.entity(entity).try_despawn();
        } else if life.0 <= 0. {
            commands.entity(entity).try_despawn();
        } else {
            let volume = Volume::Linear(settings.level());
            playback.volume = volume;
            if let Some(mut sink) = sink {
                sink.set_volume(volume);
            }
        }
    }
}
