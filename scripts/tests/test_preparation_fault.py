"""QA ACK-loss wrapper exercises native replies, without Docker or credentials."""
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / 'container_supervisor_live/preparation_fault.py'
SPEC = importlib.util.spec_from_file_location('preparation_fault', SOURCE)
fault = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fault)


class FaultTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.home = Path(self.temporary.name)
        (self.home / 'preparation-fault-agent').write_text('owned-agent')
        self.native = b'{"result":{"state":"prepared"}}'
        self.identity = dict(operation_id='original-operation', generation='original-generation')
        if os.name != 'posix':
            flag = patch.object(fault.os, 'O_NOFOLLOW', 0, create=True)
            flag.start()
            self.addCleanup(flag.stop)

    def invoke(self, action, agent='owned-agent', generation='original-generation', code=0):
        body = json.dumps({'request': dict(action=action, operation_id='original-operation',
            policy=dict(resource_id=agent, generation=generation))}).encode()
        output = io.BytesIO()
        with patch.object(fault, 'ROOT', self.home), patch.object(fault.sys, 'argv', ['wrapper', '-c', 'native']), \
                patch.object(fault.sys, 'stdin', SimpleNamespace(buffer=io.BytesIO(body))), \
                patch.object(fault.sys, 'stdout', SimpleNamespace(buffer=output)), \
                patch.object(fault.subprocess, 'run', return_value=SimpleNamespace(returncode=code,
                    stdout=self.native)) as native:
            result = fault.main()
            self.assertEqual(native.call_args.args[0], ['/usr/bin/python3', '-c', 'native'])
            self.assertEqual(native.call_args.kwargs['input'], body)
        return result, output.getvalue()

    def test_loses_only_first_successful_owned_prepare_then_reads_same_result(self):
        self.assertEqual(self.invoke('prepare'), (1, b''))
        marker = self.home / 'preparation-fault-result.json'
        self.assertEqual(json.loads(marker.read_bytes()), self.identity)
        self.assertEqual(self.invoke('reconcile_preparation'), (0, self.native))
        observed = self.home / 'preparation-readback-observed.json'
        self.assertEqual(json.loads(observed.read_bytes()), self.identity)
        if os.name == 'posix':
            self.assertEqual(marker.stat().st_mode & 0o777, 0o600)
            self.assertEqual(observed.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.invoke('reconcile_preparation'), (0, self.native))
        self.assertEqual(self.invoke('prepare'), (0, self.native))

    def test_readback_does_not_accept_new_generation(self):
        self.invoke('prepare')
        self.assertEqual(self.invoke('reconcile_preparation', generation='new-generation'), (1, b''))
        self.assertFalse((self.home / 'preparation-readback-observed.json').exists())

    def test_other_agents_actions_and_failed_native_reply_pass_through(self):
        self.assertEqual(self.invoke('prepare', agent='peer-agent'), (0, self.native))
        self.assertEqual(self.invoke('start'), (0, self.native))
        self.assertEqual(self.invoke('prepare', code=2), (2, self.native))
        self.assertFalse((self.home / 'preparation-fault-result.json').exists())


if __name__ == '__main__':
    unittest.main()
