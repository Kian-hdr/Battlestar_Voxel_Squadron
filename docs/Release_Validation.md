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
