//! Connected, conservative fractures of the actual occupied hull grid.
use std::collections::{BTreeMap, BTreeSet};

use crate::voxel::{Tint, UNIT, Voxels};

/// Cell coordinates and colors stay in the original model grid. The pivot is the
/// centroid of their physical centers, in model units, for rigid-body rotation.
pub struct VoxelChunk {
    pub cells: Voxels,
    pub pivot: [f32; 3],
    /// Initial heat of actual occupied cells. Missing entries are cold.
    pub heat: BTreeMap<[i32; 3], f32>,
}

struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

fn center(p: [i32; 3]) -> [f32; 3] {
    p.map(|v| (v as f32 + 0.5) * UNIT)
}

fn distance(p: [i32; 3], point: [f32; 3]) -> f32 {
    let c = center(p);
    (0..3)
        .map(|i| (c[i] - point[i]).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn neighbors(p: [i32; 3]) -> impl Iterator<Item = [i32; 3]> {
    (0..3).flat_map(move |axis| {
        [-1, 1].into_iter().filter_map(move |delta| {
            let mut q = p;
            q[axis] = q[axis].checked_add(delta)?;
            Some(q)
        })
    })
}

fn chunk(cells: Voxels, heat: BTreeMap<[i32; 3], f32>) -> VoxelChunk {
    let mut pivot = [0.; 3];
    for p in cells.0.keys() {
        let c = center(*p);
        for i in 0..3 {
            pivot[i] += c[i] / cells.0.len() as f32;
        }
    }
    VoxelChunk { cells, pivot, heat }
}

/// Grow regions only through shared faces. A compact, seeded spatial priority
/// gives each region an irregular fracture boundary without scattering its cells.
fn partition(
    voxels: &Voxels,
    seeds: Vec<[i32; 3]>,
    heat: &BTreeMap<[i32; 3], f32>,
    random: &mut Random,
    target: impl Fn([i32; 3], &mut Random) -> usize,
) -> Vec<VoxelChunk> {
    let mut remaining = voxels.0.clone();
    let mut chunks = Vec::new();
    for seed in seeds {
        if !remaining.contains_key(&seed) {
            continue;
        }
        let limit = target(seed, random).max(1);
        let stretch = std::array::from_fn::<_, 3, _>(|_| 8 + random.next() % 9);
        let mut frontier = BTreeSet::from([(0_u64, seed)]);
        let mut queued = BTreeSet::from([seed]);
        let mut cells = Voxels::default();
        let mut cell_heat = BTreeMap::new();
        while cells.0.len() < limit {
            let Some((_, p)) = frontier.pop_first() else {
                break;
            };
            let Some(tint) = remaining.remove(&p) else {
                continue;
            };
            cells.0.insert(p, tint);
            if let Some(h) = heat.get(&p) {
                cell_heat.insert(p, *h);
            }
            for q in neighbors(p) {
                if remaining.contains_key(&q) && queued.insert(q) {
                    let score = (0..3)
                        .map(|i| {
                            let delta = i64::from(q[i]) - i64::from(seed[i]);
                            delta
                                .unsigned_abs()
                                .saturating_pow(2)
                                .saturating_mul(stretch[i])
                        })
                        .fold(0_u64, u64::saturating_add)
                        .saturating_add(random.next() % 49);
                    frontier.insert((score, q));
                }
            }
        }
        chunks.push(chunk(cells, cell_heat));
    }
    debug_assert!(remaining.is_empty());
    chunks
}

/// Tear the impact side into small connected pieces, retaining larger connected
/// plates farther away. Every occupied cell appears exactly once, unchanged.
/// Heat is confined to three cell widths from the nearest actual impact cell;
/// the impact core is hottest and surrounding actual hull cells are cooler.
pub fn fracture(voxels: &Voxels, impact: [f32; 3], seed: u64) -> Vec<VoxelChunk> {
    let impact = impact.map(|v| if v.is_finite() { v } else { 0. });
    let mut seeds: Vec<_> = voxels.0.keys().copied().collect();
    seeds.sort_by(|a, b| {
        distance(*a, impact)
            .total_cmp(&distance(*b, impact))
            .then(a.cmp(b))
    });
    let nearest = seeds.first().map(|p| center(*p)).unwrap_or(impact);
    let heat = seeds
        .iter()
        .filter_map(|p| {
            let h = (1. - distance(*p, nearest) / (3. * UNIT)).max(0.).powi(2);
            (h > 0.).then_some((*p, h))
        })
        .collect();
    partition(voxels, seeds, &heat, &mut Random(seed), |p, random| {
        let d = distance(p, nearest) / UNIT;
        if d <= 3. {
            2 + (random.next() % 7) as usize
        } else {
            48 + (random.next() % 63) as usize
        }
    })
}

/// Secondary fractures conserve original cells, colors and their initial heat.
/// Rendering can cool those initial values with the total flight age.
pub fn split_chunk(chunk: &VoxelChunk, seed: u64) -> Vec<VoxelChunk> {
    let mut random = Random(seed);
    let mut seeds: Vec<_> = chunk.cells.0.keys().copied().collect();
    // Randomize region origins, then grow connected small bricks from them.
    for i in (1..seeds.len()).rev() {
        seeds.swap(i, (random.next() % (i + 1) as u64) as usize);
    }
    partition(
        &chunk.cells,
        seeds,
        &chunk.heat,
        &mut random,
        |_, random| 16 + (random.next() % 17) as usize,
    )
}

/// Heat follows the same cell throughout primary and secondary flight.
pub fn cooled_heat(initial: f32, age: f32) -> f32 {
    let initial = if initial.is_finite() {
        initial.clamp(0., 1.)
    } else {
        0.
    };
    let age = if age.is_nan() { 0. } else { age.max(0.) };
    initial * (1. - age / 0.75).max(0.).powf(1.5)
}

/// Heated hull colors progress through red, orange, yellow and a white core.
/// Zero heat exactly restores the original model color, including its alpha.
pub fn thermal_tint(original: Tint, heat: f32) -> Tint {
    let heat = if heat.is_finite() {
        heat.clamp(0., 1.)
    } else {
        0.
    };
    let stops = [
        original,
        [1., 0.025, 0.005, original[3]],
        [1., 0.24, 0.008, original[3]],
        [1., 0.78, 0.06, original[3]],
        [1., 0.99, 0.84, original[3]],
    ];
    let scaled = heat * 4.;
    let index = (scaled as usize).min(3);
    let blend = scaled - index as f32;
    std::array::from_fn(|i| stops[index][i] + (stops[index + 1][i] - stops[index][i]) * blend)
}

/// Exact finite-segment traversal of occupied model-local cells. Only crossed
/// cells are queried, including boxes touched at edges/corners. Invalid or
/// impractically large segments (over one million grid crossings) are rejected.
pub fn segment_hit(voxels: &Voxels, origin: [f32; 3], end: [f32; 3]) -> Option<[f32; 3]> {
    if voxels.0.is_empty() || origin.iter().chain(end.iter()).any(|v| !v.is_finite()) {
        return None;
    }
    let unit = f64::from(UNIT);
    let start = origin.map(|v| f64::from(v) / unit);
    let finish = end.map(|v| f64::from(v) / unit);
    if start
        .iter()
        .chain(finish.iter())
        .any(|v| v.abs() > f64::from(i32::MAX - 2))
    {
        return None;
    }
    let delta: [f64; 3] = std::array::from_fn(|i| finish[i] - start[i]);
    let crossings = delta.iter().map(|v| v.abs().ceil() as u64).sum::<u64>();
    if crossings > 1_000_000 {
        return None;
    }
    let mut cell = start.map(|v| v.floor() as i32);
    // Closed voxel boxes can touch both neighboring cells at a grid plane.
    let boundary_mask = (0..3).fold(0_u8, |mask, i| {
        mask | if start[i].fract() == 0. { 1 << i } else { 0 }
    });
    let touching = |p: [i32; 3], mask: u8| {
        (0..8_u8)
            .filter(|subset| subset & !mask == 0)
            .any(|subset| {
                let q = std::array::from_fn(|i| p[i] - i32::from(subset & (1 << i) != 0));
                voxels.0.contains_key(&q)
            })
    };
    if touching(cell, boundary_mask) {
        return Some(origin);
    }
    let step: [i32; 3] = delta.map(|v| {
        if v > 0. {
            1
        } else if v < 0. {
            -1
        } else {
            0
        }
    });
    let stationary_mask = (0..3).fold(0_u8, |mask, i| {
        mask | if step[i] == 0 {
            boundary_mask & (1 << i)
        } else {
            0
        }
    });
    let mut next: [f64; 3] = std::array::from_fn(|i| {
        if step[i] == 0 {
            f64::INFINITY
        } else {
            let plane = f64::from(cell[i] + i32::from(step[i] > 0));
            (plane - start[i]) / delta[i]
        }
    });
    let stride = delta.map(|v| if v == 0. { f64::INFINITY } else { 1. / v.abs() });
    for _ in 0..=crossings + 3 {
        let t = next.into_iter().fold(f64::INFINITY, f64::min);
        if !t.is_finite() || t > 1. {
            return None;
        }
        let mask = (0..3).fold(0_u8, |mask, i| {
            mask | if (next[i] - t).abs() <= 1e-12 {
                1 << i
            } else {
                0
            }
        });
        // Check every neighbor touched by tied crossings, including a box whose
        // face is touched only at the diagonal ray's edge or corner.
        for subset in (1..8_u8).filter(|subset| subset & !mask == 0) {
            let q =
                std::array::from_fn(|i| cell[i] + if subset & (1 << i) != 0 { step[i] } else { 0 });
            if touching(q, stationary_mask) {
                return Some(std::array::from_fn(|i| {
                    (f64::from(origin[i]) + (f64::from(end[i]) - f64::from(origin[i])) * t) as f32
                }));
            }
        }
        for i in 0..3 {
            if mask & (1 << i) != 0 {
                cell[i] += step[i];
                next[i] += stride[i];
            }
        }
    }
    None
}

/// Intersect a model-local ray with actual occupied voxel boxes and return its
/// nearest forward surface contact. Direction need not have unit length.
pub fn ray_hit(voxels: &Voxels, origin: [f32; 3], direction: [f32; 3]) -> Option<[f32; 3]> {
    if origin
        .iter()
        .chain(direction.iter())
        .any(|v| !v.is_finite())
        || direction.iter().all(|v| v.abs() < f32::EPSILON)
    {
        return None;
    }
    let mut closest = f32::INFINITY;
    for p in voxels.0.keys() {
        let mut enter = f32::NEG_INFINITY;
        let mut exit = f32::INFINITY;
        for i in 0..3 {
            let low = p[i] as f32 * UNIT;
            let high = low + UNIT;
            if direction[i].abs() < f32::EPSILON {
                if origin[i] < low || origin[i] > high {
                    exit = f32::NEG_INFINITY;
                    break;
                }
            } else {
                let a = (low - origin[i]) / direction[i];
                let b = (high - origin[i]) / direction[i];
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
            }
        }
        if exit >= enter && exit >= 0. {
            let t = if enter >= 0. { enter } else { exit };
            closest = closest.min(t);
        }
    }
    closest
        .is_finite()
        .then(|| std::array::from_fn(|i| origin[i] + direction[i] * closest))
}
