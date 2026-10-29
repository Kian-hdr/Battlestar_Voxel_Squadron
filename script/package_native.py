#!/usr/bin/env python3
"""Package an existing native desktop binary; never copy private audio source files."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tarfile
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', choices=('windows', 'linux'), required=True)
    parser.add_argument('--target', required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    header = binary.read_bytes()[:4096]
    if args.platform == 'windows':
        pe_offset = int.from_bytes(header[60:64], 'little') if header[:2] == b'MZ' else -1
        if pe_offset < 0 or header[pe_offset:pe_offset + 4] != b'PE\0\0' or int.from_bytes(header[pe_offset + 4:pe_offset + 6], 'little') != 0x8664:
            raise SystemExit('Expected a native Windows PE x86_64 executable.')
    elif header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != 62:
        raise SystemExit('Expected a native Linux ELF x86_64 executable.')
    version = re.search(r'^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"',
                        (ROOT / 'Cargo.toml').read_text(), re.MULTILINE).group(1)
    platform = 'Windows' if args.platform == 'windows' else 'Linux'
    stem = f'Battlestar_Voxel_Squadron_v{version}_{platform}_x86_64'
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    extension = '.zip' if args.platform == 'windows' else '.tar.gz'
    archive = output / (stem + extension)
    audio = 'official embedded pack' if os.environ.get('BSVX_PRIVATE_AUDIO_DIR') else 'public CC BY 3.0 fallback'
    with tempfile.TemporaryDirectory(prefix='bsvx_package_') as temporary:
        bundle = Path(temporary) / stem
        bundle.mkdir()
        executable = bundle / ('Battlestar_Voxel_Squadron.exe' if args.platform == 'windows'
                               else 'Battlestar_Voxel_Squadron')
        shutil.copy2(binary, executable)
        if args.platform == 'linux':
            executable.chmod(0o755)
            launcher = bundle / 'Launch.sh'
            launcher.write_text('#!/bin/sh\nset -eu\ncd "$(dirname "$0")"\nexec ./Battlestar_Voxel_Squadron "$@"\n')
            launcher.chmod(0o755)
        notices = bundle / 'Notices'
        notices.mkdir()
        for filename in ('LICENSE', 'THIRD_PARTY_NOTICES.md'):
            shutil.copy2(ROOT / filename, bundle / filename)
        for filename in ('PressStart2P_OFL.txt', 'Silkscreen_OFL.txt'):
            shutil.copy2(ROOT / 'assets/Fonts' / filename, notices / filename)
        shutil.copytree(ROOT / 'assets/Audio/Licenses', notices / 'Audio')
        # This is an explicit public-notice allowlist, never the whole assets/ or docs/ tree.
        for filename in ('Rights_Review.md', 'Audio_Public_Source.md'):
            shutil.copy2(ROOT / 'docs' / filename, notices / filename)
        shutil.copytree(ROOT / 'docs/Dependency_Licenses', notices / 'Dependencies')
        shutil.copy2(ROOT / 'Cargo.lock', notices / 'Cargo.lock')
        controls = ('Enter: launch/retry; A/D or arrows: move; hold Space: fire; '
                    'Escape: pause/resume; M: mute; F11: fullscreen/windowed.\n'
                    'Menu settings control volume. Focus loss pauses combat.\n')
        if args.platform == 'windows':
            runtime = '''WINDOWS x86_64\n\nExtract this entire ZIP, then open Battlestar_Voxel_Squadron.exe.\nUse a current Windows10/11 x64 desktop with a working GPU driver supporting\nDirectX12 or Vulkan. No Rust installation, browser, account, or network\nconnection is required to play. This executable is unsigned.\n\nIf Windows reports a missing VCRUNTIME/MSVCP DLL, install Microsoft's current\nVisual C++2015-2022 x64 Redistributable:\nhttps://aka.ms/vs/17/release/vc_redist.x64.exe\n\n'''
        else:
            runtime = '''LINUX x86_64\n\nExtract this entire archive, then run ./Launch.sh from its folder.\nBuilt on Ubuntu22.04 x86_64; use Ubuntu22.04 or a compatible newer GNU/Linux\ndesktop. A graphical X11/Wayland session and a working Vulkan GPU driver are\nrequired. This is not a headless/server or ARM package. No Rust installation,\nbrowser, account, or network connection is required to play.\n\nOn Ubuntu/Debian, install runtime packages if absent:\nsudo apt install libasound2 libx11-6 libxi6 libxrandr2 libxcursor1 libxkbcommon0 libwayland-client0 libwayland-cursor0 libwayland-egl1 libvulkan1 mesa-vulkan-drivers\n\nThe executable intentionally uses system GPU, window, and audio libraries; it\nis not statically linked and does not bundle a GPU driver.\n\n'''
        runtime += ('CONTROLS\n' + controls + '\nPACKAGE\n'
                    f'Version: {version}\nTarget: {args.target}\nAudio: {audio}\n\n'
                    'Native CI builds and unit tests do not establish target GPU, keyboard,\n'
                    'controller, speaker, or display acceptance. See the published release\n'
                    'validation for the exact checks that ran.\n\n'
                    'This is an unofficial fan game. Original code is MIT; franchise rights,\n'
                    'font licenses, dependency notices and audio terms remain separate.\n'
                    'Keep LICENSE, THIRD_PARTY_NOTICES.md and Notices/ with the executable.\n'
                    'No raw private generated audio is included as separate files.\n')
        (bundle / 'README_RUNTIME.txt').write_text(runtime)
        manifest = {'schema': 1, 'version': version, 'target': args.target,
                    'platform': args.platform, 'audio': audio,
                    'binary': executable.name, 'binary_sha256': digest(executable),
                    'checks': 'Built/tested status is recorded by the associated CI run.'}
        (bundle / 'Release_Manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        sums = ''.join(f'{digest(path)}  {path.relative_to(bundle).as_posix()}\n'
                       for path in sorted(bundle.rglob('*')) if path.is_file())
        (bundle / 'SHA256SUMS.txt').write_text(sums)
        if args.platform == 'windows':
            with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as handle:
                for path in sorted(bundle.rglob('*')):
                    if path.is_file():
                        handle.write(path, (Path(stem) / path.relative_to(bundle)).as_posix())
        else:
            with tarfile.open(archive, 'w:gz', compresslevel=9) as handle:
                handle.add(bundle, arcname=stem)
    checksum = archive.with_name(archive.name + '.sha256')
    checksum.write_text(f'{digest(archive)}  {archive.name}\n')
    print(archive)
    print(checksum)


if __name__ == '__main__':
    main()
