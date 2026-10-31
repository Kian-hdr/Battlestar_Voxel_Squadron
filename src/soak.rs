//! Optional long-run diagnostics. Normal gameplay does not start or write a soak.
//! CSV update_us/render_us are window means of the latest system timing sampled
//! each display frame. They are not whole-process CPU time or sums of fixed steps.
//! cpu_percent is the native ps process snapshot, not a derived window interval.
use super::shatter::ShatterEffects;
use crate::Session;
use battlestar_voxel::simulation::Phase;
use bevy::prelude::*;
use std::{fs, path::PathBuf};

#[derive(Resource)]
pub(super) struct Soak {
    pub enabled: bool,
    pub draining: bool,
    duration: f64,
    boot: Option<f64>,
    started: Option<f64>,
    sampled_at: f64,
    frames: u64,
    samples: usize,
    window: Vec<f64>,
    window_update_us: f64,
    window_render_us: f64,
    window_timing_samples: usize,
    csv: String,
    directory: PathBuf,
    initial_mesh_assets: usize,
    initial_material_assets: usize,
    initial_entities: usize,
    peak_bodies: usize,
    peak_effects: usize,
    peak_vertices: usize,
    peak_mesh_assets: usize,
    finished: bool,
}
impl Default for Soak {
    fn default() -> Self {
        Self {
            enabled: std::env::args().any(|a| a == "--soak-test"),
            draining: false,
            duration: std::env::var("BSVX_SOAK_SECONDS")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v > 20.)
                .unwrap_or(180.),
            boot: None,
            started: None,
            sampled_at: 0.,
            frames: 0,
            samples: 0,
            window: Vec::new(),
            window_update_us: 0.,
            window_render_us: 0.,
            window_timing_samples: 0,
            csv: "elapsed_s,phase,frames,frame_mean_ms,frame_p95_ms,effects,bodies,cubes,vertices,outside_bodies,oldest_age_s,update_us,render_us,mesh_assets,material_assets,entities,rss_kib,physical_width,physical_height,wave,score,enemies,shots,cpu_percent\n".into(),
            directory: std::env::var_os("BSVX_EVIDENCE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("evidence")),
            initial_mesh_assets: 0,
            initial_material_assets: 0,
            initial_entities: 0,
            peak_bodies: 0,
            peak_effects: 0,
            peak_vertices: 0,
            peak_mesh_assets: 0,
            finished: false,
        }
    }
}

fn process_usage() -> (Option<f64>, Option<u64>) {
    #[cfg(unix)]
    {
        let result = (|| {
            let output = std::process::Command::new("ps")
                .args(["-o", "%cpu=,rss=", "-p", &std::process::id().to_string()])
                .output()
                .ok()?;
            if !output.status.success() {
                return None;
            }
            let text = std::str::from_utf8(&output.stdout).ok()?;
            let mut columns = text.split_whitespace();
            let cpu = columns.next()?.parse::<f64>().ok()?;
            let rss = columns.next()?.parse::<u64>().ok()?;
            Some((cpu.is_finite().then_some(cpu), Some(rss)))
        })();
        result.unwrap_or((None, None))
    }
    #[cfg(not(unix))]
    {
        (None, None)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run(
    time: Res<Time<Real>>,
    mut soak: ResMut<Soak>,
    mut session: ResMut<Session>,
    effects: Res<ShatterEffects>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    entities: Query<Entity>,
    window: Single<&Window>,
    mut exit: MessageWriter<AppExit>,
) {
    if !soak.enabled || soak.finished {
        return;
    }
    let now = time.elapsed_secs_f64();
    let boot = *soak.boot.get_or_insert(now);
    let Some(started) = soak.started else {
        if now - boot >= 1. {
            session.0.start();
            session.1 = session.1.wrapping_add(1);
            session.0.invulnerable = 999.;
            soak.started = Some(now);
            soak.initial_mesh_assets = meshes.len();
            soak.initial_material_assets = materials.len();
            soak.initial_entities = entities.iter().count();
            if let Err(error) = fs::create_dir_all(&soak.directory) {
                error!("SOAK could not create evidence directory: {error}");
                soak.finished = true;
                exit.write(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
            }
            info!("SOAK started duration_s={} drain_s=20", soak.duration);
        }
        return;
    };
    let elapsed = now - started;
    soak.frames += 1;
    let frame_ms = time.delta_secs_f64() * 1000.;
    if frame_ms.is_finite() {
        soak.window.push(frame_ms);
    }
    if elapsed >= soak.duration - 20. {
        if !soak.draining {
            info!("SOAK draining: no new combat or destruction");
        }
        soak.draining = true;
        // Title prevents Game::step from spawning another wave after a clear.
        // Shatter simulation continues in Title; only Paused freezes debris.
        session.0.phase = Phase::Title;
        session.0.enemies.clear();
        session.0.shots.clear();
    } else {
        session.0.invulnerable = 999.;
    }
    let (update_us, render_us) = effects.timings();
    if update_us.is_finite() && render_us.is_finite() {
        soak.window_update_us += update_us;
        soak.window_render_us += render_us;
        soak.window_timing_samples += 1;
    }
    let finished = elapsed >= soak.duration;
    if !finished && elapsed - soak.sampled_at < 5. {
        return;
    }
    let d = effects.diagnostics();
    soak.peak_bodies = soak.peak_bodies.max(d.bodies);
    soak.peak_effects = soak.peak_effects.max(d.effects);
    soak.peak_vertices = soak.peak_vertices.max(d.vertices);
    soak.peak_mesh_assets = soak.peak_mesh_assets.max(meshes.len());
    let mean = soak.window.iter().sum::<f64>() / soak.window.len().max(1) as f64;
    soak.window.sort_by(f64::total_cmp);
    let p95 = soak
        .window
        .get(((soak.window.len() as f64 * 0.95).ceil() as usize).saturating_sub(1))
        .copied()
        .unwrap_or(0.);
    let mean_update_us = soak.window_update_us / soak.window_timing_samples.max(1) as f64;
    let mean_render_us = soak.window_render_us / soak.window_timing_samples.max(1) as f64;
    let (cpu, rss) = process_usage();
    let cpu_csv = cpu.map(|v| format!("{v:.2}")).unwrap_or_default();
    let rss_csv = rss.map(|v| v.to_string()).unwrap_or_default();
    let phase = if soak.draining { "drain" } else { "combat" };
    let row = format!(
        "{elapsed:.3},{phase},{},{mean:.4},{p95:.4},{},{},{},{},{},{:.4},{:.3},{:.3},{},{},{},{rss_csv},{},{},{},{},{},{},{cpu_csv}\n",
        soak.frames,
        d.effects,
        d.bodies,
        d.cubes,
        d.vertices,
        d.outside_bodies,
        d.oldest_age,
        mean_update_us,
        mean_render_us,
        meshes.len(),
        materials.len(),
        entities.iter().count(),
        window.physical_width(),
        window.physical_height(),
        session.0.wave,
        session.0.score,
        session.0.enemies.len(),
        session.0.shots.len(),
    );
    soak.csv.push_str(&row);
    soak.sampled_at = elapsed;
    soak.samples += 1;
    soak.window.clear();
    soak.window_update_us = 0.;
    soak.window_render_us = 0.;
    soak.window_timing_samples = 0;
    let mut result = fs::write(soak.directory.join("Soak_Samples.csv"), &soak.csv);
    info!(
        "SOAK t={elapsed:.1}s phase={phase} bodies={} effects={} meshes={} oldest={:.2}s mean={mean:.2}ms p95={p95:.2}ms",
        d.bodies,
        d.effects,
        meshes.len(),
        d.oldest_age,
    );
    if finished {
        let cpu_json = cpu
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "null".into());
        let report = format!(
            "{{\"schema\":1,\"complete\":true,\"report_only\":true,\"duration_s\":{elapsed:.3},\"requested_duration_s\":{:.3},\"drain_s\":20,\"samples\":{},\"frames\":{},\"initial_mesh_assets\":{},\"initial_material_assets\":{},\"initial_entities\":{},\"final_effects\":{},\"final_bodies\":{},\"final_cubes\":{},\"final_vertices\":{},\"final_outside_bodies\":{},\"final_oldest_age_s\":{:.4},\"final_mesh_assets\":{},\"final_material_assets\":{},\"final_entities\":{},\"final_rss_kib\":{},\"peak_sampled_effects\":{},\"peak_sampled_bodies\":{},\"peak_sampled_vertices\":{},\"peak_sampled_mesh_assets\":{},\"drained\":{},\"final_window_mean_ms\":{mean:.4},\"final_window_p95_ms\":{p95:.4},\"final_window_update_us\":{mean_update_us:.3},\"final_window_render_us\":{mean_render_us:.3},\"final_cpu_percent\":{cpu_json},\"timing_columns\":\"display-frame sampled latest system durations averaged over each window\",\"cpu_percent_source\":\"native ps process snapshot\"}}\n",
            soak.duration,
            soak.samples,
            soak.frames,
            soak.initial_mesh_assets,
            soak.initial_material_assets,
            soak.initial_entities,
            d.effects,
            d.bodies,
            d.cubes,
            d.vertices,
            d.outside_bodies,
            d.oldest_age,
            meshes.len(),
            materials.len(),
            entities.iter().count(),
            rss.map(|v| v.to_string()).unwrap_or_else(|| "null".into()),
            soak.peak_effects,
            soak.peak_bodies,
            soak.peak_vertices,
            soak.peak_mesh_assets,
            d.effects == 0 && d.bodies == 0,
        );
        result = result.and_then(|()| fs::write(soak.directory.join("Soak_Result.json"), report));
        soak.finished = true;
    }
    if let Err(error) = result {
        error!("SOAK evidence write failed: {error}");
        soak.finished = true;
        exit.write(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
    } else if finished {
        info!("SOAK complete; retained bodies={} (report only)", d.bodies);
        exit.write(AppExit::Success);
    }
}
