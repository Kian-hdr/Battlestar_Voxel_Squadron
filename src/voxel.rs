//! Editable voxel models and exposed-face meshing. No textures or imported artwork.
use std::collections::BTreeMap;
pub type Tint = [f32; 4];
#[derive(Default)]
pub struct Voxels(pub BTreeMap<[i32; 3], Tint>);
#[derive(Default)]
pub struct Geometry {
    pub positions: Vec<[f32; 3]>,
    /// Source occupied cell for each vertex; damage heat belongs to this material cell.
    pub cells: Vec<[i32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<Tint>,
    pub indices: Vec<u32>,
}
impl Voxels {
    pub fn block(&mut self, from: [i32; 3], to: [i32; 3], color: Tint) {
        for x in from[0]..=to[0] {
            for y in from[1]..=to[1] {
                for z in from[2]..=to[2] {
                    self.0.insert([x, y, z], color);
                }
            }
        }
    }
    pub fn mesh(&self, unit: f32) -> Geometry {
        let faces = [
            ([1, 0, 0], [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]]),
            ([-1, 0, 0], [[0, 1, 0], [0, 0, 0], [0, 0, 1], [0, 1, 1]]),
            ([0, 1, 0], [[1, 1, 0], [0, 1, 0], [0, 1, 1], [1, 1, 1]]),
            ([0, -1, 0], [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]]),
            ([0, 0, 1], [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]]),
            ([0, 0, -1], [[0, 1, 0], [1, 1, 0], [1, 0, 0], [0, 0, 0]]),
        ];
        let mut g = Geometry::default();
        for (p, color) in &self.0 {
            for (n, corners) in faces {
                if self
                    .0
                    .contains_key(&[p[0] + n[0], p[1] + n[1], p[2] + n[2]])
                {
                    continue;
                }
                let base = g.positions.len() as u32;
                for c in corners {
                    g.cells.push(*p);
                    g.positions.push([
                        (p[0] + c[0]) as f32 * unit,
                        (p[1] + c[1]) as f32 * unit,
                        (p[2] + c[2]) as f32 * unit,
                    ]);
                    g.normals.push(n.map(|v| v as f32));
                    let hash = p[0].abs().wrapping_mul(73856093)
                        ^ p[1].wrapping_mul(19349663)
                        ^ p[2].wrapping_mul(83492791);
                    let shade = 0.955 + (hash.unsigned_abs() % 5) as f32 * 0.015;
                    g.colors.push([
                        color[0] * shade,
                        color[1] * shade,
                        color[2] * shade,
                        color[3],
                    ]);
                }
                g.indices
                    .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        g
    }
}
// Palettes follow the neutral miniature/replica reference photographs in docs/Art_References.md.
const PLATE: Tint = [0.64, 0.62, 0.58, 1.];
const WHITE: Tint = [0.78, 0.76, 0.70, 1.];
/// Shared physical block size. More cells refine the silhouette without stretching cubes.
pub const UNIT: f32 = 0.12;

impl Voxels {
    // Positive-x cells are authoritative, including surface relief and color.
    fn lateral_symmetry(mut self) -> Self {
        self.0.retain(|p, _| p[0] >= 0);
        let half: Vec<_> = self.0.iter().map(|(p, c)| (*p, *c)).collect();
        for (p, color) in half {
            self.0.insert([-p[0], p[1], p[2]], color);
        }
        self
    }
    fn ellipsoid(&mut self, c: [i32; 3], r: [f32; 3], color: Tint) {
        for x in -(r[0].ceil() as i32)..=r[0].ceil() as i32 {
            for y in -(r[1].ceil() as i32)..=r[1].ceil() as i32 {
                for z in -(r[2].ceil() as i32)..=r[2].ceil() as i32 {
                    if (x as f32 / r[0]).powi(2)
                        + (y as f32 / r[1]).powi(2)
                        + (z as f32 / r[2]).powi(2)
                        <= 1.
                    {
                        self.0.insert([c[0] + x, c[1] + y, c[2] + z], color);
                    }
                }
            }
        }
    }
    fn game_grid(self) -> Self {
        // Preserve the earlier voxel resolution. Drafting coordinates only describe arrangement.
        let ratio = 0.045 / UNIT;
        let mut cells: BTreeMap<[i32; 3], (f32, Tint)> = BTreeMap::new();
        for (p, color) in self.0 {
            let q = p.map(|v| ((v as f32 + 0.5) * ratio).floor() as i32);
            let distance = (0..3)
                .map(|i| ((p[i] as f32 + 0.5) * ratio - (q[i] as f32 + 0.5)).powi(2))
                .sum::<f32>();
            let cell = cells.entry(q).or_insert((f32::INFINITY, color));
            if distance < cell.0 {
                *cell = (distance, color);
            }
        }
        Self(cells.into_iter().map(|(p, (_, c))| (p, c)).collect())
    }
    // Cross-section loft: (longitudinal y, half-width, half-height, vertical center).
    fn loft(&mut self, cx: i32, sections: &[(i32, f32, f32, f32)], color: Tint) {
        for pair in sections.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            for y in a.0..=b.0 {
                let t = (y - a.0) as f32 / (b.0 - a.0) as f32;
                let w = a.1 + (b.1 - a.1) * t;
                let h = a.2 + (b.2 - a.2) * t;
                let zc = a.3 + (b.3 - a.3) * t;
                for x in -(w.ceil() as i32)..=w.ceil() as i32 {
                    for z in (zc - h).floor() as i32..=(zc + h).ceil() as i32 {
                        if (x as f32 / w).abs().powi(4) + ((z as f32 - zc) / h).abs().powi(3) <= 1.
                        {
                            self.0.insert([cx + x, y, z], color);
                        }
                    }
                }
            }
        }
    }
    fn tube_y(&mut self, c: [i32; 3], radius: f32, length: i32, color: Tint) {
        let r = radius.ceil() as i32;
        for x in -r..=r {
            for z in -r..=r {
                if (x * x + z * z) as f32 <= radius * radius {
                    self.block(
                        [c[0] + x, c[1], c[2] + z],
                        [c[0] + x, c[1] + length, c[2] + z],
                        color,
                    );
                }
            }
        }
    }
    fn carve(&mut self, a: [i32; 3], b: [i32; 3]) {
        self.0
            .retain(|p, _| !(0..3).all(|i| p[i] >= a[i] && p[i] <= b[i]));
    }
    fn paint(&mut self, a: [i32; 3], b: [i32; 3], color: Tint) {
        for (p, c) in &mut self.0 {
            if (0..3).all(|i| p[i] >= a[i] && p[i] <= b[i]) {
                *c = color;
            }
        }
    }
    fn top(&self, x: i32, y: i32) -> Option<i32> {
        (-25..=35).rev().find(|z| self.0.contains_key(&[x, y, *z]))
    }
    fn panel_detail(&mut self, stride: i32, color: Tint) {
        let positions: Vec<_> = self
            .0
            .keys()
            .filter(|p| {
                p[0] % stride == 0
                    && p[1] % (stride + 3) == 0
                    && !self.0.contains_key(&[p[0], p[1], p[2] + 1])
            })
            .copied()
            .collect();
        for p in positions {
            // Low relief plates remain cubes and follow the actual surface instead of floating.
            self.block(
                [p[0], p[1], p[2] + 1],
                [p[0] + 2, p[1] + 3, p[2] + 1],
                color,
            );
        }
    }
}

pub fn viper(classic: bool) -> Voxels {
    // Preserve the first playable sculpture and its original local palette.
    const HULL: Tint = [0.60, 0.66, 0.70, 1.];
    const DARK: Tint = [0.12, 0.18, 0.23, 1.];
    const WHITE: Tint = [0.88, 0.86, 0.76, 1.];
    const RED: Tint = [0.66, 0.095, 0.055, 1.];
    const GLASS: Tint = [0.07, 0.54, 0.68, 1.];
    let mut v = Voxels::default();
    v.ellipsoid(
        [0, 0, 0],
        [2.5, 7.5, 2.3],
        if classic { WHITE } else { HULL },
    );
    v.block([-1, 5, 0], [1, 9, 1], WHITE);
    v.block([0, 10, 0], [0, 11, 0], RED);
    // Raised canopy, armored belly, three distinct engine pods and dorsal stabilizer.
    v.ellipsoid([0, 1, 3], [1.6, 2.8, 1.5], DARK);
    v.block([0, 0, 4], [0, 2, 4], GLASS);
    v.block([-1, -6, -2], [1, 3, -2], DARK);
    for side in [-1, 1] {
        let x = side * 3;
        v.ellipsoid([x, -5, 0], [1.4, 3.5, 1.4], WHITE);
        v.block(
            [x, -8, 0],
            [x, -8, 0],
            if classic { [1., 0.45, 0.1, 1.] } else { GLASS },
        );
        for wing in 2..=7 {
            let y = if classic { -2 } else { -wing / 2 };
            v.block([side * wing, y - 2, -1], [side * wing, y, 0], WHITE);
            if wing % 2 == 0 {
                v.block([side * wing, y - 1, 1], [side * wing, y - 1, 1], RED);
            }
        }
        v.block([side * 7, -4, 0], [side * 7, 3, 0], DARK);
    }
    v.ellipsoid([0, -5, 2], [1.2, 3.5, 1.2], HULL);
    v.block([0, -7, 3], [0, -2, 5], if classic { WHITE } else { RED });
    v.block([0, 4, 2], [0, 7, 2], RED);
    if !classic {
        v.block([-1, 6, 1], [1, 8, 1], RED);
        v.block([0, 9, 1], [0, 10, 1], HULL);
    }
    v.lateral_symmetry()
}
// Vertex colors are linear albedo. Author palettes in sRGB, then decode once.
fn linear_albedo(srgb: [f32; 3]) -> Tint {
    let rgb = srgb.map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    [rgb[0], rgb[1], rgb[2], 1.]
}
pub fn raider(classic: bool) -> Voxels {
    let mut v = Voxels::default();
    let (hull, plate, panel, dark) = if classic {
        (
            linear_albedo([0.46, 0.48, 0.50]),
            linear_albedo([0.54, 0.56, 0.58]),
            linear_albedo([0.30, 0.32, 0.34]),
            linear_albedo([0.12, 0.14, 0.16]),
        )
    } else {
        (
            linear_albedo([0.20, 0.22, 0.24]),
            linear_albedo([0.28, 0.30, 0.32]),
            linear_albedo([0.16, 0.18, 0.20]),
            linear_albedo([0.07, 0.08, 0.09]),
        )
    };
    let red = linear_albedo([0.66, 0.045, 0.025]);
    if classic {
        // Broad shallow saucer with swept clipped shoulders and a recessed trapezoid grille.
        for x in -22i32..=22 {
            for y in -14i32..=14 {
                let r = (x as f32 / 22.).powi(2) + (y as f32 / 14.).powi(2);
                if r <= 1. {
                    let h = (1.0 + (1. - r) * 2.5) as i32;
                    v.block([x, y, -1], [x, y, h], hull);
                }
            }
        }
        v.loft(
            0,
            &[
                (-6, 7., 1.5, 3.),
                (0, 6., 2., 4.),
                (8, 5., 1.5, 4.),
                (12, 3., 1., 3.),
            ],
            plate,
        );
        v.carve([-9, -14, 1], [9, -4, 7]);
        for y in -13..=-4 {
            let w = 5 + (-y - 4) / 3;
            v.block([-w, y, 1], [w, y, 1], dark);
            for x in (-w..=w).step_by(3) {
                v.block([x, y, 2], [x, y, 2], panel);
            }
        }
        for side in [-1, 1] {
            v.block([side * 15, -16, 0], [side * 15, -7, 1], panel);
            v.block([side * 16, -11, 1], [side * 16, 6, 2], dark);
            v.block([side * 17, -9, 2], [side * 17, 5, 2], plate);
            v.tube_y([side * 5, 8, 5], 2.5, 3, panel);
            v.tube_y([side * 5, 8, 5], 1.4, 1, dark);
        }
        v.panel_detail(7, plate);
    } else {
        // Horseshoe with thin swept forward tips, open crescent and a narrow central head.
        for x in -23i32..=23 {
            for y in -21i32..=20 {
                let outer = (x as f32 / 23.).powi(2) + (y as f32 / 21.).powi(2);
                let inner = (x as f32 / 15.5).powi(2) + ((y as f32 + 10.) / 23.).powi(2);
                if outer <= 1. && inner >= 1. {
                    let h = (1. + (1. - outer) * 3.) as i32;
                    v.block(
                        [x, y, -1],
                        [x, y, h],
                        if x.abs() > 19 { plate } else { panel },
                    );
                    if x.abs() % 8 == 0 {
                        v.block([x, y, h], [x, y, h], hull);
                    }
                }
            }
        }
        v.loft(
            0,
            &[
                (-11, 1., 1., 2.),
                (-8, 3., 2., 2.),
                (3, 4., 3., 2.),
                (15, 3., 2., 1.),
                (19, 2., 1., 1.),
            ],
            hull,
        );
        v.block([-2, -10, 3], [2, -10, 3], red);
        for side in [-1, 1] {
            v.block([side * 7, 7, 0], [side * 7, 14, 1], dark);
            v.tube_y([side * 5, 17, 0], 1.8, 3, dark);
        }
    }
    v.game_grid().lateral_symmetry()
}

pub fn galactica(classic: bool) -> Voxels {
    let mut v = Voxels::default();
    let (hull, plate, panel) = if classic {
        (
            linear_albedo([0.50, 0.51, 0.52]),
            linear_albedo([0.58, 0.59, 0.60]),
            linear_albedo([0.28, 0.29, 0.30]),
        )
    } else {
        (
            linear_albedo([0.30, 0.31, 0.32]),
            linear_albedo([0.38, 0.39, 0.40]),
            linear_albedo([0.18, 0.19, 0.20]),
        )
    };
    let dark = linear_albedo([0.07, 0.08, 0.09]);
    if classic {
        // Proportions traced from the photographed top silhouette: large bow + broad aft block.
        v.loft(
            0,
            &[
                (-108, 25., 7., 0.),
                (-96, 28., 10., 1.),
                (-65, 27., 9., 1.),
                (-51, 18., 7., 0.),
                (-20, 18., 8., 1.),
                (24, 18., 8., 1.),
                (40, 26., 11., 1.),
                (63, 30., 13., 1.),
                (83, 25., 11., 2.),
                (103, 14., 8., 1.),
                (110, 5., 5., 1.),
            ],
            hull,
        );
    } else {
        v.loft(
            0,
            &[
                (-108, 24., 6., -1.),
                (-96, 26., 9., 0.),
                (-67, 25., 9., 0.),
                (-47, 16., 6., -1.),
                (-11, 17., 8., 0.),
                (23, 22., 11., 1.),
                (47, 30., 14., 1.),
                (71, 29., 14., 1.),
                (94, 20., 11., 0.),
                (108, 10., 7., -1.),
            ],
            hull,
        );
    }
    // Shorter, close-set flight pods: 43% of hull length; three supports each side.
    for side in [-1, 1] {
        let cx = side * 33;
        v.loft(
            cx,
            &[
                (-58, 3., 3., -2.),
                (-52, 7., 5., -2.),
                (-39, 7., 5., -2.),
                (25, 7., 5., -2.),
                (36, 5., 4., -2.),
                (41, 2., 2., -2.),
            ],
            hull,
        );
        for y in [-42, -6, 28] {
            let a = (side * 18).min(cx);
            let b = (side * 18).max(cx);
            v.block([a, y, -3], [b, y + 3, 0], plate);
        }
        // End-on hangar openings, never a black stripe over the entire pod roof.
        v.carve([cx - 3, 32, -4], [cx + 3, 42, 0]);
        v.block([cx - 3, 30, -4], [cx + 3, 31, 0], dark);
        v.carve([cx - 3, -59, -4], [cx + 3, -51, 0]);
        v.block([cx - 3, -50, -4], [cx + 3, -49, 0], dark);
        for y in (-46..31).step_by(if classic { 10 } else { 5 }) {
            v.block(
                [cx - 6, y, 3],
                [cx + 6, y, 4],
                if classic { plate } else { panel },
            );
        }
        for y in [-47, 30] {
            v.block(
                [cx - 5, y, -1],
                [cx - 5, y + 1, -1],
                linear_albedo([0.83, 0.78, 0.61]),
            );
        }
    }
    // Aft machinery, exposed drive trenches and six separate main engine bells.
    for x in [-16, 0, 16] {
        for z in [-5, 5] {
            v.tube_y([x, -114, z], 4.4, 9, panel);
            v.tube_y([x, -115, z], 3.0, 1, dark);
        }
    }
    for side in [-1, 1] {
        v.block([side * 13 - 2, -96, 10], [side * 13 + 2, -59, 12], panel);
        for y in (-94..-57).step_by(6) {
            v.block([side * 13 - 3, y, 11], [side * 13 + 3, y + 1, 13], plate);
        }
    }
    v.loft(
        0,
        &[
            (-38, 8., 1., 9.),
            (-24, 9., 3., 10.),
            (13, 8., 3., 10.),
            (29, 6., 1., 10.),
        ],
        plate,
    );
    // Dorsal greebles and layered command head, defined in blocks instead of painted textures.
    for y in [-20, 8] {
        for x in -5i32..=5 {
            for dy in -5i32..=5 {
                if x * x + dy * dy <= 25 {
                    v.block([x, y + dy, 13], [x, y + dy, 14], panel);
                }
            }
        }
    }
    v.block([-5, 51, 13], [5, 58, 16], plate);
    v.block([-3, 57, 17], [3, 61, 18], hull);
    if !classic {
        for y in (-99..96).step_by(5) {
            for x in -29..=29 {
                if let Some(z) = v.top(x, y) {
                    v.block([x, y, z], [x, y, z], panel);
                }
            }
        }
        // Recognizable faded red bands across the armored forward hull.
        for y in [78, 84] {
            v.paint(
                [-24, y, 0],
                [24, y + 1, 18],
                linear_albedo([0.34, 0.075, 0.055]),
            );
        }
    }
    v.panel_detail(if classic { 8 } else { 7 }, plate);
    v.game_grid().lateral_symmetry()
}
pub fn projectile() -> Voxels {
    let mut v = Voxels::default();
    for y in -2..=2 {
        v.block([0, y, 0], [0, y, 0], if y % 2 == 0 { WHITE } else { PLATE });
    }
    v
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hides_internal_faces() {
        let mut v = Voxels::default();
        v.block([0, 0, 0], [1, 0, 0], WHITE);
        let g = v.mesh(1.);
        assert_eq!(g.indices.len(), 60);
        assert_eq!(g.positions.len(), 40);
    }
    #[test]
    fn model_geometry_is_finite_indexed_and_volumetric() {
        for v in [
            viper(true),
            viper(false),
            raider(true),
            raider(false),
            galactica(true),
            galactica(false),
        ] {
            let g = v.mesh(UNIT);
            assert!(!g.positions.is_empty());
            assert!(g.positions.iter().flatten().all(|v| v.is_finite()));
            assert!(g.indices.iter().all(|i| (*i as usize) < g.positions.len()));
            assert_eq!(g.normals.len(), g.positions.len());
            let min = v.0.keys().map(|p| p[2]).min().unwrap();
            let max = v.0.keys().map(|p| p[2]).max().unwrap();
            assert!(max - min >= 2, "ship must have sculpted vertical depth");
        }
    }
}
