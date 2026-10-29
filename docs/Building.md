# Building desktop packages

See the repository README for toolchain and platform prerequisites. All commands run from the source root. No service is needed at runtime.

- macOS: `./script/package_mac.sh`. Local source build selects redistributable audio and ad-hoc signing. For a public release, explicitly set `BSVX_PRIVATE_AUDIO_DIR` to licensed release inputs and `BSVX_SIGN_IDENTITY` to your Developer ID Application identity, then package. Notarize the app with `script/notarize_mac.py`, package with `script/package_dmg.sh`, notarize the DMG and verify its staple/Gatekeeper results. The notarization helper can use an existing `--profile` or the signing host's private Apple registry. No credentials are part of this repository.
- Windows: `./script/package_windows.ps1`, using native MSVC Rust/C++ tooling. The result is a ZIP containing the executable and notices. It is unsigned.
- Linux: `./script/package_linux.sh`, using native GNU Rust/linker and the listed ALSA/X11/Wayland libraries. The result is a tar.gz containing the executable, launcher and notices. It uses system graphics/audio libraries.

The manually dispatched Native release packages workflow builds Windows and Linux on native runners. Official audio inputs are exposed only to maintainer-dispatched trusted release jobs and embedded into the executable. PR checks use public fallback audio and no release secrets. Packaging does not publish a GitHub Release.

Run `cargo test --locked --no-default-features`, `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo build --release --locked`. Run `--smoke-test` in a graphical session for silent scripted rendering checks; `BSVX_EVIDENCE_DIR` selects its screenshot/result directory. Normal gameplay is not modified by that optional harness.

Before publishing, preserve the license directory and separate audio/font notices, identify the source commit, test the actual package, record platform testing limits, and checksum the final bytes. A compiler check does not establish gameplay or OS compatibility on every device.
