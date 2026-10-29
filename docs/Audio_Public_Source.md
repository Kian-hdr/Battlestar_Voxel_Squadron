# Public source and incorporated release audio

Public builds use the three redistributable fallback WAV files in `assets/Audio/Public/`. They are filtered, faded, mono PCM derivatives of Michel Baradari's CC BY 3.0 samples. Retain `Public/NOTICE.txt` and the original source notices. The fallback audio has a different character from official generated electronic effects; the 20% default and silence/mute controls are shared.

Cargo's `build.rs` selects the public fallback unless `BSVX_PRIVATE_AUDIO_DIR` explicitly names a directory containing `Player_Fire.wav`, `Enemy_Fire.wav` and `Hull_Impact.wav`. It copies that selection into Cargo's OUT_DIR and Rust embeds the bytes in the executable. A missing explicitly selected private file fails the build instead of silently using fallback audio. Public source requires no account, network, secret or proprietary media. Both selections have lengths 0.5/0.5/0.9 seconds, mono 44.1 kHz 16-bit PCM with conservative headroom.

Official incorporated-game binaries may use the private generated electronic effects for the current free, noncommercial release subject to the generating account's agreement. [ElevenCreative Terms, Exhibit 4](https://elevenlabs.io/creative-terms), checked 29-Oct-2026, permits incorporation into games subject to the underlying agreement and plan, and restricts making isolated output/audio samples available. The public code's MIT license does not cover these private provider bytes. Do not publish Generated/, Processed/, private generation receipts or private preparation history as source artifacts. Do not upload restored WAV files or secret packs as workflow artifacts; publish only incorporated game executables and their required notices. Credit: Electronic effects generated with ElevenLabs · elevenlabs.io. Any account-specific publication requirements remain applicable.

## Trusted release job contract

Three encrypted GitHub repository secrets contain base64 of gzip-compressed individual WAV bytes, without whitespace or JSON:

| Secret | Restored file |
| --- | --- |
| `BSVX_AUDIO_PLAYER_GZIP_B64` | `Player_Fire.wav` |
| `BSVX_AUDIO_ENEMY_GZIP_B64` | `Enemy_Fire.wav` |
| `BSVX_AUDIO_IMPACT_GZIP_B64` | `Hull_Impact.wav` |

Expose them only to trusted owner-controlled release jobs. Run `python3 script/restore_release_audio.py DEST_DIR` with those three environment variables. The script validates pinned SHA-256 and PCM metadata before writing all three files; it prints no secret material. Set `BSVX_PRIVATE_AUDIO_DIR` to that directory for the subsequent Cargo build/test commands. Keep the directory outside the repository, preferably in the runner's temporary directory, and remove it after the job. Fork/PR builds use the public fallback and receive no secrets. Neither source archives nor release artifacts should include private intermediate directories. Hash pins require an intentional script/secret update when the owner changes the official mix.

Both source selection and official private selection require the audio format regression test. Automated gameplay remains silent through `--smoke-test` or `BSVX_SILENT_CHECK`; a compilation/test pass is not subjective audio acceptance. The owner publishes only the incorporated game binaries.
