use std::collections::{BTreeMap, BTreeSet};

use battlestar_voxel::{
    voxel::{UNIT, Voxels},
    voxel_damage::{
        VoxelChunk, cooled_heat, fracture, ray_hit, segment_hit, split_chunk, thermal_tint,
    },
};

fn hull() -> Voxels {
    let mut v = Voxels::default();
    for x in -18..=18 {
        for y in -2..=2 {
            for z in -4..=4 {
                v.0.insert(
                    [x, y, z],
                    [(x + 18) as f32 / 36., (z + 4) as f32 / 8., 0.3, 1.],
                );
            }
        }
    }
    // An isolated actual model component must also be conserved and connected.
    v.0.insert([25, 0, 0], [0.1, 0.2, 0.3, 1.]);
    v
}

fn assert_partition(original: &Voxels, chunks: &[VoxelChunk]) {
    let mut combined = BTreeMap::new();
    for chunk in chunks {
        assert!(!chunk.cells.0.is_empty());
        let first = *chunk.cells.0.first_key_value().unwrap().0;
        let mut seen = BTreeSet::from([first]);
        let mut pending = vec![first];
        while let Some(p) = pending.pop() {
            for axis in 0..3 {
                for delta in [-1, 1] {
                    let mut q = p;
                    q[axis] += delta;
                    if chunk.cells.0.contains_key(&q) && seen.insert(q) {
                        pending.push(q);
                    }
                }
            }
        }
        assert_eq!(
            seen.len(),
            chunk.cells.0.len(),
            "every piece is face connected"
        );
        for (p, tint) in &chunk.cells.0 {
            assert!(combined.insert(*p, *tint).is_none(), "no duplicate cell");
        }
        for axis in 0..3 {
            let expected = chunk
                .cells
                .0
                .keys()
                .map(|p| (p[axis] as f32 + 0.5) * UNIT)
                .sum::<f32>()
                / chunk.cells.0.len() as f32;
            assert!((expected - chunk.pivot[axis]).abs() < 0.0001);
        }
    }
    assert_eq!(original.0, combined, "exact original cells and colors");
}

#[test]
fn primary_fractures_conserve_colored_hull_and_are_deterministic() {
    let hull = hull();
    let chunks = fracture(&hull, [-18. * UNIT, 0., 0.], 42);
    assert_partition(&hull, &chunks);
    let again = fracture(&hull, [-18. * UNIT, 0., 0.], 42);
    assert_eq!(chunks.len(), again.len());
    for (a, b) in chunks.iter().zip(again) {
        assert_eq!(a.cells.0, b.cells.0);
        assert_eq!(a.heat, b.heat);
        assert_eq!(a.pivot, b.pivot);
    }
}

#[test]
fn side_contact_produces_small_hot_pieces_and_large_cold_remote_plates() {
    let hull = hull();
    let chunks = fracture(&hull, [-18. * UNIT, 0., 0.], 91);
    let near: Vec<_> = chunks
        .iter()
        .filter(|c| c.heat.values().any(|h| *h == 1.))
        .collect();
    let far: Vec<_> = chunks
        .iter()
        .filter(|c| c.pivot[0] > 0. && c.cells.0.len() > 1)
        .collect();
    let near_average =
        near.iter().map(|c| c.cells.0.len()).sum::<usize>() as f32 / near.len() as f32;
    let far_average = far.iter().map(|c| c.cells.0.len()).sum::<usize>() as f32 / far.len() as f32;
    assert!(near_average < 9.);
    assert!(chunks.iter().any(|c| (48..=110).contains(&c.cells.0.len())));
    assert!(far_average > 25., "remote plates average {far_average}");
    assert!(far_average > near_average * 3.);
    let heat: BTreeMap<_, _> = chunks
        .iter()
        .flat_map(|c| c.heat.iter().map(|(p, h)| (*p, *h)))
        .collect();
    assert!(heat.values().any(|h| *h == 1.));
    assert!(heat.values().any(|h| *h > 0. && *h < 0.5));
    assert!(heat.len() < hull.0.len() / 10);
    for (p, h) in heat {
        assert!(hull.0.contains_key(&p));
        assert!((0.0..=1.).contains(&h));
        assert!(p[0] < -14, "heat remains within compact impact side core");
    }
}

#[test]
fn secondary_bricks_preserve_cells_tints_heat_and_connectivity() {
    let hull = hull();
    for original in fracture(&hull, [-18. * UNIT, 0., 0.], 7) {
        let bricks = split_chunk(&original, 14);
        assert_partition(&original.cells, &bricks);
        assert!(bricks.iter().all(|b| b.cells.0.len() <= 32));
        if original.cells.0.len() > 32 {
            assert!(bricks.len() > 1);
            assert!(bricks.iter().any(|b| b.cells.0.len() >= 16));
        }
        let heat: BTreeMap<_, _> = bricks
            .iter()
            .flat_map(|b| b.heat.iter().map(|(p, h)| (*p, *h)))
            .collect();
        assert_eq!(heat, original.heat);
    }
}

#[test]
fn empty_and_single_cells_are_valid() {
    assert!(fracture(&Voxels::default(), [0.; 3], 0).is_empty());
    let mut one = Voxels::default();
    one.0.insert([0; 3], [0.1, 0.4, 0.7, 1.]);
    let chunks = fracture(&one, [f32::NAN; 3], 0);
    assert_partition(&one, &chunks);
    assert_eq!(chunks[0].heat.get(&[0; 3]), Some(&1.));
}

#[test]
fn cooling_is_monotonic_and_restores_original_color() {
    let original = [0.2, 0.3, 0.4, 0.75];
    let mut previous = 1.;
    for tick in 0..=150 {
        let heat = cooled_heat(1., tick as f32 * 0.01);
        assert!(heat <= previous);
        assert!((0.0..=1.).contains(&heat));
        assert_eq!(thermal_tint(original, heat)[3], original[3]);
        previous = heat;
    }
    assert_eq!(previous, 0.);
    assert_eq!(thermal_tint(original, 0.), original);
    assert!(thermal_tint(original, 1.)[2] > thermal_tint(original, 0.5)[2]);
    assert_eq!(cooled_heat(1., f32::INFINITY), 0.);
    assert_eq!(cooled_heat(1., 0.75), 0.);
}

fn assert_contact(actual: Option<[f32; 3]>, expected: [f32; 3]) {
    let actual = actual.expect("occupied surface should be hit");
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 1e-5,
            "contact {actual:?} differs from {expected:?}"
        );
    }
}

#[test]
fn finite_segments_handle_stationary_starts_and_negative_axes() {
    let mut hull = Voxels::default();
    hull.0.insert([0; 3], [1.; 4]);
    let inside = [UNIT * 0.5; 3];
    assert_eq!(segment_hit(&hull, inside, inside), Some(inside));
    assert_eq!(segment_hit(&hull, [-UNIT; 3], [-UNIT; 3]), None);
    assert_contact(
        segment_hit(&hull, [2. * UNIT, inside[1], inside[2]], inside),
        [UNIT, inside[1], inside[2]],
    );
    assert_contact(
        segment_hit(
            &hull,
            [-UNIT, inside[1], inside[2]],
            [0., inside[1], inside[2]],
        ),
        [0., inside[1], inside[2]],
    );
    assert_eq!(
        segment_hit(
            &hull,
            [-UNIT, inside[1], inside[2]],
            [-UNIT * 0.01, inside[1], inside[2]]
        ),
        None
    );
    assert_eq!(segment_hit(&hull, inside, [100.; 3]), Some(inside));
}

#[test]
fn segments_cover_corner_edge_and_stationary_grid_plane_contacts() {
    let mut hull = Voxels::default();
    hull.0.insert([0, -1, 0], [1.; 4]);
    // This box is touched only at the diagonal's corner, never entered.
    assert_contact(
        segment_hit(&hull, [-UNIT, -UNIT, UNIT * 0.5], [UNIT, UNIT, UNIT * 0.5]),
        [0., 0., UNIT * 0.5],
    );
    // Travel exactly on the top grid plane of the occupied box.
    assert_contact(
        segment_hit(&hull, [-UNIT, 0., UNIT * 0.5], [2. * UNIT, 0., UNIT * 0.5]),
        [0., 0., UNIT * 0.5],
    );
    assert_eq!(segment_hit(&hull, [0., 0., 0.], [0.; 3]), Some([0.; 3]));
    hull.0.clear();
    hull.0.insert([0, -1, -1], [1.; 4]);
    assert_contact(segment_hit(&hull, [-UNIT; 3], [UNIT; 3]), [0.; 3]);
}

#[test]
fn long_segments_do_not_tunnel_and_empty_gaps_remain_empty() {
    let mut hull = Voxels::default();
    hull.0.insert([-2, 0, 0], [1.; 4]);
    hull.0.insert([4, 0, 0], [1.; 4]);
    assert_contact(
        segment_hit(
            &hull,
            [-100., UNIT * 0.5, UNIT * 0.5],
            [100., UNIT * 0.5, UNIT * 0.5],
        ),
        [-2. * UNIT, UNIT * 0.5, UNIT * 0.5],
    );
    assert_eq!(
        segment_hit(
            &hull,
            [0., UNIT * 0.5, UNIT * 0.5],
            [3. * UNIT, UNIT * 0.5, UNIT * 0.5]
        ),
        None
    );
    assert_eq!(
        segment_hit(
            &hull,
            [-100., UNIT * 2., UNIT * 0.5],
            [100., UNIT * 2., UNIT * 0.5]
        ),
        None
    );
}

#[test]
fn invalid_or_unbounded_segment_inputs_are_rejected() {
    let mut hull = Voxels::default();
    hull.0.insert([0; 3], [1.; 4]);
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX] {
        assert_eq!(segment_hit(&hull, [invalid; 3], [0.; 3]), None);
        assert_eq!(segment_hit(&hull, [0.; 3], [invalid; 3]), None);
    }
    assert_eq!(segment_hit(&hull, [-1e8; 3], [1e8; 3]), None);
    assert_eq!(segment_hit(&Voxels::default(), [0.; 3], [1.; 3]), None);
}

#[test]
fn seeded_segments_match_slab_first_contact_on_both_raider_hulls() {
    let mut random = 41_u64;
    let mut next = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        (random >> 40) as f32 / 16_777_216.
    };
    let mut hits = 0;
    for classic in [false, true] {
        let hull = battlestar_voxel::voxel::raider(classic);
        for _ in 0..4000 {
            let origin = [
                if next() < 0.5 { -3. } else { 3. },
                next() * 4. - 2.,
                next() * 2. - 1.,
            ];
            let end = [next() * 4. - 2., next() * 4. - 2., next() * 2. - 1.];
            let delta: [f32; 3] = std::array::from_fn(|i| end[i] - origin[i]);
            let expected = ray_hit(&hull, origin, delta).filter(|point| {
                let t = (point[0] - origin[0]) / delta[0];
                (0.0..=1.).contains(&t)
            });
            let actual = segment_hit(&hull, origin, end);
            assert_eq!(
                actual.is_some(),
                expected.is_some(),
                "origin={origin:?} end={end:?}"
            );
            if let Some(expected) = expected {
                hits += 1;
                assert_contact(actual, expected);
            }
        }
    }
    assert!(
        hits > 100,
        "randomized test must exercise actual hull contacts"
    );
}

#[test]
fn contact_ray_hits_actual_hull_surface_and_misses_empty_gaps() {
    let mut voxels = Voxels::default();
    voxels.0.insert([4, 0, 0], [1.; 4]);
    voxels.0.insert([-4, 0, 0], [1.; 4]);
    let hit = ray_hit(&voxels, [-2., UNIT * 0.5, UNIT * 0.5], [1., 0., 0.]).unwrap();
    assert!((hit[0] - -4. * UNIT).abs() < 0.00001);
    assert_eq!(
        ray_hit(&voxels, [-2., UNIT * 2., UNIT * 0.5], [1., 0., 0.]),
        None
    );
    assert_eq!(ray_hit(&voxels, [0.; 3], [0.; 3]), None);
    let inside = ray_hit(&voxels, [4.5 * UNIT, 0.5 * UNIT, 0.5 * UNIT], [1., 0., 0.]).unwrap();
    assert!((inside[0] - 5. * UNIT).abs() < 0.00001);
}

#[test]
fn all_six_ship_models_conserve_connected_cells_across_seeded_side_impacts() {
    use battlestar_voxel::voxel::{galactica, raider, viper};
    for model in [viper, raider, galactica] {
        for classic in [false, true] {
            let hull = model(classic);
            for seed in [0, 1, 42, u64::MAX] {
                let side = if seed % 2 == 0 { -100. } else { 100. };
                let impact = ray_hit(&hull, [side, UNIT * 0.5, UNIT * 0.5], [-side, 0., 0.])
                    .unwrap_or([side, 0., 0.]);
                let chunks = fracture(&hull, impact, seed);
                assert_partition(&hull, &chunks);
                let repeated = fracture(&hull, impact, seed);
                assert_eq!(chunks.len(), repeated.len());
                for (chunk, repeated) in chunks.iter().zip(repeated) {
                    assert_eq!(chunk.cells.0, repeated.cells.0);
                    assert_eq!(chunk.heat, repeated.heat);
                    let bricks = split_chunk(chunk, seed.wrapping_add(11));
                    assert_partition(&chunk.cells, &bricks);
                    let heat: BTreeMap<_, _> = bricks
                        .iter()
                        .flat_map(|b| b.heat.iter().map(|(p, h)| (*p, *h)))
                        .collect();
                    assert_eq!(heat, chunk.heat);
                }
            }
        }
    }
}
