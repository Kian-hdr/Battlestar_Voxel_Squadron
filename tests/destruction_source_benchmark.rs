//! Qualify borrowing the already-warm immutable collision hulls for destruction.
//! Run the ignored benchmark in release mode while native timing is stopped.
use battlestar_voxel::{
    combat_plane::view_forward,
    voxel::{self, UNIT, Voxels},
    voxel_damage::{VoxelChunk, fracture, ray_hit},
};
use glam::{Quat, Vec3};
use std::{hint::black_box, time::Instant};

#[derive(Clone, Copy)]
struct Request {
    player: bool,
    classic: bool,
    local_contact: Vec3,
    local_ray: Vec3,
    seed: u64,
}

fn source(player: bool, classic: bool) -> Voxels {
    if player {
        voxel::viper(classic)
    } else {
        voxel::raider(classic)
    }
}

// These are precisely indices 0..4 of shatter_render::hull_models(), excluding
// Galactica because it cannot produce a ship-destruction request.
fn sources() -> [Voxels; 4] {
    [
        voxel::viper(false),
        voxel::viper(true),
        voxel::raider(false),
        voxel::raider(true),
    ]
}
fn index(request: Request) -> usize {
    usize::from(request.classic) + if request.player { 0 } else { 2 }
}
fn requests(models: &[Voxels; 4]) -> Vec<Request> {
    let mut requests = Vec::new();
    for player in [false, true] {
        for classic in [false, true] {
            let model = &models[usize::from(classic) + if player { 0 } else { 2 }];
            for seed in 0..16_u64 {
                let p = *model
                    .0
                    .keys()
                    .nth((seed as usize * 37) % model.0.len())
                    .unwrap();
                let rotation = Quat::from_rotation_z(seed as f32 * 0.08 - 0.5)
                    * Quat::from_rotation_y(seed as f32 * 0.04 - 0.3);
                requests.push(Request {
                    player,
                    classic,
                    local_contact: Vec3::from_array(p.map(|v| (v as f32 + 0.5) * UNIT)),
                    local_ray: rotation.conjugate() * view_forward(),
                    seed: seed ^ (u64::from(classic) << 32) ^ (u64::from(player) << 40),
                });
            }
        }
    }
    requests
}
fn destroy(model: &Voxels, request: Request) -> (Vec3, Vec<VoxelChunk>) {
    let origin = request.local_contact - request.local_ray * 8.;
    let contact = ray_hit(model, origin.to_array(), request.local_ray.to_array())
        .map(Vec3::from_array)
        .unwrap_or(request.local_contact);
    (contact, fracture(model, contact.to_array(), request.seed))
}
fn assert_same(model: &Voxels, request: Request) {
    let regenerated = source(request.player, request.classic);
    assert_eq!(
        model.0, regenerated.0,
        "cached source must exactly match its constructor"
    );
    let (contact_a, a) = destroy(&regenerated, request);
    let (contact_b, b) = destroy(model, request);
    assert_eq!(contact_a, contact_b, "viewing-ray surface contact changed");
    assert_eq!(a.len(), b.len(), "fracture count changed");
    for (a, b) in a.iter().zip(&b) {
        assert_eq!(
            a.cells.0, b.cells.0,
            "occupied cells or per-cell colors changed"
        );
        assert_eq!(a.pivot, b.pivot, "piece centroid changed");
        assert_eq!(a.heat, b.heat, "localized heat changed");
    }
}

#[test]
fn borrowed_immutable_hulls_preserve_exact_ray_fracture_cells_pivots_and_heat() {
    let models = sources();
    for request in requests(&models) {
        assert_same(&models[index(request)], request);
    }
}

#[test]
#[ignore = "explicit release-mode CPU benchmark; stop native performance capture first"]
fn benchmark_regenerated_versus_warm_borrowed_destruction_sources() {
    let models = sources();
    let requests = requests(&models);
    for &request in &requests {
        assert_same(&models[index(request)], request);
    }
    // Compare identical ray/fracture work and result destruction. Only source
    // regeneration differs; cache initialization is intentionally outside timing
    // because targets() already warms the six collision models before requests.
    let iterations = 3;
    let started = Instant::now();
    for _ in 0..iterations {
        for &request in &requests {
            let model = source(black_box(request.player), black_box(request.classic));
            black_box(destroy(&model, black_box(request)));
        }
    }
    let regenerated = started.elapsed();
    let started = Instant::now();
    for _ in 0..iterations {
        for &request in &requests {
            black_box(destroy(
                black_box(&models[index(request)]),
                black_box(request),
            ));
        }
    }
    let borrowed = started.elapsed();
    let count = requests.len() * iterations;
    println!(
        "destruction source {count} requests: regenerate={:.3}ms ({:.3}us/request), warm_borrow={:.3}ms ({:.3}us/request), ratio={:.3}x",
        regenerated.as_secs_f64() * 1000.,
        regenerated.as_secs_f64() * 1e6 / count as f64,
        borrowed.as_secs_f64() * 1000.,
        borrowed.as_secs_f64() * 1e6 / count as f64,
        regenerated.as_secs_f64() / borrowed.as_secs_f64(),
    );
}
