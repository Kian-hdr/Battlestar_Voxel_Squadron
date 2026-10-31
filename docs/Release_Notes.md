# v0.4.4 · More efficient debris cleanup

- Retire debris using tighter rotated geometry bounds for the current viewport, retaining partially visible pieces.
- Skip render-resource creation for fragments already fully outside the view.
- Reuse immutable source hulls during destruction instead of regenerating each ship model.
- Preserve momentum, opaque shadowed fragments, voxel exhaust, strict 2D hits, stable Viper position, Galactica framing and the proper app display name.

See [release validation](Release_Validation.md) for sustained measurements and platform limits.

# v0.4.3 · Strict 2D hits, stable Viper and voxel effects

- Restored the Galactica’s full right-side framing and earlier apparent size; its complete hull stays visible as the window resizes.

- Exhaust is now made from separate, fixed-size glowing cubes, with a steady voxel core and an irregular tapered stream. No smooth cone skin or static rods.
- Debris launches faster with impact and hull momentum, then follows a straight constant-velocity path while unobstructed. Removed continuous background steering and delayed subdivision kicks.
- Split fragments inherit parent translation and spin. Genuine hull contacts keep bounded deflections/escape slides; visible geometry still retires only outside the viewport and darkens in deep-space shadow.
- Shots use exact projected occupied voxel silhouettes independently of cosmetic depth. Rendering and the visible beam tip match the same 2D hit plane.
- Nonfatal damage preserves the Viper position, momentum and rotation instead of resetting it to the center.
- macOS shows “Battlestar: Voxel Squadron” as its localized app name while retaining the portable underscore filename.
- Projectile colors, menu Quit Game and the existing audio mix are retained.

See [release validation](Release_Validation.md) for actual measurements and platform coverage.

# v0.4.2 · Combat alignment and natural exhaust/debris

- Fixed depth targeting when enemies enter the firing lane after a shot has launched; stale locks are released and replaced. Horizontal aiming remains player-controlled, and exact occupied-voxel contact still determines hits.
- Removed the red projectile-shaped impact effect. Player shots retain their color; existing heated hull fragments keep their localized glow.
- Replaced rigid exhaust rods with continuously deforming, tapered luminous plumes. Nozzle faces stay fixed behind the hull.
- Debris now recedes camera-away and downward into deep-space shadow. Contact responses are bounded, including near-frontal moving hulls. Visible pieces remain opaque until outside the actual viewport.
- Added Quit Game to the shared title, pause and game-over menu, using normal application shutdown.

See [release validation](Release_Validation.md) for actual tests and platform coverage.

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
