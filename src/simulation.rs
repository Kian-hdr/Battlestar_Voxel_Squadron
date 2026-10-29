//! Deterministic fixed-step gameplay. Coordinates are screen-oriented world units.
pub const DT: f32 = 1.0 / 120.0;
pub const PLAYER_LIMIT: f32 = 9.4;
pub const PLAYER_Y: f32 = -9.0;
pub const FIRE_COOLDOWN: f32 = 0.19;
pub const MAX_ENEMY_DEPTH: f32 = 2.8;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Input {
    pub axis: f32,
    pub fire: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Title,
    Playing,
    Paused,
    GameOver,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyMode {
    Entering,
    Formation,
    Diving,
    Returning,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Enemy {
    pub id: u64,
    pub pos: Point,
    pub velocity: Point,
    pub depth: f32,
    pub velocity_z: f32,
    pub rotation: [f32; 4],
    pub home: Point,
    pub mode: EnemyMode,
    pub age: f32,
    pub origin: Point,
    pub target_x: f32,
    pub classic: bool,
    avoidance: Point,
    motion_acceleration: glam::Vec3,
    blocked_time: f32,
    yield_time: f32,
    yield_depth: f32,
    yield_waypoint: Point,
    escape_normal: glam::Vec3,
    contact_blocked: bool,
    yield_routing: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Shot {
    pub id: u64,
    pub pos: Point,
    pub previous: Point,
    pub velocity: Point,
    pub depth: f32,
    pub previous_depth: f32,
    pub velocity_z: f32,
    /// Stable depth-only fire-control target; horizontal aim stays player-owned.
    pub target_id: Option<u64>,
    pub friendly: bool,
    pub ttl: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Shot(bool),
    PlayerDestroyed {
        pos: Point,
        classic: bool,
        velocity: Point,
        impact: Point,
        shot_velocity: Point,
        rotation: [f32; 4],
    },
    EnemyDestroyed {
        id: u64,
        pos: Point,
        classic: bool,
        velocity: Point,
        depth: f32,
        velocity_z: f32,
        impact: Point,
        shot_velocity: Point,
        rotation: [f32; 4],
    },
    Wave,
}
pub use crate::waves::{Era, WaveConfig};
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub phase: Phase,
    pub player: Point,
    pub velocity_x: f32,
    pub player_rotation: [f32; 4],
    pub lives: u8,
    pub score: u32,
    pub wave: u32,
    pub enemies: Vec<Enemy>,
    pub shots: Vec<Shot>,
    pub events: Vec<Event>,
    pub invulnerable: f32,
    pub elapsed: f32,
    pub cooldown: f32,
    pub muted: bool,
    pub next_id: u64,
    dive_timer: f32,
    fire_timer: f32,
    wave_break: f32,
    random: u64,
}
impl Default for Game {
    fn default() -> Self {
        Self {
            phase: Phase::Title,
            player: Point { x: 0., y: PLAYER_Y },
            velocity_x: 0.,
            player_rotation: glam::Quat::IDENTITY.to_array(),
            lives: 3,
            score: 0,
            wave: 0,
            enemies: vec![],
            shots: vec![],
            events: vec![],
            invulnerable: 0.,
            elapsed: 0.,
            cooldown: 0.,
            muted: false,
            next_id: 1,
            dive_timer: 0.,
            fire_timer: 0.,
            wave_break: 0.,
            random: 0xB4771E57A2,
        }
    }
}
impl Game {
    pub fn start(&mut self) {
        let muted = self.muted;
        *self = Self::default();
        self.muted = muted;
        self.phase = Phase::Playing;
        self.invulnerable = 2.;
        self.begin_wave();
    }
    pub fn toggle_pause(&mut self) {
        self.phase = match self.phase {
            Phase::Playing => Phase::Paused,
            Phase::Paused => Phase::Playing,
            p => p,
        };
    }
    fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    fn pick(&mut self, n: usize) -> usize {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        self.random as usize % n
    }
    fn begin_wave(&mut self) {
        self.wave = self.wave.saturating_add(1);
        self.wave_break = 0.;
        self.shots.clear();
        self.dive_timer = 4.5;
        self.fire_timer = 4.;
        for row in 0..3 {
            for col in 0..8 {
                let home = Point {
                    x: (col as f32 - 3.5) * 2.25,
                    y: 3.4 + row as f32 * 2.05,
                };
                let id = self.id();
                let entry = Point {
                    // Parallel approach corridors avoid a single converging
                    // off-screen spawn point while arrival times stay staggered.
                    x: home.x,
                    y: 22. + row as f32 * 4. + col as f32 * 0.7,
                };
                self.enemies.push(Enemy {
                    id,
                    pos: entry,
                    velocity: Point::default(),
                    depth: 0.,
                    velocity_z: 0.,
                    rotation: glam::Quat::IDENTITY.to_array(),
                    home,
                    mode: EnemyMode::Entering,
                    age: -(row * 8 + col) as f32 * 0.055,
                    origin: entry,
                    target_x: 0.,
                    classic: Era::for_wave(self.wave) == Era::Classic,
                    avoidance: Point::default(),
                    motion_acceleration: glam::Vec3::ZERO,
                    blocked_time: 0.,
                    yield_time: 0.,
                    yield_depth: 0.,
                    yield_waypoint: Point::default(),
                    escape_normal: glam::Vec3::ZERO,
                    contact_blocked: false,
                    yield_routing: false,
                });
            }
        }
        self.events.push(Event::Wave);
    }
    pub fn inject_shot(&mut self, pos: Point, friendly: bool) {
        let id = self.id();
        let target_id = if friendly {
            self.enemies
                .iter()
                .filter(|e| {
                    let extent = enemy_half_extents(e);
                    e.pos.y > pos.y && (e.pos.x - pos.x).abs() <= extent[0]
                })
                .min_by(|a, b| {
                    (a.pos.y - pos.y)
                        .total_cmp(&(b.pos.y - pos.y))
                        .then(a.id.cmp(&b.id))
                })
                .map(|e| e.id)
        } else {
            None
        };
        self.shots.push(Shot {
            id,
            pos,
            previous: pos,
            velocity: Point {
                x: 0.,
                y: if friendly { 24. } else { -11. },
            },
            depth: 0.15,
            previous_depth: 0.15,
            velocity_z: 0.,
            target_id,
            friendly,
            ttl: 4.,
        });
    }
    fn hit_player(&mut self, impact: Point, shot_velocity: Point) {
        if self.invulnerable > 0. || self.phase != Phase::Playing {
            return;
        }
        self.lives = self.lives.saturating_sub(1);
        self.events.push(Event::PlayerDestroyed {
            pos: self.player,
            classic: self.wave < 4,
            velocity: Point {
                x: self.velocity_x,
                y: 0.,
            },
            impact,
            shot_velocity,
            rotation: self.player_rotation,
        });
        self.invulnerable = 2.5;
        self.player = Point { x: 0., y: PLAYER_Y };
        self.velocity_x = 0.;
        self.shots.retain(|s| s.friendly);
        if self.lives == 0 {
            self.phase = Phase::GameOver;
        }
    }
    pub fn step(&mut self, input: Input, dt: f32) {
        if self.phase != Phase::Playing || !dt.is_finite() || dt <= 0. {
            return;
        }
        let dt = dt.min(0.05);
        self.events.clear();
        self.elapsed += dt;
        self.invulnerable = (self.invulnerable - dt).max(0.);
        self.cooldown = (self.cooldown - dt).max(0.);
        let target = input.axis.clamp(-1., 1.) * 13.;
        self.velocity_x += (target - self.velocity_x) * (1. - (-dt / 0.12).exp());
        if self.velocity_x.abs() < 0.001 {
            self.velocity_x = 0.;
        }
        self.player.x += self.velocity_x * dt;
        if self.player.x.abs() > PLAYER_LIMIT {
            self.player.x = self.player.x.clamp(-PLAYER_LIMIT, PLAYER_LIMIT);
            self.velocity_x = 0.;
        }
        let drift = self.velocity_x / 13.;
        let player_target = glam::Quat::from_rotation_z(-drift * 0.10)
            * glam::Quat::from_rotation_y(-drift * 0.55)
            * glam::Quat::from_rotation_x(drift.abs() * 0.16);
        self.player_rotation = glam::Quat::from_array(self.player_rotation)
            .slerp(player_target, 1. - (-dt / 0.09).exp())
            .normalize()
            .to_array();
        if input.fire && self.cooldown <= 0. {
            self.inject_shot(
                Point {
                    x: self.player.x,
                    y: self.player.y + 0.75,
                },
                true,
            );
            self.cooldown = FIRE_COOLDOWN;
            self.events.push(Event::Shot(true));
        }
        let cfg = WaveConfig::for_wave(self.wave);
        let previous_positions: Vec<_> = self
            .enemies
            .iter()
            .map(|e| glam::Vec3::new(e.pos.x, e.pos.y, e.depth))
            .collect();
        let previous_velocities: Vec<_> = self
            .enemies
            .iter()
            .map(|e| glam::Vec3::new(e.velocity.x, e.velocity.y, e.velocity_z))
            .collect();
        let previous_rotations: Vec<_> = self.enemies.iter().map(|e| e.rotation).collect();
        for e in &mut self.enemies {
            let previous = Point {
                x: e.pos.x - e.avoidance.x,
                y: e.pos.y - e.avoidance.y,
            };
            e.age += dt;
            let home = Point {
                x: e.home.x + (self.elapsed * 0.85).sin() * 0.65,
                y: e.home.y,
            };
            match e.mode {
                EnemyMode::Entering => {
                    let t = (e.age / 2.3).clamp(0., 1.);
                    let t = t * t * (3. - 2. * t);
                    e.pos = e.origin.lerp(home, t);
                    if t >= 1. {
                        e.mode = EnemyMode::Formation;
                        e.age = 0.;
                    }
                }
                EnemyMode::Formation => e.pos = home,
                EnemyMode::Diving => {
                    let t = (e.age / cfg.dive_duration).min(1.);
                    let bend = (t * std::f32::consts::PI * 2.).sin() * 2.2;
                    e.pos = Point {
                        x: (e.origin.x + (e.target_x - e.origin.x) * t + bend).clamp(-11., 11.),
                        y: e.origin.y + (-13. - e.origin.y) * t,
                    };
                    if t >= 1. {
                        e.mode = EnemyMode::Returning;
                        e.origin = e.pos;
                        e.age = 0.;
                    }
                }
                EnemyMode::Returning => {
                    // Fly a continuous loop outside the combat lane, with no visible wrap.
                    let t = (e.age / 4.0).min(1.);
                    let u = t * t * (3. - 2. * t);
                    let side = if e.origin.x >= 0. { 1. } else { -1. };
                    let c1 = Point {
                        x: side * 36.,
                        y: -20.,
                    };
                    let c2 = Point {
                        x: side * 36.,
                        y: 30.,
                    };
                    let a = e.origin.lerp(c1, u);
                    let b = c1.lerp(c2, u);
                    let c = c2.lerp(home, u);
                    e.pos = a.lerp(b, u).lerp(b.lerp(c, u), u);
                    if t >= 1. {
                        e.mode = EnemyMode::Formation;
                        e.age = 0.;
                    }
                }
            }
            e.velocity = Point {
                x: (e.pos.x - previous.x) / dt,
                y: (e.pos.y - previous.y) / dt,
            };
        }
        avoid_fleet(
            &mut self.enemies,
            &previous_positions,
            &previous_velocities,
            &previous_rotations,
            dt,
        );
        self.dive_timer -= dt;
        if self.dive_timer <= 0. {
            let available: Vec<_> = self
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    e.mode == EnemyMode::Formation
                        && e.yield_time <= 0.
                        && e.depth.abs() < 0.5
                        && (e.pos.x - e.home.x - (self.elapsed * 0.85).sin() * 0.65)
                            .hypot(e.pos.y - e.home.y)
                            < 0.7
                })
                .map(|(i, _)| i)
                .collect();
            if !available.is_empty()
                && self
                    .enemies
                    .iter()
                    .filter(|e| e.mode == EnemyMode::Diving)
                    .count()
                    < cfg.max_divers
            {
                let i = available[self.pick(available.len())];
                let e = &mut self.enemies[i];
                e.mode = EnemyMode::Diving;
                e.age = 0.;
                e.origin = Point {
                    x: e.pos.x - e.avoidance.x,
                    y: e.pos.y - e.avoidance.y,
                };
                e.target_x = self.player.x;
            }
            self.dive_timer = cfg.dive_interval;
        }
        self.fire_timer -= dt;
        if self.fire_timer <= 0. {
            let candidates: Vec<_> = self
                .enemies
                .iter()
                .filter(|e| e.pos.y > -6. && e.mode != EnemyMode::Entering)
                .map(|e| (e.pos, e.depth))
                .collect();
            if !candidates.is_empty() {
                let (p, depth) = candidates[self.pick(candidates.len())];
                self.inject_shot(p, false);
                let s = self.shots.last_mut().unwrap();
                let dx = (self.player.x - p.x).clamp(-5., 5.);
                s.velocity.x = dx * 0.6;
                s.depth = depth + 0.15;
                s.previous_depth = s.depth;
                let flight_time = ((self.player.y - p.y) / s.velocity.y).max(0.1);
                s.velocity_z = (0.15 - s.depth) / flight_time;
                self.events.push(Event::Shot(false));
            }
            self.fire_timer = cfg.fire_interval;
        }
        for s in &mut self.shots {
            if s.friendly
                && let Some(enemy) = s
                    .target_id
                    .and_then(|id| self.enemies.iter().find(|e| e.id == id))
                && enemy.pos.y + enemy_half_extents(enemy)[1] > s.pos.y
            {
                // Continuous depth-only intercept steering preserves the original
                // screen-space aim, trajectory and occupied-voxel hit test.
                let closing = (s.velocity.y - enemy.velocity.y).max(2.);
                let remaining = ((enemy.pos.y - s.pos.y) / closing).clamp(0.04, 3.);
                let predicted = (enemy.depth + enemy.velocity_z * remaining.min(0.2))
                    .clamp(-MAX_ENEMY_DEPTH, MAX_ENEMY_DEPTH)
                    + 0.15;
                let desired = ((predicted - s.depth) / remaining).clamp(-18., 18.);
                s.velocity_z += (desired - s.velocity_z).clamp(-160. * dt, 160. * dt);
            }
            s.previous = s.pos;
            s.previous_depth = s.depth;
            s.pos.x += s.velocity.x * dt;
            s.pos.y += s.velocity.y * dt;
            s.depth += s.velocity_z * dt;
            s.ttl -= dt;
        }
        // Mark consumed bullets before resolving player hits; no double-scoring or double life loss.
        let mut killed = Vec::new();
        let mut consumed = Vec::new();
        let mut player_hit = None;
        for s in &self.shots {
            if s.friendly {
                if let Some((e, contact)) = self
                    .enemies
                    .iter()
                    .filter(|e| !killed.contains(&e.id))
                    .filter_map(|e| occupied_hull_contact(s, e).map(|hit| (e, hit)))
                    .min_by(|a, b| {
                        let distance =
                            |p: Point| (p.x - s.previous.x).powi(2) + (p.y - s.previous.y).powi(2);
                        distance(a.1).total_cmp(&distance(b.1))
                    })
                {
                    self.score = self.score.saturating_add(if e.mode == EnemyMode::Diving {
                        200
                    } else {
                        100
                    });
                    killed.push(e.id);
                    consumed.push(s.id);
                    self.events.push(Event::EnemyDestroyed {
                        id: e.id,
                        pos: e.pos,
                        classic: e.classic,
                        velocity: e.velocity,
                        depth: e.depth,
                        velocity_z: e.velocity_z,
                        impact: contact,
                        shot_velocity: s.velocity,
                        rotation: e.rotation,
                    });
                }
            } else if let Some(contact) = occupied_model_contact(
                s,
                self.player,
                0.,
                self.player_rotation,
                hull_model(self.wave < 4, true),
            ) {
                consumed.push(s.id);
                player_hit.get_or_insert((contact, s.velocity));
            }
        }
        if let Some(enemy) = self.enemies.iter().find(|e| {
            !killed.contains(&e.id)
                && (e.pos.x - self.player.x).powi(2)
                    + (e.pos.y - self.player.y).powi(2)
                    + e.depth.powi(2)
                    <= 0.85_f32.powi(2)
        }) {
            player_hit.get_or_insert((enemy.pos, enemy.velocity));
        }
        self.enemies.retain(|e| !killed.contains(&e.id));
        self.shots.retain(|s| {
            s.ttl > 0.
                && s.pos.y.abs() < 17.
                && s.pos.x.abs() < 15.
                && s.depth.is_finite()
                && s.depth.abs() < 32.
                && !consumed.contains(&s.id)
        });
        if let Some((impact, momentum)) = player_hit {
            self.hit_player(impact, momentum);
        }
        if self.enemies.is_empty() && self.phase == Phase::Playing {
            self.wave_break += dt;
            if self.wave_break >= 1.5 {
                self.begin_wave();
            }
        }
    }
}
/// Conservative actual-model extents around the shared model origin, transformed
/// into world axes using the same rigid rotation as contact tests and rendering.
pub fn enemy_half_extents(enemy: &Enemy) -> [f32; 3] {
    use glam::{Mat3, Quat, Vec3};
    use std::sync::LazyLock;
    static EXTENTS: LazyLock<[Vec3; 2]> = LazyLock::new(|| {
        [false, true].map(|classic| {
            let model = crate::voxel::raider(classic);
            model.0.keys().fold(Vec3::ZERO, |bounds, p| {
                bounds.max(Vec3::from_array(
                    p.map(|v| (v.abs().max((v + 1).abs()) as f32) * crate::voxel::UNIT),
                ))
            })
        })
    });
    let matrix = Mat3::from_quat(Quat::from_array(enemy.rotation));
    let absolute = Mat3::from_cols(
        matrix.x_axis.abs(),
        matrix.y_axis.abs(),
        matrix.z_axis.abs(),
    );
    (absolute * EXTENTS[usize::from(enemy.classic)]).to_array()
}

/// True occupied-material intersection. The broad boxes only prune pairs;
/// oriented source voxel boxes are tested against occupied target grid cells.
pub fn enemy_hulls_overlap(a: &Enemy, b: &Enemy) -> bool {
    material_contact(a, b).is_some()
}

struct OccupiedGrid {
    minimum: glam::IVec3,
    maximum: glam::IVec3,
    width: usize,
    height: usize,
    occupied: Vec<bool>,
    centers: Vec<glam::Vec3>,
}
fn occupied_grid(classic: bool) -> &'static OccupiedGrid {
    use glam::{IVec3, Vec3};
    use std::sync::LazyLock;
    static GRIDS: LazyLock<[OccupiedGrid; 2]> = LazyLock::new(|| {
        [false, true].map(|classic| {
            let model = hull_model(classic, false);
            let minimum = model
                .0
                .keys()
                .fold(IVec3::splat(i32::MAX), |v, p| v.min(IVec3::from_array(*p)));
            let maximum = model
                .0
                .keys()
                .fold(IVec3::splat(i32::MIN), |v, p| v.max(IVec3::from_array(*p)));
            let size = (maximum - minimum + IVec3::ONE).as_uvec3();
            let width = size.x as usize;
            let height = size.y as usize;
            let mut occupied = vec![false; width * height * size.z as usize];
            let mut centers = Vec::with_capacity(model.0.len());
            for p in model.0.keys() {
                let q = (IVec3::from_array(*p) - minimum).as_uvec3();
                occupied[q.x as usize + width * (q.y as usize + height * q.z as usize)] = true;
                centers.push(Vec3::from_array(
                    p.map(|v| (v as f32 + 0.5) * crate::voxel::UNIT),
                ));
            }
            OccupiedGrid {
                minimum,
                maximum,
                width,
                height,
                occupied,
                centers,
            }
        })
    });
    &GRIDS[usize::from(classic)]
}

fn material_contact(a: &Enemy, b: &Enemy) -> Option<(glam::Vec3, f32)> {
    // Canonical pair order keeps tolerance decisions identical when the moving
    // craft is tested from either side of a resting contact.
    if a.id > b.id {
        return material_contact(b, a).map(|(normal, depth)| (-normal, depth));
    }
    use glam::{Mat3, Quat, Vec3};
    let delta_world = Vec3::new(a.pos.x - b.pos.x, a.pos.y - b.pos.y, a.depth - b.depth);
    let broad = Vec3::from_array(enemy_half_extents(a)) + Vec3::from_array(enemy_half_extents(b));
    if (delta_world.abs() - broad).max_element() >= 0. {
        return None;
    }
    let rb = Quat::from_array(b.rotation);
    let inverse = rb.conjugate();
    let relative = inverse * Quat::from_array(a.rotation);
    let matrix = Mat3::from_quat(relative);
    let columns = [matrix.x_axis, matrix.y_axis, matrix.z_axis];
    let mut axes = [Vec3::ZERO; 15];
    let mut radii = [0.; 15];
    axes[..3].copy_from_slice(&[Vec3::X, Vec3::Y, Vec3::Z]);
    axes[3..6].copy_from_slice(&columns);
    for (i, column) in columns.iter().enumerate() {
        for (j, basis) in [Vec3::X, Vec3::Y, Vec3::Z].iter().enumerate() {
            axes[6 + i * 3 + j] = column.cross(*basis).normalize_or_zero();
        }
    }
    let half = crate::voxel::UNIT * 0.5;
    for (axis, radius) in axes.iter().zip(&mut radii) {
        *radius = half
            * (axis.abs().element_sum() + columns.iter().map(|c| axis.dot(*c).abs()).sum::<f32>());
    }
    let source = occupied_grid(a.classic);
    let target = occupied_grid(b.classic);
    let offset = inverse * delta_world;
    let extent = (matrix.x_axis.abs() + matrix.y_axis.abs() + matrix.z_axis.abs()) * half;
    let unit = crate::voxel::UNIT;
    for p in &source.centers {
        let center = offset + relative * *p;
        let low = ((center - extent) / unit)
            .floor()
            .as_ivec3()
            .max(target.minimum);
        let high = ((center + extent) / unit)
            .floor()
            .as_ivec3()
            .min(target.maximum);
        for x in low.x..=high.x {
            for y in low.y..=high.y {
                for z in low.z..=high.z {
                    let q = glam::IVec3::new(x, y, z) - target.minimum;
                    let index =
                        q.x as usize + target.width * (q.y as usize + target.height * q.z as usize);
                    if !target.occupied[index] {
                        continue;
                    }
                    let d =
                        center - Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) * unit;
                    let mut depth = f32::INFINITY;
                    let mut normal = Vec3::ZERO;
                    let mut separate = false;
                    for (axis, radius) in axes.iter().zip(radii) {
                        if radius < 1e-8 {
                            continue;
                        }
                        let projected = d.dot(*axis);
                        let penetration = radius - projected.abs();
                        if penetration <= 1e-5 {
                            separate = true;
                            break;
                        }
                        if penetration < depth {
                            depth = penetration;
                            normal = *axis * if projected >= 0. { 1. } else { -1. };
                        }
                    }
                    if !separate {
                        return Some((rb * normal, depth));
                    }
                }
            }
        }
    }
    None
}

fn avoid_fleet(
    enemies: &mut [Enemy],
    previous: &[glam::Vec3],
    velocities: &[glam::Vec3],
    rotations: &[[f32; 4]],
    dt: f32,
) {
    use glam::{Quat, Vec3};
    let count = enemies.len();
    let mut order: Vec<_> = (0..count).collect();
    order.sort_unstable_by_key(|i| enemies[*i].id);
    let extents: Vec<_> = enemies
        .iter()
        .map(|e| Vec3::from_array(enemy_half_extents(e)))
        .collect();
    let nominal: Vec<_> = enemies
        .iter()
        .map(|e| Vec3::new(e.pos.x, e.pos.y, 0.))
        .collect();
    let mut depth_targets = vec![0_f32; count];
    for (rank, &i) in order.iter().enumerate() {
        for &j in &order[rank + 1..] {
            let dx = previous[i].x - previous[j].x;
            let dy = previous[i].y - previous[j].y;
            let vx = enemies[i].velocity.x - enemies[j].velocity.x;
            let vy = enemies[i].velocity.y - enemies[j].velocity.y;
            let t = (-(dx * vx + dy * vy) / (vx * vx + vy * vy).max(1e-6)).clamp(0., 0.8);
            let clearance = extents[i] + extents[j];
            if (dx + vx * t).abs() < clearance.x
                && (dy + vy * t).abs() < clearance.y
                && dx * vx + dy * vy < -0.03
            {
                // A gradually weighted route preference, never a contact impulse.
                let weight = (1. - t / 1.2).clamp(0., 1.);
                let lane = clearance.z * 0.62 * weight;
                depth_targets[i] += lane;
                depth_targets[j] -= lane;
            }
        }
    }
    for (rank, &i) in order.iter().enumerate() {
        let error = (nominal[i] - previous[i]).truncate().length();
        let trying = enemies[i].mode != EnemyMode::Entering || enemies[i].age > 0.;
        if trying
            && error > 0.25
            && velocities[i].truncate().length() < 0.7
            && enemies[i].contact_blocked
        {
            enemies[i].blocked_time += dt;
        } else {
            enemies[i].blocked_time = (enemies[i].blocked_time - dt * 2.).max(0.);
        }
        enemies[i].yield_time = (enemies[i].yield_time - dt).max(0.);
        if enemies[i].blocked_time > 0.6 {
            let lower = order[..rank]
                .iter()
                .filter(|j| (previous[i] - previous[**j]).truncate().length() < 3.5)
                .count();
            // The lowest nearby ID keeps right of way. Others commit to stable
            // alternating escape lanes rather than repeatedly switching sides.
            enemies[i].yield_depth = [0., 2.5, -2.5, 1.3, -1.3][lower % 5];
            let normal = enemies[i].escape_normal.normalize_or_zero();
            if normal.z.abs() > 0.2 {
                enemies[i].yield_depth = normal.z.signum() * 2.5;
            }
            let mut outward = Vec3::new(normal.x, normal.y, 0.).normalize_or_zero();
            if outward.length_squared() < 0.1 {
                let neighbors: Vec<_> = previous
                    .iter()
                    .enumerate()
                    .filter(|(j, p)| *j != i && (**p - previous[i]).truncate().length() < 3.5)
                    .map(|(_, p)| *p)
                    .collect();
                let center =
                    neighbors.iter().copied().sum::<Vec3>() / neighbors.len().max(1) as f32;
                outward = Vec3::new(previous[i].x - center.x, previous[i].y - center.y, 0.)
                    .normalize_or_zero();
            }
            if outward.length_squared() < 0.1 {
                outward = Vec3::X * if lower % 2 == 0 { -1. } else { 1. };
            }
            let waypoint = previous[i] + outward * 1.2;
            enemies[i].yield_waypoint = Point {
                x: waypoint.x,
                y: waypoint.y,
            };
            enemies[i].yield_routing = true;
            enemies[i].yield_time = 2.5;
            enemies[i].blocked_time = 0.;
        }
        if enemies[i].yield_time > 0. {
            depth_targets[i] = enemies[i].yield_depth;
        }
    }
    for (i, e) in enemies.iter_mut().enumerate() {
        let desired_depth = depth_targets[i].clamp(-MAX_ENEMY_DEPTH + 0.2, MAX_ENEMY_DEPTH - 0.2);
        if !e.contact_blocked
            && (Vec3::new(e.yield_waypoint.x, e.yield_waypoint.y, 0.) - previous[i])
                .truncate()
                .length()
                < 0.25
        {
            e.yield_routing = false;
        }
        let yielding = e.yield_time > 0. && e.yield_routing;
        let desired_velocity = if yielding {
            Vec3::ZERO
        } else {
            Vec3::new(e.velocity.x, e.velocity.y, 0.)
        };
        let goal = if yielding {
            Vec3::new(e.yield_waypoint.x, e.yield_waypoint.y, 0.)
        } else {
            nominal[i]
        };
        let error = goal - previous[i];
        let desired_acceleration = Vec3::new(
            (desired_velocity.x - velocities[i].x) * 9. + error.x * 28.,
            (desired_velocity.y - velocities[i].y) * 9. + error.y * 28.,
            (desired_depth - previous[i].z) * 28. - velocities[i].z * 10.,
        );
        let desired_acceleration = Vec3::new(
            desired_acceleration.x.clamp(-60., 60.),
            desired_acceleration.y.clamp(-60., 60.),
            desired_acceleration.z.clamp(-14., 14.),
        );
        let change = desired_acceleration - e.motion_acceleration;
        e.motion_acceleration += Vec3::new(
            change.x.clamp(-dt * 360., dt * 360.),
            change.y.clamp(-dt * 360., dt * 360.),
            change.z.clamp(-dt * 90., dt * 90.),
        );
        let mut velocity = velocities[i] + e.motion_acceleration * dt;
        velocity.z = velocity.z.clamp(-3., 3.);
        let mut position = previous[i] + velocity * dt;
        position.z = position.z.clamp(-MAX_ENEMY_DEPTH, MAX_ENEMY_DEPTH);
        e.pos = Point {
            x: position.x,
            y: position.y,
        };
        e.depth = position.z;
        e.velocity = Point {
            x: velocity.x,
            y: velocity.y,
        };
        e.velocity_z = velocity.z;
        let dx = velocity.x;
        let dy = velocity.y;
        let yaw = match e.mode {
            EnemyMode::Diving => dx.atan2(-dy).clamp(-0.65, 0.65),
            EnemyMode::Entering | EnemyMode::Returning if dx.hypot(dy) > 0.1 => dx.atan2(-dy),
            _ => 0.,
        };
        let bank = Quat::from_rotation_z(yaw)
            * Quat::from_rotation_y((-dx * 0.10).clamp(-0.5, 0.5))
            * Quat::from_rotation_x((dy * 0.025).clamp(-0.18, 0.18));
        e.rotation = Quat::from_array(e.rotation)
            .slerp(bank, 1. - (-dt / 0.2).exp())
            .normalize()
            .to_array();
    }
    // Resolve each degree of freedom independently in stable right-of-way
    // order. An inward XY contact must never freeze a safe depth escape.
    let planned: Vec<_> = enemies
        .iter()
        .map(|e| Vec3::new(e.pos.x, e.pos.y, e.depth))
        .collect();
    let planned_rotations: Vec<_> = enemies.iter().map(|e| e.rotation).collect();
    for (i, e) in enemies.iter_mut().enumerate() {
        e.pos = Point {
            x: previous[i].x,
            y: previous[i].y,
        };
        e.depth = previous[i].z;
        e.rotation = rotations[i];
    }
    for &i in &order {
        let depth_end = Vec3::new(enemies[i].pos.x, enemies[i].pos.y, planned[i].z);
        let depth_contact = advance_safe(enemies, i, depth_end, rotations[i]);
        let mut was_blocked = depth_contact.is_some();
        let mut contact_normal = depth_contact.unwrap_or(Vec3::ZERO);
        let xy_end = Vec3::new(planned[i].x, planned[i].y, enemies[i].depth);
        if let Some(normal) = advance_safe(enemies, i, xy_end, rotations[i]) {
            was_blocked = true;
            contact_normal += normal;
            let at = Vec3::new(enemies[i].pos.x, enemies[i].pos.y, enemies[i].depth);
            let remaining = xy_end - at;
            let side = Vec3::new(normal.x, normal.y, 0.).normalize_or_zero();
            let slide = remaining - side * remaining.dot(side).min(0.);
            if side.length_squared() > 0.5 && slide.length_squared() > 1e-10 {
                advance_safe(enemies, i, at + slide, rotations[i]);
            }
        }
        let position = Vec3::new(enemies[i].pos.x, enemies[i].pos.y, enemies[i].depth);
        let bank_contact = advance_safe(enemies, i, position, planned_rotations[i]);
        was_blocked |= bank_contact.is_some();
        contact_normal += bank_contact.unwrap_or(Vec3::ZERO);
        if contact_normal.length_squared() > 0.01 {
            enemies[i].escape_normal = contact_normal.normalize();
        }
        enemies[i].contact_blocked = was_blocked;
    }

    for (i, e) in enemies.iter_mut().enumerate() {
        e.avoidance = Point {
            x: e.pos.x - nominal[i].x,
            y: e.pos.y - nominal[i].y,
        };
        e.velocity = Point {
            x: (e.pos.x - previous[i].x) / dt,
            y: (e.pos.y - previous[i].y) / dt,
        };
        e.velocity_z = (e.depth - previous[i].z) / dt;
    }
}

// Continuous short-step occupied-material checks. Stopping one craft does not
// stop its neighbor or tangential/depth motion. Substeps are half a voxel wide.
fn advance_safe(
    enemies: &mut [Enemy],
    index: usize,
    end: glam::Vec3,
    end_rotation: [f32; 4],
) -> Option<glam::Vec3> {
    use glam::{Quat, Vec3};
    let start = Vec3::new(
        enemies[index].pos.x,
        enemies[index].pos.y,
        enemies[index].depth,
    );
    let rotation = Quat::from_array(enemies[index].rotation);
    let start_rotation = enemies[index].rotation;
    let target = Quat::from_array(end_rotation);
    let steps = ((start.distance(end) / (crate::voxel::UNIT * 0.5))
        .max(rotation.angle_between(target).abs() / 0.04)
        .ceil() as usize)
        .clamp(1, 64);
    let pose = |e: &mut Enemy, t: f32| {
        let p = start.lerp(end, t);
        e.pos = Point { x: p.x, y: p.y };
        e.depth = p.z;
        e.rotation = if t == 0. || start_rotation == end_rotation {
            start_rotation
        } else if t == 1. {
            end_rotation
        } else {
            rotation.slerp(target, t).normalize().to_array()
        };
    };
    let contact = |enemies: &[Enemy]| {
        enemies
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != index)
            .filter_map(|(_, other)| {
                material_contact(&enemies[index], other).map(|hit| (other.id, hit))
            })
            .min_by_key(|(id, _)| *id)
            .map(|(_, hit)| hit)
    };
    let mut safe = 0.;
    for part in 1..=steps {
        let t = part as f32 / steps as f32;
        pose(&mut enemies[index], t);
        if let Some((normal, _)) = contact(enemies) {
            let mut blocked = t;
            for _ in 0..12 {
                let candidate = (safe + blocked) * 0.5;
                pose(&mut enemies[index], candidate);
                if contact(enemies).is_some() {
                    blocked = candidate;
                } else {
                    safe = candidate;
                }
            }
            // Keep a sub-millimeter skin from the exact SAT tolerance boundary.
            // Re-normalizing a later bank pose must not turn resting contact
            // into tiny material penetration from floating-point roundoff.
            pose(&mut enemies[index], (safe - 0.001).max(0.));
            if contact(enemies).is_some() {
                pose(&mut enemies[index], safe);
            }
            debug_assert!(
                contact(enemies).is_none(),
                "advance_safe failed for{}",
                enemies[index].id
            );
            return Some(normal);
        }
        safe = t;
    }
    None
}

fn segment_hits(a: Point, b: Point, p: Point, r: f32) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = dx * dx + dy * dy;
    let t = if len > 0. {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / len
    } else {
        0.
    }
    .clamp(0., 1.);
    (a.x + t * dx - p.x).powi(2) + (a.y + t * dy - p.y).powi(2) <= r * r
}

/// Projectile contact comes from occupied model cells, using the same rigid pose as rendering.
fn occupied_hull_contact(shot: &Shot, enemy: &Enemy) -> Option<Point> {
    occupied_model_contact(
        shot,
        enemy.pos,
        enemy.depth,
        enemy.rotation,
        hull_model(enemy.classic, false),
    )
}
fn hull_model(classic: bool, player: bool) -> &'static crate::voxel::Voxels {
    use std::sync::LazyLock;
    static MODELS: LazyLock<[crate::voxel::Voxels; 4]> = LazyLock::new(|| {
        [
            crate::voxel::raider(false),
            crate::voxel::raider(true),
            crate::voxel::viper(false),
            crate::voxel::viper(true),
        ]
    });
    &MODELS[usize::from(classic) + usize::from(player) * 2]
}
fn occupied_model_contact(
    shot: &Shot,
    pos: Point,
    depth: f32,
    rotation: [f32; 4],
    model: &crate::voxel::Voxels,
) -> Option<Point> {
    use glam::{Quat, Vec3};
    // Cheap conservative broad phase; exact occupied-cell slabs decide the hit.
    if !segment_hits(shot.previous, shot.pos, pos, 3.) {
        return None;
    }
    let rotation = Quat::from_array(rotation);
    let inverse = rotation.inverse();
    let origin = Vec3::new(pos.x, pos.y, depth);
    let a = inverse * (Vec3::new(shot.previous.x, shot.previous.y, shot.previous_depth) - origin);
    let b = inverse * (Vec3::new(shot.pos.x, shot.pos.y, shot.depth) - origin);
    let hit = Vec3::from_array(crate::voxel_damage::segment_hit(
        model,
        a.to_array(),
        b.to_array(),
    )?);
    let world = origin + rotation * hit;
    Some(Point {
        x: world.x,
        y: world.y,
    })
}
