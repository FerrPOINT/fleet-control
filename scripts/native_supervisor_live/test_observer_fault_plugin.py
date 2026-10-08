"""Light host checks for the independent observer race fixture only."""
import asyncio
import importlib.util
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('observer_fault', Path(__file__).with_name('observer_fault_plugin.py'))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
RUN = 'run_' + 'a' * 32


class ObserverFaultTests(unittest.IsolatedAsyncioTestCase):
    async def test_only_final_get_waits_and_preserves_actual_response_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'armed-run').write_text(RUN)
            pause = fixture.FinalCapsPause(root)
            response = SimpleNamespace(status=200, body=b'{"incarnation":"genuine"}')
            caps = SimpleNamespace(method='GET', path=fixture.CAPS)
            self.assertIs(await pause.after(caps, response), response)
            self.assertFalse((root / 'ready.json').exists())
            await pause.after(SimpleNamespace(method='GET', path=fixture.READ + RUN), response)
            pending = asyncio.create_task(pause.after(caps, response))
            async def ready():
                while not (root / 'ready.json').exists():
                    await asyncio.sleep(0.005)
            await asyncio.wait_for(ready(), 1)
            self.assertFalse(pending.done())
            self.assertEqual((root / 'ready.json').read_bytes(), response.body)
            (root / 'release').write_bytes(b'owned release')
            self.assertIs(await pending, response)
            self.assertEqual((root / 'returned').read_bytes(), b'genuine-200')

    async def test_foreign_auth_never_calls_handler_or_reads_arm(self):
        with tempfile.TemporaryDirectory() as directory:
            denied = object()
            fault = fixture.middleware(SimpleNamespace(_check_auth=lambda _: denied), {
                'FLEET_NATIVE_SUPERVISOR_TEST': '1',
                'FLEET_NATIVE_OBSERVER_FAULT_ROOT': directory,
            })
            async def handler(_):
                self.fail('foreign credential reached the real handler')
            with patch.object(fixture.FinalCapsPause, 'armed_run', side_effect=AssertionError('foreign arm read')):
                self.assertIs(await fault(SimpleNamespace(), handler), denied)

    async def test_expired_pause_cannot_return_a_success_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'armed-run').write_text(RUN)
            pause = fixture.FinalCapsPause(root)
            pause.observed_run = RUN
            with patch.object(fixture, 'PAUSE_SECONDS', 0.02):
                with self.assertRaisesRegex(RuntimeError, 'expired'):
                    await pause.after(SimpleNamespace(method='GET', path=fixture.CAPS),
                                      SimpleNamespace(status=200, body=b'{}'))
            self.assertFalse((root / 'returned').exists())

    async def test_foreign_run_and_non_get_do_not_arm_pause(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'armed-run').write_text(RUN)
            pause = fixture.FinalCapsPause(root)
            response = SimpleNamespace(status=200, body=b'{}')
            await pause.after(SimpleNamespace(method='GET', path=fixture.READ + 'run_' + 'b' * 32), response)
            await pause.after(SimpleNamespace(method='POST', path=fixture.READ + RUN), response)
            self.assertIs(await pause.after(SimpleNamespace(method='GET', path=fixture.CAPS), response), response)
            self.assertFalse((root / 'ready.json').exists())

    def test_opt_in_and_owned_root_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(RuntimeError, 'opt-in'):
                fixture.owned_root({'FLEET_NATIVE_OBSERVER_FAULT_ROOT': directory})
            with self.assertRaisesRegex(RuntimeError, 'owned existing root'):
                fixture.owned_root({'FLEET_NATIVE_SUPERVISOR_TEST': '1',
                                    'FLEET_NATIVE_OBSERVER_FAULT_ROOT': 'relative'})


if __name__ == '__main__':
    unittest.main()
