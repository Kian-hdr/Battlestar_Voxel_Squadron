use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=BSVX_PRIVATE_AUDIO_DIR");
    let source = env::var_os("BSVX_PRIVATE_AUDIO_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("assets/Audio/Public"));
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    for name in ["Player_Fire.wav", "Enemy_Fire.wav", "Hull_Impact.wav"] {
        let path = source.join(name);
        println!("cargo:rerun-if-changed={}", path.display());
        let bytes = fs::read(&path)
            .expect("selected audio file must exist; private selection never silently falls back");
        assert!(
            bytes.len() > 44 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
            "selected audio must be WAV"
        );
        fs::write(output.join(name), bytes).expect("copy selected audio into Cargo build output");
    }
}
