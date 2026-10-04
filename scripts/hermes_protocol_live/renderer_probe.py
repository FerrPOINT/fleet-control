#!/usr/bin/env python3
"""Read real Fleet renderer output with the pinned native Hermes config loader."""
import hashlib
import inspect
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


ROOT = Path('/renderer-evidence')
FILES = {'config.yaml', '.env', 'SOUL.md', '.fleet-config-revision.json'}


def guarded(path):
    path = Path(path)
    if not path.is_absolute() or not path.is_relative_to(ROOT):
        raise RuntimeError('renderer evidence path is outside owned root')
    current = ROOT
    if current.is_symlink():
        raise RuntimeError('renderer evidence root is linked')
    for part in path.relative_to(ROOT).parts:
        current = current / part
        if part in ('.', '..') or current.is_symlink():
            raise RuntimeError('renderer evidence contains a linked path')
    return path


def verify_files(agent):
    home = guarded(agent['home'])
    if set(agent['hashes']) != FILES:
        raise RuntimeError('renderer evidence file inventory differs')
    for name, expected in agent['hashes'].items():
        path = guarded(home / name)
        if not path.is_file() or path.stat().st_size > 1048576:
            raise RuntimeError('renderer evidence file is invalid')
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise RuntimeError('renderer evidence file hash differs')


def yaml_only_config(home, loader, constructor, platform):
    data = {}
    # The public loader catches YAML errors; call the native layer directly.
    loader(home, data)
    return constructor(data).platforms[platform]


def child(agent):
    verify_files(agent)
    home = guarded(agent['home'])
    os.environ.update(HERMES_HOME=str(home), API_SERVER_HOST='0.0.0.0',
                      API_SERVER_PORT='80', API_SERVER_KEY='fixture-shell-stale')
    os.chdir(guarded(agent['workspace']))
    from hermes_cli.env_loader import load_hermes_dotenv
    from hermes_constants import get_process_hermes_home
    from gateway.config import load_gateway_config, GatewayConfig, Platform
    from gateway.config_loader import load_yaml_layer
    from gateway.platforms.api_server import APIServerAdapter
    source = Path('/qa/source')
    for item in [load_hermes_dotenv, load_gateway_config, load_yaml_layer, APIServerAdapter]:
        if not Path(inspect.getsourcefile(item)).resolve().is_relative_to(source):
            raise RuntimeError('renderer probe imported a different native source')
    for key in list(os.environ):
        if key.startswith('API_SERVER_'):
            os.environ.pop(key)
    with tempfile.TemporaryDirectory(prefix='native-renderer-negative-') as directory:
        invalid = Path(directory)
        (invalid / 'config.yaml').write_text(json.dumps({
            'gateway': {'platforms': {'api_server': {'extra': None}}},
        }), encoding='utf-8')
        try:
            yaml_only_config(invalid, load_yaml_layer, GatewayConfig.from_dict, Platform.API_SERVER)
        except TypeError:
            pass
        else:
            raise RuntimeError('native malformed YAML was not rejected by the direct layer')
    yaml = yaml_only_config(home, load_yaml_layer, GatewayConfig.from_dict, Platform.API_SERVER)
    if not yaml.enabled or yaml.extra['host'] != '127.0.0.1' or yaml.extra['port'] != agent['port']:
        raise RuntimeError('native YAML layer did not use managed listener')
    if yaml.extra.get('key') is not None or yaml.extra['cors_origins'] != agent['cors_origins']:
        raise RuntimeError('native YAML layer did not use managed security settings')
    os.environ.update(API_SERVER_HOST='0.0.0.0', API_SERVER_PORT='80',
                      API_SERVER_KEY='fixture-shell-stale')
    load_hermes_dotenv(hermes_home=home, load_external_secrets=False)
    config = load_gateway_config().platforms[Platform.API_SERVER]
    adapter = APIServerAdapter(config)
    if not config.enabled or adapter._host != '127.0.0.1' or adapter._port != agent['port']:
        raise RuntimeError('native loader did not use managed listener')
    if list(adapter._cors_origins) != agent['cors_origins']:
        raise RuntimeError('native loader did not use managed CORS settings')
    credential = hashlib.sha256(adapter._api_key.encode()).hexdigest()
    if credential != agent['credential_sha256'] or get_process_hermes_home() != home:
        raise RuntimeError('native loader did not use isolated managed identity')
    verify_files(agent)
    return {'home': str(home), 'port': adapter._port, 'credential_sha256': credential}


def main():
    if len(sys.argv) == 3 and sys.argv[1] == '--agent':
        try:
            result = child(json.loads(sys.argv[2]))
        except Exception:
            raise RuntimeError('native renderer child validation failed') from None
        print(json.dumps(result))
        return
    for name in ['uv.lock', 'pyproject.toml']:
        if hashlib.sha256((Path('/qa/source') / name).read_bytes()).digest() != hashlib.sha256((Path('/opt/hermes') / name).read_bytes()).digest():
            raise RuntimeError('native dependency resolution differs')
    fixture = guarded(ROOT / 'fixture.json')
    if not fixture.is_file() or fixture.stat().st_size > 16384:
        raise RuntimeError('renderer evidence manifest is invalid')
    content = json.loads(fixture.read_text(encoding='utf-8'))
    if content['renderer_version'] != 2 or len(content['agents']) != 2:
        raise RuntimeError('renderer evidence needs two actual rendered agents')
    records = []
    for agent in content['agents']:
        verify_files(agent)
        result = subprocess.run([sys.executable, __file__, '--agent', json.dumps(agent)],
                                capture_output=True, timeout=60, check=False)
        if result.returncode:
            raise RuntimeError('native renderer child failed; private diagnostics are not echoed')
        records.append(json.loads(result.stdout))
    if any(records[0][key] == records[1][key] for key in ['home', 'port', 'credential_sha256']):
        raise RuntimeError('renderer native isolation observations are not distinct')
    evidence = {'native_source_sha': os.environ['HERMES_PROTOCOL_SOURCE_SHA'],
                'cases': ['actual-rust-renderer-two-homes-native-dotenv-config-listener-identity'],
                'fixture_sha256': hashlib.sha256(fixture.read_bytes()).hexdigest(),
                'agents': records}
    Path(os.environ['HERMES_PROTOCOL_RESULT']).write_text(json.dumps(evidence), encoding='utf-8')
    print('Native loader verified two actual Rust-rendered homes, listeners and credentials; no model run.')


if __name__ == '__main__':
    main()
