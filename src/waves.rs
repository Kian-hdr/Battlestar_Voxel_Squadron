//! Bounded difficulty tuning, independent of rendering.
#[derive(Clone, Copy, Debug)]
pub struct WaveConfig {
    pub dive_interval: f32,
    pub fire_interval: f32,
    pub max_divers: usize,
    pub dive_duration: f32,
}
impl WaveConfig {
    pub fn for_wave(wave: u32) -> Self {
        let n = wave.saturating_sub(1).min(30) as f32;
        Self {
            dive_interval: (2.4 - n * 0.15).max(0.65),
            fire_interval: (1.05 - n * 0.06).max(0.28),
            max_divers: (2 + wave.saturating_sub(1) as usize / 3).min(4),
            dive_duration: (3.6 - n * 0.08).max(2.1),
        }
    }
}

/// Era changes at a sector boundary, never by randomly mixing ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Era {
    Classic,
    Reimagined,
}
impl Era {
    pub fn for_wave(wave: u32) -> Self {
        if wave <= 3 {
            Self::Classic
        } else {
            Self::Reimagined
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "1978 / ORIGINAL SERIES",
            Self::Reimagined => "2004 / REIMAGINED",
        }
    }
}
