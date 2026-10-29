#!/usr/bin/env python3
"""Collect pinned Cargo license notices; no builds, uploads, or private credentials.

Includes the union of resolved graphics dependencies for the release targets,
including conservative build/proc-macro dependencies. Missing upstream notices
can be fetched at registry-recorded commits using --fetch-missing. Output has no
local source/cache paths. Retains every available license alternative; no GPL or
LGPL alternative is selected when permissive alternatives are available.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import subprocess
import urllib.request

TARGETS = ('aarch64-apple-darwin', 'x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu')
ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / 'docs' / 'Dependency_Licenses'


def get(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'Battlestar-Dependency-Notice-Audit'})
    with urllib.request.urlopen(request, timeout=20) as response:
        return response.read()


def is_notice(name):
    return any(word in name.lower() for word in ('license', 'licence', 'copying', 'notice', 'copyright', 'ofl.txt'))


def upstream(package, directory):
    repository = package.get('repository') or ''
    match = re.search(r'github\.com/([^/]+/[^/#]+)', repository)
    if not match:
        return []
    repo = match[1].removesuffix('.git')
    vcs = directory / '.cargo_vcs_info.json'
    commit = json.loads(vcs.read_text()).get('git', {}).get('sha1') if vcs.exists() else None
    if not commit:
        # Old crates may omit a VCS receipt. Resolve a version tag to immutable SHA.
        for tag in ('v' + package['version'], package['version']):
            try:
                commit = json.loads(get(f'https://api.github.com/repos/{repo}/commits/{tag}'))['sha']
                break
            except Exception:
                continue
    if not commit:
        return []
    tree = json.loads(get(f'https://api.github.com/repos/{repo}/git/trees/{commit}'))['tree']
    notices = []
    for entry in tree:
        if entry['type'] == 'blob' and is_notice(entry['path']):
            url = f'https://raw.githubusercontent.com/{repo}/{commit}/{entry["path"]}'
            notices.append((url, get(url)))
    return notices


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fetch-missing', action='store_true')
    args = parser.parse_args()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    text_dir = OUTPUT / 'Texts'
    text_dir.mkdir(exist_ok=True)
    packages = {}
    target_ids = {}
    for target in TARGETS:
        raw = subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version', '1', '--filter-platform', target], cwd=ROOT)
        metadata = json.loads(raw)
        ids = {node['id'] for node in metadata['resolve']['nodes']}
        target_ids[target] = ids
        for package in metadata['packages']:
            if package['id'] in ids and package['source']:
                packages[package['id']] = package
    old_index = OUTPUT / 'Inventory.json'
    old = json.loads(old_index.read_text()) if old_index.exists() else {'packages': []}
    old_packages = {(p['name'], p['version']): p for p in old['packages']}

    def collect(package):
        directory = Path(package['manifest_path']).parent
        found = []
        for file in sorted(directory.rglob('*')):
            if file.is_file() and is_notice(file.name):
                found.append((f'crate:{package["name"]}@{package["version"]}/{file.relative_to(directory).as_posix()}', file.read_bytes()))
        if package['name'] == 'bevy_text':
            # default_font embeds Fira Mono; its OFL lives outside the crate.
            previous = old_packages.get((package['name'], package['version']), {})
            cached_font = [n for n in previous.get('notices', []) if 'FiraMono-LICENSE' in n['origin']]
            for notice in cached_font:
                found.append((notice['origin'], (OUTPUT / notice['file']).read_bytes()))
            if not cached_font and args.fetch_missing:
                vcs = json.loads((directory / '.cargo_vcs_info.json').read_text())
                commit = vcs['git']['sha1']
                url = f'https://raw.githubusercontent.com/bevyengine/bevy/{commit}/assets/fonts/FiraMono-LICENSE'
                found.append((url, get(url)))
        cached = old_packages.get((package['name'], package['version']), {}).get('notices', [])
        cached_valid = any((OUTPUT / n['file']).is_file() and hashlib.sha256((OUTPUT / n['file']).read_bytes()).hexdigest() == n['sha256'] for n in cached)
        if not found and args.fetch_missing and not cached_valid:
            try:
                found = upstream(package, directory)
            except Exception as error:
                print(f'Upstream notice fetch failed: {package["name"]} {package["version"]}: {type(error).__name__}')
        records = []
        for origin, data in found:
            digest = hashlib.sha256(data).hexdigest()
            name = f'Texts/{digest}.txt'
            (OUTPUT / name).write_bytes(data)
            records.append({'file': name, 'sha256': digest, 'origin': origin})
        if not records:
            previous = old_packages.get((package['name'], package['version']), {})
            for record in previous.get('notices', []):
                file = OUTPUT / record['file']
                if file.is_file() and hashlib.sha256(file.read_bytes()).hexdigest() == record['sha256']:
                    records.append(record)
        for record in records:
            if 'supplement' in record:
                data = (OUTPUT / record['file']).read_bytes()
                if b'<year> <copyright holders>' in data:
                    data = data.replace(b'<year> <copyright holders>', ', '.join(package['authors']).encode())
                    digest = hashlib.sha256(data).hexdigest()
                    record['file'] = f'Texts/{digest}.txt'
                    record['sha256'] = digest
                    (OUTPUT / record['file']).write_bytes(data)
        reconstructed = False
        if not records:
            # The released crates below declare these licenses but omit their text.
            # Preserve the declaration and attribution, never fabricate historical
            # copyright statements. Standard text is explicitly distinguished.
            choices = {'block': 'MIT', 'dispatch': 'MIT', 'malloc_buf': 'MIT',
                       'constgebra': 'Apache-2.0', 'hexf-parse': 'CC0-1.0'}
            chosen = choices.get(package['name'])
            if chosen:
                url = f'https://raw.githubusercontent.com/spdx/license-list-data/v3.27.0/text/{chosen}.txt'
                try:
                    data = get(url)
                    prefix = (f"Standard license text supplement for {package['name']} {package['version']}\n"
                              f"Declared package license: {package['license']}\n"
                              f"Upstream package authors: {', '.join(package['authors'])}\n"
                              "The upstream release omits a license text. This supplement preserves its Cargo license declaration and authors; it is not an upstream copyright notice. No historical copyright year is asserted.\n\n").encode()
                    if chosen == 'MIT':
                        data = data.replace(b'<year> <copyright holders>', ', '.join(package['authors']).encode())
                    data = prefix + data
                    digest = hashlib.sha256(data).hexdigest()
                    name = f'Texts/{digest}.txt'
                    (OUTPUT / name).write_bytes(data)
                    records.append({'file': name, 'sha256': digest, 'origin': url,
                                    'supplement': 'Standard SPDX license text with original crate metadata attribution; upstream notice absent'})
                    reconstructed = True
                except Exception as error:
                    print(f'Standard text supplement failed: {package["name"]}: {type(error).__name__}')
        return {'standard_text_supplement': reconstructed or any('supplement' in r for r in records), 'name': package['name'], 'version': package['version'], 'license': package['license'],
                'repository': package['repository'], 'targets': [t for t in TARGETS if package['id'] in target_ids[t]],
                'notices': records}

    with ThreadPoolExecutor(max_workers=6) as executor:
        records = list(executor.map(collect, sorted(packages.values(), key=lambda p: (p['name'], p['version']))))
    missing = [f'{p["name"]}@{p["version"]}' for p in records if not p['notices']]
    inventory = {'schema': 1, 'scope': 'Union of Cargo resolved graphics/build dependencies; conservative, not binary reachability proof',
                 'targets': TARGETS, 'cargo_lock_sha256': hashlib.sha256((ROOT / 'Cargo.lock').read_bytes()).hexdigest(),
                 'missing_notices': missing, 'packages': records}
    old_index.write_text(json.dumps(inventory, indent=2) + '\n')
    lines = ['# Dependency license notices', '',
             'Retain this entire directory with release archives. Original license texts are preserved byte-for-byte in Texts/; Inventory.json maps package versions, declared license expressions, target graphs, provenance and SHA-256 hashes.', '',
             'The inventory is the conservative union of the pinned macOS Apple Silicon, Windows MSVC x64 and Linux GNU x64 graphics dependency graphs, including build/proc-macro dependencies. It is not proof of exactly which code is linked, nor a vulnerability or legal-clearance opinion. Platform operating-system libraries and separately shipped assets have additional notices outside this inventory.', '',
             'Where OR alternatives exist, use a permissive option (MIT, Apache-2.0, BSD, ISC, Zlib, CC0 or Unlicense as offered). Slash-separated historical MIT/Apache declarations describe alternatives. AND clauses retain both obligations, including Unicode-3.0. All available texts are included without selecting GPL/LGPL alternatives. No dependency code is modified by this collection.', '',
             'Regenerate with python3 script/generate_dependency_notices.py --fetch-missing after dependency changes; Cargo must already have the pinned crates cached. Immutable upstream commit URLs fill notices omitted from crate packages. Review missing notices before distribution.', '',
             f'Packages: {len(records)}. Missing notice records: {len(missing)}.', '',
             'Five crates omit upstream license texts: block 0.1.6, dispatch 0.2.0, malloc_buf 0.0.6, constgebra 0.1.4 and hexf-parse 0.2.1. Their records explicitly use standard SPDX text supplements plus original crate authors/license declarations, not reconstructed historical copyright notices. These are the remaining provenance limitation; no historical year or additional upstream grant is invented.', '',
             '| Package | Declared license | Notice texts |', '| --- | --- | --- |']
    for p in records:
        links = ' '.join(f'[text {i+1}]({n["file"]})' for i, n in enumerate(p['notices'])) or '**MISSING**'
        lines.append(f'| {p["name"]} {p["version"]} | {p["license"]} | {links} |')
    (OUTPUT / 'README.md').write_text('\n'.join(lines) + '\n')
    referenced = {n['file'] for p in records for n in p['notices']}
    for file in text_dir.glob('*.txt'):
        if str(file.relative_to(OUTPUT)) not in referenced:
            file.unlink()  # Only collector-generated, now-unreferenced notice copies.
    print(f'{len(records)} packages; {len(missing)} missing notices: {missing}')
    return bool(missing)


if __name__ == '__main__':
    raise SystemExit(main())
