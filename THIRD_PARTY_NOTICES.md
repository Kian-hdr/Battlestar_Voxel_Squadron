# Third-party notices

## Project software

Created by **Kian Konrad Tajbakhsh**. Original software and documentation authored for this project are offered under the [MIT License](LICENSE), only to the extent the author owns the relevant rights. Retain its copyright and permission notice when copying or modifying substantial portions.

## Fan theme and names

Battlestar: Voxel Squadron (formerly Battlestar Voxel) is an unofficial, unaffiliated, noncommercial fan prototype. Battlestar Galactica, Viper, Cylon, Galactica and associated fictional ship designs and other franchise elements remain the property of their respective rights holders. GALAGA is a Bandai Namco Entertainment mark; its name identifies the arcade inspiration. This project is not endorsed, sponsored or licensed by NBCUniversal, Bandai Namco or the relevant rights holders. Rights-holder permission has **not** been obtained.

The MIT license does not grant rights to third-party names, marks, fictional designs or other protected material. A procedural recreation can still implicate those rights. No TV footage, extracted show models, franchise soundtrack, voices or external textures are imported into the game; ships are generated from source. Audio has the separate licenses described below. Reference materials used for design comparison are not licensed for redistribution by this notice.

“Noncommercial” describes this fan prototype's purpose. It adds no noncommercial restriction to MIT-licensed original code, whose license permits commercial use. It also supplies no franchise clearance. A disclaimer does not prevent claims or establish fair use. See the [rights review](docs/Rights_Review.md).

## Software dependencies

Bevy 0.17.3 is offered under **MIT OR Apache-2.0**, as recorded in the installed crate's Cargo.toml and accompanying LICENSE-MIT and LICENSE-APACHE. See [Bevy's source](https://github.com/bevyengine/bevy/tree/v0.17.3). Bevy and transitive dependencies retain their own copyrights and licenses; the project MIT license does not replace them.

glam 0.30.10 is used directly for shared simulation/rendering vector and quaternion mathematics, under **MIT OR Apache-2.0**, as recorded in its installed Cargo.toml and license files. Source: https://github.com/bitshifter/glam-rs. It was already a Bevy dependency.

The complete [dependency-notice directory](docs/Dependency_Licenses/README.md) must accompany release archives. Inventory.json records 388 resolved registry packages across the Apple Silicon macOS, Windows MSVC x64 and Linux GNU x64 graphics/build dependency graphs, package versions, declared licenses, original-text origins and SHA-256 hashes. This is a conservative union including build tools, not exact binary reachability. It includes Bevy's bundled default-font OFL and Unicode notices in addition to Rust license texts.

Five released crates omit upstream license texts: block 0.1.6, dispatch 0.2.0, malloc_buf 0.0.6, constgebra 0.1.4 and hexf-parse 0.2.1. Their entries distinguish standard SPDX text supplements and original Cargo authors/license declarations from upstream-supplied notices; no historical copyright year is invented. All declared dependency expressions offer permissive licensing paths; retain both terms for AND expressions. Copying all notice alternatives does not elect GPL/LGPL where a permissive OR alternative is available. System-library or separately bundled runtime notices remain target-specific.

## Public source audio

The three public fallback WAV files in assets/Audio/Public are modified samples by **Michel Baradari** (apollo-music.de), under **Creative Commons Attribution 3.0 Unported (CC BY 3.0)**. Sources: [Chaingun, pistol, rifle, shotgun shots](https://opengameart.org/content/chaingun-pistol-rifle-shotgun-shots) and [2 High Quality Explosions](https://opengameart.org/content/2-high-quality-explosions). [License](https://creativecommons.org/licenses/by/3.0/), [legal code](https://creativecommons.org/licenses/by/3.0/legalcode).

Modifications: trimmed, pitch-adjusted, filtered, layered, mixed to mono, faded, peak-scaled and PCM-encoded by Kian Konrad Tajbakhsh/Battlestar Voxel. Original and modified samples retain CC BY 3.0, separate from MIT code. No endorsement is implied. Retain Public/NOTICE.txt, the original source notices and assets/Audio/Licenses/Attribution.txt. Public source builds select these fallback samples without an account or proprietary media; see [audio source selection](docs/Audio_Public_Source.md).

## Incorporated generated electronic effects

Official noncommercial game binaries may incorporate ElevenLabs Sound Effects v2 output under the generating account's underlying agreement and the [ElevenCreative Terms, Exhibit 4 section 2](https://elevenlabs.io/creative-terms), checked 29-Oct-2026. These permit incorporation in games subject to the underlying agreement; commercial use requires an appropriate plan. Commercial entitlement is unverified and is not claimed for this free noncommercial release.

Generated audio is **not MIT or CC BY**. Do not redistribute raw takes or processed WAV files separately, offer them as isolated samples, or include them in a public source/sound library. Public source excludes Generated/, Processed/, private generation receipts and audio preparation history. Trusted release jobs restore the private effects temporarily and embed them in the game executable; downloadable source uses the CC BY fallback. Retain the account-specific terms/attribution requirements where applicable. Credit: **Electronic effects generated with ElevenLabs · elevenlabs.io**. Credit supplies no additional permission. Soundtrack licensing requires its own provider/model review; no music rights are granted by this SFX notice.

## Fonts

The pixel title uses original **Press Start 2P**, by CodeMan38 / the Press Start 2P Project Authors, under **SIL Open Font License 1.1**. The other visible UI uses original **Silkscreen Regular**, by the Silkscreen Project Authors, also under **SIL OFL 1.1**. Original bytes, source/hash notes and full license texts are retained in assets/Fonts. Release bundles retain PressStart2P_OFL.txt and Silkscreen_OFL.txt. These font licenses remain separate from MIT code. Provider branding need not appear in game controls for these written notices to remain available.
