"""Pure/static regressions. Never start frontend, browsers, containers, or Cargo."""
import io
import hashlib
from contextlib import redirect_stdout
import json
import os
from pathlib import Path
import random
import re
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import MagicMock, patch
import zipfile
import zlib

from scripts import hosted_frontend_gate as gate

ROOT = Path(__file__).resolve().parents[2]


def git_blob_inventory(sha):
    inventory = {}
    entries = gate.git(ROOT, "ls-tree", "-rz", sha).split(b"\0")
    process = subprocess.Popen(
        ["git", "--no-replace-objects", "-C", str(ROOT), "cat-file", "--batch"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
    )
    try:
        for entry in filter(None, entries):
            meta, name = entry.split(b"\t", 1)
            mode, kind, blob = meta.split()
            gate.require(mode in (b"100644", b"100755") and kind == b"blob", "Non-blob source")
            process.stdin.write(blob + b"\n")
            process.stdin.flush()
            actual_blob, actual_kind, raw_size = process.stdout.readline(256).split()
            size = int(raw_size)
            gate.require(actual_blob == blob and actual_kind == kind and 0 <= size <= gate.MAX_FILE,
                         "Invalid Git batch header")
            data = process.stdout.read(size)
            gate.require(len(data) == size and process.stdout.read(1) == b"\n", "Invalid Git batch frame")
            gate.require(hashlib.sha1(b"blob " + str(size).encode() + b"\0" + data).hexdigest() == blob.decode(),
                         "Invalid Git blob bytes")
            inventory[name.decode()] = gate.digest(data)
        process.stdin.close()
        gate.require(process.wait(timeout=10) == 0, "Git batch failed")
    finally:
        if process.poll() is None:
            process.kill()
        process.wait(timeout=10)
        process.stdin.close()
        process.stdout.close()
    return inventory


def png(width=375, height=812):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    row = b"\0" + random.Random(23).randbytes(width * 3)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(row * height)) + chunk(b"IEND", b""))


def png_stream(body, *, width=375, height=812, encoding=(8, 2, 0, 0, 0)):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, *encoding))
            + chunk(b"IDAT", body) + chunk(b"IEND", b""))


def checksums(files):
    files["SHA256SUMS"] = "".join(gate.digest(files[name]) + "  " + name + "\n"
                                  for name in sorted(files) if name != "SHA256SUMS").encode()


def fixture_files():
    workflow_sha = "a" * 40
    provenance = dict(version=1, repository=gate.REPOSITORY, branch=gate.BRANCH,
        source_sha=gate.SOURCE_SHA, source_tree=gate.SOURCE_TREE, source_parents=gate.SOURCE_PARENTS,
        base_sha=gate.BASE_SHA, base_tree=gate.BASE_TREE, schema_sha256=gate.SCHEMA_SHA256,
        generated_client_sha256="b" * 64, node=gate.NODE, pnpm=gate.PNPM,
        workflow_sha=workflow_sha, workflow_path=gate.WORKFLOW, run_id=123, run_attempt=2,
        gates=list(gate.GATES), compat_main_sha="c" * 40, compat_schema_sha256="d" * 64,
        helper_sha256=gate.digest((ROOT / "scripts/hosted_frontend_gate.py").read_text(encoding="utf-8").encode()),
        workflow_sha256=gate.digest((ROOT / gate.WORKFLOW).read_text(encoding="utf-8").encode()),
        build_manifest_sha256="f" * 64, build_file_count=5, build_index_sha256="f" * 64,
        runner_image="ubuntu24", runner_image_version="test-only", **gate.QUALIFIED_INPUTS, **gate.SCOPE)
    pictures = {viewport: png(*map(int, viewport.split("x"))) for viewport in gate.VIEWPORTS}
    files = {"screens/" + path.removeprefix("docs/assets/screens/"): pictures[path.split("/")[3]]
             for path in gate.capture_paths(ROOT)}
    manifest = dict(kind="fresh_chromium_fixture_screens", count=len(files), files=sorted(files),
                    original_manifest_sha256="e" * 64)
    for browser in ("chromium", "firefox", "webkit"):
        files[f"fixtures/example-{browser}/fixture.png"] = pictures["375x812"]
    summary = dict(unit=gate.QUALIFIED_UNIT_COUNTS,
                   browsers={b: dict(passed=1, flaky=0, skipped=9) for b in ("chromium", "firefox", "webkit")},
                   **gate.SCOPE)
    files.update({"provenance.json": gate.canonical(provenance), "fixture-summary.json": gate.canonical(summary),
                  "screens/manifest.json": gate.canonical(manifest)})
    checksums(files)
    return files


def zipped(files, extra=None):
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in files.items():
            archive.writestr(name, data)
        if extra:
            archive.writestr(*extra)
    return output.getvalue()


def readback_metadata(payload):
    run = dict(id=123, run_attempt=2, status="completed", conclusion="success", event="push",
               head_sha="a" * 40, head_branch=gate.BRANCH, path=gate.WORKFLOW,
               repository=dict(full_name=gate.REPOSITORY))
    artifact = dict(id=456, expired=False, workflow_run=dict(id=123, head_sha="a" * 40, head_branch=gate.BRANCH),
                    name="fleet-frontend-chats-123-2", digest="sha256:" + gate.digest(payload))
    return run, artifact


def validate(payload, run=None, artifact=None):
    actual_run, actual_artifact = readback_metadata(payload)
    return gate.validate_readback(run or actual_run, artifact or actual_artifact, payload,
        run_id=123, attempt=2, workflow_sha="a" * 40, artifact_id=456, artifact_digest=gate.digest(payload))


class SourceContracts(unittest.TestCase):
    def test_materialization_is_closed_to_one_base_revision_path_and_exact_bytes(self):
        name = "crates/auth-server/migrations/0001_users_sessions.sql"
        self.assertEqual(gate.BASE_MATERIALIZED_FILES, {name: (
            "69f7ec8116b228dede62375c7c8b45e1af233887",
            "ef0fae09d1a5359eb23ade564541b03bc1f1514c2017317fc7922ced72c26d75",
        )})
        blob, actual, data = "a" * 40, "b" * 40, b"synthetic\r\n"
        with patch.dict(gate.BASE_MATERIALIZED_FILES, {name: (blob, gate.digest(data))}, clear=True):
            gate.verify_materialized_file(gate.BASE_SHA, name, blob, actual, data)
            for sha, path, expected_blob, body in (
                (gate.SOURCE_SHA, name, blob, data),
                ("f" * 40, name, blob, data),
                (gate.BASE_SHA, name + ".copy", blob, data),
                (gate.BASE_SHA, name, "c" * 40, data),
                (gate.BASE_SHA, name, blob, b"synthetic\n"),
                (gate.BASE_SHA, name, blob, data + b"altered"),
            ):
                with self.subTest(sha=sha, path=path), self.assertRaises(ValueError):
                    gate.verify_materialized_file(sha, path, expected_blob, actual, body)
        gate.verify_materialized_file(gate.SOURCE_SHA, "normal.txt", blob, blob, b"synthetic")
        with self.assertRaises(ValueError):
            gate.verify_materialized_file(gate.BASE_SHA, "unattested.txt", blob, actual, data)

    def test_failure_hints_never_echo_unrecognized_private_diagnostics(self):
        self.assertEqual(gate.safe_failure_hint(ValueError("Source bytes differ from exact committed tree")),
                         "source_bytes")
        self.assertEqual(gate.safe_failure_hint(ValueError(
            "Base materialized bytes differ from the pinned attribute contract")), "base_materialization")
        for error in (OSError("PRIVATE_SENTINEL"), KeyError("PRIVATE_SENTINEL"),
                      ValueError("Source bytes differ from exact committed tree PRIVATE_SENTINEL")):
            self.assertEqual(gate.safe_failure_hint(error), "unclassified")

    def test_exact_two_parent_tuple(self):
        gate.validate_source_tuple(gate.SOURCE_SHA, gate.SOURCE_TREE, gate.SOURCE_PARENTS)
        for parents in ([], ["f" * 40], gate.SOURCE_PARENTS + ["f" * 40]):
            with self.subTest(parents=parents), self.assertRaises(ValueError):
                gate.validate_source_tuple(gate.SOURCE_SHA, gate.SOURCE_TREE, parents)
        # Retain ordered merge-parent coverage even though this source has one parent.
        merge_parents = ["1" * 40, "2" * 40]
        with patch.object(gate, "SOURCE_PARENTS", merge_parents):
            gate.validate_source_tuple(gate.SOURCE_SHA, gate.SOURCE_TREE, merge_parents)
            for parents in (merge_parents[:1], list(reversed(merge_parents)),
                            merge_parents + ["f" * 40], ["f" * 40, merge_parents[1]]):
                with self.subTest(parents=parents), self.assertRaises(ValueError):
                    gate.validate_source_tuple(gate.SOURCE_SHA, gate.SOURCE_TREE, parents)

    def test_wrong_commit_or_tree(self):
        for sha, tree in (("f" * 40, gate.SOURCE_TREE), (gate.SOURCE_SHA, "f" * 40)):
            with self.assertRaises(ValueError):
                gate.validate_source_tuple(sha, tree, gate.SOURCE_PARENTS)

    def test_delta_requires_exactly_three_additions(self):
        good = "\n".join("A\t" + name for name in sorted(gate.WRITE_SET))
        gate.validate_delta(good)
        for value in (good + "\nM\tfrontend/package.json", good.replace("A\t", "M\t", 1),
                      good + "\n" + good.splitlines()[0], "\n".join(good.splitlines()[:2]),
                      good.replace("scripts/hosted_frontend_gate.py", "scripts/openapi_codegen.py")):
            with self.subTest(delta=value), self.assertRaises(ValueError):
                gate.validate_delta(value)

    def test_public_repo_is_explicit(self):
        value = dict(full_name=gate.REPOSITORY, private=False, visibility="public", default_branch="main")
        gate.public_repository(value)
        for key, bad in (("private", True), ("private", 0), ("visibility", "private"),
                         ("full_name", "other/fleet-control"), ("default_branch", "other")):
            with self.subTest(key=key), self.assertRaises(ValueError):
                gate.public_repository(dict(value, **{key: bad}))

    def test_schema_pin_is_current(self):
        self.assertEqual(gate.digest((ROOT / "openapi/openapi.json").read_bytes()), gate.SCHEMA_SHA256)
        self.assertEqual(gate.SCHEMA_SHA256, "afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501")
        self.assertEqual(gate.digest(gate.git(ROOT, "show", "59d00fe3269d67ab09819f19c6b2b5703a6e2268:openapi/openapi.json")),
                         "1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953")

    def test_no_local_or_external_runner(self):
        with patch.dict(os.environ, {}, clear=True), self.assertRaises(ValueError):
            gate.hosted_identity()

    def test_no_live_environment_or_pool_overrides(self):
        for key in ("PLAYWRIGHT_BASE_URL", "SDLC_LIVE_QA", "SDLC_QA_SESSION_FILE", "VITEST_POOL", "NODE_OPTIONS"):
            with patch.dict(os.environ, {key: "override"}, clear=True), self.assertRaises(ValueError):
                gate.require_fixture_environment()

    def test_client_is_ignored_by_existing_convention(self):
        package = json.loads((ROOT / "frontend/package.json").read_bytes())
        self.assertEqual(package["scripts"]["postinstall"], "openapi-typescript ../openapi/openapi.json -o src/api/generated.ts")
        self.assertIn(gate.GENERATED, (ROOT / ".gitignore").read_text().splitlines())
        self.assertEqual(package["dependencies"]["@sdlc/ui"], "file:../../services-base/frontend")

    def test_qualified_inventory_cannot_drift(self):
        state = dict(source={"frontend/pnpm-lock.yaml": "a" * 64}, base={"frontend/pnpm-lock.yaml": "b" * 64})
        self.assertNotEqual(gate.qualified_inputs(state), gate.QUALIFIED_INPUTS)
        original = gate.qualified_inputs(state)
        state["base"]["frontend/src/ui/button.tsx"] = "c" * 64
        self.assertNotEqual(gate.qualified_inputs(state), original)

    def test_fixture_successor_exact_source_delta_inventory_and_original_key_body(self):
        frozen = "bf0ca7a182ed6344483a0ee4ce3a4333b4fc58c3"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", frozen).decode().strip(),
                         "d40b5a8f5e8cff28e54a670b1dc117ec6ee4bb1e 2398ff09974a1fbcdaf8f898568edb4f85bf9ed1")
        gate.qualify_source(ROOT)
        changes = gate.git(ROOT, "diff", "--no-renames", "--name-status",
                           "5cc1fbb75f9096f10dc32250bf7bab5c36386f9c", frozen)
        self.assertEqual(changes.decode().splitlines(), [
            "M\tfrontend/e2e/fleet-control.spec.ts",
            "M\tfrontend/e2e/runtime-controls.spec.ts",
        ])
        inventory = git_blob_inventory(frozen)
        self.assertEqual(len(inventory), 835)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "bb4ee11daf691177c14abc7a408372a899be301ab96cd11af47ba0bddb162263")
        runtime = gate.git(ROOT, "show", frozen + ":frontend/e2e/runtime-controls.spec.ts").decode()
        for assertion in (
            "expect(commands[1]).toEqual(commands[0])",
            "expect(commands[2]).toEqual(commands[0])",
            "expect(sameAnswerRequest(command.request, payload)).toBe(true)",
            "expect(command.request).toEqual(payload)",
            "expect(original.state).toBe('uncertain')",
            "expect(original.answer).toBeNull()",
            "expect(stored).toEqual([original])",
            "expect(deliveries).toEqual(Array(4).fill({ path: deliveryPath, body: null }))",
        ):
            self.assertIn(assertion, runtime)
        self.assertEqual(runtime.count("stored.push("), 1)
        self.assertNotIn("clarifications/q1/answers", runtime)
        self.assertNotIn("state: 'delivered'", runtime)
        chats = gate.git(ROOT, "show", frozen + ":frontend/e2e/fleet-control.spec.ts").decode()
        self.assertIn("expect(fixture.commands).toEqual([original])", chats)
        self.assertIn("body: original.request", chats)
        self.assertIn("const retainedAnswer = page.getByRole('status').filter({ has: resume })", chats)

    def test_capture_permissions_successor_exact_source_closure_and_mandatory_native_tests(self):
        frozen = "60f35db0b73922a0d5f753d370e556edcf4d20b0"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", frozen).decode().strip(),
                         "78547522533dcf5c5bc6b8d5e750e4bd94b96e3c f9644cb1963dee198a0abd7ccd534ade1f9f3110")
        frozen_inventory = git_blob_inventory(frozen)
        self.assertEqual(len(frozen_inventory), 837)
        self.assertEqual(gate.digest(gate.canonical(frozen_inventory)),
                         "32e20f4613f580cea502ba03c4ed854df16f6573cc18bcd645214bbdee848df1")
        baseline = "59d00fe3269d67ab09819f19c6b2b5703a6e2268"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", baseline).decode().strip(),
                         "18d42f2c02af23a369f32d177829b84c5fad0978 cd3027575a37f5401e4a3b2970b134aa5a506312")
        inventory = git_blob_inventory(baseline)
        self.assertEqual(len(inventory), 837)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "72ef3e8d14cfc66642694ea13c14cf6eb386c1e0b15dd554b1fc1bdb589c3f27")
        previous = "7dd60204bd352f5dbf3e61b8c2db14706450eea4"
        self.assertEqual(gate.git(ROOT, "diff", "--no-renames", "--name-status", previous,
                                  frozen).decode().splitlines(), [
            "M\tdocs/plans/2026-10-09-parallel-remaining-work.md",
            "M\tfrontend/package.json",
            "M\tfrontend/scripts/capture-screenshots.mjs",
            "A\tfrontend/scripts/capture-screenshots.test.mjs",
            "M\tfrontend/src/pages/chat-detail/index.test.tsx",
            "M\tfrontend/src/pages/chat-detail/index.tsx",
        ])
        self.assertEqual(gate.git(ROOT, "diff", "--no-renames", "--name-status", frozen,
                                  baseline).decode().splitlines(), [
            "M\tdocs/CURRENT_STATE.md",
            "M\tdocs/GAP_REGISTER.md",
            "M\tdocs/REMAINING_DELIVERY_WORK.md",
            "M\tdocs/contracts/CHAT_CLARIFICATION_CONTRACT.md",
            "M\tdocs/contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md",
            "M\tdocs/plans/2026-10-09-parallel-remaining-work.md",
            "M\tfrontend/src/pages/chat-detail/index.test.tsx",
        ])
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", previous, baseline,
                                  "--", "frontend/e2e"), b"")
        package = json.loads(gate.git(ROOT, "show", previous + ":frontend/package.json"))
        package["scripts"]["screenshots:verify"] = (
            "node --test scripts/capture-screenshots.test.mjs && node scripts/verify-screenshots.mjs")
        self.assertEqual(json.loads((ROOT / "frontend/package.json").read_bytes()), package)
        self.assertEqual(gate.GATES["screens-before"], [["pnpm", "screenshots:verify"]])
        self.assertEqual(gate.GATES["screens-after"], [["pnpm", "screenshots:verify"]])
        native = (ROOT / "frontend/scripts/capture-screenshots.test.mjs").read_text(encoding="utf-8")
        self.assertEqual(re.findall(r"^test\('([^']+)'", native, re.M), [
            "chat capture can read the control journal for every fixture session run",
            "unknown and mismatched run targets remain unhandled",
            "read-only control fixture does not accept mutations or lookups",
            "capture retains 45 views at each of the three required viewports",
        ])
        self.assertNotRegex(native, r"\btest\.(skip|only|todo)\b")
        self.assertIn("assert.equal(unhandled.size, 0", native)
        self.assertIn("['POST', 'PUT', 'PATCH', 'DELETE']", native)
        capture = gate.git(ROOT, "show", frozen + ":frontend/scripts/capture-screenshots.mjs").decode()
        original_capture = gate.git(ROOT, "show", previous + ":frontend/scripts/capture-screenshots.mjs").decode()
        start = capture.index("    const runtimeControlsMatch = pathName.match(")
        end = capture.index("    const sessionLeaderMatch =", start)
        self.assertEqual(capture[:start] + capture[end:], original_capture)
        for guard in ("method === 'GET'", "runtimeControlsMatch &&",
                      "sessionRuns[runtimeControlsMatch[1]]?.some((run) => run.id === runtimeControlsMatch[2])"):
            self.assertIn(guard, capture[start:end])
        self.assertEqual(len(gate.capture_paths(ROOT)), 135)
        source = gate.git(ROOT, "show", baseline + ":frontend/src/pages/chat-detail/index.tsx").decode()
        original_source = gate.git(ROOT, "show", previous + ":frontend/src/pages/chat-detail/index.tsx").decode()
        self.assertEqual(source.replace("                        answerCommands.isError ||\n", "", 1)
                         .replace("                        questions.isError ||\n                        answerCommands.isError\n",
                                  "                        questions.isError\n", 1), original_source)
        tests = gate.git(ROOT, "show", baseline + ":frontend/src/pages/chat-detail/index.test.tsx").decode()
        before = (
            "      expect(screen.getByRole('tab', { name: /Требования/ })).toHaveFocus()\n"
            "      act(() => screen.getByRole('combobox', { name: 'Редакция требований' }).focus())")
        after = (
            "      const requirements = screen.getByRole('tab', { name: /Требования/ })\n"
            "      await waitFor(() => expect(requirements).toHaveAttribute('aria-selected', 'true'))\n"
            "      expect(requirements).toHaveFocus()\n"
            "      const revisionSelect = await screen.findByRole('combobox', { name: 'Редакция требований' })\n"
            "      act(() => revisionSelect.focus())")
        self.assertEqual(tests.count(after), 1)
        self.assertEqual(tests.replace(after, before), gate.git(ROOT, "show", frozen +
                         ":frontend/src/pages/chat-detail/index.test.tsx").decode())
        tests = tests.replace(after, before)
        start = tests.index("  describe('answer custody permissions and session isolation'")
        end = tests.index("  it('requires explicit answer and does not publish after saving it'", start)
        self.assertEqual(tests[:start] + tests[end:], gate.git(ROOT, "show", previous +
                         ":frontend/src/pages/chat-detail/index.test.tsx").decode())
        self.assertEqual(len(re.findall(r"\bit\(", tests[start:end])), 4)
        self.assertIn("it.each(['session owner', 'Tracker access', 'journal access'])", tests[start:end])
        self.assertIn("expect(stored[0]!.request).toEqual(original![2])", tests[start:end])
        self.assertIn("expect(stored).toEqual([{ ...persisted, state: 'uncertain' }])", tests[start:end])
        self.assertNotIn("state: 'delivered'", tests[start:end])
        previous_helper = gate.git(ROOT, "show", "ea0d6338a63c1e08426ffdd3db315228d14c4e26:scripts/hosted_frontend_gate.py").decode()
        self.assertIn("QUALIFIED_UNIT_COUNTS = dict(files_passed=36, tests_passed=355, files_skipped=0, tests_skipped=0)",
                      previous_helper)

    def test_main_union_exact_source_inventory_and_additive_test_counts(self):
        frozen = "32b9f063f9b5099ff61bca24ecdfeb9952889034"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", frozen).decode().strip(),
                         "f2e319df5875bbd05d08f503c8727376498969cb d71b14f5f61bc58200a085d97aac0b80a891c324")
        inventory = git_blob_inventory(frozen)
        self.assertEqual(len(inventory), 872)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "f5a2256ec0dc268f1b90ac7f2a955a57ff1c411512edaa8387fdb08d454b777d")
        with self.assertRaisesRegex(ValueError, "Source bytes differ from exact committed tree"):
            gate.tracked_inventory(ROOT, frozen)
        union = "9e0bb491282ba8c13bc11b66b6d83cc59045a04d"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", union).decode().strip(),
                         "2d24440b5c1e464623f17297c9f4ceab8d3eaa39 7c7f9dd448cb103a47a74db6f4f84c73f3b68957 c39ff84d82277004bf8170fbac2f3b122ea6bcad")
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", union, frozen,
                                  "--", "frontend", "openapi"), b"")
        baseline = git_blob_inventory("59d00fe3269d67ab09819f19c6b2b5703a6e2268")
        tests = lambda items: {p: h for p, h in items.items()
                               if re.fullmatch(r"frontend/src/.+\.test\.(ts|tsx)", p)}
        old, current = tests(baseline), tests(inventory)
        self.assertEqual((len(old), len(current)), (36, 38))
        self.assertEqual(set(old) - set(current), set())
        self.assertEqual({p for p in current if current[p] != old.get(p)}, {
            "frontend/src/pages/chat-detail/binding.test.tsx", "frontend/src/pages/chat-detail/core.test.ts",
            "frontend/src/pages/chat-detail/index.test.tsx", "frontend/src/pages/chats/index.test.tsx",
        })
        for path, plain, parameters in (
            ("chat-detail/binding.test.tsx", 5, []),
            ("chat-detail/core.test.ts", 10, [
                "['pending', 'running', 'waiting', 'stopping']", "['pending', 'dispatched']",
                "['runtime_session_id', 'runtime_run_id'] as const"]),
            ("chats/index.test.tsx", 19, []),
        ):
            body = gate.git(ROOT, "show", frozen + ":frontend/src/pages/" + path).decode()
            self.assertEqual(len(re.findall(r"\bit\(", body)), plain)
            self.assertEqual(re.findall(r"\bit\.each\((.*?)\)\(", body, re.S), parameters)
            self.assertNotRegex(body, r"\b(?:it|test|describe)\.(?:skip|only|todo)\b")
        # Parent's actual focused 102 = detail 60 + directory 19 + core 18 + binding 5.
        # Only those four files differ from the hosted 355/36 baseline: 355 - 60 - 15 + 102.
        historical_helper = gate.git(ROOT, "show", "a5c01429e7db8a71db6b7629fa9f11256cb618b0:scripts/hosted_frontend_gate.py").decode()
        self.assertIn(f"QUALIFIED_UNIT_COUNTS = dict(files_passed=38, tests_passed={355 - 60 - 15 + 102}, files_skipped=0, tests_skipped=0)",
                      historical_helper)
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", union, frozen,
                                  "--", "frontend/playwright.config.ts"), b"")

    def test_pm_stream_union_exact_source_inventory_and_original_frontend(self):
        source = "83091f055e3b34fcfe6a6d59b1703c117261c027"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", source).decode().strip(),
                         "12f57611eae3d7b9bf73e3ebca41afd60cd36a1a af0a9d14360bc91875e54602c3340d5fcd8dbd59 8e6c25b9f77ed5f43d52661d8fdd6b2019a93804")
        inventory = git_blob_inventory(source)
        self.assertEqual(len(inventory), 885)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "6c11bea33f3f3856e4eec47db2669bc678378147cf7f54e0bf3407089b314246")
        with self.assertRaisesRegex(ValueError, "Source bytes differ from exact committed tree"):
            gate.tracked_inventory(ROOT, source)
        frozen = "32b9f063f9b5099ff61bca24ecdfeb9952889034"
        path = "frontend/e2e/chats-directory.spec.ts"
        self.assertEqual(gate.git(ROOT, "diff", "--name-only", frozen, source,
                                  "--", "frontend", "openapi").decode().splitlines(), [path])
        old = gate.git(ROOT, "show", frozen + ":" + path).decode()
        current = gate.git(ROOT, "show", source + ":" + path).decode()
        self.assertEqual(gate.digest(current.encode()), "e20b01fb9cd482bcdf26c5c601e3da188dafe12117e5d01673b810c87a623c55")
        marker = "  await page.goto('/chats')"
        self.assertEqual(old[old.index(marker):], current[current.index(marker):])
        self.assertIn("await installSsoMocks(page, () => owner)", current)
        self.assertNotIn("async function mockLogin", current)
        for setting in ("'access-control-allow-origin': '*'", "'access-control-allow-methods': 'GET, POST, OPTIONS'",
                        "'access-control-allow-headers': 'Authorization, Content-Type, Last-Event-ID'",
                        "route.request().method() === 'OPTIONS'", "const reply = (json: unknown)"):
            self.assertIn(setting, current)
        self.assertRegex(current, r"contentType: 'text/event-stream',\s+headers,")

    def test_directory_messages_successor_exact_source_and_preserved_assertions(self):
        source = "089ee0c7066adf459849556f511d81dc859cb6c3"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", source).decode().strip(),
                         "8f288ba7db64911fc5ca8f3703ee981528e557f6 facb25b9eae66db0c8b762ab68a5963422edf58f a8b045a080dd11da9827279c7cd79088f30542e7")
        inventory = git_blob_inventory(source)
        self.assertEqual(len(inventory), 885)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "53a917fdffc74d764532760ad1b80ac2e97df2b054ab63c6c6617d034319ad6a")
        previous = "83091f055e3b34fcfe6a6d59b1703c117261c027"
        path = "frontend/e2e/chats-directory.spec.ts"
        self.assertEqual(gate.git(ROOT, "diff", "--name-only", previous, source,
                                  "--", "frontend", "openapi").decode().splitlines(),
                         [path, "openapi/openapi.json"])
        old = gate.git(ROOT, "show", previous + ":" + path).decode()
        current = gate.git(ROOT, "show", source + ":" + path).decode()
        self.assertEqual(gate.digest(current.encode()), "0cb8a1963ec1fb2c5ab73a2077d1318150ae83fd6dbe51a08c257bfcf0373401")
        added = "    if (url.pathname === `/api/v1/sessions/${last}/messages` && route.request().method() === 'GET')\n      return reply([])\n"
        self.assertEqual(current.replace(added, "", 1), old)
        self.assertEqual(current.count("expect("), 27)
        self.assertLess(current.index(added), current.index("if (url.pathname !== '/api/v1/chats/directory') return reply({})"))

    def test_pm_decoder_successor_preserves_all_units_and_adds_three_cases(self):
        source = "34ee5f0b8f4c7f65d9b1f1503b38c21316c1b49b"
        previous = "089ee0c7066adf459849556f511d81dc859cb6c3"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", source).decode().strip(),
                         "fc74ed299d89cc872874ae7314eb2f3a16de419b " + previous)
        inventory = git_blob_inventory(source)
        self.assertEqual(len(inventory), 885)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "5f4e020c89d16405b27aa7db01bfb0465a72d2eb6270673e455767be6a73546a")
        prior = git_blob_inventory(previous)
        tests = lambda items: {p: h for p, h in items.items()
                               if re.fullmatch(r"frontend/src/.+\.test\.(ts|tsx)", p)}
        old_tests, new_tests = tests(prior), tests(inventory)
        self.assertEqual(set(new_tests), set(old_tests))
        self.assertEqual(len(new_tests), 38)
        path = "frontend/src/api/pm-drafts.test.ts"
        self.assertEqual({p for p in new_tests if new_tests[p] != old_tests[p]}, {path})
        old = gate.git(ROOT, "show", previous + ":" + path).decode()
        current = gate.git(ROOT, "show", source + ":" + path).decode()
        self.assertEqual(gate.digest(current.encode()), "cbe9af9b55ae82bc231321eb129adbbf7917df2d05cfacd6ce551403f49182cc")
        start = current.index("  it.each(['awaiting_runtime_acceptance', 'runtime_accepted'] as const)(")
        end = current.index("  it('rejects invented dispatch, incomplete identities and invalid partial-success states'")
        self.assertEqual(current[:start] + current[end:], old)
        addition = current[start:end]
        self.assertEqual(len(re.findall(r"\bit\(", old)), 5)
        self.assertEqual(len(re.findall(r"\bit\(", addition)), 1)
        self.assertEqual(re.findall(r"\bit\.each\((.*?)\)\(", addition, re.S),
                         ["['awaiting_runtime_acceptance', 'runtime_accepted'] as const"])
        self.assertNotRegex(addition, r"\b(?:it|test|describe)\.(?:skip|only|todo)\b")
        for p in prior:
            if p.startswith("frontend/e2e/") or p in ("frontend/playwright.config.ts", "openapi/openapi.json"):
                self.assertEqual(inventory[p], prior[p], p)
        historical_helper = gate.git(ROOT, "show", "66a446c91e03a8bc9161f3f6ebdb2f2b4de9e706:scripts/hosted_frontend_gate.py").decode()
        self.assertIn("QUALIFIED_UNIT_COUNTS = dict(files_passed=38, tests_passed=385, files_skipped=0, tests_skipped=0)",
                      historical_helper)

    def test_human_controls_successor_source_inventory_and_additive_units(self):
        source = "d4584769c925c2a92829251960b85a14b63c31dd"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", source).decode().strip(),
                         "33bfce243f52ac198ef88be37c331052c88c1451 153242c5cbe3dfd93eca65529c81c6e422ec9fff")
        inventory = git_blob_inventory(source)
        self.assertEqual(len(inventory), 888)
        self.assertEqual(gate.digest(gate.canonical(inventory)),
                         "c7a996dc1de77be51df3fa8d10aae689e85aa43a6100a8a287265e3fe2485430")
        previous = "34ee5f0b8f4c7f65d9b1f1503b38c21316c1b49b"
        prior = git_blob_inventory(previous)
        paths = [p for p in inventory if re.fullmatch(r"frontend/src/.+\.test\.(ts|tsx)", p)]
        self.assertEqual(len(paths), 38)
        self.assertEqual(set(paths), {p for p in prior if re.fullmatch(r"frontend/src/.+\.test\.(ts|tsx)", p)})
        changed = {p for p in paths if inventory[p] != prior[p]}
        self.assertEqual(changed, {"frontend/src/api/task-chats.test.ts", "frontend/src/pages/chat-detail/index.test.tsx"})
        for path, marker, end_marker in (
            ("frontend/src/api/task-chats.test.ts", "\ndescribe('answer continuation recovery inventory'", None),
            ("frontend/src/pages/chat-detail/index.test.tsx", "\ndescribe('PM delivered answer continuation receipt'", "describe('production chat'"),
        ):
            old = gate.git(ROOT, "show", previous + ":" + path).decode()
            current = gate.git(ROOT, "show", source + ":" + path).decode()
            start = current.index(marker)
            if end_marker is None:
                # API imports expand, but the entire original describe/body remains exact.
                body = "describe('clarification answer validation'"
                self.assertEqual(current[current.index(body):start].rstrip(), old[old.index(body):].rstrip())
            else:
                end = current.index(end_marker, start)
                self.assertEqual(current[:start] + current[end:], old)
            self.assertNotRegex(current, r"\b(?:it|test|describe)\.(?:skip|only|todo)\b")
        fixture = "frontend/e2e/chats-directory.spec.ts"
        self.assertEqual(inventory[fixture], "3a0d2fc1f243b52e1f6a310e1f46940218a2146de34cf76b3d3a01a45f4adb0b")
        old_fixture = gate.git(ROOT, "show", previous + ":" + fixture).decode()
        current_fixture = gate.git(ROOT, "show", source + ":" + fixture).decode()
        self.assertEqual(current_fixture, old_fixture.replace("\u0412\u0435\u0440\u043d\u0443\u0442\u044c\u0441\u044f \u043a \u0447\u0430\u0442\u0430\u043c", "\u041d\u0430\u0437\u0430\u0434 \u043a \u0447\u0430\u0442\u0430\u043c", 1))
        self.assertEqual(current_fixture.count("expect("), 27)
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", previous, source,
                                  "--", "frontend/playwright.config.ts", "frontend/pnpm-lock.yaml",
                                  "frontend/src/previews/pm-draft", "frontend/e2e/pm-draft-preview.spec.ts"), b"")

        qualified_source = "3c900b00f15aeda2016a45d080d850fa028cdcfd"
        historical_schema = "e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76"
        self.assertEqual(gate.git(ROOT, "show", "-s", "--format=%T %P", qualified_source).decode().strip(),
                         "b975beb628bb68c03fb5bce45e7c43085837e41f ad2b6ac1a2d4f286edd00eeb1e1ecc137c1f3223")
        final = git_blob_inventory(qualified_source)
        self.assertEqual(len(final), 895)
        self.assertEqual(gate.digest(gate.canonical(final)),
                         "500c6e9adfbb5b52ebca9edaea93261bdefcf36ca765d0691f28b8e272411447")
        self.assertEqual({p: final[p] for p in paths}, {p: inventory[p] for p in paths})
        history = "frontend/e2e/fleet-control.spec.ts"
        runtime_fixture = "frontend/e2e/runtime-controls.spec.ts"
        self.assertEqual(gate.git(ROOT, "diff", "--name-only", source, qualified_source,
                                  "--", "frontend", "openapi").decode().splitlines(), [history, runtime_fixture])
        old_history = gate.git(ROOT, "show", source + ":" + history).decode()
        final_history = gate.git(ROOT, "show", qualified_source + ":" + history).decode()
        self.assertEqual(final[history], "e08e845d51739a8bfbbe7c698649290cfb6f5cf9acece60f261866516b052707")
        start = old_history.index("test('chat history preserves server order after clock rollback and page overlap'")
        old_setup = "  const state = createState()\n  await installMocks(page, state)\n"
        self.assertEqual(final_history, old_history[:start] + old_history[start:].replace(
            old_setup, "  await installChatUxFixtures(page)\n", 1))
        self.assertEqual(final_history.count("expect("), old_history.count("expect("))
        fixture_fix = "8f53740e6102c2fc9c5547aa572a8f413c2d9d8b"
        self.assertEqual(final[runtime_fixture], "49264145bb9ebfef37a24e28ef37fd7c1711b4ead7b286e99aa7f9726ed68723")
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", fixture_fix, qualified_source,
                                  "--", runtime_fixture), b"")
        old_runtime = gate.git(ROOT, "show", "c59dbea1ab73a782a28cf6106155c84d73e2e14d:" + runtime_fixture).decode()
        new_runtime = gate.git(ROOT, "show", qualified_source + ":" + runtime_fixture).decode()
        clarification = "test('fixture: uncertain clarification"
        self.assertEqual(new_runtime[new_runtime.index(clarification):], old_runtime[old_runtime.index(clarification):])
        self.assertEqual(new_runtime.count("expect("), old_runtime.count("expect(") + 1)
        self.assertIn("json: { binding: null, tracker: null }", new_runtime)
        self.assertIn("data-runtime-controls-owner=\"true\"", new_runtime)
        for route in ("/messages", "/stream"):
            self.assertIn("path.endsWith('" + route + "')", new_runtime[:new_runtime.index(clarification)])
        # Historical receipt 38048577514 remains scoped to c59.
        prior_source = "c59dbea1ab73a782a28cf6106155c84d73e2e14d"
        codegen_source = "4449a3b1cdd915e265543a24054506f15385393d"
        self.assertEqual(gate.digest(gate.git(ROOT, "show", codegen_source + ":openapi/openapi.json")),
                         historical_schema)
        self.assertEqual(gate.git(ROOT, "diff", "--name-only", codegen_source, prior_source,
                                  "--", "backend").decode().splitlines(),
                         ["backend/infra/src/runtime/pm_recovery_pg_tests.rs",
                          "backend/infra/src/runtime/pm_recovery_tests.rs"])
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", codegen_source, prior_source,
                                  "--", "backend/api", "backend/app", "backend/domain", "backend/shared",
                                  "backend/Cargo.toml", "backend/Cargo.lock", ".base-revision", "openapi"), b"")
        # New authenticated Rust codegen: run38052082418/1, workflow91a1c48c7a3f0cfad33586fb55c45f47ff864bc2.
        # Artifact11669374937 ZIP8283b599e66dc66d5a05e961ae2b59bc5dffc1c408f7d2f35352608198730701.
        # Provenance64cdd95b3631c573af4c991a601948caff30b62cee3491735880814d6b8b36e1;
        # canonical303 inputs4ab1c70324a3b184096a69ed1dbdd72bcbf5f993fbdc1ecc6b01fb643f650457.
        codegen_source = "3c900b00f15aeda2016a45d080d850fa028cdcfd"
        self.assertEqual(codegen_source, qualified_source)
        self.assertEqual(gate.digest(gate.git(ROOT, "show", codegen_source + ":openapi/openapi.json")),
                         historical_schema)
        self.assertEqual(gate.git(ROOT, "rev-parse", codegen_source + ":backend/api/src/routes/task_chats.rs").decode().strip(),
                         "4db07a426b6f7d38e518bd2adc0e28b6067bbdbe")

    def test_current_unit_counts_expand_all_source_declarations(self):
        self.assertEqual(gate.SOURCE_SHA, "f9c5fa5c8014e51818bb9e67ced84edd40e7e892")
        self.assertEqual(gate.SOURCE_TREE, "60e2fc0701a983b0b31294d0ea8b1330e795ed22")
        self.assertEqual(gate.SOURCE_PARENTS, ["ce4153f453e730dad1e315131d65ca030243264c"])
        gate.qualify_source(ROOT)
        inventory = git_blob_inventory(gate.SOURCE_SHA)
        self.assertEqual(len(inventory), 903)
        self.assertEqual(gate.digest(gate.canonical(inventory)), gate.QUALIFIED_INPUTS["source_inventory_sha256"])
        self.assertEqual(gate.tracked_inventory(ROOT, gate.SOURCE_SHA), inventory)
        # Authentic Rust codegen38058114502/1, workflowe8fd29e4d45c749ac601d743a09400867f9635e6.
        # Artifact11671964606 ZIPd209ef81e0f962dae8f1465852a0b06dcb6faacdd350ff85b5fda95a980c75cf.
        # Provenancee3a535e094def852111cb63df3fc3ebb09c335121aed75b50ec3aa8959aa1276;
        # canonical303 inputs95ab4d1ff7b20675b38dfdab686360ab6348bde52409b92849c60475b35a5546.
        self.assertEqual(gate.digest(gate.git(ROOT, "show", gate.SOURCE_SHA + ":openapi/openapi.json")),
                         gate.SCHEMA_SHA256)
        for path, blob in {
            ".base-revision": "1716308f859d23508a6ca0bae105434221c00419",
            "backend/Cargo.lock": "1f2a6fee32bf3dabedafc3927c56c286e14c6daf",
            "backend/api/src/bin/gen_openapi.rs": "254b94c98ee232763c040fb50629080e4282af3f",
            "backend/api/src/lib.rs": "d5b488ca8ed99d4f7e1bddf8e0498e4490fce9f4",
            "backend/api/src/routes/agents.rs": "f512a6f5e4a0fb2c5e86e9c64fca8d827bb828da",
            "backend/api/src/routes/clarification_commands.rs": "51513bee1b4cf823bf48d4482a47f1729c33ca62",
            "backend/api/src/routes/sdlc_configuration.rs": "a9478a23db4e6f9d7db35b8342f382d1fa50775b",
            "backend/api/src/routes/sessions.rs": "faca62d5e4d75157f3abf080aa1a6631b1bc00d3",
            "backend/api/src/routes/task_chats.rs": "a4a632956aacc9b80c1773c99e0d7a89522a361f",
            "backend/domain/src/clarification_commands.rs": "0c5d1610f2cd649cf1754489df6bac556bb19686",
            "backend/domain/src/lib.rs": "a0c51b3e6683463698db5b7596c7565a90ed7ae4",
        }.items():
            self.assertEqual(gate.git(ROOT, "rev-parse", gate.SOURCE_SHA + ":" + path).decode().strip(), blob)
        index = "frontend/src/pages/chat-detail/index.test.tsx"
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code",
                                  "9d7775cd1b871a0de5579a5047268b244ba8e395", gate.SOURCE_PARENTS[0],
                                  "--", index), b"")
        self.assertEqual(gate.git(ROOT, "diff", "--name-only", gate.SOURCE_PARENTS[0], gate.SOURCE_SHA),
                         (index + "\n").encode())
        self.assertEqual(inventory[index], "4dc0d8da86896dbf814845250b8b62c0bac4c539d0856423aa32a757bdad98a4")
        for removed in ("frontend/src/pages/chat-detail/run-controls.test.tsx",
                        "frontend/src/shared/chat-control-recovery.test.ts"):
            self.assertNotIn(removed, inventory)

        def rows(expression):
            # Count only top-level literal-array rows; do not execute TypeScript.
            stack, quote, escaped, count = [], None, False, 0
            for char in expression[expression.index("["):]:
                if quote:
                    if escaped:
                        escaped = False
                    elif char == "\\":
                        escaped = True
                    elif char == quote:
                        quote = None
                    continue
                if char in "'\"`":
                    quote = char
                elif char in "[({":
                    stack.append(char)
                elif char in "])}":
                    stack.pop()
                    if not stack:
                        return count + (1 if last != "," else 0)
                elif char == "," and len(stack) == 1:
                    count += 1
                if not char.isspace():
                    last = char
            self.fail("Unclosed source test array")

        totals = {}
        for path in inventory:
            if not re.fullmatch(r"frontend/src/.+\.test\.(ts|tsx)", path):
                continue
            text = gate.git(ROOT, "show", gate.SOURCE_SHA + ":" + path).decode()
            direct = len(re.findall(r"\b(?:it|test)\(", text))
            arrays = re.findall(r"\b(?:it|test)\.each\((.*?)\)\(", text, re.S)
            self.assertEqual(len(arrays), len(re.findall(r"\b(?:it|test)\.each\(", text)), path)
            self.assertNotRegex(text, r"\b(?:it|test|describe)\.(?:skip|only|todo)\b")
            totals[path] = direct + sum(rows(array) for array in arrays)
        self.assertEqual(totals["frontend/src/api/task-chats.test.ts"], 20)
        self.assertEqual(totals[index], 87)
        self.assertEqual(totals["frontend/src/pages/chat-detail/control-journal.test.ts"], 20)
        self.assertEqual(totals["frontend/src/pages/chat-detail/answer-payload.test.ts"], 2)
        self.assertEqual(totals["frontend/src/pages/chat-detail/dispatch-recovery.test.tsx"], 16)
        self.assertEqual((len(totals), sum(totals.values())), (41, 460))
        self.assertEqual(gate.QUALIFIED_UNIT_COUNTS,
                         dict(files_passed=len(totals), tests_passed=sum(totals.values()), files_skipped=0, tests_skipped=0))


class CompletionContracts(unittest.TestCase):
    def test_unit_summary_needs_completed_tests(self):
        for output in (b"RUN v4\nWaiting for worker...", b"Test Files 0 passed\nTests 0 passed",
                       b"Test Files 2 passed | 1 failed\nTests 7 passed | 1 failed", b"Aborted"):
            with self.subTest(output=output), self.assertRaises(ValueError):
                gate.unit_counts(output)

    def test_unit_counts_record_preserved_skips(self):
        self.assertEqual(gate.unit_counts(b"\x1b[32mTest Files 2 passed (2)\nTests 8 passed | 1 skipped (9)\x1b[0m"),
                         dict(files_passed=2, tests_passed=8, files_skipped=0, tests_skipped=1))

    def test_exact_frozen_baseline_counts(self):
        self.assertEqual(gate.unit_counts(b"Test Files 41 passed (41)\nTests 460 passed (460)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 38 passed (38)\nTests 411 passed (411)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 38 passed (38)\nTests 385 passed (385)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 38 passed (38)\nTests 382 passed (382)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 36 passed (36)\nTests 355 passed (355)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 36 passed (36)\nTests 348 passed (348)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 36 passed (36)\nTests 337 passed (337)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 35 passed (35)\nTests 336 passed (336)"), gate.QUALIFIED_UNIT_COUNTS)

    @staticmethod
    def browser_report():
        return dict(errors=[], suites=[dict(specs=[dict(tests=[
            dict(projectName=name, status="expected", expectedStatus="passed", results=[dict(status="passed")])
            for name in ("chromium", "firefox", "webkit")])])])

    def test_actual_three_browser_completion(self):
        counts = gate.browser_counts(self.browser_report())
        self.assertEqual(set(counts), {"chromium", "firefox", "webkit"})
        self.assertTrue(all(p["passed"] == 1 for p in counts.values()))

    def test_no_empty_missing_failed_or_expected_failure_browser(self):
        for kind in ("empty", "missing", "failed", "expected-failure", "global-error"):
            report = self.browser_report()
            test = report["suites"][0]["specs"][0]["tests"][0]
            if kind == "empty":
                report["suites"] = []
            elif kind == "missing":
                report["suites"][0]["specs"][0]["tests"].pop()
            elif kind == "failed":
                test["status"] = "unexpected"
            elif kind == "expected-failure":
                test.update(expectedStatus="failed", results=[dict(status="failed")])
            else:
                report["errors"] = [dict(message="private diagnostics")]
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                gate.browser_counts(report)

    def test_existing_retries_and_live_skips_are_reported(self):
        report = self.browser_report()
        tests = report["suites"][0]["specs"][0]["tests"]
        tests[0].update(status="flaky", results=[dict(status="failed"), dict(status="passed")])
        tests.append(dict(projectName="webkit", status="skipped", expectedStatus="skipped", results=[dict(status="skipped")]))
        counts = gate.browser_counts(report)
        self.assertEqual(counts["chromium"]["flaky"], 1)
        self.assertEqual(counts["webkit"]["skipped"], 1)

    def test_fail_fast_cannot_complete_gate_on_nonzero_or_failed_report(self):
        for code in (1, 0):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as folder:
                private = Path(folder)
                prefix = list(gate.GATES)[:list(gate.GATES).index("fixtures")]
                state = dict(gates=prefix.copy())
                report = self.browser_report()
                report["suites"][0]["specs"][0]["tests"][0].update(
                    status="unexpected", results=[dict(status="failed")])
                with patch.object(gate, "load_state", return_value=(ROOT, private, state)), \
                        patch.object(gate, "verify_parity", return_value=(ROOT, ROOT)), \
                        patch.object(gate.subprocess, "run", return_value=MagicMock(returncode=code)) as run, \
                        patch.object(gate, "bounded_file", return_value=gate.canonical(report)), \
                        patch.object(gate, "save_state") as save:
                    with self.assertRaises(gate.GateFailure if code else ValueError) as caught:
                        gate.gate("fixtures")
                    if code:
                        self.assertEqual((caught.exception.category, caught.exception.code), ("exit", code))
                    self.assertEqual(state["gates"], prefix)
                    save.assert_not_called()
                    self.assertEqual(run.call_args.args[0], gate.GATES["fixtures"][0])
                    self.assertEqual(run.call_args.kwargs["timeout"], 1200)

    def test_fail_fast_timeout_still_cannot_complete_gate(self):
        with tempfile.TemporaryDirectory() as folder:
            state = dict(gates=list(gate.GATES)[:list(gate.GATES).index("fixtures")])
            with patch.object(gate, "load_state", return_value=(ROOT, Path(folder), state)), \
                    patch.object(gate, "verify_parity", return_value=(ROOT, ROOT)), \
                    patch.object(gate.subprocess, "run", side_effect=subprocess.TimeoutExpired("private", 1200)), \
                    patch.object(gate, "save_state") as save:
                with self.assertRaises(gate.GateFailure) as caught:
                    gate.gate("fixtures")
                self.assertEqual((caught.exception.category, caught.exception.code), ("timeout", None))
                self.assertNotIn("fixtures", state["gates"])
                save.assert_not_called()

    def test_reordered_gate_never_executes(self):
        with patch.object(gate, "load_state", return_value=(ROOT, ROOT, dict(gates=[]))), \
                patch.object(gate.subprocess, "run") as run, self.assertRaises(ValueError):
            gate.gate("unit")
        run.assert_not_called()

    def test_incomplete_pipeline_cannot_publish(self):
        with patch.object(gate, "load_state", return_value=(ROOT, ROOT, dict(gates=["base"]))), \
                self.assertRaises(ValueError):
            gate.finish()

    def test_schema_and_client_parity_after_tests(self):
        state = dict(source={}, base={}, generated_sha256=gate.digest(b"generated"))
        with patch.object(gate, "checkout"), patch.object(gate, "git", return_value=b""), \
                patch.object(gate, "bounded_file", side_effect=[b"schema", b"modified"]), \
                patch.object(gate, "SCHEMA_SHA256", gate.digest(b"schema")), self.assertRaises(ValueError):
            gate.verify_parity(ROOT, state)

    def test_tracked_base_drift_fails(self):
        state = dict(source={}, base={"frontend/package.json": gate.digest(b"original")})
        with patch.object(gate, "checkout"), patch.object(gate, "git", return_value=b""), \
                patch.object(gate, "bounded_file", return_value=b"modified"), self.assertRaises(ValueError):
            gate.verify_parity(ROOT, state)

    def test_main_fixture_screenshot_writes_are_exact_and_stage_bound(self):
        expected = {f"docs/assets/screens/chats-core-main-20261007/{name}-{viewport}.png"
                    for name in ("dialogue", "clarification-unavailable", "requirements-unavailable", "denied", "read-only")
                    for viewport in gate.VIEWPORTS}
        self.assertEqual(gate.SOURCE_FIXTURE_SCREENS, expected)
        self.assertEqual(len(expected), 15)
        body = (ROOT / "frontend/e2e/chats-core.spec.ts").read_text(encoding="utf-8")
        self.assertIn("if (info.project.name !== 'chromium') return", body)
        self.assertIn("const dir = resolve('../docs/assets/screens/chats-core-main-20261007')", body)
        self.assertIn("path: resolve(dir, `${name}-${viewport.width}x${viewport.height}.png`)", body)
        for name in ("'dialogue'", "'clarification-unavailable'", "'requirements-unavailable'", "'denied'", "'read-only'"):
            self.assertIn(name, body)
        schema = (ROOT / "openapi/openapi.json").read_bytes()
        state = dict(source={p: gate.digest(b"original") for p in expected}, base={}, screens=[], gates=[])
        state["source"]["openapi/openapi.json"] = gate.digest(schema)
        changed = set(expected)

        def git(root, *args):
            if root == ROOT / "fleet-control" and args[:3] == ("diff", "--name-only", "HEAD"):
                return "\n".join(sorted(changed)).encode()
            return b""

        def read(root, name):
            return schema if name == "openapi/openapi.json" else b"changed" if name in changed else b"original"

        with patch.object(gate, "checkout"), patch.object(gate, "git", side_effect=git), \
                patch.object(gate, "bounded_file", side_effect=read), patch.object(gate, "png_valid") as validate_png:
            with self.assertRaises(ValueError):
                gate.verify_parity(ROOT, state)
            gate.verify_parity(ROOT, state, fixtures=True)
            self.assertEqual(validate_png.call_count, 15)
            state["gates"] = ["fixtures"]
            gate.verify_parity(ROOT, state)
            validate_png.side_effect = ValueError("Invalid/bounded fixture PNG required")
            with self.assertRaises(ValueError):
                gate.verify_parity(ROOT, state)
            validate_png.side_effect = None
            for extra in ("docs/assets/screens/chats-core-main-20261007/private.log",
                          "docs/assets/screens/chats-core-main-20261007/unknown-375x812.png",
                          "docs/assets/screens/375x812/chats.png", "frontend/src/pages/chat-detail/index.tsx"):
                changed.add(extra)
                with self.subTest(extra=extra), self.assertRaises(ValueError):
                    gate.verify_parity(ROOT, state)
                changed.remove(extra)
            del state["source"][next(iter(expected))]
            with self.assertRaises(ValueError):
                gate.verify_parity(ROOT, state)

    def test_build_requires_index_js_css_and_records_content(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            dist = root / "frontend/dist"
            dist.mkdir(parents=True)
            with self.assertRaises(ValueError):
                gate.build_inventory(root)
            for name in ("index.html", "assets/main.js", "assets/main.css"):
                path = dist / name
                path.parent.mkdir(exist_ok=True)
                path.write_bytes(b"fixture-only")
            before = gate.build_inventory(root)
            self.assertEqual(before["build_file_count"], 3)
            (dist / "assets/main.js").write_bytes(b"changed")
            self.assertNotEqual(gate.build_inventory(root), before)


class WorkflowContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / gate.WORKFLOW).read_text()
        cls.original = (ROOT / ".github/workflows/ci.yml").read_text().split("\n  frontend:\n", 1)[1].split("\n  minimum-rust:", 1)[0]

    def test_all_original_frontend_commands_preserved(self):
        exceptions = {"base", "compat", "generated", "theme", "fixtures"}
        for name, commands in gate.GATES.items():
            if name not in exceptions:
                for command in commands:
                    with self.subTest(gate=name):
                        self.assertIn(" ".join(command), self.original)
        self.assertEqual(gate.GATES["unit"], [["pnpm", "test", "--", "--run"]])
        self.assertEqual(gate.GATES["fixtures"], [["pnpm", "exec", "playwright", "test", "--reporter=list,json", "--max-failures=1"]])
        self.assertIn("pnpm exec playwright test", self.original)
        self.assertIn("pnpm openapi:compat", self.original)
        self.assertEqual(gate.GATES["compat"][-1], ["pnpm", "openapi:compat"])
        self.assertIn("+refs/heads/main:refs/remotes/origin/main", gate.GATES["compat"][0])
        self.assertIn("--depth=1", gate.GATES["compat"][0])
        self.assertIn("pnpm theme:check http://127.0.0.1:4173", gate.THEME)
        self.assertIn("trap '", gate.THEME)
        self.assertIn("--strictPort", gate.THEME)

    def test_fail_fast_preserves_all_fixture_sources_and_selection(self):
        parent = "a909757575582dacc4e7b1c21eb7cef115aef3a8"
        paths = ("frontend/e2e", "frontend/playwright.config.ts")
        directory = b"frontend/e2e/chats-directory.spec.ts"
        retained = lambda sha: [entry for entry in gate.git(ROOT, "ls-tree", "-rz", sha, *paths).split(b"\0")
                                if entry and entry.split(b"\t", 1)[1] != directory]
        previous = "d4584769c925c2a92829251960b85a14b63c31dd"
        self.assertEqual(retained(previous), retained(parent))
        history = b"frontend/e2e/fleet-control.spec.ts"
        accepted = gate.git(ROOT, "ls-tree", "-z", "2a3491683df0e5e19f7a60114b0e99f36fbfa8d2",
                            history.decode()).rstrip(b"\0")
        runtime_fixture = b"frontend/e2e/runtime-controls.spec.ts"
        accepted_runtime = gate.git(ROOT, "ls-tree", "-z", "8f53740e6102c2fc9c5547aa572a8f413c2d9d8b",
                                    runtime_fixture.decode()).rstrip(b"\0")
        accepted_fixtures = {history: accepted, runtime_fixture: accepted_runtime}
        qualified_source = "3c900b00f15aeda2016a45d080d850fa028cdcfd"
        self.assertEqual(retained(qualified_source),
                         [accepted_fixtures.get(entry.split(b"\t", 1)[1], entry)
                          for entry in retained(previous)])
        # Exact parent-reviewed fixture adaptations, not a different selector/budget.
        final_fixtures = {
            b"frontend/e2e/chats-core.spec.ts": b"100644 blob 621d19c380ea1f18c0ac6274692f4e2ac14f98e6\tfrontend/e2e/chats-core.spec.ts",
            history: b"100644 blob 0dfd2f6cbdc8b6d1cee74300ba8ce7828081e140\tfrontend/e2e/fleet-control.spec.ts",
            runtime_fixture: b"100644 blob 7c19a2d03b0d075bb55c719d2b668c693c3d08df\tfrontend/e2e/runtime-controls.spec.ts",
        }
        self.assertEqual(retained(gate.SOURCE_SHA),
                         [final_fixtures.get(entry.split(b"\t", 1)[1], entry)
                          for entry in retained(qualified_source)])
        self.assertEqual(gate.git(ROOT, "diff", "--exit-code", qualified_source, gate.SOURCE_SHA,
                                  "--", "frontend/playwright.config.ts", "frontend/pnpm-lock.yaml"), b"")
        self.assertEqual(gate.GATES["fixtures"][0][:-1],
                         ["pnpm", "exec", "playwright", "test", "--reporter=list,json"])
        self.assertEqual(gate.GATES["fixtures"][0][-1], "--max-failures=1")
        config = gate.git(ROOT, "show", gate.SOURCE_SHA + ":frontend/playwright.config.ts").decode()
        for setting in ("testDir: 'e2e'", "retries: process.env.CI ? 2 : 0,",
                        "workers: process.env.CI ? 1 : undefined,", "name: 'chromium'",
                        "name: 'firefox'", "name: 'webkit'"):
            self.assertIn(setting, config)

    def test_workflow_gate_order_and_scope(self):
        names = re.findall(r"run: python3 -B controls/scripts/hosted_frontend_gate.py gate ([a-z-]+)", self.workflow)
        self.assertEqual(names, list(gate.GATES))
        self.assertIn("branches: [" + gate.BRANCH + "]", self.workflow)
        for forbidden in ("workflow_dispatch:", "pull_request:", "self-hosted", "continue-on-error", "contents: write",
                          "cargo ", "docker ", "--passWithNoTests", "--pool", "--grep", "--project="):
            self.assertNotIn(forbidden, self.workflow)

    def test_secret_is_after_preflight_and_read_only(self):
        self.assertLess(self.workflow.index("hosted_frontend_gate.py preflight"), self.workflow.index("secrets.SERVICES_BASE_TOKEN"))
        self.assertEqual(self.workflow.count("secrets.SERVICES_BASE_TOKEN"), 1)
        self.assertIn("permissions:\n  contents: read", self.workflow)
        self.assertEqual(self.workflow.count("persist-credentials: false"), 3)
        self.assertIn("ref: " + gate.SOURCE_SHA, self.workflow)
        self.assertIn("ref: " + gate.BASE_SHA, self.workflow)
        self.assertIn("node-version: \"" + gate.NODE + "\"", self.workflow)
        self.assertIn("version: " + gate.PNPM, self.workflow)

    def test_only_verified_official_action_shas(self):
        expected = {"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
                    "actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1",
                    "pnpm/action-setup@f520eceda224fe1a4aed5a2a27a194379a409996",
                    "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9"}
        self.assertEqual(set(re.findall(r"uses: (\S+)", self.workflow)), expected)

    def test_artifact_is_success_only_and_public_allowlist(self):
        block = self.workflow.split("- name: Upload qualified", 1)[1].split("- name: Record artifact", 1)[0]
        self.assertNotIn("always()", block)
        self.assertIn("path: ${{ runner.temp }}/fleet-frontend-public", block)
        self.assertIn("include-hidden-files: false", block)
        self.assertIn("if-no-files-found: error", block)
        self.assertIn("archive: true", block)
        self.assertIn("if: always()", self.workflow.split("- name: Remove only", 1)[1])


class EvidenceContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = fixture_files()
        cls.payload = zipped(cls.files)

    def test_authenticated_exact_run_readback(self):
        self.assertEqual(validate(self.payload), self.files)

    def test_failed_wrong_attempt_branch_workflow_repo_or_commit_rejected(self):
        for key, value in (("conclusion", "failure"), ("status", "in_progress"), ("event", "workflow_dispatch"),
                           ("run_attempt", 1), ("head_branch", "main"), ("path", ".github/workflows/ci.yml"),
                           ("head_sha", "f" * 40), ("repository", dict(full_name="other/repo"))):
            run, _ = readback_metadata(self.payload)
            run[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(self.payload, run=run)

    def test_wrong_artifact_identity_digest_or_expiry_rejected(self):
        for key, value in (("expired", True), ("id", 1), ("name", "other"), ("digest", "sha256:" + "0" * 64),
                           ("workflow_run", dict(id=123, head_sha="f" * 40, head_branch=gate.BRANCH))):
            _, artifact = readback_metadata(self.payload)
            artifact[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(self.payload, artifact=artifact)

    def test_zip_member_allowlist_blocks_source_logs_and_traversal(self):
        for name in ("../source.png", "/tmp/source.png", "fixtures\\secret.png", "source.rs", ".git/config",
                     "services-base/frontend/src/ui/button.tsx", "raw.log", "test-results/trace.zip",
                     "fixtures/a/secret.txt", "fixtures/a/nested/image.png"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                validate(zipped(self.files, (name, b"private")))

    def test_duplicate_members_rejected(self):
        with self.assertWarns(UserWarning):
            payload = zipped(self.files, ("provenance.json", self.files["provenance.json"]))
        with self.assertRaises(ValueError):
            validate(payload)

    def test_zip_symlink_rejected_before_read(self):
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            info = zipfile.ZipInfo("fixtures/example-chromium/link.png")
            info.create_system = 3
            info.external_attr = 0o120777 << 16
            archive.writestr(info, b"../../private")
        with self.assertRaises(ValueError):
            validate(output.getvalue())

    def test_compressed_and_uncompressed_bounds(self):
        with patch.object(gate, "MAX_ARTIFACT", len(self.payload) - 1), self.assertRaises(ValueError):
            validate(self.payload)
        with patch.object(gate, "MAX_FILE", 100), self.assertRaises(ValueError):
            validate(self.payload)
        with patch.object(gate, "MAX_MEMBERS", 2), self.assertRaises(ValueError):
            validate(self.payload)

    def test_file_checksum_tampering_rejected(self):
        files = dict(self.files, **{"fixture-summary.json": b"{}"})
        with self.assertRaises(ValueError):
            validate(zipped(files))

    def test_provenance_cannot_claim_live_acceptance_or_other_inputs(self):
        for key, value in (("all_sdlc_acceptance", True), ("live_pm_acceptance", True),
                           ("live_runtime_acceptance", True), ("source_parents", gate.SOURCE_PARENTS[:-1]),
                           ("schema_sha256", "874" + "0" * 61), ("base_sha", "f" * 40),
                           ("base_inventory_sha256", "0" * 64), ("gates", ["build"]),
                           ("build_file_count", 0), ("build_manifest_sha256", ""),
                           ("raw_private_source", "forbidden")):
            files = dict(self.files)
            provenance = json.loads(files["provenance.json"])
            provenance[key] = value
            files["provenance.json"] = gate.canonical(provenance)
            checksums(files)
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(zipped(files))

    def test_no_evidence_for_zero_unit_or_missing_browser(self):
        for kind in ("unit", "browser", "live"):
            files = dict(self.files)
            summary = json.loads(files["fixture-summary.json"])
            if kind == "unit":
                summary["unit"]["tests_passed"] = 0
            elif kind == "browser":
                summary["browsers"]["webkit"]["passed"] = 0
            else:
                summary["live_pm_acceptance"] = True
            files["fixture-summary.json"] = gate.canonical(summary)
            checksums(files)
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                validate(zipped(files))

    def test_missing_browser_screenshot_or_manifest_entry(self):
        for name in ("fixtures/example-webkit/fixture.png", next(n for n in self.files if n.startswith("screens/") and n.endswith(".png"))):
            files = dict(self.files)
            del files[name]
            checksums(files)
            with self.assertRaises(ValueError):
                validate(zipped(files))

    def test_png_corruption_or_metadata_is_rejected(self):
        data = png()
        for bad in (b"not a PNG" * 300, data[:-1] + b"x", data + b"private source", data[:20]):
            with self.assertRaises(ValueError):
                gate.png_valid(bad)

    def test_png_invalid_pixel_stream_rejected_by_full_readback(self):
        bad = png_stream(b"PRIVATE_SENTINEL" * 100)
        files = dict(self.files)
        files["fixtures/example-chromium/fixture.png"] = bad
        checksums(files)
        with self.assertRaises(ValueError):
            gate.png_valid(bad)
        with self.assertRaises(ValueError):
            validate(zipped(files))

    def test_png_complete_scanlines_encoding_filters_and_stream_end_required(self):
        row = b"\0" + random.Random(23).randbytes(375 * 3)
        pixels = row * 812
        compressed = zlib.compress(pixels)
        invalid_later_filter = bytearray(pixels)
        invalid_later_filter[65 * len(row)] = 5
        for body in (compressed[:-1], zlib.compress(pixels[:-1]), zlib.compress(pixels + row),
                     zlib.compress(b"\5" + pixels[1:]), compressed + b"PRIVATE_SENTINEL",
                     compressed + zlib.compress(pixels), zlib.compress(invalid_later_filter)):
            with self.subTest(size=len(body)), self.assertRaises(ValueError):
                gate.png_valid(png_stream(body))
        for encoding in ((16, 2, 0, 0, 0), (8, 3, 0, 0, 0), (8, 2, 1, 0, 0),
                         (8, 2, 0, 1, 0), (8, 2, 0, 0, 1)):
            with self.subTest(encoding=encoding), self.assertRaises(ValueError):
                gate.png_valid(png_stream(compressed, encoding=encoding))
        with self.assertRaises(ValueError):
            gate.png_valid(png_stream(compressed, width=8192, height=32768))

    def test_png_header_and_end_chunk_lengths_and_truncated_chunks_rejected(self):
        data = png()
        def chunk(kind, body):
            return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
        for bad in (data[:8] + chunk(b"IHDR", data[16:29] + b"\0") + data[33:],
                    data[:-12] + chunk(b"IEND", b"\0"), data + b"\0" * 4,
                    data[:33] + chunk(b"IHDR", data[16:29]) + data[33:]):
            with self.assertRaises(ValueError):
                gate.png_valid(bad)

    def test_png_rgb_rgba_multi_idat_and_streaming_filter_boundaries(self):
        for channels, color in ((3, 2), (4, 6)):
            row = b"\4" + random.Random(23).randbytes(375 * channels)
            data = png_stream(zlib.compress(row * 812), encoding=(8, color, 0, 0, 0))
            gate.png_valid(data)
            size = int.from_bytes(data[33:37], "big")
            body = data[41:41 + size]
            def chunk(kind, part):
                return struct.pack(">I", len(part)) + kind + part + struct.pack(">I", zlib.crc32(kind + part))
            middle = len(body) // 2
            gate.png_valid(data[:33] + chunk(b"IDAT", body[:middle])
                           + chunk(b"IDAT", body[middle:]) + data[45 + size:])
            with self.assertRaises(ValueError):
                gate.png_valid(data[:33] + chunk(b"IDAT", body[:middle])
                               + chunk(b"sRGB", b"\0") + chunk(b"IDAT", body[middle:]) + data[45 + size:])

    def test_source_file_symlink_and_path_escape_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "real.txt").write_bytes(b"okay")
            self.assertEqual(gate.bounded_file(root, "real.txt"), b"okay")
            for name in ("../outside", "C:/secrets", "a\\b", "/absolute"):
                with self.assertRaises(ValueError):
                    gate.bounded_file(root, name)
            with self.assertRaises(ValueError):
                gate.bounded_file(root, "real.txt", limit=1)
            with patch.object(Path, "is_symlink", return_value=True), self.assertRaises(ValueError):
                gate.bounded_file(root, "real.txt")

    def test_bounded_command_stops_before_oversized_response_consumed(self):
        process = MagicMock()
        process.poll.return_value = None
        def spawn(*args, **kwargs):
            kwargs["stdout"].write(b"x" * 21)
            kwargs["stdout"].flush()
            return process
        with patch.object(gate.subprocess, "Popen", side_effect=spawn), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"], limit=20)
        process.stdout.read.assert_not_called()
        process.kill.assert_called_once()
        process.wait.assert_called_once_with(timeout=5)

    def test_bounded_command_success_and_timeout(self):
        process = MagicMock()
        process.wait.return_value = 0
        process.poll.return_value = 0
        def spawn(*args, **kwargs):
            kwargs["stdout"].write(b"bounded")
            kwargs["stdout"].flush()
            return process
        with patch.object(gate.subprocess, "Popen", side_effect=spawn):
            self.assertEqual(gate.bounded_command(["gh", "api", "fixture"], limit=20), b"bounded")
        process.kill.assert_not_called()
        process.poll.return_value = None
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate.time, "monotonic", side_effect=[0, 121]), self.assertRaises(TimeoutError):
            gate.bounded_command(["gh", "api", "fixture"], limit=20)
        process.kill.assert_called_once()

    def test_command_timeout_returns_with_retained_stdout_handle_and_reaps_only_child(self):
        process, retained = MagicMock(), []
        process.poll.return_value = None
        def spawn(*args, **kwargs):
            retained.append(os.dup(kwargs["stdout"].fileno()))
            return process
        try:
            with patch.object(gate.subprocess, "Popen", side_effect=spawn), \
                    patch.object(gate.time, "monotonic", side_effect=[0, 121]), self.assertRaises(TimeoutError):
                gate.bounded_command(["gh", "api", "fixture"], limit=20)
            self.assertEqual(os.fstat(retained[0]).st_size, 0)
            process.kill.assert_called_once_with()
            process.wait.assert_called_once_with(timeout=5)
            process.stdout.read.assert_not_called()
        finally:
            for descriptor in retained:
                os.close(descriptor)

    def test_command_deadline_not_renewed_and_checked_after_readback(self):
        process = MagicMock()
        process.poll.return_value = None
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate.time, "monotonic", side_effect=[0, 119, 121]), \
                patch.object(gate.time, "sleep") as sleep, self.assertRaises(TimeoutError):
            gate.bounded_command(["gh", "api", "fixture"])
        sleep.assert_called_once_with(0.02)
        process.kill.assert_called_once()
        process.poll.return_value = 0
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate.time, "monotonic", side_effect=[0, 1, 121]), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"])

    def test_command_spool_read_is_bounded_and_nonzero_or_oversized_errors_refuse(self):
        process = MagicMock()
        process.poll.return_value = 0
        with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
            wrapped = MagicMock(wraps=output)
            wrapped.__enter__.return_value = wrapped
            def spawn(*args, **kwargs):
                kwargs["stdout"].write(b"bounded")
                kwargs["stdout"].flush()
                return process
            with patch.object(gate.tempfile, "TemporaryFile", side_effect=[wrapped, errors]), \
                    patch.object(gate.subprocess, "Popen", side_effect=spawn):
                self.assertEqual(gate.bounded_command(["gh", "api", "fixture"], limit=20), b"bounded")
            wrapped.read.assert_called_once_with(21)
        process.poll.return_value = 7
        with patch.object(gate.subprocess, "Popen", return_value=process), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"])
        process.poll.return_value = None
        def noisy(*args, **kwargs):
            kwargs["stderr"].write(b"PRIVATE_SENTINEL" * 10)
            kwargs["stderr"].flush()
            return process
        with patch.object(gate.subprocess, "Popen", side_effect=noisy), patch.object(gate, "MAX_FILE", 20), \
                self.assertRaisesRegex(ValueError, "^Control response exceeds size limit$"):
            gate.bounded_command(["gh", "api", "fixture"])
        process.kill.assert_called_once()


class FailureContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.locations = gate.attested_test_locations(ROOT)

    def browser_report(self):
        return dict(errors=[dict(message="PRIVATE_SENTINEL")], suites=[dict(title="PRIVATE_SENTINEL", specs=[dict(
            title="PRIVATE_SENTINEL", file="runtime-controls.spec.ts", line=95, tests=[dict(
                projectName="webkit", status="unexpected", results=[dict(status="failed",
                    error=dict(message="PRIVATE_SENTINEL"), stdout=["PRIVATE_SENTINEL"],
                    attachments=[dict(path="PRIVATE_SENTINEL")])])])])])

    def browser(self, value=None):
        return gate.safe_browser_failure(gate.canonical(value or self.browser_report()), self.locations)

    def receipt(self):
        return dict(version=1, kind="safe_frontend_failure", status="failure", repository=gate.REPOSITORY,
            branch=gate.BRANCH, workflow_path=gate.WORKFLOW, workflow_sha="a" * 40, run_id=123, run_attempt=2,
            source_sha=gate.SOURCE_SHA, source_tree=gate.SOURCE_TREE, source_parents=gate.SOURCE_PARENTS,
            base_sha=gate.BASE_SHA, base_tree=gate.BASE_TREE, schema_sha256=gate.SCHEMA_SHA256,
            test_inventory_sha256=gate.digest(gate.canonical(self.locations)),
            control_sha256={name: gate.digest((ROOT / name).read_text(encoding="utf-8").encode())
                            for name in sorted(gate.WRITE_SET)},
            gate="fixtures", completed_gates=list(gate.GATES)[:list(gate.GATES).index("fixtures")],
            category="exit", exit_code=1, browser=self.browser(), cleanup=dict(private_absent=True),
            **gate.QUALIFIED_INPUTS, **gate.FAILURE_SCOPE)

    def verify(self, value):
        return gate.validate_failure(value, workflow_sha="a" * 40, run_id=123, attempt=2, locations=self.locations)

    def files(self, value=None):
        data = gate.canonical(value or self.receipt())
        return {gate.FAILURE_FILE: data, "SHA256SUMS": (gate.digest(data) + "  " + gate.FAILURE_FILE + "\n").encode()}

    def metadata(self, payload):
        run, artifact = readback_metadata(payload)
        run["conclusion"] = "failure"
        artifact["name"] = "fleet-frontend-failure-chats-123-2"
        return run, artifact

    def readback(self, payload, run=None, artifact=None):
        actual_run, actual_artifact = self.metadata(payload)
        return gate.validate_failure_readback(run or actual_run, artifact or actual_artifact, payload,
            run_id=123, attempt=2, workflow_sha="a" * 40, artifact_id=456, artifact_digest=gate.digest(payload))

    def test_fixture_parser_never_serializes_private_fields_or_runtime_titles(self):
        result = self.browser()
        self.assertEqual(result["report"], "valid")
        self.assertEqual(result["diagnostics"], [dict(file="frontend/e2e/runtime-controls.spec.ts", line=95,
            project="webkit", status="unexpected", results=["failed"])])
        self.assertEqual(result["browsers"]["webkit"]["unexpected"], 1)
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        self.verify(self.receipt())

    def compiler_receipt(self):
        return dict(self.receipt(), gate="typecheck", exit_code=2,
            completed_gates=list(gate.GATES)[:list(gate.GATES).index("typecheck")],
            browser=gate.safe_browser_failure(None, self.locations),
            compiler=gate.safe_compiler_failure(
                b"src/pages/chat-detail/index.test.tsx(1969,7): error TS2322: PRIVATE_SENTINEL\n",
                gate.attested_compiler_paths(ROOT)))

    def unit_receipt(self):
        return dict(self.receipt(), gate="unit",
            completed_gates=list(gate.GATES)[:list(gate.GATES).index("unit")],
            browser=gate.safe_browser_failure(None, self.locations),
            unit=gate.safe_unit_failure(b" \xe2\x9d\xaf src/pages/chat-detail/index.test.tsx:2013:18\n",
                                        gate.attested_compiler_paths(ROOT)))

    def test_unit_frames_drop_methods_titles_messages_source_and_dependency_paths(self):
        paths = gate.attested_compiler_paths(ROOT)
        data = (" FAIL src/pages/chat-detail/index.test.tsx > PRIVATE_SENTINEL\n"
                "AssertionError: PRIVATE_SENTINEL\n 2013| PRIVATE_SENTINEL\n"
                " \u276f PRIVATE_SENTINEL node_modules/private/index.js:9:1\n"
                " \u276f PRIVATE_SENTINEL src/pages/chat-detail/index.test.tsx:2013:18\n"
                " \x1b[36m\u276f \x1b[2mfrontend/src/pages/chat-detail/index.test.tsx:2013:18\x1b[0m\n").encode()
        result = gate.safe_unit_failure(data, paths)
        self.assertEqual(result, self.unit_receipt()["unit"])
        self.assertEqual(result["diagnostics"], [dict(file="frontend/src/pages/chat-detail/index.test.tsx",
                                                    line=2013, column=18)])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_unit_paths_injection_malformed_and_size_bounds_never_escape(self):
        paths = gate.attested_compiler_paths(ROOT)
        for suffix in ("src/PRIVATE_SENTINEL.ts:1:1", "../src/pages/chat-detail/index.test.tsx:1:1",
                       "/tmp/src/pages/chat-detail/index.test.tsx:1:1", "src/../api/generated.ts:1:1",
                       "src/pages/chat-detail/index.test.tsx:0:1", "src/pages/chat-detail/index.test.tsx:1:1000000",
                       "src/pages/chat-detail/index.test.tsx:1:1 PRIVATE_SENTINEL", "e2e/private.ts:1:1"):
            self.assertEqual(gate.safe_unit_failure((" \u276f " + suffix).encode(), paths)["diagnostics"], [])
        self.assertEqual(gate.safe_unit_failure(None, paths)["report"], "unavailable")
        self.assertEqual(gate.safe_unit_failure(b"\xff", paths), dict(report="rejected", diagnostics=[]))
        self.assertEqual(gate.safe_unit_failure(b"x" * (gate.COMPILER_LOG_LIMIT + 1), paths),
                         dict(report="truncated", diagnostics=[]))
        frames = [f" \u276f src/pages/chat-detail/index.test.tsx:{n}:1\n".encode() for n in range(1, 130)]
        self.assertEqual(len(gate.safe_unit_failure(b"".join(frames[:128]), paths)["diagnostics"]), 128)
        self.assertEqual(gate.safe_unit_failure(b"".join(frames), paths), dict(report="rejected", diagnostics=[]))

    def test_unit_closed_schema_gate_and_original_authenticated_reader(self):
        value = self.unit_receipt()
        self.assertEqual(self.readback(zipped(self.files(value))), self.files(value))
        for change in (dict(message="PRIVATE_SENTINEL"), dict(file="src/pages/chat-detail/index.test.tsx"),
                       dict(line=True), dict(column="18"), dict(line=0), dict(column=1000000)):
            value = self.unit_receipt()
            value["unit"]["diagnostics"][0].update(change)
            with self.assertRaises(ValueError):
                self.verify(value)
        for change in (dict(raw="PRIVATE_SENTINEL"), dict(report="PRIVATE_SENTINEL"), dict(diagnostics=[]),
                       dict(report="rejected"), dict(diagnostics=self.unit_receipt()["unit"]["diagnostics"] * 129)):
            value = self.unit_receipt()
            value["unit"].update(change)
            with self.assertRaises(ValueError):
                self.verify(value)
        value = self.unit_receipt()
        for name in gate.GATES:
            if name != "unit":
                with self.assertRaises(ValueError):
                    self.verify(dict(value, gate=name, completed_gates=list(gate.GATES)[:list(gate.GATES).index(name)]))
        with self.assertRaises(ValueError):
            self.verify(dict(value, category="timeout", exit_code=None))
        with self.assertRaises(ValueError):
            self.verify(dict(value, compiler=self.compiler_receipt()["compiler"]))

    def test_unit_log_record_cleanup_roundtrip_withholds_all_private_text(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private = self.cleanup_fixture(root, self.unit_receipt())
            (private / "failure-pending.json").unlink()
            for name in gate.WRITE_SET:
                target = root / "controls" / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((ROOT / name).read_text(encoding="utf-8").encode())
            (private / "unit.log").write_bytes(
                "PRIVATE_SENTINEL\n \u276f src/pages/chat-detail/index.test.tsx:2013:18\n".encode())
            (private / "preflight.json").write_bytes(gate.canonical(dict(
                workflow_sha="a" * 40, run_id="123", attempt="2", public=True)))
            (private / "state.json").write_bytes(gate.canonical(dict(workflow_sha="a" * 40,
                gates=self.unit_receipt()["completed_gates"])))
            with patch.object(gate, "controls_preflight", return_value=(root, "a" * 40)), \
                    patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.object(gate, "attested_compiler_paths", return_value=gate.attested_compiler_paths(ROOT)), \
                    patch.object(gate, "qualified_inputs", return_value=gate.QUALIFIED_INPUTS), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"):
                gate.record_failure("unit", gate.GateFailure("exit", 1))
                gate.cleanup()
            self.assertFalse(private.exists())
            files = {p.name: p.read_bytes() for p in (root / "fleet-frontend-failure").iterdir()}
            self.assertEqual(self.readback(zipped(files)), files)
            self.assertNotIn(b"PRIVATE_SENTINEL", files[gate.FAILURE_FILE])
            self.assertEqual(json.loads(files[gate.FAILURE_FILE])["unit"], self.unit_receipt()["unit"])

    def test_compiler_frames_copy_only_attested_header_numbers_and_path(self):
        paths = gate.attested_compiler_paths(ROOT)
        self.assertIn("frontend/src/pages/chat-detail/index.test.tsx", paths)
        self.assertTrue(all(p.startswith("frontend/src/") and p.endswith((".ts", ".tsx")) for p in paths))
        data = (b"> private command\nfrontend/src/pages/chat-detail/index.test.tsx(1969,7): error TS2322: PRIVATE_SENTINEL\n"
                b"  Type PRIVATE_SENTINEL is not assignable\n"
                b"src/pages/chat-detail/index.test.tsx(1969,7): error TS2322: PRIVATE_SENTINEL\n")
        result = gate.safe_compiler_failure(data, paths)
        self.assertEqual(result, self.compiler_receipt()["compiler"])
        self.assertEqual(result["diagnostics"], [dict(code=2322,
            file="frontend/src/pages/chat-detail/index.test.tsx", line=1969, column=7)])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_compiler_wrong_private_traversal_or_malformed_headers_reject_all_frames(self):
        paths = gate.attested_compiler_paths(ROOT)
        valid = b"src/pages/chat-detail/index.test.tsx(1,1): error TS2322: PRIVATE_SENTINEL\n"
        for header in ("../src/private.ts(1,1)", "/tmp/src/private.ts(1,1)", "src/../api/generated.ts(1,1)",
                       "src/PRIVATE_SENTINEL.ts(1,1)", "e2e/runtime-controls.spec.ts(1,1)",
                       "node_modules/private.ts(1,1)", "src/pages/chat-detail/index.test.tsx(0,1)",
                       "src/pages/chat-detail/index.test.tsx(1,1000000)", "src/pages/chat-detail/index.test.tsx(1,x)"):
            with self.subTest(header=header):
                result = gate.safe_compiler_failure(valid + (header + ": error TS2322: PRIVATE_SENTINEL\n").encode(), paths)
                self.assertEqual(result, dict(report="rejected", diagnostics=[]))
        for suffix in (b"error TS0: PRIVATE_SENTINEL", b"error TS2322 PRIVATE_SENTINEL", b"error TS999999: PRIVATE_SENTINEL"):
            self.assertEqual(gate.safe_compiler_failure(valid + suffix, paths)["report"], "rejected")

    def test_compiler_byte_frame_encoding_and_absent_bounds(self):
        paths = gate.attested_compiler_paths(ROOT)
        self.assertEqual(gate.safe_compiler_failure(None, paths)["report"], "unavailable")
        self.assertEqual(gate.safe_compiler_failure(b"PRIVATE_SENTINEL", paths)["diagnostics"], [])
        self.assertEqual(gate.safe_compiler_failure(b"\xff", paths)["report"], "rejected")
        self.assertEqual(gate.safe_compiler_failure(b"x" * (gate.COMPILER_LOG_LIMIT + 1), paths),
                         dict(report="truncated", diagnostics=[]))
        frames = [f"src/pages/chat-detail/index.test.tsx({n},1): error TS2322: PRIVATE_SENTINEL\n".encode()
                  for n in range(1, 130)]
        self.assertEqual(len(gate.safe_compiler_failure(b"".join(frames[:128]), paths)["diagnostics"]), 128)
        self.assertEqual(gate.safe_compiler_failure(b"".join(frames), paths), dict(report="rejected", diagnostics=[]))

    def test_compiler_schema_and_numeric_types_fail_closed(self):
        self.verify(self.compiler_receipt())
        for change in (dict(message="PRIVATE_SENTINEL"), dict(file="src/pages/chat-detail/index.test.tsx"),
                       dict(code=True), dict(code=0), dict(code=100000), dict(line=False), dict(line=0),
                       dict(column=1000000), dict(column="7")):
            value = self.compiler_receipt()
            value["compiler"]["diagnostics"][0].update(change)
            with self.assertRaises(ValueError):
                self.verify(value)
        for change in (dict(raw="PRIVATE_SENTINEL"), dict(report="PRIVATE_SENTINEL"),
                       dict(report="rejected"), dict(diagnostics=[])):
            value = self.compiler_receipt()
            value["compiler"].update(change)
            with self.assertRaises(ValueError):
                self.verify(value)
        value = self.compiler_receipt()
        value["compiler"]["diagnostics"] *= 2
        with self.assertRaises(ValueError):
            self.verify(value)

    def test_compiler_optional_field_is_typecheck_exit_only_with_original_authenticated_reader(self):
        self.verify(self.receipt())  # Older version-1 receipts have no compiler field.
        value = self.compiler_receipt()
        payload = zipped(self.files(value))
        self.assertEqual(self.readback(payload), self.files(value))
        for name in gate.GATES:
            if name != "typecheck":
                other = dict(value, gate=name, completed_gates=list(gate.GATES)[:list(gate.GATES).index(name)])
                with self.assertRaises(ValueError):
                    self.verify(other)
        with self.assertRaises(ValueError):
            self.verify(dict(value, category="timeout", exit_code=None))

    def test_compiler_log_record_cleanup_and_readback_never_publish_private_text(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private = self.cleanup_fixture(root, self.compiler_receipt())
            for name in gate.WRITE_SET:
                target = root / "controls" / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes((ROOT / name).read_text(encoding="utf-8").encode())
            (private / "failure-pending.json").unlink()
            (private / "typecheck.log").write_bytes(
                b"src/pages/chat-detail/index.test.tsx(1969,7): error TS2322: PRIVATE_SENTINEL\n")
            (private / "preflight.json").write_bytes(gate.canonical(dict(
                workflow_sha="a" * 40, run_id="123", attempt="2", public=True)))
            (private / "state.json").write_bytes(gate.canonical(dict(workflow_sha="a" * 40,
                gates=self.compiler_receipt()["completed_gates"])))
            with patch.object(gate, "controls_preflight", return_value=(root, "a" * 40)), \
                    patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.object(gate, "attested_compiler_paths", return_value=gate.attested_compiler_paths(ROOT)), \
                    patch.object(gate, "qualified_inputs", return_value=gate.QUALIFIED_INPUTS), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"):
                gate.record_failure("typecheck", gate.GateFailure("exit", 2))
                gate.cleanup()
            self.assertFalse(private.exists())
            files = {p.name: p.read_bytes() for p in (root / "fleet-frontend-failure").iterdir()}
            self.assertEqual(self.readback(zipped(files)), files)
            self.assertNotIn(b"PRIVATE_SENTINEL", files[gate.FAILURE_FILE])
            self.assertEqual(json.loads(files[gate.FAILURE_FILE])["compiler"], self.compiler_receipt()["compiler"])

    def test_file_and_line_are_canonical_source_declarations_only(self):
        self.assertIn(95, self.locations["frontend/e2e/runtime-controls.spec.ts"])
        self.assertNotIn(93, self.locations["frontend/e2e/runtime-controls.spec.ts"])
        for file, line in (("../runtime-controls.spec.ts", 95), ("services-base/private.spec.ts", 95),
                           ("/tmp/e2e/runtime-controls.spec.ts", 95), ("runtime-controls.spec.tsPRIVATE_SENTINEL", 95),
                           ("runtime-controls.spec.ts", 90), ("runtime-controls.spec.ts", 93), ("runtime-controls.spec.ts", True),
                           ("runtime-controls.spec.ts", "95")):
            report = self.browser_report()
            report["suites"][0]["specs"][0].update(file=file, line=line)
            self.assertEqual(self.browser(report)["report"], "rejected")
        for file in ("runtime-controls.spec.ts", "e2e/runtime-controls.spec.ts", "frontend/e2e/runtime-controls.spec.ts"):
            report = self.browser_report()
            report["suites"][0]["specs"][0]["file"] = file
            self.assertEqual(self.browser(report)["report"], "valid")

    def test_unknown_browser_status_project_attempt_and_partial_data_rejected(self):
        for change in (dict(projectName="PRIVATE_SENTINEL"), dict(status="PRIVATE_SENTINEL"),
                       dict(results=[dict(status="PRIVATE_SENTINEL")]), dict(results=[dict(status="failed")] * 4)):
            report = self.browser_report()
            test = report["suites"][0]["specs"][0]["tests"][0]
            report["suites"][0]["specs"][0]["tests"].append(dict(test, **change))
            result = self.browser(report)
            self.assertEqual(result["report"], "rejected")
            self.assertEqual(result["diagnostics"], [])
            self.assertTrue(all(not any(p.values()) for p in result["browsers"].values()))

    def test_browser_report_bounds_missing_invalid_and_recursive_input(self):
        for raw, expected in ((None, "unavailable"), (b"PRIVATE_SENTINEL", "rejected"),
                              (b"x" * (4 * 1024 ** 2 + 1), "truncated"), (b"[]", "rejected")):
            self.assertEqual(gate.safe_browser_failure(raw, self.locations)["report"], expected)
        report = self.browser_report()
        leaf = report["suites"][0]
        for _ in range(18):
            leaf["suites"] = [dict()]
            leaf = leaf["suites"][0]
        self.assertEqual(self.browser(report)["report"], "rejected")
        self.assertEqual(self.browser(dict(suites=[dict()] * 4097))["report"], "rejected")

    def test_failure_receipt_rejects_private_fields_pins_and_acceptance_claims(self):
        for change in (dict(message="PRIVATE_SENTINEL"), dict(source_sha="f" * 40), dict(workflow_sha="f" * 40),
                       dict(schema_sha256="0" * 64), dict(test_inventory_sha256="0" * 64),
                       dict(control_sha256={}), dict(live_pm_acceptance=True), dict(frontend_unit_build_fixture=True),
                       dict(cleanup=dict(private_absent=1)), dict(exit_code=True), dict(exit_code=0),
                       dict(category="PRIVATE_SENTINEL"), dict(category="timeout"), dict(gate="PRIVATE_SENTINEL"),
                       dict(completed_gates=["fixtures"]), dict(completed_gates=list(gate.GATES))):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.verify(dict(self.receipt(), **change))
        self.verify(dict(self.receipt(), category="timeout", exit_code=None))

    def test_failure_browser_schema_cannot_smuggle_raw_values(self):
        for kind in ("extra", "title", "line", "file", "project", "status", "results", "count", "duplicate", "unavailable"):
            value = self.receipt()
            browser, row = value["browser"], value["browser"]["diagnostics"][0]
            if kind == "extra":
                browser["raw"] = "PRIVATE_SENTINEL"
            elif kind == "title":
                row["title"] = "PRIVATE_SENTINEL"
            elif kind in ("file", "project", "status"):
                row[kind] = "PRIVATE_SENTINEL"
            elif kind == "line":
                row[kind] = 84
            elif kind == "results":
                row[kind] = ["PRIVATE_SENTINEL"]
            elif kind == "count":
                browser["browsers"]["webkit"]["unexpected"] = True
            elif kind == "duplicate":
                browser["diagnostics"].append(dict(row))
            else:
                browser["report"] = "unavailable"
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                self.verify(value)

    def test_full_authenticated_failure_readback_is_distinct_from_success(self):
        files = self.files()
        payload = zipped(files)
        self.assertEqual(self.readback(payload), files)
        self.assertNotIn("PRIVATE_SENTINEL", files[gate.FAILURE_FILE].decode())
        with self.assertRaises(ValueError):
            validate(payload)
        with self.assertRaises(ValueError):
            self.readback(zipped(fixture_files()))

    def test_failure_readback_requires_exact_run_artifact_digest_and_cleanup_schema(self):
        payload = zipped(self.files())
        for key, value in (("conclusion", "success"), ("run_attempt", 1), ("head_sha", "f" * 40),
                           ("event", "workflow_dispatch"), ("status", "in_progress"), ("head_branch", "main"),
                           ("path", ".github/workflows/ci.yml"), ("repository", dict(full_name="other/repo"))):
            run, _ = self.metadata(payload)
            run[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.readback(payload, run=run)
        for key, value in (("id", 1), ("expired", True), ("digest", "sha256:" + "0" * 64), ("name", "other"),
                           ("workflow_run", dict(id=123, head_sha="a" * 40, head_branch="other"))):
            _, artifact = self.metadata(payload)
            artifact[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.readback(payload, artifact=artifact)
        value = self.receipt()
        value["browser"]["diagnostics"][0]["line"] = 84
        with self.assertRaises(ValueError):
            self.readback(zipped(self.files(value)))

    def test_failure_zip_member_checksum_size_and_canonical_json_bounds(self):
        files = self.files()
        for name in ("../failure.json", "PRIVATE_SENTINEL.log", "source.ts", "fixtures/image.png"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.readback(zipped(files, (name, b"PRIVATE_SENTINEL")))
        with self.assertWarns(UserWarning):
            payload = zipped(files, (gate.FAILURE_FILE, files[gate.FAILURE_FILE]))
        with self.assertRaises(ValueError):
            self.readback(payload)
        with self.assertRaises(ValueError):
            self.readback(zipped(dict(files, SHA256SUMS=b"0" * 64)))
        with patch.object(gate, "FAILURE_LIMIT", 100), self.assertRaises(ValueError):
            self.readback(zipped(files))
        files[gate.FAILURE_FILE] = b" " + files[gate.FAILURE_FILE]
        files["SHA256SUMS"] = (gate.digest(files[gate.FAILURE_FILE]) + "  failure.json\n").encode()
        with self.assertRaises(ValueError):
            self.readback(zipped(files))

    def cleanup_fixture(self, root, value):
        private = root / "fleet-frontend-private"
        private.mkdir()
        (private / "private.log").write_bytes(b"PRIVATE_SENTINEL")
        (private / "failure-pending.json").write_bytes(gate.canonical(value))
        return private

    def test_finally_cleanup_seals_absence_and_only_safe_receipt(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private = self.cleanup_fixture(root, dict(self.receipt(), cleanup=dict(private_absent=False)))
            with patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"):
                gate.cleanup()
            self.assertFalse(private.exists())
            files = {p.name: p.read_bytes() for p in (root / "fleet-frontend-failure").iterdir()}
            self.assertEqual(self.readback(zipped(files)), files)
            self.assertTrue(json.loads(files[gate.FAILURE_FILE])["cleanup"]["private_absent"])

    def test_cleanup_error_retains_failure_not_false_absence(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            self.cleanup_fixture(root, dict(self.receipt(), cleanup=dict(private_absent=False)))
            with patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"), \
                    patch.object(gate.shutil, "rmtree", side_effect=OSError("PRIVATE_SENTINEL")), self.assertRaises(OSError):
                gate.cleanup()
            value = json.loads((root / "fleet-frontend-failure/failure.json").read_bytes())
            self.assertFalse(value["cleanup"]["private_absent"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(value).decode())

    def test_invalid_pending_receipt_still_cleans_private_and_cannot_publish(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private = self.cleanup_fixture(root, dict(self.receipt(), raw="PRIVATE_SENTINEL"))
            with patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"), self.assertRaises(ValueError):
                gate.cleanup()
            self.assertFalse(private.exists())
            self.assertFalse((root / "fleet-frontend-failure").exists())

    def test_gate_command_exit_and_timeout_are_numeric_or_fixed_only(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            for error in (subprocess.CompletedProcess(["pnpm"], 17), subprocess.TimeoutExpired(["PRIVATE_SENTINEL"], 1200)):
                state = dict(gates=[])
                with patch.object(gate, "load_state", return_value=(ROOT, root, state)), \
                        patch.object(gate, "verify_parity", return_value=(ROOT, ROOT)), \
                        patch.object(gate.subprocess, "run", side_effect=error if isinstance(error, Exception) else None,
                                     return_value=error), self.assertRaises(gate.GateFailure) as caught:
                    gate.gate("base")
                self.assertNotIn("PRIVATE_SENTINEL", str(caught.exception))
                self.assertEqual(caught.exception.category, "timeout" if isinstance(error, Exception) else "exit")
                self.assertEqual(caught.exception.code, None if isinstance(error, Exception) else 17)
                self.assertEqual(state["gates"], [])
                (root / "base.log").unlink()

    def test_record_failure_reads_only_json_and_retains_original_numeric_exit(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private, controls = root / "fleet-frontend-private", root / "controls"
            private.mkdir()
            for name in gate.WRITE_SET:
                path = controls / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((ROOT / name).read_text(encoding="utf-8").encode())
            (private / "preflight.json").write_bytes(gate.canonical(dict(
                workflow_sha="a" * 40, run_id="123", attempt="2", public=True)))
            (private / "state.json").write_bytes(gate.canonical(dict(workflow_sha="a" * 40,
                gates=self.receipt()["completed_gates"])))
            (private / "fixtures.log").write_bytes(b"PRIVATE_SENTINEL")
            (private / "browser.json").write_bytes(gate.canonical(self.browser_report()))
            with patch.object(gate, "controls_preflight", return_value=(root, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.object(gate, "qualified_inputs", return_value=gate.QUALIFIED_INPUTS), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"), \
                    patch.object(gate, "bounded_file", wraps=gate.bounded_file) as read:
                gate.record_failure("fixtures", gate.GateFailure("exit", 17))
                self.assertFalse(any(call.args[1].endswith(".log") for call in read.call_args_list))
                value = json.loads((private / "failure-pending.json").read_bytes())
                self.assertEqual(value["exit_code"], 17)
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(value).decode())
                self.assertFalse(value["cleanup"]["private_absent"])
                self.assertEqual(value["browser"]["report"], "valid")
                with self.assertRaises(FileExistsError):
                    gate.record_failure("fixtures", gate.GateFailure("timeout"))
                self.assertEqual(json.loads((private / "failure-pending.json").read_bytes()), value)
                (private / "failure-pending.json").unlink()
                (private / "browser.json").write_bytes(b"x" * (4 * 1024 ** 2 + 1))
                gate.record_failure("fixtures", OSError("PRIVATE_SENTINEL"))
                value = json.loads((private / "failure-pending.json").read_bytes())
                self.assertEqual(value["category"], "control_error")
                self.assertIsNone(value["exit_code"])
                self.assertEqual(value["browser"]["report"], "truncated")
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(value).decode())
                (private / "failure-pending.json").unlink()
                gate.record_failure("fixtures", TimeoutError("PRIVATE_SENTINEL"))
                value = json.loads((private / "failure-pending.json").read_bytes())
                self.assertEqual(value["category"], "timeout")
                self.assertIsNone(value["exit_code"])

    def test_record_failure_error_cannot_replace_original_failure_or_echo_details(self):
        error = gate.GateFailure("exit", 17)
        stream = io.StringIO()
        with patch("sys.argv", ["helper", "gate", "fixtures"]), \
                patch.object(gate, "gate", side_effect=error), \
                patch.object(gate, "record_failure", side_effect=OSError("PRIVATE_SENTINEL")) as record, \
                redirect_stdout(stream), self.assertRaises(gate.GateFailure) as caught:
            gate.main()
        self.assertIs(caught.exception, error)
        record.assert_called_once_with("fixtures", error)
        self.assertNotIn("PRIVATE_SENTINEL", stream.getvalue())

    def test_attested_test_inventory_refuses_worktree_line_drift(self):
        actual_git = gate.git
        def changed(root, name, limit=gate.MAX_FILE):
            return b"test('PRIVATE_SENTINEL', async () => {})\n"
        with patch.object(gate, "bounded_file", side_effect=changed), self.assertRaisesRegex(ValueError, "canonical Git blob"):
            gate.attested_test_locations(ROOT)
        self.assertIs(gate.git, actual_git)

    def test_failure_readback_rejects_zip_symlink_and_true_claim_even_with_rehashed_receipt(self):
        files = self.files(dict(self.receipt(), live_runtime_acceptance=True))
        with self.assertRaises(ValueError):
            self.readback(zipped(files))
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            info = zipfile.ZipInfo(gate.FAILURE_FILE)
            info.create_system = 3
            info.external_attr = 0o120777 << 16
            archive.writestr(info, self.files()[gate.FAILURE_FILE])
            archive.writestr("SHA256SUMS", self.files()["SHA256SUMS"])
        with self.assertRaises(ValueError):
            self.readback(output.getvalue())

    def test_workflow_failure_upload_is_separate_after_always_cleanup(self):
        workflow = (ROOT / gate.WORKFLOW).read_text()
        self.assertLess(workflow.index("hosted_frontend_gate.py cleanup"), workflow.index("Upload bounded safe failure"))
        block = workflow.split("- name: Upload bounded safe failure", 1)[1]
        self.assertIn("if: failure()", block)
        self.assertIn("path: ${{ runner.temp }}/fleet-frontend-failure", block)
        self.assertNotIn("fleet-frontend-private", block)
        self.assertNotIn("continue-on-error", workflow)
        self.assertEqual(len(gate.GATES), 23)


if __name__ == "__main__":
    unittest.main()
