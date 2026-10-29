#!/usr/bin/env python3
"""Notarize a Developer-ID artifact with private local Apple credentials.
No credentials are copied to source or written to logs. Not for App Store builds.
"""
import argparse
import json
import subprocess
import tempfile
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('artifact', type=Path)
parser.add_argument('--profile', help='existing notarytool Keychain profile')
parser.add_argument('--submission', help='resume an existing submission without uploading again')
args = parser.parse_args()
artifact = args.artifact.resolve(strict=True)
if args.profile:
    auth = ['--keychain-profile', args.profile]
else:
    registry = json.loads((Path.home()/'.config/apple-dev/teams.json').read_text())
    team = registry['teams'][registry['defaultTeam']]
    auth = ['--key', str(Path(team['privateKey']).expanduser()),
            '--key-id', team['keyId'], '--issuer', team['issuerId']]
temporary = None
submit_path = artifact
if artifact.suffix == '.app' and not args.submission:
    temporary = tempfile.TemporaryDirectory(prefix='bsvx-notary-')
    submit_path = Path(temporary.name)/(artifact.name+'.zip')
    subprocess.run(['ditto', '-c', '-k', '--keepParent', '--norsrc', str(artifact), str(submit_path)], check=True)
if args.submission:
    command = ['wait', args.submission, '--timeout', '60s']
else:
    command = ['submit', str(submit_path), '--wait', '--timeout', '60s']
result = subprocess.run(['xcrun', 'notarytool', *command, *auth, '--output-format', 'json'],
                        capture_output=True, text=True)
# Provider JSON contains submission metadata, never private key contents.
print(result.stdout)
if result.returncode:
    print(result.stderr)
    raise SystemExit(result.returncode)
status = json.loads(result.stdout)
if status.get('status') != 'Accepted':
    raise SystemExit('Submission is not yet accepted; resume with --submission ID.')
if artifact.suffix in ('.app', '.dmg', '.pkg'):
    subprocess.run(['xcrun', 'stapler', 'staple', str(artifact)], check=True)
    subprocess.run(['xcrun', 'stapler', 'validate', str(artifact)], check=True)
if temporary:
    temporary.cleanup()
