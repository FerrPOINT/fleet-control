"""Read-only host safety tests; no Docker, native runtime or credentials."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace


def module(name):
    spec = importlib.util.spec_from_file_location('native_supervisor_'+name,Path(__file__).with_name(name+'.py'))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


runner = module('run')
preflight = module('preflight')
fault_plugin = module('discard_ack_plugin')
approval_plugin = module('approval_fault_plugin')


def archive(name, symlink=False):
    output = io.BytesIO()
    with tarfile.open(fileobj=output,mode='w') as tar:
        member = tarfile.TarInfo(name)
        if symlink:
            member.type = tarfile.SYMTYPE
            member.linkname = '../foreign'
            tar.addfile(member)
        else:
            body = b'owned-native-source\n'
            member.size = len(body)
            tar.addfile(member,io.BytesIO(body))
    return output.getvalue()


class SafetyTests(unittest.TestCase):
    def test_scenarios_select_distinct_exact_tests(self):
        self.assertEqual(set(runner.TEST_NAMES), {'lifecycle', 'recovery', 'controls', 'approvals', 'approval-recovery'})
        self.assertEqual(len(set(runner.TEST_NAMES.values())), 5)
        self.assertTrue(all(name.rsplit('::', 1)[-1].startswith('managed_native_')
                            for name in runner.TEST_NAMES.values()))

    def test_recovery_requires_complete_committed_inventory(self):
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode='w') as tar:
            for name in runner.PLUGIN_FILES:
                member = tarfile.TarInfo('deploy/hermes-recovery-plugin/'+name)
                member.size = 5
                tar.addfile(member, io.BytesIO(b'owned'))
        with patch.object(runner, 'git', return_value=output.getvalue()) as git:
            self.assertEqual(set(runner.recovery_files('repo', 'exact-head')), set(runner.PLUGIN_FILES))
            self.assertEqual(git.call_args.args[2:4], ('--format=tar', 'exact-head'))
        with patch.object(runner, 'git', return_value=archive('deploy/hermes-recovery-plugin/plugin.py')):
            with self.assertRaises(RuntimeError): runner.recovery_files('repo', 'exact-head')

    def test_source_archive_preserves_original_bytes(self):
        self.assertEqual(runner.archive_files(archive('gateway/native.py')),
                         {'gateway/native.py':b'owned-native-source\n'})

    def test_archive_traversal_absolute_and_links_are_refused(self):
        for path in ['../foreign','/foreign','C:/foreign','gateway/../foreign','gateway\\foreign']:
            with self.subTest(path=path),self.assertRaises(RuntimeError):
                runner.archive_files(archive(path))
        with self.assertRaises(RuntimeError):
            runner.archive_files(archive('gateway/native.py',symlink=True))
        with self.assertRaises(RuntimeError):
            runner.archive_files(archive('.venv/bin/python'))

    def test_source_layer_uses_only_exact_image_and_preserves_inherited_user(self):
        image = 'sdlc-qa-fleet-native-aaaaaaaaaaaa-deps:qa'
        self.assertEqual(runner.source_dockerfile(image),f'FROM {image}\nADD source.tar /opt/hermes/\n')
        for value in ['mutable:tag', image+'\nUSER root','sha256:invalid']:
            with self.subTest(value=value),self.assertRaises(RuntimeError):
                runner.source_dockerfile(value)

    def test_source_layer_preserves_all_dependency_layers_and_non_root_user(self):
        dependency = {'RootFS':{'Layers':['dependency']},'Config':{'User':'fleet-control'}}
        candidate = {'RootFS':{'Layers':['dependency','owned-source']},'Config':{'User':'fleet-control'}}
        runner.verify_source_layer(dependency,candidate)
        for layers,user in [(['foreign','owned-source'],'fleet-control'),
                            (['dependency'],'fleet-control'),(['dependency','owned-source'],'root')]:
            with self.subTest(layers=layers,user=user),self.assertRaises(RuntimeError):
                runner.verify_source_layer(dependency,{'RootFS':{'Layers':layers},'Config':{'User':user}})

    def test_mounts_are_read_only_and_cannot_create_missing_host_paths(self):
        mount = runner.bind(Path('owned-source'),'native-source')
        self.assertTrue(mount['read_only'])
        self.assertFalse(mount['bind']['create_host_path'])

    def test_service_has_owner_and_explicit_internal_network(self):
        service = runner.service('read-only-native-verification')
        self.assertEqual(service['labels']['sdlc.task'],'fleet-native-supervisor')
        self.assertEqual(service['labels']['sdlc.purpose'],'read-only-native-verification')
        self.assertEqual(service['networks'],['qa'])

    def test_complete_inventory_matches_and_tamper_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'module.py').write_bytes(b'owned-native-source\n')
            hashes = {'module.py':hashlib.sha256(b'owned-native-source\n').hexdigest()}
            preflight.verify(root,hashes)
            (root/'module.py').write_bytes(b'tampered')
            with self.assertRaises(RuntimeError): preflight.verify(root,hashes)

    def test_missing_and_empty_inventories_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            for hashes in [{},[],{'missing.py':'a'*64}]:
                with self.subTest(hashes=hashes),self.assertRaises(RuntimeError):
                    preflight.verify(Path(directory),hashes)

    def test_unsafe_inventory_paths_and_non_digest_values_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            for path in ['../foreign','/foreign','C:/foreign','gateway\\foreign']:
                with self.subTest(path=path),self.assertRaises(RuntimeError):
                    preflight.verify(Path(directory),{path:'a'*64})
            with self.assertRaises(RuntimeError): preflight.verify(Path(directory),{'module.py':'invalid'})


class FaultFixtureTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        # Host unit tests check middleware ordering, not Linux filesystem flags.
        if not hasattr(os, 'O_NOFOLLOW'):
            self.flags = patch.object(os, 'O_NOFOLLOW', 0, create=True)
            self.flags.start()
            self.addCleanup(self.flags.stop)

    def install(self, root, denied=None, opt_in='1'):
        app = SimpleNamespace(middlewares=[])
        response = lambda body, status: SimpleNamespace(body=json.dumps(body).encode(), status=status)
        web = SimpleNamespace(middleware=lambda function: function, json_response=response)
        handlers = []
        fault_plugin.register(SimpleNamespace(register_platform_handler=lambda name, wire: handlers.append((name, wire))))
        self.assertEqual(handlers[0][0], 'api_server')
        with patch.dict('sys.modules', {'aiohttp':SimpleNamespace(web=web)}), patch.dict(os.environ, {
            'FLEET_NATIVE_SUPERVISOR_TEST':opt_in, 'FLEET_NATIVE_FAULT_ROOT':str(root),
        }):
            handlers[0][1](app, SimpleNamespace(_check_auth=lambda request: denied))
        return app.middlewares[0]

    def request(self, path='/v1/runs', method='POST', order=None):
        async def read(): return b'{"input":"synthetic QA prompt"}'
        return SimpleNamespace(path=path, method=method, headers={'Idempotency-Key':'original-key'},
                               read=read, transport=SimpleNamespace(close=lambda: order.append('closed')))

    async def test_disconnect_only_after_real_handler_acceptance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fault = self.install(root)
            order = []
            response = SimpleNamespace(status=202, body=b'{"run_id":"observed-native-id"}')
            async def handler(request):
                order.append('accepted')
                return response
            self.assertIs(await fault(self.request(order=order), handler), response)
            self.assertEqual(order, ['accepted', 'closed'])
            observations = [json.loads(line) for line in (root/'native-events.jsonl').read_text().splitlines()]
            self.assertEqual([event['kind'] for event in observations], ['post','accepted'])
            self.assertEqual(observations[1]['run_id'], 'observed-native-id')
            self.assertEqual(observations[0]['sha256'], observations[1]['sha256'])
            self.assertNotIn('synthetic QA prompt', (root/'native-events.jsonl').read_text())

    async def test_auth_denial_has_no_handler_or_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            denied = object()
            fault = self.install(root, denied=denied)
            async def handler(request): self.fail('auth denial must precede handler')
            self.assertIs(await fault(self.request(), handler), denied)
            self.assertFalse((root/'native-events.jsonl').exists())

    async def test_held_lookup_does_not_invoke_native_handler(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'hold-lookup').touch()
            fault = self.install(root)
            async def handler(request): self.fail('held lookup must not reach native handler')
            result = await fault(self.request('/fleet/v1/recovery/lookup'), handler)
            self.assertEqual(result.status, 503)
            observation = json.loads((root/'native-events.jsonl').read_text())
            self.assertEqual(observation, {'kind':'lookup', 'blocked':True})

    async def test_non_accepted_response_is_not_disconnected(self):
        with tempfile.TemporaryDirectory() as directory:
            fault = self.install(Path(directory))
            order = []
            response = SimpleNamespace(status=409, body=b'{}')
            async def handler(request): return response
            self.assertIs(await fault(self.request(order=order), handler), response)
            self.assertEqual(order, [])

    def test_fixture_requires_opt_in_and_existing_absolute_root(self):
        with tempfile.TemporaryDirectory() as directory:
            for root, opt_in in [(Path(directory), '0'), (Path('relative'), '1'),
                                 (Path(directory)/'missing', '1')]:
                with self.subTest(root=root, opt_in=opt_in), self.assertRaises(RuntimeError):
                    self.install(root, opt_in=opt_in)


class ApprovalFaultFixtureTests(unittest.IsolatedAsyncioTestCase):
    setUp = FaultFixtureTests.setUp

    def install(self, root, denied=None, opt_in='1', map_linux_root=True, recovery='0'):
        app = SimpleNamespace(middlewares=[])
        web = SimpleNamespace(middleware=lambda function: function,
                              Response=lambda status: SimpleNamespace(status=status))
        handlers = []
        approval_plugin.register(SimpleNamespace(
            register_platform_handler=lambda name, wire: handlers.append((name, wire))))
        self.assertEqual(handlers[0][0], 'api_server')
        # Only host units map the fixed Linux QA directory to their disposable root.
        def path(value):
            return root if value == '/tmp/fleet-native-supervisor/approval-fault' else Path(value)
        with patch.dict('sys.modules', {'aiohttp':SimpleNamespace(web=web)}), patch.dict(os.environ, {
            'FLEET_NATIVE_SUPERVISOR_TEST':opt_in, 'FLEET_NATIVE_APPROVAL_FAULT_ROOT':str(root),
            'FLEET_NATIVE_APPROVAL_RECOVERY_TEST':recovery,
        }), patch.object(approval_plugin, 'Path', path if map_linux_root else Path):
            handlers[0][1](app, SimpleNamespace(_check_auth=lambda request: denied))
        return app.middlewares[0]

    def request(self, order, path='/v1/runs/native-run/approval'):
        async def body():
            order.append('body')
            return {'request_id':'native-request', 'choice':'once', 'resolve_all':False,
                    'command':'not recorded', 'token':'not recorded'}
        return SimpleNamespace(path=path, method='POST', json=body,
                               transport=SimpleNamespace(close=lambda: order.append('closed')))

    def response(self, status=200, **override):
        body = {'object':'hermes.run.approval_response', 'run_id':'native-run',
                'request_id':'native-request', 'choice':'once', 'resolved':1}
        body.update(override)
        return SimpleNamespace(status=status, body=json.dumps(body).encode())

    async def test_exact_real_ack_is_dropped_only_once_after_effect(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'drop-next-approval').touch()
            observer = self.install(root)
            order = []
            response = self.response()
            async def handler(request):
                order.append('effect')
                return response
            self.assertIs(await observer(self.request(order), handler), response)
            self.assertEqual(order, ['body', 'effect', 'closed'])
            self.assertFalse((root/'drop-next-approval').exists())
            order.clear()
            self.assertIs(await observer(self.request(order), handler), response)
            self.assertEqual(order, ['body', 'effect'])
            raw = (root/'native-events.jsonl').read_text()
            events = [json.loads(line) for line in raw.splitlines()]
            self.assertEqual([row['kind'] for row in events],
                             ['approval_post', 'approval_ack', 'approval_ack_dropped',
                              'approval_post', 'approval_ack'])
            self.assertNotIn('not recorded', raw)
            self.assertNotIn('command', raw)
            self.assertNotIn('token', raw)

    async def test_auth_denial_precedes_body_handler_and_observations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            denied = object()
            observer = self.install(root, denied=denied)
            order = []
            async def handler(request): self.fail('denied request cannot reach handler')
            self.assertIs(await observer(self.request(order), handler), denied)
            self.assertEqual(order, [])
            self.assertFalse((root/'native-events.jsonl').exists())

    async def test_error_or_foreign_ack_preserves_drop_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root/'drop-next-approval'
            marker.touch()
            observer = self.install(root)
            for response in [self.response(status=409), self.response(run_id='foreign'),
                             self.response(request_id='foreign'), self.response(choice='deny'),
                             self.response(resolved=2), self.response(object='foreign')]:
                with self.subTest(response=response.body):
                    order = []
                    async def handler(request): return response
                    self.assertIs(await observer(self.request(order), handler), response)
                    self.assertEqual(order, ['body'])
                    self.assertTrue(marker.exists())
            events = [json.loads(line) for line in (root/'native-events.jsonl').read_text().splitlines()]
            self.assertTrue(all(row['kind'] == 'approval_post' for row in events))

    async def test_non_approval_requests_are_not_observed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            observer = self.install(root)
            order = []
            response = self.response()
            async def handler(request):
                order.append('handler')
                return response
            self.assertIs(await observer(self.request(order, '/v1/runs/native-run/stop'), handler), response)
            self.assertEqual(order, ['handler'])
            self.assertFalse((root/'native-events.jsonl').exists())

    def test_opt_in_and_exact_existing_root_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for opt_in, mapped in [('0', True), ('1', False)]:
                with self.subTest(opt_in=opt_in, mapped=mapped), self.assertRaises(RuntimeError):
                    self.install(root, opt_in=opt_in, map_linux_root=mapped)
            with self.assertRaises(RuntimeError): self.install(root/'missing')

    async def test_recovery_denied_get_never_observes_or_calls_handler(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            denied = object()
            observer = self.install(root, denied=denied, recovery='1')
            request = self.request([], '/v1/runs/native-run')
            request.method = 'GET'
            async def handler(request): self.fail('auth must precede recovery')
            self.assertIs(await observer(request, handler), denied)
            self.assertFalse((root/'native-events.jsonl').exists())

    async def test_recovery_sse_fault_is_not_a_fake_native_event_stream(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            observer = self.install(root, recovery='1')
            request = self.request([], '/v1/runs/native-run/events')
            request.method = 'GET'
            async def handler(request): self.fail('owned SSE fault cannot attach handler')
            self.assertEqual((await observer(request, handler)).status, 503)
            self.assertEqual(json.loads((root/'native-events.jsonl').read_text()),
                             {'kind':'events_held', 'run_id':'native-run'})

    async def test_recovery_drops_only_real_current_waiting_snapshot_after_handler(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root/'hold-approval-readback'
            marker.touch()
            observer = self.install(root, recovery='1')
            order = []
            request = self.request(order, '/v1/runs/native-run')
            request.method = 'GET'
            body = {'object':'hermes.run', 'run_id':'native-run', 'session_id':'native-session',
                    'status':'waiting_for_approval', 'approval':{'event':'approval.request',
                    'run_id':'native-run', 'request_id':'native-request', 'command':'never record this secret'}}
            response = SimpleNamespace(status=200, body=json.dumps(body).encode())
            async def handler(request):
                order.append('handler')
                return response
            self.assertIs(await observer(request, handler), response)
            self.assertEqual(order, ['handler', 'closed'])
            marker.unlink()
            order.clear()
            self.assertIs(await observer(request, handler), response)
            self.assertEqual(order, ['handler'])
            raw = (root/'native-events.jsonl').read_text()
            events = [json.loads(line) for line in raw.splitlines()]
            self.assertEqual([row['kind'] for row in events], ['waiting_held', 'status_read'])
            self.assertNotIn('never record', raw)
            self.assertNotIn('command', raw)

    async def test_recovery_running_or_error_snapshot_is_not_disconnected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'hold-approval-readback').touch()
            observer = self.install(root, recovery='1')
            for status, body in [(404, {}), (200, {'object':'hermes.run', 'run_id':'native-run',
                                                    'status':'running', 'session_id':'native-session'})]:
                order = []
                request = self.request(order, '/v1/runs/native-run')
                request.method = 'GET'
                response = SimpleNamespace(status=status, body=json.dumps(body).encode())
                async def handler(request): return response
                self.assertIs(await observer(request, handler), response)
                self.assertEqual(order, [])


if __name__ == '__main__':
    unittest.main()
