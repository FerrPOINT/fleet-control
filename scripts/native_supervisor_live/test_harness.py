"""Read-only host safety tests; no Docker, native runtime or credentials."""
import hashlib
import importlib.util
import io
from pathlib import Path
import tarfile
import tempfile
import unittest


def module(name):
    spec = importlib.util.spec_from_file_location('native_supervisor_'+name,Path(__file__).with_name(name+'.py'))
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


runner = module('run')
preflight = module('preflight')


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


if __name__ == '__main__':
    unittest.main()
