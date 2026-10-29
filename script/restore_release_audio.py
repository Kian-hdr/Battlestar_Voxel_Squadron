#!/usr/bin/env python3
"""Restore private audio only inside trusted release jobs. Never prints secret bytes."""
import base64
import gzip
import hashlib
import io
import os
from pathlib import Path
import sys
import wave

FILES = [
    ('Player_Fire.wav', 'BSVX_AUDIO_PLAYER_GZIP_B64', '148d649f986de6902a9ca6cc1070af979f3aa3b4cd03178c60afe16eb191d4a8', 22050),
    ('Enemy_Fire.wav', 'BSVX_AUDIO_ENEMY_GZIP_B64', 'ad842c4c92428442a769e270babfb2c2eff687df29d6be7c13343e321589c376', 22050),
    ('Hull_Impact.wav', 'BSVX_AUDIO_IMPACT_GZIP_B64', '2d910e792bc2d3a96af377cef0de4ebf240707584b136e0fac8a300d5ef0111e', 39690),
]

def main():
    destination = Path(sys.argv[1])
    validated = []
    for name, key, digest, frames in FILES:
        encoded = os.environ.get(key, '')
        if not encoded or len(encoded) > 48000:
            raise SystemExit('Missing or oversized release audio secret: ' + key)
        try:
            data = gzip.decompress(base64.b64decode(encoded, validate=True))
        except Exception:
            raise SystemExit('Invalid release audio encoding: ' + key) from None
        if hashlib.sha256(data).hexdigest() != digest:
            raise SystemExit('Release audio checksum mismatch: ' + name)
        with wave.open(io.BytesIO(data)) as handle:
            assert (handle.getnchannels(), handle.getsampwidth(), handle.getframerate(), handle.getnframes()) == (1, 2, 44100, frames)
        validated.append((name, data))
    destination.mkdir(parents=True, exist_ok=True)
    for name, data in validated:
        (destination / name).write_bytes(data)
    print('Restored and verified three private release effects.')

if __name__ == '__main__':
    main()
