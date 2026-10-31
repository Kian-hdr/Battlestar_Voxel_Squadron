//! Small, continuously deforming exhaust surfaces. The nozzle face never moves.
use crate::voxel::UNIT;

pub const RINGS: usize = 5;
pub const VERTICES: usize = RINGS * 4;
pub const NOZZLE_Y: f32 = -8. * UNIT - 0.004;

pub struct Plume {
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
}

pub fn sample(time: f32, nozzle: u8, classic: bool, thrust: f32) -> Plume {
    let phase = f32::from(nozzle) * 2.13;
    let length = 0.39
        + 0.065 * (time * 17. + phase).sin()
        + 0.04 * (time * 31. + phase * 0.7).sin()
        + thrust.clamp(0., 1.) * 0.07;
    let distances = [0., 0.18, 0.45, 0.75, 1.];
    let radii = [0.46, 0.54, 0.38, 0.20, 0.01];
    let alpha = [0.72, 0.62, 0.42, 0.18, 0.];
    let warm = [
        [8., 6.8, 3.8],
        [5., 3.1, 0.8],
        [3.5, 1.3, 0.22],
        [1.8, 0.48, 0.05],
        [1., 0.2, 0.02],
    ];
    let cool = [
        [3.8, 8., 10.],
        [1.2, 5., 8.],
        [0.25, 3., 5.],
        [0.07, 1.5, 2.5],
        [0.02, 0.4, 0.8],
    ];
    let palette = if classic { warm } else { cool };
    let mut positions = Vec::with_capacity(VERTICES);
    let mut colors = Vec::with_capacity(VERTICES);
    for ring in 0..RINGS {
        let d = distances[ring];
        let ripple = if ring == 0 {
            1.
        } else {
            0.9 + 0.1 * (time * 23. - d * 8. + phase).sin()
        };
        let radius = radii[ring] * UNIT * ripple;
        let x = d * 0.015 * (time * 13. - d * 8. + phase).sin();
        let z = d * 0.012 * (time * 19. - d * 6. + phase).cos();
        let energy = 0.92 + 0.08 * (time * 23. - d * 9. + phase).sin();
        for [sx, sz] in [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]] {
            positions.push([x + sx * radius, -length * d, z + sz * radius]);
            colors.push([
                palette[ring][0] * energy,
                palette[ring][1] * energy,
                palette[ring][2] * energy,
                alpha[ring],
            ]);
        }
    }
    Plume { positions, colors }
}

pub fn indices() -> Vec<u32> {
    let mut result = vec![0, 2, 1, 0, 3, 2];
    for ring in 0..RINGS - 1 {
        for side in 0..4 {
            let a = (ring * 4 + side) as u32;
            let b = (ring * 4 + (side + 1) % 4) as u32;
            result.extend([a, b, b + 4, a, b + 4, a + 4]);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animated_plume_never_intersects_either_hull_or_goes_dark_at_nozzle() {
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
                    assert!(p.colors[..4].iter().all(|c| c[3] > 0.5));
                    assert!(p.colors[VERTICES - 4..].iter().all(|c| c[3] == 0.));
                }
            }
        }
    }
    #[test]
    fn plumes_flow_independently_with_taper_and_no_discontinuous_geometry() {
        let a = sample(0., 0, true, 0.);
        assert_ne!(a.positions, sample(0.13, 0, true, 0.).positions);
        assert_ne!(a.positions, sample(0., 1, true, 0.).positions);
        assert!(a.positions[12][0].abs() < a.positions[0][0].abs());
        for step in 0..240 {
            let t = step as f32 / 120.;
            let a = sample(t, 0, true, 0.);
            let b = sample(t + 1. / 120., 0, true, 0.);
            assert!(
                a.positions
                    .iter()
                    .zip(&b.positions)
                    .all(|(a, b)| (0..3).all(|i| (a[i] - b[i]).abs() < 0.04))
            );
        }
        assert!(indices().iter().all(|i| (*i as usize) < VERTICES));
    }
}
