# Desktop release validation · v0.4.4

Mac build 20; compiled source `c72133fef9c2ea1a5b1c75299de3b07c961dd55c`. All 108 regression tests and release graphics Clippy pass locally; native Windows/Linux run [41201979259](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41201979259) passed 108 tests per platform, graphics checks, linking and packaging.

## Sustained comparison

Two 600-second runs used the same executable and verified 3600×2160 window. Reference mode retains sphere retirement and per-event procedural hull generation; candidate mode uses cached camera planes, rotated box retirement and immutable source hulls. Both retain all partially visible bounding geometry and have a 20-second final drain. The normal game does not activate this test mode.

| Clean-window metric | Reference | Optimized |
| --- | ---: | ---: |
| Early frame mean |18.098ms|18.176ms|
| Late frame mean |18.080ms|18.047ms|
| Early debris update CPU |16.79µs|12.37µs|
| Late debris update CPU |20.47µs|14.28µs|
| Late CPU mesh building |24.44µs|26.50µs|
| Final live debris bodies |0|0|
| Final mesh/material/entity assets |15/11/949|15/11/949|

Late debris-update CPU cost fell about 30%; total frame pacing was essentially unchanged. Do not interpret the routine-level improvement as a 30% game-FPS gain. An independent 192-request ray/contact/fracture benchmark fell 66.932→34.211ms (~49%), with identical contacts, chunk order, cells/colors, pivots and heat. Rendering mesh rebuild CPU was a small fraction of frame time; the measurements did not justify replacing the renderer or adding complex GPU geometry machinery.

The candidate had no retained offscreen box in any sampled clean window; reference mode retained up to 14. Both modes released debris, meshes and entities back to their exact starting counts after the drain. No cumulative resource leak or progressive frame collapse was reproduced in this scenario. Later-wave workload and the original symptom on other sessions/hardware remain practical limits of this test.

Native Metal traces were taken early and late. Profiler/finalization windows 230–320s and 545–610s are excluded from clean frame/CPU comparisons. Late GPU busy samples were 5.302ms reference and 5.579ms candidate; no GPU-speed improvement is claimed. GPU allocation baselines differed but remained stable within each run. Process RSS also stayed bounded; lower total memory consumption is not claimed. No thermal warning was reported, which does not independently prove constant clock speeds. The high-resolution windowed test has display/compositor pacing and is not directly comparable with earlier fullscreen/1080p FPS results.

After manual Mac unlock, the foreground v0.4.4 renderer replay passed gameplay, appearance, resize and normal Quit checks: 1,020 physical 1080p frames averaged 8.334 ms / p95 8.923 ms (about 120 FPS on this M5). Earlier locked/background results were not foreground performance evidence. No whole-game FPS increase is claimed. Prior visual/gameplay fixes remain required. Windows/Linux graphical gameplay is not covered by native build/archive checks.

## v0.4.4 release artifacts

Native Windows/Linux build and package validation: [run 41201979259](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41201979259). Windows x64 has 254 files and 253 internal checksums; Linux x64 has 255 files and 254 internal checksums. Architecture, executable modes, source notices, safe paths and incorporated audio checks pass. Graphical gameplay on Windows/Linux remains untested.

Mac build 20 is Developer ID signed, notarized, stapled and Gatekeeper-accepted. App receipt `2be4fa33-78f2-4259-9dcb-cf7d358b2729`; DMG receipt `49fa8c37-04ee-4f93-9e8a-993fbca6c315`. A read-only mounted copy matches the tested signed executable. The same verified build 20 is installed; movement, firing, pause, localized app title and retained visuals were checked after the manual unlock. DMG SHA-256: `8bc0246279fc017eb2eb2b5675acaf509c56d63a2b63b9985d851e0706df3e85`. Seven release assets include each archive, its checksum and aggregate SHA256SUMS.txt. Actual v0.4.4 screenshots were refreshed from the unlocked native replay; no AI imagery or content retouching was used.

## Historical v0.4.3 validation

# Desktop release validation · v0.4.3

Mac build 19, verified 31-Oct-2026. All 100 pure regression tests, formatting and release graphics Clippy pass locally. Tests reproduce the former depth misses and life-loss X reset, compare projected occupied-voxel collision against an independent convex-hull reference, and check both complete Galactica hulls across desktop, Retina, tall and ultrawide views.

Rendering uses orthographic projection with angled voxel models. Cosmetic depth cannot shift drawn XY. Actual renderer audits check enemy/shot alignment, projectile ownership/color, leading-face contact alignment and continuous player visibility. The full native replay verifies nonfatal position continuity, both eras, exhaust, debris collision/cooling/subdivision, pause/restart, resizing and normal Quit Game shutdown. Seven native viewport/era checks confirm that the complete Galactica stays inside the view; its right-side position and apparent size match the earlier composition.

Exhaust consists of separate fixed-size glowing cubes, a steady nozzle core and irregular flowing streams. The topology regression rejects the former connected skin. Debris launches faster once and remains ballistic while unobstructed. Across 104,949 native unobstructed steps, velocity/course changes and path error were zero; all 257 split samples inherited translation without impulse. Peak speed was 36.000; there were no significant camera-approach steps. Maximum collision-skin correction was 0.005633 world units, below the 0.01 diagnostic tolerance. Genuine hull contacts may deflect fragments. Opaque geometry darkens into spatial shadow and retires only outside the viewport.

At physical 1920×1080, 1,020 frames averaged 8.334 ms / p95 8.707 ms (about 120 FPS) on Apple M5. The complete 5,168-frame scenario averaged 8.37 ms / p95 8.74 ms. These are bounded hardware/workload measurements. Compiled source is `c0e7d554d2e0b40d42c710ecdf91c216c58d74e2`.

The macOS bundle keeps its portable underscore filename and uses the localized display title “Battlestar: Voxel Squadron.” The final app and DMG are Developer ID signed, notarized, stapled and Gatekeeper-accepted. The installed app came from a read-only DMG mount and matches the native-tested signed executable. Windows/Linux native packages and independent archive checks pass. Each native runner passed all 100 tests and graphics Clippy before linking and packaging. Graphical Windows/Linux gameplay remains untested. Existing incorporated-audio, licensing and fan-rights boundaries remain applicable.

## v0.4.3 Mac package identity

App notarization receipt: `1c57cef4-efec-4d6f-92a4-bcf430cd8e88`. DMG receipt: `71641aee-413d-462b-afbc-d555115141fd`. Final DMG SHA-256: `e36f8e6f1bcb02bc94be4ecb7849f9c79013267313c38be018ca096669383ae5`. The installed build 19 passed movement, firing, pause and visual framing checks and retains the proper localized application name. Signed executable SHA-256: `7eb9c3025ff83186bf795a1518486bc0f52399863430885f9e992cad05d8085b`.

## v0.4.3 native package checks

[Native Windows/Linux run](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41198148699) and [source checks](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41198148915) succeeded for the compiled source above.

- Windows x64: PE32+ GUI executable, 254 files and 253 internal checksums; safe paths, source notices and approved embedded audio verified. Ordinary Git text files contain only expected CRLF conversion; byte-preserved notices remain exact. Windows is unsigned.
- Linux x64: ELF64 native executable, 255 files and 254 internal checksums; executable/launcher modes, safe paths, source-pinned notices and approved embedded audio pass all 17 checks.
- Mac Apple Silicon: final signed/notarized DMG and mounted/copied installed app pass the checks above.

Three platform archives, their individual checksum files and aggregate `SHA256SUMS.txt` form seven release assets. Final documentation/screenshots are the only changes after the compiled source. Screenshots are actual final game captures resized to at most 1920 pixels; original captures are retained. No AI raster generation or content retouching was used. Windows/Linux graphical gameplay remains untested; native builds and archive checks are distinct from target-device gameplay.

## Historical v0.4.2 validation

# Desktop release validation · v0.4.2

Mac build 15, verified 31-Oct-2026. All 86 pure tests, formatting and release graphics Clippy pass locally. Native both-era/death/restart/pause/resize checks pass, with normal shutdown through the visible Quit Game menu interaction.

The final replay checked 29,385 actual projectile mesh/color associations (including hit frames), 17,502 exhaust mesh updates with changing taper/length, and 424,384 debris movement steps. No significant camera-approach step was observed; the maximum backward correction was 0.004834 world units from collision skin, below the 0.01 diagnostic tolerance. Peak debris speed remained 36.000 after regression coverage fixed near-frontal amplification. Geometry size/opacity and viewport-only retirement are preserved.

At physical 1920×1080, 1,021 frames averaged 8.333 ms / p95 8.775 ms (about 120 FPS) on Apple M5. The complete replay averaged 8.37 ms / p95 8.84 ms. These are bounded native measurements, not universal hardware guarantees. Target acquisition and damage remain deterministic and material-accurate; player horizontal aiming is unchanged.

Native Windows/Linux package jobs and final artifact receipts are complete below. Graphical gameplay on those systems remains untested. Existing licensing, private incorporated-audio and fan-rights boundaries below remain applicable.

## v0.4.2 package identity and completed checks

Compiled code: `26ba773a796d72655e9cde0f37db00eea81575df`. [Native packages](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41149829188) and [source checks](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41149829922) succeeded. Actual native job logs record 86 passing tests on Windows and Linux, plus successful graphics Clippy, release linking and packaging.

- **Mac Apple Silicon:** final app and DMG signed, notarized, stapled and Gatekeeper-accepted; the DMG was mounted read-only and its copied app/executable hash verified against the native tested build. App receipt `1390005a-1dcc-4c73-89c2-c7df17afd0ab`; DMG `c0a39094-d80b-486d-bad2-122ab5d89115`.
- **Windows x64:** native PE32+ GUI executable, 254-file roster, 253 internal checksums, approved embedded effects, safe paths and source notices verified. Ordinary text files use expected Windows CRLF checkout conversion; byte-preserved notices remain exact. Unsigned; graphical gameplay not tested.
- **Linux x64:** native ELF64 PIE, executable/launcher permissions, 255-file roster, 254 internal checksums, source notices and approved embedded effects verified. All 15 archive checks pass. Graphical gameplay not tested.

Three platform packages, individual checksum files and aggregate SHA256SUMS.txt form seven release assets; their uploaded digests and sizes match final local bytes. Final documentation/screenshots are the only differences from compiled source. Screenshots are actual game captures, resized to at most 1920 pixels for documentation; raw captures are retained. No AI raster generation or content retouching was used. Previous releases and private audio boundaries remain intact.

## Historical v0.4.1 validation

# Desktop release validation · v0.4.1

macOS bundle build 14. Verified 30-Oct-2026 on Apple M5 / macOS 27.2. Native Windows/Linux package jobs and final notarization receipts are complete and recorded below.

- All 72 regression tests pass, including 2,000 exact collision comparisons against the previous algorithm, viewport/shadow/infinite-camera tests and exhaust/hull separation. Formatting and all-target release Clippy pass.
- Full silent native scenario passes both eras, debris contacts/cooling/subdivision, pause, restart, death, resize and normal runner shutdown. Player visibility is checked on every gameplay frame, including respawn protection.
- Debris retires only when its full bounding sphere leaves the actual infinite camera view. Natural outward/background drift clears tall and wide views without a lifetime or oldest-effect eviction. The scenario retained pieces beyond the old 1.05-second timeout, retired 2,059 pieces offscreen and peaked at 2,128 cells across nine concurrent events. Spatial shadow retains opaque, legible geometry. Restart explicitly clears the old run.
- With emissive exhaust/projectiles, restrained HDR bloom and red impact lighting enabled, 992 physical 1920×1080 frames average 8.577 ms, p95 10.440 ms (about 117 FPS). The complete 6,143-frame scenario averages 8.47 ms, p95 8.95 ms. Results are specific to this Mac and scripted workload.
- A CPU microbenchmark of 1,024 identical short Galactica collision sweeps improved 72.06 ms to 3.21 ms (22.44×); this is not a whole-game FPS multiplier.

The old build reproduced severe stalls in a separate scripted sample (56.934 ms mean / 203.704 ms p95). Focus/workload and timing-instrumentation differences limit a direct causal FPS ratio: the new build retains more debris and records uncapped real time, while the old telemetry used capped virtual time. The collision benchmark compares identical inputs with exact output equivalence. The final native test includes HDR lighting cost.

License inventory covers 389 pinned packages with zero missing notices; historical provenance limits below remain. Native Windows/Linux graphical gameplay and other-device performance remain untested. Owner aesthetic acceptance is separate from these checks. macOS normal installed movement/fire, pause/resume, mute, fullscreen and Command+Q were also checked; the final correction changes debris projection/drift only.

## v0.4.1 package and source identity

Compiled game source: `267d2ecb98b0e518ba389405578e4d77f9e46f44`. [Native package run](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41113758169) completed successfully: the original successful Linux job was reused, and only the Windows job canceled for the owner's pause was retried. [Source checks](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/41113757168) pass. Actual native job logs record 72 passing tests on each OS, plus successful graphics Clippy, release link and packaging.

| Package | Completed checks | Runtime boundary |
| --- | --- | --- |
| macOS Apple Silicon, build 14 | Developer ID, hardened runtime, timestamp, app and DMG Accepted notarization, stapling, Gatekeeper, final-DMG install and binary hash; native controls and complete renderer scenario | Tested on Apple M5 / macOS 27.2 |
| Windows x64 | Native MSVC build; PE32+ x64 GUI; 254-file roster, 253 internal checksums, source notices and approved embedded sound mix; external checksum and safe paths | Unsigned; graphical gameplay not tested |
| Linux x64 | Native Ubuntu 22.04 build; ELF64 x64, executable/launcher permissions, 255-file roster, 254 internal checksums, source notices and approved embedded sound mix; external checksum and safe paths | Graphical gameplay not tested; needs compatible desktop graphics/audio libraries |

Mac app notarization: `cc7773f0-1afd-4d54-a784-0a5dd563fa65`; DMG: `3a1299f1-46a2-404a-86e9-9bb826b3d08f`. Both final artifacts are Accepted and stapled. The installed copy came from the final DMG and matches its verified executable hash.

Windows ordinary Git text files use CRLF checkout line endings; their canonical source content matches. Notices explicitly marked for byte preservation remain exact. All internal checksums identify actual packaged bytes. Neither native archive includes loose private audio, credentials or private histories. The selected sound mix is verified as embedded in each executable.

The release tag also contains final documentation and seven refreshed actual game screenshots. Gameplay/rendering/build inputs are unchanged from the compiled source above. Documentation images are resized to at most 1920 pixels on their longest edge; original native captures remain retained as validation evidence. No AI raster generation or content retouching was used.

Three platform packages, individual checksum files and aggregate `SHA256SUMS.txt` are provided as seven release assets. Their SHA-256 values identify the final downloadable bytes. Native Windows/Linux graphical qualification remains a separate follow-up; build and archive checks do not establish target GPU acceptance.

## Historical v0.4.0 validation

# Desktop release validation · v0.4.0

Source commit: `16e1f28e52689f0d0cacaf3e3a95a32b1beec001`, tag `v0.4.0`. macOS bundle build 13. This document records checks, not owner aesthetic approval or universal device compatibility.

| Platform | Build and automated tests | Runtime checks |
| --- | --- | --- |
| macOS, Apple Silicon | Release build, formatting, graphics Clippy and 65 tests pass | Full silent renderer scenario and installed-copy native controls pass on an Apple M5 Mac |
| Linux, x64 | Native Ubuntu 22.04 release link, graphics Clippy and 65 tests pass; ELF/archive/checksums/notices verified | Graphical gameplay not tested |
| Windows, x64 | Native Windows 2022 MSVC release link, graphics Clippy and 65 tests pass; PE x64 GUI/archive/checksums/notices verified | Graphical gameplay not tested |

A pristine public source archive passes all-target compilation and 65 tests using only the public fallback audio. No provider credentials are required. [Checks](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/40894317695) and [native packages](https://github.com/Kian-hdr/Battlestar_Voxel_Squadron/actions/runs/40894306994) retain runner evidence.

## Mac package

Developer ID Application signing, Hardened Runtime and secure timestamp verified. App notarization `d42bc3c9-afb8-40f1-b6d7-67a169b04d41` and DMG notarization `d70080f4-fbe8-4b06-863e-01beb0f6cfb8` were Accepted. Both tickets are stapled and validated. Gatekeeper accepts the final DMG and copied installed app. The actual DMG was mounted, the app copied to Applications, the image detached, and the installed app launched.

The full scenario checks three cleared waves and the modern era transition, connected debris/cooling/contacts, pause/resume, mute, resizing, death/restart and normal shutdown. The final committed build captured 1,021 physical 1920×1080 frames: 8.342 ms mean, 10.370 ms p95, about 120 FPS on this Mac under the observed conditions. Two earlier slow runs did not reproduce after foreground/host conditions changed; exact focus/memory contributions remain unproven. These results do not guarantee performance on other hardware or under heavy system load.

Native installed-copy input checks confirmed Enter, held movement and firing, pause/resume, mute/unmute, F11 windowed switching and Command+Q. Automated runs are silent. Subjective sound preference remains an owner review.

## Contents and boundaries

Original source is MIT. Fonts, fallback audio, generated incorporated effects, dependency licenses and franchise rights remain separate. All packages retain notices; source excludes isolated generated effects, credentials and private histories. The dependency inventory conservatively covers 388 packages and records five explicit standard-license supplements for upstream omissions.

This is a free, unofficial fan prototype. No franchise-owner permission, legal immunity, App Store review, Intel-Mac build, ARM Windows/Linux package or Windows/Linux gameplay acceptance is claimed. Windows is unsigned. Linux uses system graphics/audio libraries and requires a graphical session and compatible Vulkan driver. Release checksums identify final downloadable bytes.

## Published package integrity

The Windows archive was independently inspected after download from the successful native run: x64 GUI PE header, 253 internal file checksums, required notices, all three selected embedded electronic effects, safe archive paths and no loose private audio/credential files. Linux archive checks cover its x64 ELF header, executable mode, internal manifest/checksums/notices and embedded sound mix. Windows and Linux binaries have not been graphically played on target machines.

Final package SHA-256 values are distributed in `SHA256SUMS.txt` alongside the downloads. Public download bytes are checked against these values after publication. The source tag identifies the compiled source; the main branch also includes this final documentation update.
