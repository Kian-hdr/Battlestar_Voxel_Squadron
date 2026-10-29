use battlestar_voxel::simulation::*;
use glam::Vec3;

fn point(e: &Enemy) -> Vec3 {
    Vec3::new(e.pos.x, e.pos.y, e.depth)
}
fn assert_separated(g: &Game) {
    for (i, a) in g.enemies.iter().enumerate() {
        assert!(a.depth.is_finite() && a.depth.abs() <= MAX_ENEMY_DEPTH + 0.0001);
        assert!(a.velocity_z.is_finite());
        for b in &g.enemies[i + 1..] {
            assert!(
                !enemy_hulls_overlap(a, b),
                "occupied material of hulls{} and{} overlaps",
                a.id,
                b.id
            );
        }
    }
}
fn pair(mode: EnemyMode) -> Game {
    let mut g = Game::default();
    g.start();
    g.invulnerable = 999.;
    g.enemies.truncate(2);
    for (i, e) in g.enemies.iter_mut().enumerate() {
        let side = if i == 0 { -1. } else { 1. };
        e.pos = Point {
            x: side * 4.,
            y: 6.,
        };
        e.origin = e.pos;
        e.home = Point {
            x: -side * 4.,
            y: 6.,
        };
        e.mode = mode;
        e.age = 0.;
        e.target_x = -side * 4.;
        e.depth = 0.;
        e.velocity = Point::default();
        e.rotation = glam::Quat::IDENTITY.to_array();
        if mode == EnemyMode::Returning {
            e.pos.y = -13.;
            e.origin = e.pos;
        }
    }
    g
}

#[test]
fn converging_entry_dive_and_return_paths_keep_actual_conservative_hulls_separate() {
    for mode in [EnemyMode::Entering, EnemyMode::Diving, EnemyMode::Returning] {
        let mut g = pair(mode);
        let mut saw_depth = false;
        for _ in 0..520 {
            let previous: Vec<_> = g.enemies.iter().map(point).collect();
            g.step(Input::default(), DT);
            assert_separated(&g);
            for (e, p) in g.enemies.iter().zip(previous) {
                assert!(
                    point(e).distance(p) < 1.,
                    "flight correction must not teleport"
                );
                saw_depth |= e.depth.abs() > 0.1;
            }
        }
        if mode != EnemyMode::Returning {
            assert!(saw_depth);
        }
    }
}

#[test]
fn crossing_priority_is_id_deterministic_and_clear_routes_return_to_combat_plane() {
    let mut a = pair(EnemyMode::Entering);
    let mut b = a.clone();
    b.enemies.reverse();
    let mut avoided = false;
    for _ in 0..300 {
        a.step(Input::default(), DT);
        b.step(Input::default(), DT);
        for e in &a.enemies {
            let other = b.enemies.iter().find(|other| other.id == e.id).unwrap();
            assert_eq!(e, other);
            avoided |= e.depth.abs() > 0.1;
        }
    }
    assert!(avoided);
    a.enemies.truncate(1);
    a.enemies[0].mode = EnemyMode::Formation;
    let previous = a.enemies[0].depth.abs();
    for _ in 0..420 {
        a.step(Input::default(), DT);
    }
    assert!(a.enemies[0].depth.abs() < 0.001);
    assert!(a.enemies[0].depth.abs() <= previous);
}

#[test]
fn full_fleet_starts_spaced_and_stays_finite_separate_and_continuous() {
    for classic in [false, true] {
        let mut g = Game::default();
        g.start();
        g.wave = if classic { 1 } else { 4 };
        for e in &mut g.enemies {
            e.classic = classic;
        }
        g.invulnerable = 999.;
        assert_separated(&g);
        for _ in 0..4200 {
            let before: Vec<_> = g.enemies.iter().map(point).collect();
            g.step(Input::default(), DT);
            assert_separated(&g);
            assert_eq!(g.lives, 3);
            assert_eq!(g.enemies.len(), 24);
            assert_eq!(g.score, 0);
            for (e, p) in g.enemies.iter().zip(before) {
                assert!(point(e).is_finite());
                assert!(
                    point(e).distance(p) < 1.,
                    "continuous flight violated by ship{} {:?}->{:?}",
                    e.id,
                    p,
                    point(e)
                );
            }
        }
    }
}

#[test]
fn shots_obey_actual_enemy_depth_and_events_keep_vertical_motion() {
    let mut g = pair(EnemyMode::Formation);
    g.enemies.truncate(1);
    g.enemies[0].home = Point { x: 0., y: 4. };
    g.enemies[0].pos = Point { x: 0., y: 4. };
    g.enemies[0].depth = 2.;
    g.inject_shot(g.enemies[0].pos, true);
    g.step(Input::default(), DT);
    assert_eq!(g.score, 0);
    g.shots.clear();
    g.enemies[0].depth = 0.;
    g.enemies[0].rotation = glam::Quat::IDENTITY.to_array();
    g.inject_shot(g.enemies[0].pos, true);
    g.step(Input::default(), DT);
    assert_eq!(g.score, 100);
    assert!(g.events.iter().any(|e|matches!(e,Event::EnemyDestroyed{depth,velocity_z,..} if depth.is_finite()&&velocity_z.is_finite())));
}

#[test]
fn hostile_shots_start_at_raider_height_and_aim_back_to_player_plane() {
    let mut g = pair(EnemyMode::Formation);
    g.enemies.truncate(1);
    let mut observed = false;
    for _ in 0..510 {
        g.enemies[0].depth = 2.;
        g.step(Input::default(), DT);
        if let Some(shot) = g.shots.iter().find(|s| !s.friendly) {
            assert!((shot.previous_depth - g.enemies[0].depth - 0.15).abs() < 1e-5);
            assert!(shot.velocity_z < 0.);
            assert!((shot.depth - shot.previous_depth - shot.velocity_z * DT).abs() < 1e-5);
            let flight_time = (g.player.y - shot.previous.y) / shot.velocity.y;
            assert!((shot.previous_depth + shot.velocity_z * flight_time - 0.15).abs() < 1e-5);
            observed = true;
            break;
        }
    }
    assert!(
        observed,
        "normal enemy firing must exercise the depth-aware shot path"
    );
}

#[test]
fn fast_depth_axis_shots_sweep_actual_player_hull_without_tunneling() {
    let mut g = Game::default();
    g.start();
    g.enemies.clear();
    g.invulnerable = 0.;
    g.inject_shot(g.player, false);
    let shot = g.shots.last_mut().unwrap();
    shot.velocity = Point::default();
    shot.depth = 3.;
    shot.previous_depth = 3.;
    g.step(Input::default(), DT);
    assert_eq!(g.lives, 3, "projected XY overlap alone is not a hit");
    g.shots.clear();
    g.inject_shot(g.player, false);
    let shot = g.shots.last_mut().unwrap();
    shot.velocity = Point::default();
    shot.depth = 3.;
    shot.previous_depth = 3.;
    shot.velocity_z = -720.;
    g.step(Input::default(), DT);
    assert_eq!(
        g.lives, 2,
        "swept Z path passes through occupied player voxels"
    );
    assert!(
        g.events
            .iter()
            .any(|e| matches!(e, Event::PlayerDestroyed { .. }))
    );
}

#[test]
fn injected_friendly_shots_remain_in_the_original_flat_fire_plane() {
    let mut g = Game::default();
    g.start();
    g.inject_shot(Point { x: 0., y: -6. }, true);
    let id = g.shots.last().unwrap().id;
    for _ in 0..10 {
        g.step(Input::default(), DT);
    }
    let shot = g.shots.iter().find(|s| s.id == id).unwrap();
    assert_eq!(shot.depth, 0.15);
    assert_eq!(shot.previous_depth, 0.15);
    assert_eq!(shot.velocity_z, 0.);
    assert_eq!(shot.target_id, None);
}

fn aim_fixture(classic: bool, depth: f32) -> Game {
    let mut g = Game::default();
    g.start();
    g.invulnerable = 999.;
    g.enemies.truncate(1);
    g.wave = if classic { 1 } else { 4 };
    // At the formation sway's turning point, the target remains under the fixed
    // horizontal aim. This isolates cinematic depth from ordinary X lead.
    g.elapsed = std::f32::consts::FRAC_PI_2 / 0.85;
    let e = &mut g.enemies[0];
    e.classic = classic;
    e.mode = EnemyMode::Formation;
    e.pos = Point { x: 0., y: 3. };
    e.home = Point { x: -0.65, y: 3. };
    e.origin = e.pos;
    e.age = 0.;
    e.depth = depth;
    e.rotation = glam::Quat::IDENTITY.to_array();
    g
}

#[test]
fn depth_only_fire_control_hits_both_eras_at_positive_and_negative_cinematic_height() {
    for classic in [false, true] {
        for depth in [-2.4, 2.4] {
            let mut g = aim_fixture(classic, depth);
            let enemy_id = g.enemies[0].id;
            g.step(
                Input {
                    axis: 0.,
                    fire: true,
                },
                DT,
            );
            let shot = g.shots.iter().find(|s| s.friendly).unwrap();
            let shot_id = shot.id;
            let initial_x = shot.pos.x;
            assert_eq!(shot.target_id, Some(enemy_id));
            for _ in 0..160 {
                for s in g.shots.iter().filter(|s| s.id == shot_id) {
                    assert_eq!(
                        s.pos.x, initial_x,
                        "depth assist must never home horizontally"
                    );
                    assert_eq!(s.velocity.x, 0.);
                    assert_eq!(s.velocity.y, 24.);
                    assert!(s.depth.is_finite() && s.depth.abs() < 4.);
                    assert!(s.velocity_z.is_finite() && s.velocity_z.abs() <= 18.);
                    assert!(
                        (s.depth - s.previous_depth).abs() <= 18. * DT + 0.00001,
                        "no depth teleport"
                    );
                }
                g.step(Input::default(), DT);
                if g.enemies.is_empty() {
                    break;
                }
            }
            assert_eq!(
                g.score, 100,
                "aligned fixed-X shot missed classic={classic} initialdepth={depth}"
            );
        }
    }
}

#[test]
fn depth_tracking_follows_a_lifting_target_without_changing_xy_aim() {
    for classic in [false, true] {
        let mut g = aim_fixture(classic, 0.);
        g.step(
            Input {
                axis: 0.,
                fire: true,
            },
            DT,
        );
        let shot_id = g.shots.iter().find(|s| s.friendly).unwrap().id;
        let mut hit = false;
        for tick in 0..160 {
            if let Some(e) = g.enemies.first_mut() {
                e.depth = (tick as f32 * DT * 6.).min(2.);
            }
            g.step(Input::default(), DT);
            for s in g.shots.iter().filter(|s| s.id == shot_id) {
                assert_eq!(s.pos.x, 0.);
                assert_eq!(s.velocity.x, 0.);
                assert_eq!(s.velocity.y, 24.);
                assert!(s.depth.is_finite() && s.velocity_z.abs() <= 18.);
                assert!((s.depth - s.previous_depth).abs() <= 18. * DT + 0.00001);
            }
            if g.score == 100 {
                hit = true;
                break;
            }
        }
        assert!(
            hit,
            "fixed-X shot must track actual lifting hull classic={classic}"
        );
    }
}

#[test]
fn unaligned_targets_do_not_create_horizontal_homing_or_depth_assist() {
    let mut g = aim_fixture(true, 2.4);
    g.enemies[0].pos.x = 5.;
    g.enemies[0].home.x = 5.;
    g.step(
        Input {
            axis: 0.,
            fire: true,
        },
        DT,
    );
    let shot = g.shots.iter().find(|s| s.friendly).unwrap();
    assert_eq!(shot.target_id, None);
    assert_eq!(shot.depth, 0.15);
    assert_eq!(shot.velocity_z, 0.);
}

#[test]
fn separated_flight_has_bounded_acceleration_and_jerk_through_all_modes() {
    let mut g = Game::default();
    g.start();
    g.invulnerable = 999.;
    g.enemies.truncate(1);
    let mut previous_velocity = Vec3::ZERO;
    let mut previous_acceleration = Vec3::ZERO;
    let mut modes = [false; 4];
    for _ in 0..1800 {
        g.step(Input::default(), DT);
        let e = &g.enemies[0];
        let velocity = Vec3::new(e.velocity.x, e.velocity.y, e.velocity_z);
        let acceleration = (velocity - previous_velocity) / DT;
        assert!(
            acceleration.abs().max_element() < 62.,
            "uncommanded acceleration: {acceleration:?}"
        );
        let jerk = (acceleration - previous_acceleration) / DT;
        assert!(
            jerk.abs().max_element() < 390.,
            "flight impulse without contact: {jerk:?}"
        );
        previous_velocity = velocity;
        previous_acceleration = acceleration;
        modes[match e.mode {
            EnemyMode::Entering => 0,
            EnemyMode::Formation => 1,
            EnemyMode::Diving => 2,
            EnemyMode::Returning => 3,
        }] = true;
    }
    assert!(modes.into_iter().all(|seen| seen));
}

#[test]
fn empty_crescent_space_does_not_trigger_a_hard_bounding_box_response() {
    use battlestar_voxel::voxel::{UNIT, raider};
    let hull = raider(false);
    let mut g = pair(EnemyMode::Formation);
    for e in &mut g.enemies {
        e.classic = false;
        e.rotation = glam::Quat::IDENTITY.to_array();
    }
    let extents = Vec3::from_array(enemy_half_extents(&g.enemies[0])) * 2.;
    // Independent integer-grid material oracle: boxes overlap substantially,
    // but the translated actual occupied sets are disjoint within the crescent.
    let offset = (0..18)
        .flat_map(|x| (0..18).map(move |y| [x, y, 0]))
        .find(|offset| {
            let shift = Vec3::from_array(offset.map(|v| v as f32 * UNIT));
            (extents - shift).min_element() > 0.12
                && !hull.0.keys().any(|p| {
                    hull.0
                        .contains_key(&[p[0] - offset[0], p[1] - offset[1], p[2]])
                })
        })
        .expect("crescent hull exposes a material-free overlapping bounds fixture");
    for (i, e) in g.enemies.iter_mut().enumerate() {
        e.pos = Point {
            x: offset[0] as f32 * UNIT * i as f32,
            y: offset[1] as f32 * UNIT * i as f32,
        };
        e.home = e.pos;
        e.depth = 0.;
        e.velocity = Point::default();
    }
    assert!(!enemy_hulls_overlap(&g.enemies[0], &g.enemies[1]));
    let before: Vec<_> = g.enemies.iter().map(point).collect();
    g.step(Input::default(), DT);
    for (e, p) in g.enemies.iter().zip(before) {
        assert!(
            point(e).distance(p) < 0.001,
            "empty bounding space caused an impulse"
        );
        assert!(Vec3::new(e.velocity.x, e.velocity.y, e.velocity_z).length() < 0.1);
    }
    let a = g.enemies[0].clone();
    let mut b = a.clone();
    b.id += 100;
    assert!(
        enemy_hulls_overlap(&a, &b),
        "coincident actual material must register contact"
    );
}

#[test]
fn banked_left_formation_jam_yields_without_freezing_escape_or_losing_material_separation() {
    for classic in [false, true] {
        let mut g = Game::default();
        g.start();
        g.invulnerable = 999.;
        g.enemies.truncate(6);
        g.wave = if classic { 1 } else { 4 };
        let slots: Vec<_> = (0..6)
            .map(|i| Point {
                x: -8. + (i % 2) as f32 * 2.5,
                y: 1.8 + (i / 2) as f32 * 2.5,
            })
            .collect();
        for (i, e) in g.enemies.iter_mut().enumerate() {
            e.classic = classic;
            e.mode = EnemyMode::Formation;
            e.age = 0.;
            e.pos = slots[i];
            e.home = slots[(i + 3) % 6];
            e.origin = e.pos;
            e.depth = 0.;
            e.rotation = (glam::Quat::from_rotation_y(if i % 2 == 0 { 0.4 } else { -0.4 })
                * glam::Quat::from_rotation_z(0.2))
            .to_array();
        }
        assert_separated(&g);
        let mut stalled = [0; 6];
        let mut rejoined = [false; 6];
        for _ in 0..2400 {
            let previous: Vec<_> = g.enemies.iter().map(point).collect();
            g.step(Input::default(), DT);
            assert_separated(&g);
            for (i, (e, p)) in g.enemies.iter().zip(previous).enumerate() {
                let home = Vec3::new(e.home.x + (g.elapsed * 0.85).sin() * 0.65, e.home.y, 0.);
                let error = point(e).truncate().distance(home.truncate());
                let needs_progress = (e.mode != EnemyMode::Formation || error > 0.4)
                    && !(e.mode == EnemyMode::Entering && e.age < 0.);
                if needs_progress && point(e).distance(p) < 0.001 {
                    stalled[i] += 1;
                } else {
                    stalled[i] = 0;
                }
                assert!(
                    stalled[i] < 120,
                    "left-cluster ship{} stalled for1s classic={classic}",
                    e.id
                );
                assert!(
                    point(e).distance(p) < 0.5,
                    "jam recovery teleported a craft"
                );
                rejoined[i] |= e.mode == EnemyMode::Formation && error < 0.5 && e.depth.abs() < 0.5;
            }
            if rejoined.iter().all(|done| *done) {
                break;
            }
        }
        assert!(
            rejoined.iter().all(|done| *done),
            "some jammed craft never rejoined classic={classic}: {rejoined:?} ships={:?}",
            g.enemies
        );
    }
}

#[test]
fn crowded_dive_return_traffic_keeps_progress_and_rejoins_in_both_eras() {
    for classic in [false, true] {
        let mut g = Game::default();
        g.start();
        g.invulnerable = 999.;
        g.wave = if classic { 1 } else { 4 };
        for e in &mut g.enemies {
            e.classic = classic;
        }
        let mut stalled = [0; 24];
        let mut left_formation = [false; 24];
        let mut rejoined = [false; 24];
        let mut flight_started = [None; 24];
        for _ in 0..7200 {
            let before: Vec<_> = g.enemies.iter().map(point).collect();
            g.step(Input::default(), DT);
            assert_separated(&g);
            for (i, (e, p)) in g.enemies.iter().zip(before).enumerate() {
                if e.mode == EnemyMode::Diving || e.mode == EnemyMode::Returning {
                    left_formation[i] = true;
                    flight_started[i].get_or_insert(g.elapsed);
                }
                if left_formation[i] && e.mode == EnemyMode::Formation {
                    let goal = Point {
                        x: e.home.x + (g.elapsed * 0.85).sin() * 0.65,
                        y: e.home.y,
                    };
                    if (e.pos.x - goal.x).hypot(e.pos.y - goal.y) < 0.7 {
                        rejoined[i] = true;
                        flight_started[i] = None;
                    }
                }
                if let Some(start) = flight_started[i] {
                    assert!(
                        g.elapsed - start < 15.,
                        "ship{} failed to rejoin within15s classic={classic}",
                        e.id
                    );
                }
                if e.mode != EnemyMode::Formation && e.age > 0. && point(e).distance(p) < 0.001 {
                    stalled[i] += 1;
                } else {
                    stalled[i] = 0;
                }
                assert!(
                    stalled[i] < 120,
                    "nonformation ship{} stopped for1s classic={classic}",
                    e.id
                );
            }
        }
        assert!(left_formation.iter().filter(|left| **left).count() >= 8);
        for i in 0..24 {
            // Late launches have not yet completed their ordinary loop.
            let e = &g.enemies[i];
            if left_formation[i] && e.mode == EnemyMode::Formation && flight_started[i].is_none() {
                assert!(rejoined[i]);
            }
        }
    }
}
