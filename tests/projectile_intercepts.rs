use battlestar_voxel::simulation::*;
use glam::Quat;

fn fixture(classic: bool, depth: f32, x: f32) -> Game {
    let mut g = Game::default();
    g.start();
    g.invulnerable = 999.;
    g.enemies.truncate(1);
    g.wave = if classic { 1 } else { 4 };
    g.elapsed = std::f32::consts::FRAC_PI_2 / 0.85;
    let e = &mut g.enemies[0];
    e.classic = classic;
    e.mode = EnemyMode::Formation;
    e.pos = Point { x, y: 3. };
    e.home = Point { x: -0.65, y: 3. };
    e.origin = e.pos;
    e.depth = depth;
    e.velocity_z = depth.signum() * 3.;
    e.rotation = Quat::IDENTITY.to_array();
    g
}

#[test]
fn a_target_entering_the_firing_lane_after_launch_gets_smooth_depth_assistance() {
    for classic in [true, false] {
        for depth in [-2.4, 2.4] {
            let mut g = fixture(classic, depth, 2.);
            g.inject_shot(Point { x: 0., y: -8.25 }, true);
            let id = g.shots[0].id;
            assert_eq!(g.shots[0].target_id, None);
            for _ in 0..100 {
                g.step(Input::default(), DT);
                for shot in g.shots.iter().filter(|s| s.id == id) {
                    assert_eq!(shot.pos.x, 0.);
                    assert_eq!(shot.velocity, Point { x: 0., y: 24. });
                    assert!(shot.velocity_z.abs() <= 18.);
                    assert!((shot.depth - shot.previous_depth).abs() <= 18. * DT + 1e-5);
                }
                if g.score > 0 {
                    break;
                }
            }
            assert_eq!(
                g.score, 100,
                "entered firing lane but shot missed: classic={classic}, depth={depth}"
            );
        }
    }
}

#[test]
fn aligned_wing_lanes_still_hit_exact_material_in_both_eras() {
    for classic in [true, false] {
        for x in [-0.9, -0.65, 0.65, 0.9] {
            let mut g = fixture(classic, 1.8, 0.);
            g.enemies[0].velocity_z = 0.;
            g.inject_shot(Point { x, y: -8.25 }, true);
            for _ in 0..100 {
                g.step(Input::default(), DT);
                if g.score > 0 {
                    break;
                }
            }
            assert_eq!(
                g.score, 100,
                "occupied wing missed: classic={classic}, x={x}"
            );
        }
    }
}

#[test]
fn a_departed_target_releases_its_depth_lock_and_a_surviving_aligned_target_is_acquired() {
    for classic in [true, false] {
        let mut g = fixture(classic, 2.4, 0.);
        g.enemies[0].velocity_z = 0.;
        let previous_id = g.enemies[0].id;
        g.inject_shot(Point { x: 0., y: -8.25 }, true);
        assert_eq!(g.shots[0].target_id, Some(previous_id));
        // Another shot destroyed the original target before this one arrived.
        let mut replacement = g.enemies.remove(0);
        replacement.id += 100;
        replacement.depth = -2.4;
        let replacement_id = replacement.id;
        g.enemies.push(replacement);
        g.step(Input::default(), DT);
        assert_eq!(g.shots[0].target_id, Some(replacement_id));
        for _ in 0..100 {
            g.step(Input::default(), DT);
            if g.score > 0 {
                break;
            }
        }
        assert_eq!(g.score, 100, "replacement on fixed-X lane was missed");
    }
}

#[test]
fn moving_outside_the_fixed_horizontal_lane_cannot_keep_a_depth_lock() {
    let mut g = fixture(false, 2.4, 0.);
    g.inject_shot(Point { x: 0., y: -8.25 }, true);
    let id = g.shots[0].id;
    assert!(g.shots[0].target_id.is_some());
    g.enemies[0].pos.x = 8.;
    g.enemies[0].home.x = 7.35;
    g.step(Input::default(), DT);
    let shot = g.shots.iter().find(|s| s.id == id).unwrap();
    assert_eq!(shot.target_id, None);
    assert_eq!(shot.velocity, Point { x: 0., y: 24. });
    assert_eq!(g.score, 0);
}
