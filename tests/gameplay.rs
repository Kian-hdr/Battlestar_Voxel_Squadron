use battlestar_voxel::simulation::*;
fn playing() -> Game {
    let mut g = Game::default();
    g.start();
    g
}
fn settle(g: &mut Game) {
    for _ in 0..500 {
        g.step(Input::default(), DT);
    }
}
#[test]
fn starts_with_three_lives_and_twenty_four_raiders() {
    let g = playing();
    assert_eq!(g.lives, 3);
    assert_eq!(g.enemies.len(), 24);
    assert_eq!(g.wave, 1);
    assert_eq!(g.score, 0);
}
#[test]
fn movement_clamps_both_edges() {
    let mut g = playing();
    g.invulnerable = 999.;
    for _ in 0..600 {
        g.step(
            Input {
                axis: 1.,
                fire: false,
            },
            DT,
        );
    }
    assert_eq!(g.player.x, PLAYER_LIMIT);
    for _ in 0..1200 {
        g.step(
            Input {
                axis: -1.,
                fire: false,
            },
            DT,
        );
    }
    assert_eq!(g.player.x, -PLAYER_LIMIT);
}
#[test]
fn fire_cooldown_prevents_frame_rate_machine_gun() {
    let mut g = playing();
    g.step(
        Input {
            axis: 0.,
            fire: true,
        },
        DT,
    );
    let n = g.next_id;
    g.step(
        Input {
            axis: 0.,
            fire: true,
        },
        DT,
    );
    assert_eq!(g.next_id, n);
    for _ in 0..30 {
        g.step(
            Input {
                axis: 0.,
                fire: true,
            },
            DT,
        );
    }
    assert!(g.next_id > n);
}
#[test]
fn scores_formation_and_diving_kills_once() {
    let mut g = playing();
    settle(&mut g);
    g.shots.clear();
    let p = g.enemies[0].pos;
    g.inject_shot(p, true);
    g.step(Input::default(), DT);
    assert_eq!(g.score, 100);
    g.enemies[0].mode = EnemyMode::Diving;
    g.enemies[0].age = 0.;
    g.enemies[0].origin = g.enemies[0].pos;
    let p = g.enemies[0].pos;
    g.inject_shot(p, true);
    g.step(Input::default(), DT);
    assert_eq!(g.score, 300);
}
#[test]
fn simultaneous_hits_cost_one_life_and_grant_protection() {
    let mut g = playing();
    g.invulnerable = 0.;
    let p = g.player;
    g.inject_shot(p, false);
    g.inject_shot(p, false);
    g.step(Input::default(), DT);
    assert_eq!(g.lives, 2);
    assert!(g.invulnerable > 1.);
    g.inject_shot(g.player, false);
    g.step(Input::default(), DT);
    assert_eq!(g.lives, 2);
}
#[test]
fn pause_freezes_every_gameplay_timer_and_entity() {
    let mut g = playing();
    settle(&mut g);
    g.toggle_pause();
    let before = g.clone();
    for _ in 0..60 {
        g.step(
            Input {
                axis: 1.,
                fire: true,
            },
            DT,
        );
    }
    assert_eq!(g, before);
    g.toggle_pause();
    assert_eq!(g.phase, Phase::Playing);
}
#[test]
fn clearing_wave_advances_after_break_and_frequency_is_capped() {
    let mut g = playing();
    g.enemies.clear();
    for _ in 0..240 {
        g.step(Input::default(), DT);
    }
    assert_eq!(g.wave, 2);
    assert_eq!(g.enemies.len(), 24);
    let a = WaveConfig::for_wave(1);
    let b = WaveConfig::for_wave(999);
    assert!(b.dive_interval < a.dive_interval);
    assert!(b.dive_interval >= 0.65);
    assert!(b.fire_interval >= 0.28);
    assert!(b.max_divers <= 4);
}
#[test]
fn death_and_restart_clear_all_transient_state() {
    let mut g = playing();
    g.lives = 1;
    g.invulnerable = 0.;
    g.inject_shot(g.player, false);
    g.step(Input::default(), DT);
    assert_eq!(g.phase, Phase::GameOver);
    g.score = 1234;
    g.muted = true;
    g.start();
    let mut fresh = playing();
    fresh.muted = true;
    assert_eq!(g, fresh);
}
#[test]
fn entering_diving_and_returning_are_reachable() {
    let mut g = playing();
    assert!(g.enemies.iter().all(|e| e.mode == EnemyMode::Entering));
    let mut formed = false;
    let mut diving = false;
    let mut returned = false;
    let mut diver_id = 0;
    g.invulnerable = 999.;
    for _ in 0..3000 {
        g.step(Input::default(), DT);
        formed |= g.enemies.iter().any(|e| e.mode == EnemyMode::Formation);
        if !diving && let Some(e) = g.enemies.iter().find(|e| e.mode == EnemyMode::Diving) {
            diving = true;
            diver_id = e.id;
        }
        if diving
            && g.enemies
                .iter()
                .any(|e| e.id == diver_id && e.mode == EnemyMode::Returning)
        {
            returned = true;
            break;
        }
    }
    assert!(formed && diving && returned);
}
#[test]
fn projectiles_expire_and_long_runs_remain_bounded() {
    let mut g = playing();
    g.invulnerable = 999.;
    for _ in 0..20000 {
        g.step(
            Input {
                axis: 0.,
                fire: true,
            },
            DT,
        );
        assert!(g.shots.len() < 200);
        assert!(g.enemies.len() <= 24);
    }
    assert!(g.score > 0);
}
#[test]
fn fast_projectile_uses_swept_collision() {
    let mut g = playing();
    settle(&mut g);
    g.shots.clear();
    let p = g.enemies[0].pos;
    g.inject_shot(
        Point {
            x: p.x,
            y: p.y - 0.8,
        },
        true,
    );
    g.shots.last_mut().unwrap().velocity = Point { x: 0., y: 250. };
    g.step(Input::default(), DT);
    assert_eq!(g.score, 100);
}

#[test]
fn eras_progress_coherently_after_three_classic_sectors() {
    let mut g = playing();
    for sector in 1..=4 {
        assert_eq!(g.wave, sector);
        let classic = sector <= 3;
        assert_eq!(Era::for_wave(sector) == Era::Classic, classic);
        assert!(g.enemies.iter().all(|e| e.classic == classic));
        if sector < 4 {
            g.enemies.clear();
            for _ in 0..181 {
                g.step(Input::default(), DT);
            }
        }
    }
    g.start();
    assert_eq!(Era::for_wave(g.wave), Era::Classic);
    assert!(g.enemies.iter().all(|e| e.classic));
}

#[test]
fn viper_accelerates_and_coasts_instead_of_snapping() {
    let mut g = playing();
    g.step(
        Input {
            axis: 1.,
            fire: false,
        },
        DT,
    );
    assert!(g.velocity_x > 0. && g.velocity_x < 13.);
    for _ in 0..30 {
        g.step(
            Input {
                axis: 1.,
                fire: false,
            },
            DT,
        );
    }
    let x = g.player.x;
    let speed = g.velocity_x;
    g.step(Input::default(), DT);
    assert!(g.player.x > x);
    assert!(g.velocity_x > 0. && g.velocity_x < speed);
    for _ in 0..120 {
        g.step(Input::default(), DT);
    }
    assert!(g.velocity_x.abs() < 0.01);
}

#[test]
fn raider_return_path_is_continuous_and_rejoins_formation() {
    let mut g = playing();
    g.invulnerable = 999.;
    let mut tracked = None;
    let mut saw_return = false;
    let mut rejoined = false;
    for _ in 0..3000 {
        let before = tracked.and_then(|id| {
            g.enemies
                .iter()
                .find(|e| e.id == id)
                .map(|e| (e.pos, e.mode))
        });
        g.step(Input::default(), DT);
        if tracked.is_none() {
            tracked = g
                .enemies
                .iter()
                .find(|e| e.mode == EnemyMode::Diving)
                .map(|e| e.id);
        }
        if let Some(e) = tracked.and_then(|id| g.enemies.iter().find(|e| e.id == id)) {
            if let Some((p, mode)) = before
                && (mode == EnemyMode::Diving || mode == EnemyMode::Returning)
            {
                assert!(
                    (e.pos.x - p.x).hypot(e.pos.y - p.y) < 1.,
                    "visible jump between flight segments"
                );
            }
            saw_return |= e.mode == EnemyMode::Returning;
            if saw_return && e.mode == EnemyMode::Formation {
                rejoined = true;
                break;
            }
        }
    }
    assert!(saw_return && rejoined);
}

#[test]
fn reference_silhouettes_keep_ship_proportions() {
    use battlestar_voxel::voxel;
    for classic in [true, false] {
        let g = voxel::galactica(classic);
        let width =
            g.0.keys().map(|p| p[0]).max().unwrap() - g.0.keys().map(|p| p[0]).min().unwrap() + 1;
        let length =
            g.0.keys().map(|p| p[1]).max().unwrap() - g.0.keys().map(|p| p[1]).min().unwrap() + 1;
        let ratio = width as f32 / length as f32;
        assert!(
            (0.32..0.45).contains(&ratio),
            "Galactica must retain a long carrier silhouette, width/length={ratio}"
        );
    }
}

#[test]
fn each_unprotected_hit_consumes_one_fighter_until_reserves_are_exhausted() {
    let mut g = playing();
    for remaining in [2, 1, 0] {
        g.invulnerable = 0.;
        g.player.x = 4.;
        g.velocity_x = 6.;
        g.inject_shot(g.player, false);
        g.step(Input::default(), DT);
        assert_eq!(g.lives, remaining);
        assert_eq!(g.player, Point { x: 0., y: PLAYER_Y });
        assert_eq!(g.velocity_x, 0.);
        assert!(g.invulnerable > 2.);
        assert!(
            g.events
                .iter()
                .any(|event| matches!(event, Event::PlayerDestroyed { .. }))
        );
        assert_eq!(
            g.phase,
            if remaining == 0 {
                Phase::GameOver
            } else {
                Phase::Playing
            }
        );
    }
    let before = g.clone();
    g.step(
        Input {
            axis: 1.,
            fire: true,
        },
        DT,
    );
    assert_eq!(g, before);
    g.start();
    assert_eq!(g.lives, 3);
    assert_eq!(g.phase, Phase::Playing);
}

#[test]
fn all_ship_models_have_exact_lateral_geometry_and_tint_symmetry() {
    use battlestar_voxel::voxel;
    for classic in [true, false] {
        for (name, model) in [
            ("Viper", voxel::viper(classic)),
            ("Raider", voxel::raider(classic)),
            ("Galactica", voxel::galactica(classic)),
        ] {
            assert!(!model.0.is_empty());
            for (p, color) in &model.0 {
                assert_eq!(
                    model.0.get(&[-p[0], p[1], p[2]]),
                    Some(color),
                    "{name} classic={classic} has an unmatched voxel/tint at {p:?}"
                );
            }
        }
    }
}

#[test]
fn cylon_and_carrier_palettes_are_gray_and_modern_eras_are_darker() {
    use battlestar_voxel::voxel;
    for (name, classic, modern) in [
        ("Raider", voxel::raider(true), voxel::raider(false)),
        ("Galactica", voxel::galactica(true), voxel::galactica(false)),
    ] {
        let luminance = |model: &voxel::Voxels| {
            let mut values: Vec<_> = model
                .0
                .values()
                .map(|c| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2])
                .collect();
            values.sort_by(f32::total_cmp);
            let average = values.iter().sum::<f32>() / values.len() as f32;
            (average, values[values.len() / 2])
        };
        let (classic_mean, classic_median) = luminance(&classic);
        let (modern_mean, modern_median) = luminance(&modern);
        assert!(
            classic_mean < 0.30 && classic_median < 0.30,
            "{name} classic body is too pale"
        );
        assert!(
            modern_mean < classic_mean * 0.6 && modern_median < classic_median * 0.6,
            "{name} modern body must remain distinctly darker than classic"
        );
    }
}

#[test]
fn destroyed_enemy_emits_exactly_one_typed_shatter_event() {
    let mut g = playing();
    settle(&mut g);
    g.shots.clear();
    let enemy = g.enemies[0].clone();
    g.inject_shot(enemy.pos, true);
    g.inject_shot(enemy.pos, true);
    g.step(Input::default(), DT);
    let events: Vec<_> = g
        .events
        .iter()
        .filter(|e| matches!(e,Event::EnemyDestroyed{id,..} if *id==enemy.id))
        .collect();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        Event::EnemyDestroyed { classic: true, .. }
    ));
}

#[test]
fn destruction_records_swept_contact_and_projectile_momentum() {
    let mut g = playing();
    settle(&mut g);
    g.shots.clear();
    let enemy = g.enemies[0].clone();
    g.inject_shot(
        Point {
            x: enemy.pos.x + 0.3,
            y: enemy.pos.y - 2.,
        },
        true,
    );
    g.shots.last_mut().unwrap().velocity = Point { x: 0., y: 600. };
    g.step(Input::default(), DT);
    let event = g
        .events
        .iter()
        .find(|e| matches!(e, Event::EnemyDestroyed{id,..} if *id==enemy.id))
        .unwrap();
    if let Event::EnemyDestroyed {
        impact,
        shot_velocity,
        pos,
        ..
    } = event
    {
        assert_eq!(shot_velocity.y, 600.);
        assert!((impact.x - enemy.pos.x - 0.3).abs() < 0.001);
        assert!(
            impact.y < pos.y,
            "entry contact is on incoming side, not ship center"
        );
    }
}

#[test]
fn projectile_in_empty_hull_cell_does_not_score_a_fake_circle_hit() {
    use battlestar_voxel::voxel::{UNIT, raider};
    let mut g = playing();
    // Isolate the exact-cell fixture before flight avoidance has added offsets.
    g.shots.clear();
    g.enemies.truncate(1);
    g.elapsed = 0.;
    let next_x = (DT * 0.85).sin() * 0.65;
    let e = &mut g.enemies[0];
    e.home = Point { x: 0., y: 0. };
    e.pos = Point { x: next_x, y: 0. };
    e.rotation = glam::Quat::IDENTITY.to_array();
    e.mode = EnemyMode::Formation;
    let hull = raider(true);
    let empty = (-5_i32..5)
        .flat_map(|x| (-5_i32..5).map(move |y| [x, y, 1]))
        .find(|p| {
            !hull.0.contains_key(p)
                && ((p[0] as f32 + 0.5) * UNIT).hypot((p[1] as f32 + 0.5) * UNIT) < 0.7
        })
        .expect("classic silhouette has empty space inside old hit circle");
    g.inject_shot(
        Point {
            x: next_x + (empty[0] as f32 + 0.5) * UNIT,
            y: (empty[1] as f32 + 0.5) * UNIT,
        },
        true,
    );
    g.shots.last_mut().unwrap().velocity = Point::default();
    let score = g.score;
    g.step(Input::default(), DT);
    assert_eq!(g.score, score);
    assert_eq!(g.enemies.len(), 1);
}

#[test]
fn player_damage_keeps_pre_respawn_pose_contact_and_momentum() {
    let mut g = playing();
    settle(&mut g);
    g.enemies.clear();
    g.shots.clear();
    g.invulnerable = 0.;
    g.velocity_x = 9.;
    g.player_rotation = glam::Quat::from_rotation_y(-0.3).to_array();
    g.inject_shot(g.player, false);
    g.step(
        Input {
            axis: 1.,
            fire: false,
        },
        DT,
    );
    let event = g
        .events
        .iter()
        .find(|e| matches!(e, Event::PlayerDestroyed { .. }))
        .unwrap();
    if let Event::PlayerDestroyed {
        velocity,
        rotation,
        shot_velocity,
        ..
    } = event
    {
        assert!(velocity.x > 0.);
        assert_ne!(*rotation, glam::Quat::IDENTITY.to_array());
        assert!(shot_velocity.y < 0.);
    }
    assert_eq!(g.velocity_x, 0.);
    assert_eq!(g.lives, 2);
}
