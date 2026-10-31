use battlestar_voxel::{simulation::*, voxel};
use glam::{Quat, Vec2, Vec3};

const K: f32 = 23.5 / 28.;
fn project(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.y + K * v.z)
}
fn fixture(classic: bool, depth: f32, bank: f32) -> Game {
    let mut g = Game::default();
    g.start();
    g.invulnerable = 999.;
    g.enemies.truncate(1);
    g.wave = if classic { 1 } else { 4 };
    g.elapsed = std::f32::consts::FRAC_PI_2 / 0.85;
    let e = &mut g.enemies[0];
    e.classic = classic;
    e.mode = EnemyMode::Formation;
    e.pos = Point { x: 0., y: 3. };
    e.home = Point { x: -0.65, y: 3. };
    e.origin = e.pos;
    e.depth = depth;
    e.rotation = Quat::from_rotation_y(bank).to_array();
    g
}
#[test]
fn occupied_visible_silhouettes_hit_independently_of_both_cosmetic_depths() {
    for classic in [true, false] {
        for depth in [-2.6, 0., 2.6] {
            for bank in [-0.5, 0., 0.5] {
                let g = fixture(classic, depth, bank);
                let mut preview = g.clone();
                preview.step(Input::default(), DT);
                let target = &preview.enemies[0];
                let model = voxel::raider(classic);
                let cell = *model.0.keys().nth(model.0.len() / 3).unwrap();
                let offset = project(
                    Quat::from_array(target.rotation)
                        * Vec3::from_array(cell.map(|v| (v as f32 + 0.5) * voxel::UNIT)),
                );
                let hit = Point {
                    x: target.pos.x + offset.x,
                    y: target.pos.y + offset.y,
                };
                for shot_depth in [-12., 0.15, 12.] {
                    let mut trial = g.clone();
                    trial.inject_shot(hit, true);
                    let shot = trial.shots.last_mut().unwrap();
                    shot.velocity = Point::default();
                    shot.depth = shot_depth;
                    shot.previous_depth = shot_depth;
                    trial.step(Input::default(), DT);
                    assert_eq!(
                        trial.score, 100,
                        "visible cell missed classic={classic}, bank={bank}, enemy_depth={depth}, shot_depth={shot_depth}"
                    );
                }
            }
        }
    }
}
#[test]
fn losing_a_life_preserves_viper_position_and_momentum() {
    let mut g = Game::default();
    g.start();
    g.invulnerable = 0.;
    g.enemies.clear();
    g.player.x = 6.;
    g.velocity_x = 8.;
    g.inject_shot(g.player, false);
    let mut uninterrupted = g.clone();
    uninterrupted.shots.clear();
    uninterrupted.step(
        Input {
            axis: 1.,
            fire: false,
        },
        DT,
    );
    g.step(
        Input {
            axis: 1.,
            fire: false,
        },
        DT,
    );
    assert_eq!(g.lives, 2);
    assert_eq!(
        g.player, uninterrupted.player,
        "life loss must not teleport the Viper"
    );
    assert_eq!(
        g.velocity_x, uninterrupted.velocity_x,
        "life loss must not reset momentum"
    );
    assert!(g.invulnerable > 2.);
}

fn convex_hull(mut points: Vec<glam::DVec2>) -> Vec<glam::DVec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    let mut hull = Vec::<glam::DVec2>::new();
    for point in &points {
        while hull.len() >= 2
            && (hull[hull.len() - 1] - hull[hull.len() - 2]).perp_dot(*point - hull[hull.len() - 1])
                <= 0.
        {
            hull.pop();
        }
        hull.push(*point);
    }
    let lower = hull.len();
    for point in points.iter().rev().skip(1) {
        while hull.len() > lower
            && (hull[hull.len() - 1] - hull[hull.len() - 2]).perp_dot(*point - hull[hull.len() - 1])
                <= 0.
        {
            hull.pop();
        }
        hull.push(*point);
    }
    hull.pop();
    hull
}
fn polygon_reference(
    model: &voxel::Voxels,
    start: Vec2,
    end: Vec2,
    rotation: Quat,
) -> Option<Vec2> {
    let mut closest = f64::INFINITY;
    let a = start.as_dvec2();
    let delta = (end - start).as_dvec2();
    for p in model.0.keys() {
        let points = (0..8)
            .map(|mask| {
                let corner = Vec3::from_array(std::array::from_fn(|i| {
                    (p[i] as f32 + if mask & (1 << i) == 0 { 0. } else { 1. }) * voxel::UNIT
                }));
                project(rotation * corner).as_dvec2()
            })
            .collect();
        let hull = convex_hull(points);
        let mut enter = 0_f64;
        let mut exit = 1_f64;
        for i in 0..hull.len() {
            let edge = hull[(i + 1) % hull.len()] - hull[i];
            let distance = edge.perp_dot(a - hull[i]);
            let speed = edge.perp_dot(delta);
            if speed.abs() < 1e-14 {
                if distance < 0. {
                    exit = -1.;
                    break;
                }
            } else if speed > 0. {
                enter = enter.max(-distance / speed);
            } else {
                exit = exit.min(-distance / speed);
            }
        }
        if enter <= exit {
            closest = closest.min(enter);
        }
    }
    closest.is_finite().then(|| start.lerp(end, closest as f32))
}
#[test]
fn projected_cell_slabs_match_an_independent_convex_polygon_reference() {
    let mut hull = voxel::Voxels::default();
    hull.0.insert([-2, 0, 1], [1.; 4]);
    hull.0.insert([1, -2, 0], [1.; 4]);
    hull.0.insert([0, 2, -2], [1.; 4]);
    let mut seed = 0x68ba_1124_u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 40) as f32 / 16777216.
    };
    for _ in 0..1024 {
        let rotation =
            Quat::from_scaled_axis(Vec3::new(random(), random(), random()) * 4. - Vec3::splat(2.));
        let start = Vec2::new(random(), random()) * 1.2 - Vec2::splat(0.6);
        let end = Vec2::new(random(), random()) * 1.2 - Vec2::splat(0.6);
        let expected = polygon_reference(&hull, start, end, rotation);
        let actual = battlestar_voxel::combat_plane::segment_hit(&hull, start, end, rotation);
        match (expected, actual) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(
                a.distance(b) < 1e-4,
                "first contact differs: {a:?} versus {b:?}"
            ),
            pair => panic!("projected occupied cell existence differs: {pair:?}"),
        }
    }
}

#[test]
fn hostile_tilted_player_hits_are_independent_of_projectile_cosmetic_height() {
    for bank in [-0.5, 0., 0.5] {
        let mut g = Game::default();
        g.start();
        g.enemies.clear();
        g.invulnerable = 0.;
        g.player.x = 4.;
        g.player_rotation = Quat::from_rotation_y(bank).to_array();
        let mut preview = g.clone();
        preview.step(Input::default(), DT);
        let model = voxel::viper(true);
        let cell = *model.0.keys().nth(model.0.len() / 3).unwrap();
        let offset = project(
            Quat::from_array(preview.player_rotation)
                * Vec3::from_array(cell.map(|v| (v as f32 + 0.5) * voxel::UNIT)),
        );
        for depth in [-12., 0., 12.] {
            let mut trial = g.clone();
            trial.inject_shot(
                Point {
                    x: preview.player.x + offset.x,
                    y: preview.player.y + offset.y,
                },
                false,
            );
            let shot = trial.shots.last_mut().unwrap();
            shot.velocity = Point::default();
            shot.depth = depth;
            shot.previous_depth = depth;
            trial.step(Input::default(), DT);
            assert_eq!(
                trial.lives, 2,
                "hostile visible overlap missed bank={bank}, depth={depth}"
            );
        }
    }
}
