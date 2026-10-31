//! Discrete glowing exhaust voxels, flowing away from a fixed nozzle face.
use crate::voxel::UNIT;

pub const CELL: f32 = UNIT * 0.5;
pub const CUBE_EDGE: f32 = CELL * 0.88;
pub const BASE_CELLS: usize = 4;
pub const LANES: usize = 5;
pub const SLOTS: usize = 5;
pub const CUBES: usize = BASE_CELLS + LANES * SLOTS;
pub const VERTICES: usize = CUBES * 24;
pub const NOZZLE_Y: f32 = -8. * UNIT - 0.004;

pub struct Plume {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
}

const FACES: [[[f32; 3]; 4]; 6] = [
    [[1., 0., 0.], [1., 1., 0.], [1., 1., 1.], [1., 0., 1.]],
    [[0., 1., 0.], [0., 0., 0.], [0., 0., 1.], [0., 1., 1.]],
    [[1., 1., 0.], [0., 1., 0.], [0., 1., 1.], [1., 1., 1.]],
    [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
    [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    [[0., 1., 0.], [1., 1., 0.], [1., 0., 0.], [0., 0., 0.]],
];

fn cube(plume: &mut Plume, centre: [f32; 3], color: [f32; 4]) {
    for (face, shade) in FACES.iter().zip([0.70, 0.55, 0.65, 0.85, 1., 0.45]) {
        for corner in face {
            plume.positions.push(std::array::from_fn(|i| {
                centre[i] + (corner[i] - 0.5) * CUBE_EDGE
            }));
            plume.colors.push([
                color[0] * shade,
                color[1] * shade,
                color[2] * shade,
                color[3],
            ]);
        }
    }
}

fn ease(v: f32) -> f32 {
    let v = v.clamp(0., 1.);
    v * v * (3. - 2. * v)
}
fn noise(seed: u64) -> f32 {
    let mut x = seed.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((x ^ (x >> 31)) >> 40) as f32 / 16777215.
}

pub fn sample(time: f32, nozzle: u8, classic: bool, thrust: f32) -> Plume {
    let phase = f32::from(nozzle) * 2.13;
    let length = 0.44 + 0.045 * (time * 7. + phase).sin() + 0.055 * thrust.clamp(0., 1.);
    let hot = if classic {
        [4., 3.1, 1.2]
    } else {
        [1.8, 3.8, 5.]
    };
    let tail = if classic {
        [2.0, 0.55, 0.06]
    } else {
        [0.04, 1.0, 1.6]
    };
    let mut plume = Plume {
        positions: Vec::with_capacity(VERTICES),
        colors: Vec::with_capacity(VERTICES),
    };
    // A steady 2x2 voxel core keeps the nozzle lit, never moving into the hull.
    for x in [-0.5, 0.5] {
        for z in [-0.5, 0.5] {
            cube(
                &mut plume,
                [x * CELL, -CUBE_EDGE * 0.5, z * CELL],
                [hot[0], hot[1], hot[2], 0.85],
            );
        }
    }
    let offsets = [[0., 0.], [CELL, 0.], [-CELL, 0.], [0., CELL], [0., -CELL]];
    let ends = [1., 0.62, 0.70, 0.56, 0.66];
    for lane in 0..LANES {
        for slot in 0..SLOTS {
            let travel = time * 1.8
                + slot as f32 / SLOTS as f32
                + nozzle as f32 * 0.17
                + lane as f32 * 0.037;
            let age = travel.rem_euclid(1.);
            let seed = (travel.floor().max(0.) as u64)
                .wrapping_mul(131)
                .wrapping_add(nozzle as u64 * 103 + lane as u64 * 19 + slot as u64 * 7);
            let alpha = (if lane == 0 { 0.66 } else { 0.44 })
                * ease(age / 0.14)
                * ease((ends[lane] - age) / (ends[lane] * 0.22));
            let tint = ease(age / ends[lane]);
            let energy = 0.84 + noise(seed) * 0.16;
            let rgb: [f32; 3] =
                std::array::from_fn(|i| (hot[i] + (tail[i] - hot[i]) * tint) * energy);
            cube(
                &mut plume,
                [
                    offsets[lane][0] + (noise(seed + 1) - 0.5) * 0.016 * age,
                    -CELL * 1.1 - length * age - CUBE_EDGE * 0.5,
                    offsets[lane][1] + (noise(seed + 2) - 0.5) * 0.012 * age,
                ],
                [rgb[0], rgb[1], rgb[2], alpha],
            );
        }
    }
    plume
}

pub fn indices() -> Vec<u32> {
    let mut result = Vec::with_capacity(CUBES * 36);
    for face in 0..CUBES * 6 {
        let a = (face * 4) as u32;
        result.extend([a, a + 1, a + 2, a, a + 2, a + 3]);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exhaust_is_made_of_separate_fixed_size_voxel_cubes() {
        let p = sample(0.23, 0, true, 0.);
        assert_eq!(
            p.positions.len() % 24,
            0,
            "every exhaust voxel needs six independent quad faces"
        );
        assert!(p.positions.len() >= 24 * 12);
        for cube in p.positions.as_chunks::<24>().0.iter() {
            let low: [f32; 3] = std::array::from_fn(|axis| {
                cube.iter().map(|v| v[axis]).fold(f32::INFINITY, f32::min)
            });
            let high: [f32; 3] = std::array::from_fn(|axis| {
                cube.iter()
                    .map(|v| v[axis])
                    .fold(f32::NEG_INFINITY, f32::max)
            });
            assert!(
                (0..3).all(|axis| ((high[axis] - low[axis]) - CUBE_EDGE).abs() < 1e-6),
                "exhaust voxels must keep one fixed cube size"
            );
        }
        for triangle in indices().as_chunks::<3>().0.iter() {
            assert!(
                triangle.iter().all(|i| i / 24 == triangle[0] / 24),
                "no connecting skin between voxels"
            );
        }
    }

    #[test]
    fn voxel_exhaust_stays_behind_hulls_with_a_steady_nozzle_core() {
        for classic in [true, false] {
            let rear = crate::voxel::viper(classic)
                .0
                .keys()
                .map(|p| p[1])
                .min()
                .unwrap() as f32
                * UNIT;
            for step in 0..240 {
                for nozzle in 0..3 {
                    let p = sample(step as f32 / 120., nozzle, classic, 1.);
                    assert_eq!(p.positions.len(), VERTICES);
                    assert!(
                        p.positions
                            .iter()
                            .all(|v| v.iter().all(|x| x.is_finite()) && v[1] + NOZZLE_Y < rear)
                    );
                    assert!(p.colors[..BASE_CELLS * 24].iter().all(|c| c[3] > 0.5));
                    assert!(p.colors.iter().any(|c| c[3] == 0.));
                }
            }
        }
    }
    #[test]
    fn visible_cubes_flow_independently_and_recycle_only_while_dim() {
        let a = sample(0., 0, true, 0.);
        assert_ne!(a.positions, sample(0.13, 0, true, 0.).positions);
        assert_ne!(a.positions, sample(0., 1, true, 0.).positions);
        assert_eq!(
            a.positions[..BASE_CELLS * 24],
            sample(0.13, 0, true, 0.).positions[..BASE_CELLS * 24]
        );
        for step in 0..240 {
            let t = step as f32 / 120.;
            let a = sample(t, 0, true, 0.);
            let b = sample(t + 1. / 120., 0, true, 0.);
            for (index, (pa, pb)) in a
                .positions
                .as_chunks::<24>()
                .0
                .iter()
                .zip(b.positions.as_chunks::<24>().0.iter())
                .enumerate()
            {
                if a.colors[index * 24][3] > 0.02 && b.colors[index * 24][3] > 0.02 {
                    assert!((0..3).all(|axis| (pa[0][axis] - pb[0][axis]).abs() < 0.04));
                }
                assert!((a.colors[index * 24][3] - b.colors[index * 24][3]).abs() < 0.15);
            }
        }
        assert!(indices().iter().all(|i| (*i as usize) < VERTICES));
    }
}
