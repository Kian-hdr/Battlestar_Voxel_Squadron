//! Native, logical-point menu composition. No window-wide UI scale transform.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]
use crate::{
    Session,
    audio::{self, AudioSettings, Sounds},
};
use battlestar_voxel::simulation::Phase;
use bevy::{prelude::*, ui::RelativeCursorPosition, window::PrimaryWindow};

// Palette interpreted from Colonial CIC production and official-series references.
const INK: Color = Color::srgb_u8(10, 20, 28);
const WHITE: Color = Color::srgb_u8(219, 230, 234);
const SECONDARY: Color = Color::srgb_u8(153, 174, 185);
const COLONIAL_RED: Color = Color::srgb_u8(168, 45, 60);
const READOUT: Color = Color::srgb_u8(117, 184, 166);
const LINE: Color = Color::srgb_u8(46, 66, 78);
const CONTROL: Color = Color::srgb_u8(22, 38, 49);
const RED_HOVER: Color = Color::srgb_u8(177, 51, 66);
const RED_PRESSED: Color = Color::srgb_u8(136, 31, 45);
const CONTROL_HOVER: Color = Color::srgb_u8(38, 60, 74);
const CONTROL_PRESSED: Color = Color::srgb_u8(48, 75, 90);

#[derive(Component)]
pub(super) struct MenuRoot;
#[derive(Component)]
pub(super) enum Layout {
    Panel,
    Header,
    Columns,
    ActionColumn,
    SoundColumn,
    Footer,
}
#[derive(Component)]
pub(super) enum Copy {
    State,
    Context,
    Primary,
    Mute,
    Volume,
    Fill,
}
#[derive(Component, Clone, Copy)]
pub(super) enum Action {
    Primary,
    Mute,
    Step(f32),
    Slider,
    Preview,
}

fn text(commands: &mut Commands, value: &str, size: f32, color: Color) -> Entity {
    commands
        .spawn((
            Text::new(value),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
        ))
        .id()
}
fn column(commands: &mut Commands, gap: f32) -> Entity {
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(gap),
            min_width: px(0.),
            ..default()
        })
        .id()
}
fn button(commands: &mut Commands, label: &str, action: Action, primary: bool) -> Entity {
    let entity = commands
        .spawn((
            Button,
            action,
            Node {
                height: px(44.),
                min_height: px(44.),
                padding: UiRect::horizontal(px(14.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1.)),
                ..default()
            },
            BackgroundColor(if primary { COLONIAL_RED } else { CONTROL }),
            BorderColor::all(if primary { COLONIAL_RED } else { LINE }),
        ))
        .id();
    let label = text(commands, label, 16., WHITE);
    if primary {
        commands.entity(label).insert(Copy::Primary);
    }
    if matches!(action, Action::Mute) {
        commands.entity(label).insert(Copy::Mute);
    }
    commands.entity(entity).add_child(label);
    entity
}
fn control_row(commands: &mut Commands, key: &str, description: &str) -> Entity {
    let row = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: px(12.),
            ..default()
        })
        .id();
    let key_label = text(commands, key, 14., WHITE);
    commands.entity(key_label).insert(Node {
        width: px(138.),
        ..default()
    });
    let description = text(commands, description, 14., SECONDARY);
    commands.entity(row).add_children(&[key_label, description]);
    row
}

pub(super) fn spawn(commands: &mut Commands, fonts: &mut Assets<Font>) -> Entity {
    let display_font = fonts.add(
        Font::try_from_bytes(include_bytes!("../assets/Fonts/PressStart2P_Regular.ttf").to_vec())
            .expect("embedded OFL display font"),
    );
    let root = commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.52)),
            GlobalZIndex(20),
        ))
        .id();
    let panel = commands
        .spawn((
            Layout::Panel,
            Node {
                width: percent(92.),
                max_width: px(920.),
                padding: UiRect::all(px(32.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(28.),
                ..default()
            },
            BackgroundColor(INK),
        ))
        .id();
    commands.entity(root).add_child(panel);
    let header = commands
        .spawn((
            Layout::Header,
            Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::FlexStart,
                column_gap: px(24.),
                padding: UiRect::bottom(px(22.)),
                border: UiRect::bottom(px(1.)),
                ..default()
            },
            BorderColor::all(LINE),
        ))
        .id();
    let identity = column(commands, 12.);
    let title = commands
        .spawn((
            Text::new("BATTLESTAR:\nVOXEL SQUADRON"),
            TextFont {
                font: display_font,
                font_size: 22.,
                ..default()
            },
            TextColor(WHITE),
        ))
        .id();
    let subtitle = text(commands, "Defend the fleet.", 14., SECONDARY);
    commands.entity(identity).add_children(&[title, subtitle]);
    let context = text(commands, "", 14., READOUT);
    commands
        .entity(context)
        .insert((Copy::Context, TextLayout::new_with_justify(Justify::Right)));
    commands.entity(header).add_children(&[identity, context]);
    let columns = commands
        .spawn((
            Layout::Columns,
            Node {
                column_gap: px(40.),
                align_items: AlignItems::FlexStart,
                ..default()
            },
        ))
        .id();
    let actions = column(commands, 20.);
    commands.entity(actions).insert((
        Layout::ActionColumn,
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(20.),
            flex_grow: 1.,
            flex_basis: px(0.),
            min_width: px(0.),
            ..default()
        },
    ));
    let state = text(commands, "Ready to launch", 20., WHITE);
    commands.entity(state).insert(Copy::State);
    let launch = button(commands, "LAUNCH  [ENTER]", Action::Primary, true);
    commands.entity(launch).insert(Node {
        width: percent(100.),
        height: px(48.),
        min_height: px(48.),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    });
    let controls = column(commands, 10.);
    let controls_title = text(commands, "Flight controls", 14., WHITE);
    let move_row = control_row(commands, "A / D or arrows", "Move");
    let fire_row = control_row(commands, "SPACE (hold)", "Fire");
    let pause_row = control_row(commands, "ESC", "Pause / resume");
    let full_row = control_row(commands, "F11", "Fullscreen");
    commands.entity(controls).add_children(&[
        controls_title,
        move_row,
        fire_row,
        pause_row,
        full_row,
    ]);
    commands
        .entity(actions)
        .add_children(&[state, launch, controls]);
    let sound = commands
        .spawn((
            Layout::SoundColumn,
            Node {
                width: px(270.),
                min_width: px(240.),
                flex_direction: FlexDirection::Column,
                row_gap: px(18.),
                padding: UiRect::left(px(32.)),
                border: UiRect::left(px(1.)),
                ..default()
            },
            BorderColor::all(LINE),
        ))
        .id();
    let sound_title = text(commands, "Sound", 20., WHITE);
    let level = text(commands, "Effects volume   20%", 14., SECONDARY);
    commands.entity(level).insert(Copy::Volume);
    let slider_row = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: px(10.),
            ..default()
        })
        .id();
    let minus = button(commands, "-", Action::Step(-0.1), false);
    let plus = button(commands, "+", Action::Step(0.1), false);
    for id in [minus, plus] {
        commands.entity(id).insert(Node {
            width: px(40.),
            height: px(44.),
            min_width: px(40.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        });
    }
    let slider = commands
        .spawn((
            Button,
            Action::Slider,
            RelativeCursorPosition::default(),
            Node {
                height: px(44.),
                flex_grow: 1.,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(CONTROL),
        ))
        .id();
    let track = commands
        .spawn((
            Node {
                width: percent(100.),
                height: px(4.),
                ..default()
            },
            BackgroundColor(LINE),
            bevy::ui::FocusPolicy::Pass,
        ))
        .id();
    let fill = commands
        .spawn((
            Copy::Fill,
            Node {
                width: percent(20.),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(READOUT),
            bevy::ui::FocusPolicy::Pass,
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(slider).add_child(track);
    commands
        .entity(slider_row)
        .add_children(&[minus, slider, plus]);
    let mute = button(commands, "MUTE  [M]", Action::Mute, false);
    let preview = button(commands, "TEST EFFECT", Action::Preview, false);
    commands
        .entity(sound)
        .add_children(&[sound_title, level, slider_row, mute, preview]);
    commands.entity(columns).add_children(&[actions, sound]);
    let footer = commands
        .spawn((
            Layout::Footer,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(7.),
                padding: UiRect::top(px(18.)),
                border: UiRect::top(px(1.)),
                ..default()
            },
            BorderColor::all(LINE),
        ))
        .id();
    let creator = text(commands, "Created by Kian Konrad Tajbakhsh", 14., WHITE);
    let credits = text(
        commands,
        "Unofficial noncommercial fan prototype.",
        12.,
        SECONDARY,
    );
    commands.entity(footer).add_children(&[creator, credits]);
    commands
        .entity(panel)
        .add_children(&[header, columns, footer]);
    root
}

pub(super) fn interact(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut settings: ResMut<AudioSettings>,
    sounds: Res<Sounds>,
    mut buttons: Query<(
        Ref<Interaction>,
        &Action,
        &mut BackgroundColor,
        Option<&RelativeCursorPosition>,
    )>,
) {
    if session.0.phase == Phase::Playing {
        return;
    }
    for (interaction, action, mut color, cursor) in &mut buttons {
        let primary = matches!(action, Action::Primary);
        if !matches!(action, Action::Slider) {
            *color = BackgroundColor(match (*interaction, primary) {
                (Interaction::Pressed, true) => RED_PRESSED,
                (Interaction::Hovered, true) => RED_HOVER,
                (_, true) => COLONIAL_RED,
                (Interaction::Pressed, false) => CONTROL_PRESSED,
                (Interaction::Hovered, false) => CONTROL_HOVER,
                _ => CONTROL,
            });
        }
        if *interaction != Interaction::Pressed
            || (!interaction.is_changed() && !matches!(action, Action::Slider))
        {
            continue;
        }
        let name = match action {
            Action::Primary => {
                if session.0.phase == Phase::Paused {
                    session.0.toggle_pause();
                } else {
                    session.0.start();
                    session.1 += 1;
                }
                "primary"
            }
            Action::Mute => {
                session.0.muted = !session.0.muted;
                "mute"
            }
            Action::Step(step) => {
                settings.effects_volume = (settings.effects_volume + step).clamp(0., 1.);
                "volume_step"
            }
            Action::Slider => {
                if let Some(position) = cursor.and_then(|p| p.normalized) {
                    settings.effects_volume = (position.x + 0.5).clamp(0., 1.);
                }
                "volume_slider"
            }
            Action::Preview => {
                audio::preview(&mut commands, &session, &sounds, &settings);
                "preview"
            }
        };
        if std::env::var_os("BSVX_INPUT_TRACE").is_some() {
            eprintln!(
                "NATIVE_INPUT menu_action={name} volume={:.3} muted={}",
                settings.effects_volume, session.0.muted
            );
        }
    }
}

pub(super) fn update(
    session: Res<Session>,
    settings: Res<AudioSettings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut roots: Query<&mut Visibility, With<MenuRoot>>,
    mut layout: Query<(&Layout, &mut Node)>,
    mut copies: Query<(&Copy, Option<&mut Text>, Option<&mut Node>), Without<Layout>>,
) {
    let visible = session.0.phase != Phase::Playing;
    for mut root in &mut roots {
        root.set_if_neq(if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
    if !visible {
        return;
    }
    let compact = windows
        .single()
        .is_ok_and(|w| w.height() < 650. || w.width() < 1000.);
    for (part, mut node) in &mut layout {
        match part {
            Layout::Panel => {
                node.padding = UiRect::all(px(if compact { 22. } else { 32. }));
                node.row_gap = px(if compact { 20. } else { 28. });
            }
            Layout::Header => {
                node.padding.bottom = px(if compact { 16. } else { 22. });
            }
            Layout::Columns => node.column_gap = px(if compact { 26. } else { 40. }),
            Layout::ActionColumn => node.row_gap = px(if compact { 16. } else { 20. }),
            Layout::SoundColumn => {
                node.width = px(if compact { 264. } else { 290. });
                node.padding.left = px(if compact { 24. } else { 32. });
                node.row_gap = px(if compact { 16. } else { 18. });
            }
            Layout::Footer => node.padding.top = px(if compact { 14. } else { 18. }),
        }
    }
    let game = &session.0;
    for (slot, label, fill) in &mut copies {
        if matches!(slot, Copy::Fill) {
            if let Some(mut fill) = fill {
                fill.width = percent(settings.effects_volume.clamp(0., 1.) * 100.);
            }
            continue;
        }
        let Some(mut label) = label else {
            continue;
        };
        let value = match slot {
            Copy::State => match game.phase {
                Phase::Paused => "Flight paused",
                Phase::GameOver => "Fleet lost",
                _ => "Ready to launch",
            }
            .into(),
            Copy::Context => {
                if game.phase == Phase::Title {
                    "CLASSIC FLEET\nSECTOR 01\n3 LIVES".into()
                } else {
                    format!(
                        "SECTOR {:02}\nSCORE {:06}\n{} {}",
                        game.wave,
                        game.score,
                        game.lives,
                        if game.lives == 1 { "LIFE" } else { "LIVES" }
                    )
                }
            }
            Copy::Primary => if game.phase == Phase::Paused {
                "RESUME  [ESC]"
            } else if game.phase == Phase::GameOver {
                "RETRY  [ENTER]"
            } else {
                "LAUNCH  [ENTER]"
            }
            .into(),
            Copy::Mute => if game.muted {
                "UNMUTE  [M]"
            } else {
                "MUTE  [M]"
            }
            .into(),
            Copy::Volume => format!("Effects volume  {:.0}%", settings.effects_volume * 100.),
            Copy::Fill => unreachable!(),
        };
        if label.0 != value {
            label.0 = value;
        }
    }
}
