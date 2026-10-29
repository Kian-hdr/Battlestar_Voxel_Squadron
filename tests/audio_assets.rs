fn pcm(bytes: &[u8]) -> (u16, u32, u16, &[u8]) {
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut at = 12;
    let mut format = None;
    let mut data = None;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let end = at + 8 + size;
        assert!(end <= bytes.len());
        if &bytes[at..at + 4] == b"fmt " {
            let f = &bytes[at + 8..end];
            assert_eq!(u16::from_le_bytes(f[0..2].try_into().unwrap()), 1);
            format = Some((
                u16::from_le_bytes(f[2..4].try_into().unwrap()),
                u32::from_le_bytes(f[4..8].try_into().unwrap()),
                u16::from_le_bytes(f[14..16].try_into().unwrap()),
            ));
        }
        if &bytes[at..at + 4] == b"data" {
            data = Some(&bytes[at + 8..end]);
        }
        at = end + (size % 2);
    }
    let (channels, rate, bits) = format.unwrap();
    (channels, rate, bits, data.unwrap())
}
#[test]
fn bundled_electronic_effects_are_playable_bounded_pcm_with_clean_edits() {
    for (bytes, seconds) in [
        (
            include_bytes!(concat!(env!("OUT_DIR"), "/Player_Fire.wav")).as_slice(),
            0.50_f32,
        ),
        (
            include_bytes!(concat!(env!("OUT_DIR"), "/Enemy_Fire.wav")).as_slice(),
            0.50,
        ),
        (
            include_bytes!(concat!(env!("OUT_DIR"), "/Hull_Impact.wav")).as_slice(),
            0.90,
        ),
    ] {
        let (channels, rate, bits, data) = pcm(bytes);
        assert_eq!((channels, rate, bits), (1, 44100, 16));
        let samples: Vec<_> = data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| i16::from_le_bytes(*p))
            .collect();
        assert!((samples.len() as f32 / rate as f32 - seconds).abs() < 0.001);
        assert_eq!(samples[0], 0);
        assert_eq!(*samples.last().unwrap(), 0);
        let peak = samples.iter().map(|v| (*v as i32).abs()).max().unwrap();
        assert!(peak > 100 && peak < 4000, "headroom and non-silent sample");
    }
}
#[test]
fn bundled_audio_preserves_provider_and_legacy_license_boundaries() {
    let attribution = include_str!("../assets/Audio/Public/NOTICE.txt");
    assert!(attribution.contains("ElevenLabs"));
    assert!(attribution.contains("https://elevenlabs.io/creative-terms"));
    assert!(attribution.contains("Michel Baradari"));
    assert!(attribution.contains("https://creativecommons.org/licenses/by/3.0/"));
}
