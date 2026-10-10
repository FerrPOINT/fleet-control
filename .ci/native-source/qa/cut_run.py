"""Separate bounded protocol4 cut matrix. Full prepare/run require later authorization."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import time

import cut_compile_proof
import cut_contract as contract
import cut_files

# Isolated helper namespace: importing/running cut QA cannot retarget the original nine.
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("_cut_driver",HERE / "run.py")
driver = importlib.util.module_from_spec(spec)
spec.loader.exec_module(driver)
driver.PROJECT_PREFIX = "sdlc-qa-fleet-native4cut-"
driver.ACK_KIND = "native4cut"
driver.TASK = "fleet-native-protocol4-cut"
driver.PURPOSE = "original-stop-ack-cas-recovered-activation-f6"
EXTRAS = ("cut-live/Cargo.toml","cut-live/build.rs","cut-live/src/main.rs","cut-live/src/scenario.rs",
          "cut_transport.py","cut_contract.py","cut_files.py")
driver.EXTRA_SOURCE_FILES = EXTRAS
driver.EXTRA_LOCK_MANIFESTS = ("cut-live/Cargo.toml",)
driver.EXTRA_MATRIX = dict(kind="protocol4_cut_supplement",expected=contract.MATRIX,
    original_helper_commit="df6574bfe5d26445b9ce31a8f396d42914c8c653",original_nine_native_acceptance="separate_pending")
driver.HELPERS = (*driver.HELPERS,*EXTRAS,"cut_run.py","cut_compile_proof.py","build_cuts.sh","launch_cut.py",
                  "test_cuts.py","test_cut_closure.py","CUTS.md")


def qualification():
    text = driver.git(driver.FLEET,"show",driver.SOURCE_MERGED + ":backend/infra/src/runtime/container_control.rs").decode()
    blocks = re.findall(r'const BOOTSTRAP: &str = r#"(.*?)"#;',text,re.S)
    if len(blocks) != 1 or driver.SOURCE_MERGED != contract.SOURCE or list(driver.UTILITY_HASHES.values()) != contract.HASHES:
        raise ValueError("Exact captured source cut qualification required")
    return dict(source_commit=contract.SOURCE,loader_sha256=hashlib.sha256(blocks[0].encode()).hexdigest(),utility_hashes=contract.HASHES)


def services(packet, source):
    result = driver.services(packet,source)
    result["build"]["entrypoint"] = ["bash","/qa/build_cuts.sh"]
    for name in ("build_cuts.sh","cut_compile_proof.py"):
        result["build"]["volumes"].append(driver.mount(packet / name,"/qa/" + name,True,"bind"))
    return result


def trace(packet):
    events = [contract.decode(p.read_bytes()) for p in sorted((packet / "evidence").glob("ack-*.json"))]
    calls = [contract.decode(p.read_bytes()) for p in sorted((packet / "evidence").glob("dispatch-*.json"))]
    if len(events) > 128 or {e["call_id"] for e in events} != {e["call_id"] for e in calls} or len(calls) != len(events):
        raise ValueError("Missing/unknown/duplicate native ACK, no acceptance")
    by_id = {e["call_id"]:e for e in calls}
    for e in events:
        if e["request"] != by_id[e["call_id"]]["request"]:
            raise ValueError("Original dispatch/ACK binding drift")
    return events


class CutBudget:
    def __init__(self, command, event):
        self.command, self.event = command, event
        self.deadline = None
        self.witness = None
        self.samples = []
        self.refresh()

    def left(self):
        left = self.deadline - time.monotonic()
        if left <= 0:
            raise RuntimeError("Withheld original ACK cut expired; no crash-cut acceptance")
        return left

    def checked(self, argv):
        raw = driver.checked(argv, timeout=self.left())
        self.left()  # Also reject a command which ignored/overran its timeout.
        return raw

    def refresh(self):
        started = time.monotonic()
        timeout = min(5, self.left()) if self.deadline is not None else 5
        value = contract.decode(driver.checked(self.command + ["exec", "-T", "fleet-backend", "python3", "-B",
            "/qa/cut_files.py", "probe", self.event["call_id"], self.event["ack_sha256"]], timeout=timeout))
        cut_files.validate_probe(value, self.event["call_id"], self.event["ack_sha256"])
        if self.witness is not None and value["witness"] != self.witness:
            raise ValueError("Original blocked wrapper epoch changed")
        if self.samples and value["observed"] < self.samples[-1]["observed"]:
            raise ValueError("Original monotonic witness moved backwards")
        self.witness = value["witness"]
        # Clocks are NOT compared across Windows/Linux. Subtract the entire round trip
        # conservatively, retain a 2s margin, and never extend the first proven deadline.
        deadline = started + value["remaining"] - 2
        self.deadline = min(self.deadline, deadline) if self.deadline is not None else deadline
        self.samples.append(value)
        self.left()


def cut_inventory(operation, docker, budget):
    # Same original inventory guards; both CLI calls have the cut's remaining timeout.
    ids = budget.checked(docker + ["container", "ls", "-aq", "--filter",
        "label=com.docker.compose.project=" + operation.project]).decode().split()
    items = contract.decode(budget.checked(docker + ["container", "inspect", *ids])) if ids else []
    result = {}
    for item in items:
        labels = item["Config"].get("Labels") or {}
        if "sdlc.boundary.generation" not in labels:
            continue
        if (labels.get("com.docker.compose.project") != operation.project or labels.get("sdlc.task") != driver.TASK
                or labels.get("sdlc.purpose") != driver.PURPOSE or item["Image"] != driver.HERMES_IMAGE):
            raise ValueError("Foreign native namespace inventory")
        result[item["Id"]] = dict(image_id=item["Image"], started_at=item["State"]["StartedAt"],
            generation=labels["sdlc.boundary.generation"], resource_id=labels["sdlc.boundary.resource"],
            service=labels["com.docker.compose.service"])
    if not result:
        raise ValueError("Original native namespaces absent")
    return dict(sorted(result.items()))


def scenario(operation, packet, docker, controller_id, controller, manifest, logged, report):
    phase = "cut_source_qualification"
    try:
        driver.write_json(packet / "proof/cut.json",qualification())
        phase = "cut_initial"
        logged(operation.command + ["exec","-d","fleet-backend","python3","-B","/qa/launch_cut.py","cut-initial"],"cut-initial-launch")
        deadline = time.monotonic() + 900
        ack_seen = None
        while True:
            now = time.monotonic()
            if (packet / "evidence/cut-stop-ack.json").exists() and ack_seen is None:
                ack_seen = now
            if (packet / "evidence/cut-initial-exit.json").exists() or now >= deadline or (ack_seen is not None and now - ack_seen > 10):
                raise RuntimeError("Original ACK-before-CAS cut not reached within budget")
            if (packet / "evidence/cut-ready.json").exists():
                break
            time.sleep(0.1)
        gate = contract.decode((packet / "evidence/cut-stop-ack.json").read_bytes())
        ready = contract.decode((packet / "evidence/cut-ready.json").read_bytes())
        contract.closed(ready,("cut","arm","peer"))
        contract.arm(ready["arm"])
        contract.validate_gate(gate,ready["arm"])
        if gate["stdout_forwarded"] is not False or ready["cut"]["phase"] != "stopping_previous" or ready["cut"]["previous_stop"] is not None:
            raise ValueError("Actual original stop must precede unchanged PG phase")
        phase = "cut_live_withholding_budget"
        budget = CutBudget(operation.command, gate["event"])
        before = cut_inventory(operation,docker,budget)
        driver.write_json(packet / "cut-native-before-restart.json",before)
        phase = "cut_physical_controller_restart"
        current = contract.decode(budget.checked(docker + ["container","inspect",controller_id]))[0]
        if (current["Id"] != controller["Id"] or current["Image"] != controller["Image"]
                or current["State"]["StartedAt"] != controller["State"]["StartedAt"] or current["State"]["Running"] is not True):
            raise ValueError("Original controller epoch required at kill boundary")
        budget.refresh()  # Fresh live original PID/boot/ACK witness immediately before restart.
        logged(operation.command + ["restart","--no-deps","fleet-backend"],phase,budget.left())
        budget.left()
        restarted = contract.decode(budget.checked(docker + ["container","inspect",controller_id]))[0]
        if (restarted["Id"] != controller["Id"] or restarted["Image"] != controller["Image"]
                or restarted["State"]["StartedAt"] == controller["State"]["StartedAt"] or restarted["State"]["Running"] is not True):
            raise ValueError("Actual same-controller physical restart required")
        after = cut_inventory(operation,docker,budget)
        if after != before:
            raise ValueError("Restart must not replace an agent generation")
        budget.left()
        driver.write_json(packet / "cut-restart-proof.json",dict(container_id=controller_id,image_id=controller["Image"],
            original_started_at=controller["State"]["StartedAt"],current_started_at=restarted["State"]["StartedAt"],native_before=before,native_after=after,
            withholding_seconds=cut_files.WITHHOLD,live_probes=budget.samples,host_deadline=budget.deadline,
            host_completed_at=time.monotonic()))
        budget.left()
        phase = "cut_recover_and_f6"
        logged(operation.command + ["exec","-T","fleet-backend","python3","-B","/qa/launch_cut.py","cut-recover"],phase,900)
        live = contract.decode((packet / "evidence/cut-live-report.json").read_bytes())
        events = trace(packet)
        contract.validate_report(live,ready["arm"],events)
        if live["cut"] != ready["cut"] or live["peer_before"] != ready["peer"] or gate["event"] not in events:
            raise ValueError("Original gate/claim/peer evidence changed after restart")
        final_inventory = driver.native_inventory(operation,docker)
        contract.validate_inventory(final_inventory,live,ready["arm"])
        driver.write_json(packet / "cut-native-final.json",final_inventory)
        if contract.decode((packet / "evidence/cut-stop-report.json").read_bytes()) != dict(state="original_namespaces_exited",agents=2,runtime_ready=False):
            raise ValueError("Original terminal physical stop required")
        report["scenario"] = live
        report["matrix_kind"] = "protocol4_cut_supplement"
        report["original_nine_native_acceptance"] = "separate_pending"
    except BaseException as error:
        driver.remember_failure(report,phase,error)
        raise


def main():
    parser = argparse.ArgumentParser(__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--prepare",action="store_true")
    mode.add_argument("--verify",action="store_true")
    mode.add_argument("--execute",action="store_true")
    parser.add_argument("--packet",type=Path)
    parser.add_argument("--heavy-slot-ack",default="")
    parser.add_argument("--docker-context",default="desktop-linux")
    parser.add_argument("--image-build-packet",type=Path)
    args = parser.parse_args()
    try:
        if args.image_build_packet is None:
            raise ValueError("Explicit fresh qualified image packet required")
        driver.bind_candidates(args.image_build_packet)
        if args.prepare:
            qualification()
            packet = driver.prepare(driver.SOURCE_MERGED)
            print(json.dumps(dict(state="cut_prepared_not_executed",packet=str(packet))))
        elif args.packet is None:
            raise ValueError("Explicit owned cut packet required")
        elif args.verify:
            driver.verify(args.packet)
            print("cut_seal_verified_not_executed")
        else:
            # 'native4cut' is mandatory; a former native4 grant is not an authorization.
            return driver.execute(args.packet,args.heavy_slot_ack,args.docker_context,scenario_runner=scenario,
                services_provider=services,compile_validator=cut_compile_proof.verify)
        return 0
    except Exception as error:
        print("Cut QA withheld: " + type(error).__name__)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
