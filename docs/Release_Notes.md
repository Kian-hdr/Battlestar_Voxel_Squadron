# v0.4.1 · Performance and visual continuity

- Faster debris collision searches preserve the exact occupied-voxel contact rules.
- HUD, fonts and camera settings update only when their values change, avoiding repeated layout work during combat.
- The Viper stays visible during startup and respawn protection. Protection itself is unchanged.
- Connected debris remains until its complete bounding volume leaves the camera view. New destruction no longer deletes older visible pieces.
- Debris gently recedes into a soft, cool spatial shadow, while remaining opaque. Pause freezes it, and restart clears the previous run.
- Viper exhaust is anchored behind its nozzles, with no oscillation through engine voxels.
- Restrained emissive exhaust/projectile glow and brief, local red enemy-impact lighting. A single low-resolution bloom chain supplies the glow; projectiles do not spawn point lights.
- Camera-edge checks follow the current window aspect ratio, including resizing.

See [release validation](Release_Validation.md) for measured scope and platform coverage. The macOS package remains Apple Silicon; Windows and Linux packages remain x64.

# v0.4.0 · Public playable preview

Battlestar: Voxel Squadron is a native Rust voxel arcade fan game by Kian Konrad Tajbakhsh.

- Classic 1978 fleet for sectors 1–3, reimagined 2004 fleet from sector 4.
- Symmetric voxel ships, connected debris, localized heating/cooling and momentum-preserving debris contacts.
- Smooth enemy avoidance and recovery from crowded flight paths. Depth-aware shots preserve horizontal arcade aiming.
- Larger borderless playfield, sparse HUD, pixel fonts and a Colonial-colored menu.
- Quiet 20% default effects volume, menu volume/mute and a deeper electronic sound mix.
- Native Retina display resolution and synchronized presentation for high-refresh displays.

## Packages and qualification

The macOS package targets Apple Silicon/macOS 12+. Windows x64 and Linux x64 packages are built on native GitHub runners. Refer to the release's validation notes for completed checks; a platform build is not evidence of human gameplay testing on that platform. Windows binaries are unsigned.

Public source uses CC BY 3 fallback effects so anyone can compile and modify it without provider credentials. Official binaries incorporate the selected generated electronic mix; these audio bytes are separately governed and are not a standalone sample pack.

This is an unofficial, noncommercial fan prototype, without franchise-owner endorsement or permission. Original code is MIT; that license does not grant third-party franchise or asset rights. No store submission, multiplayer, campaign, upgrades or persistent high scores are included.
