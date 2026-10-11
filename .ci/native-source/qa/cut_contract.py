"""Closed QA evidence validation, never a source of native custody or permits."""
import hashlib
import json
import re
import uuid

SOURCE = "c3fc175b97168736717c72c2b32e1036c5b6f9db"
HASHES = ["2e6bfa6907b93e6d436d2b6668ae20211aca53a64c433f7e1a98ab51245b3e89",
          "5be8066b6f7dc68f8dda7f1040c0626dad272477c019b07263821df7f86b59a2",
          "1f53542606d6dc9f88f0fead7c269001f449926d8df6ccf4369d6c9874a9445b",
          "af73f6bac7dd123b2927c79dc4001edb6fcc32cf6a7de06d2be98309a2ce393e"]
MATRIX = {"original_stop_ack_before_phase_cas":"accepted",
          "recovered_original_stop_readback":"accepted", "recovered_effective_child":"accepted",
          "next_recovered_activation":"accepted", "next_failure_exact_current_rollback":"accepted",
          "peer_isolation":"accepted"}
EFFECTS = {"prepare_replacement", "attach_replacement", "start_replacement", "stop_replacement"}
AUDITED = EFFECTS | {"read_original_stop", "stop"}


def decode(raw):
    def pairs(items):
        result = {}
        for key,value in items:
            if key in result:
                raise ValueError("Duplicate JSON evidence key")
            result[key] = value
        return result
    def constant(_):
        raise ValueError("Non-finite JSON evidence")
    return json.loads(raw,object_pairs_hook=pairs,parse_constant=constant)


def closed(value, keys):
    if not isinstance(value, dict) or set(value) != set(keys):
        raise ValueError("Closed evidence shape required")


def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=True).encode()).hexdigest()


def sha(value):
    if not isinstance(value,str) or not re.fullmatch("[a-f0-9]{64}",value):
        raise ValueError("Exact SHA256 required")
    return value


def uid(value):
    if not isinstance(value,str) or str(uuid.UUID(value)) != value or uuid.UUID(value).int == 0:
        raise ValueError("Canonical nonzero identity required")
    return value


def arm(value):
    closed(value, ("source_commit","resource_id","generation","stop_id","registration_sha256",
                   "snapshot_sha256","loader_sha256","container_id"))
    if value["source_commit"] != SOURCE:
        raise ValueError("Exact source required")
    for key in ("resource_id","generation","stop_id"):
        uid(value[key])
    for key in ("registration_sha256","snapshot_sha256","loader_sha256","container_id"):
        sha(value[key])
    return value


def action(payload):
    if payload.get("entry") == "read_original_stop":
        return "read_original_stop"
    return payload["request"].get("action")


def root_registration(payload):
    request = payload["request"]
    if action(payload) == "read_original_stop" or request.get("protocol_version") == 4:
        return request["anchor"]["registration"]
    return request.get("registration", {})


def selected(payload, armed):
    return action(payload) in AUDITED and root_registration(payload).get("resource_id") == armed["resource_id"]


def cut_stop(payload, armed):
    request = payload["request"]
    return (action(payload) == "stop" and root_registration(payload).get("generation") == armed["generation"]
            and request.get("operation_id") == armed["stop_id"])


def validate_sources(args, payload, loader_sha256):
    closed(payload,("request","sources","entry"))
    if (len(args) != 4 or args[:2] != ["-I","-c"] or args[3] != "/base-runtime"
            or hashlib.sha256(args[2].encode()).hexdigest() != sha(loader_sha256)
            or not isinstance(payload["sources"],list) or len(payload["sources"]) != 4
            or [hashlib.sha256(s.encode()).hexdigest() for s in payload["sources"]] != HASHES):
        raise ValueError("Original captured loader and four sealed sources required")


def validate_loader(args, payload, armed):
    arm(armed)
    validate_sources(args,payload,armed["loader_sha256"])


def stop_receipt(receipt, armed):
    closed(receipt,("contract_version","operation_id","container_id","resource_id","generation",
                    "snapshot_sha256","state","observation"))
    if (type(receipt["contract_version"]) is not int or receipt["contract_version"] != 3
            or receipt["operation_id"] != armed["stop_id"] or receipt["resource_id"] != armed["resource_id"]
            or receipt["generation"] != armed["generation"] or receipt["snapshot_sha256"] != armed["snapshot_sha256"]
            or receipt["container_id"] != armed["container_id"]
            or receipt["state"] != "observed" or receipt["observation"] != "namespace_exited"):
        raise ValueError("Original known stop/exit required")
    sha(receipt["container_id"])
    return receipt


def validate_gate(value, armed):
    closed(value,("event","journal","stdout_forwarded"))
    event = value["event"]
    closed(event,("call_id","request","ack","ack_sha256","raw_ack_sha256"))
    uid(event["call_id"])
    sha(event["raw_ack_sha256"])
    closed(event["ack"],("protocol_version","action","result"))
    if (value["stdout_forwarded"] is not False or event["request"]["action"] != "stop"
            or event["ack"]["protocol_version"] != 2 or event["ack"]["action"] != "stop"
            or event["ack_sha256"] != digest(event["ack"])):
        raise ValueError("Actual withheld original stop ACK required")
    stop_receipt(event["ack"]["result"],armed)
    journal = value["journal"]
    closed(journal,("sha256","device","inode","size","mode","uid"))
    sha(journal["sha256"])
    if (any(type(journal[k]) is not int or journal[k] <= 0 for k in ("device","inode","size"))
            or journal["size"] > 8 * 1024 * 1024 or journal["mode"] != 0o600 or journal["uid"] != 999):
        raise ValueError("Actual private original stop journal witness required")


def validate_inventory(value, report, armed):
    peer = report["peer_before"]["launch"]["prepared"]["container"]["registration"]
    expected = {armed["generation"],peer["generation"]}
    for record in report["records"]:
        activation = record["activation"]
        expected.add(activation["claim"]["candidate"]["generation"])
        if activation["phase"] == "rolled_back":
            expected.add(activation["claim"]["rollback"]["generation"])
    if len(value) != 6 or {v["generation"] for v in value.values()} != expected:
        raise ValueError("Only original two namespaces and four reserved replacement generations allowed")


def response(payload, output, status, armed):
    """Validate only actual returned bytes; this never creates a Base response."""
    if type(status) is not int or status != 0 or len(output) > 65536:
        raise ValueError("Actual bounded Base exit0 ACK required")
    ack = decode(output)
    request, name = payload["request"], action(payload)
    if name == "read_original_stop":
        closed(ack,("protocol_version","action","result","request_sha256"))
        if (type(ack["protocol_version"]) is not int or ack["protocol_version"] != 4 or ack["action"] != name or ack["request_sha256"] != digest(request)
                or digest(request["anchor"]["registration"]) != armed["registration_sha256"]):
            raise ValueError("Original stop readback binding required")
        stop_receipt(ack["result"],armed)
    elif name == "stop":
        closed(ack,("protocol_version","action","result"))
        if (type(ack["protocol_version"]) is not int or ack["protocol_version"] != 2 or ack["action"] != name
                or digest(request["registration"]) != armed["registration_sha256"]):
            raise ValueError("Original mapped stop binding required")
        stop_receipt(ack["result"],armed)
    else:
        closed(ack,("protocol_version","action","result","command_sha256","intent_sha256","anchor_sha256"))
        identity = {k:v for k,v in request["anchor"].items() if k not in ("recovery","action","protocol_version")}
        if (type(ack["protocol_version"]) is not int or ack["protocol_version"] != 4 or ack["action"] != name
                or ack["command_sha256"] != digest(request["command"])
                or ack["intent_sha256"] != request["command"]["intent_sha256"] or ack["anchor_sha256"] != digest(identity)):
            raise ValueError("Original Base4 command/anchor/intent binding required")
        expected_state = {"prepare_replacement":"prepared","attach_replacement":"attached",
                          "start_replacement":"observed","stop_replacement":"observed"}[name]
        if ack["result"].get("state") != expected_state:
            raise ValueError("Actual positive native effect ACK required")
        if name in ("start_replacement","stop_replacement") and ack["result"].get("observation") != (
                "running" if name == "start_replacement" else "namespace_exited"):
            raise ValueError("Actual physical native observation required")
    return ack


def projection(payload, armed):
    request, name = payload["request"], action(payload)
    command = request.get("command")
    anchor = request.get("anchor", request)
    lease = anchor.get("recovery")
    return dict(action=name,request_sha256=digest(request),
        anchor_sha256=None if name == "stop" else digest({k:v for k,v in anchor.items() if k not in ("recovery","action","protocol_version")}),
        command=None if command is None else dict(operation_id=command["operation_id"],stop_id=command["stop_id"],
            generation=command["policy"]["generation"],predecessor_generation=command["predecessor_generation"],
            intent_sha256=command["intent_sha256"],command_sha256=digest(command)),
        lease=None if lease is None else dict(id=lease["request"]["id"],epoch=lease["epoch"],
            version=lease["lease_version"],controller_id=lease["request"]["controller_id"],
            launch_id=lease["request"]["launch_id"],expires_at=lease["lease_expires_at"]))


def validate_trace(events, armed, records):
    """Require real original readback and one linear native effect per reserved ID."""
    arm(armed)
    root, second, failed = records
    stops, readbacks, effects = [], [], {}
    calls, commands, anchors = set(), {}, set()
    for event in events:
        closed(event,("call_id","request","ack","ack_sha256","raw_ack_sha256"))
        uid(event["call_id"])
        if event["call_id"] in calls:
            raise ValueError("Duplicate native trace")
        calls.add(event["call_id"])
        sha(event["ack_sha256"])
        sha(event["raw_ack_sha256"])
        if event["ack_sha256"] != digest(event["ack"]):
            raise ValueError("ACK canonical hash mismatch")
        request, ack = event["request"], event["ack"]
        closed(request,("action","request_sha256","anchor_sha256","command","lease"))
        sha(request["request_sha256"])
        name = request["action"]
        if name == "stop":
            closed(ack,("protocol_version","action","result"))
            if ack["protocol_version"] != 2 or ack["action"] != name:
                raise ValueError("Mapped original stop ACK required")
            stop_receipt(ack["result"],armed)
            stops.append(event)
        elif name == "read_original_stop":
            closed(ack,("protocol_version","action","result","request_sha256"))
            if ack["protocol_version"] != 4 or ack["action"] != name:
                raise ValueError("Original stop readback ACK required")
            stop_receipt(ack["result"],armed)
            if ack["request_sha256"] != request["request_sha256"]:
                raise ValueError("Readback request hash mismatch")
            readbacks.append(event)
        elif name in EFFECTS:
            closed(ack,("protocol_version","action","result","command_sha256","intent_sha256","anchor_sha256"))
            if (ack["protocol_version"] != 4 or ack["action"] != name
                    or ack["result"].get("state") != {"prepare_replacement":"prepared","attach_replacement":"attached",
                        "start_replacement":"observed","stop_replacement":"observed"}[name]
                    or ack["anchor_sha256"] != request["anchor_sha256"]):
                raise ValueError("Positive bound Base4 ACK required")
            if name in ("start_replacement","stop_replacement") and ack["result"].get("observation") != (
                    "running" if name == "start_replacement" else "namespace_exited"):
                raise ValueError("Physical effect readback required")
            command = request["command"]
            closed(command,("operation_id","stop_id","generation","predecessor_generation","intent_sha256","command_sha256"))
            key = name, uid(command["operation_id"])
            if key in effects or ack["command_sha256"] != sha(command["command_sha256"]) or ack["intent_sha256"] != sha(command["intent_sha256"]):
                raise ValueError("Repeated/unbound native effect")
            if commands.setdefault(key[1],command) != command:
                raise ValueError("Immutable native command changed between phases")
            effects[key] = command
        else:
            raise ValueError("Unexpected audited native action")
        if name != "stop":
            anchors.add(sha(request["anchor_sha256"]))
            lease = request["lease"]
            closed(lease,("id","epoch","version","controller_id","launch_id","expires_at"))
            for key in ("id","controller_id","launch_id"):
                uid(lease[key])
            if lease["launch_id"] != armed["generation"] or type(lease["epoch"]) is not int or lease["epoch"] <= 0 or type(lease["version"]) is not int or lease["version"] <= 0:
                raise ValueError("Original recovered lease required")
            if lease["id"] != root["lease"]["request"]["id"] or lease["controller_id"] != root["lease"]["request"]["controller_id"] or lease["epoch"] != root["lease"]["epoch"]:
                raise ValueError("Original anchor/current recovered epoch drift")
    if len(anchors) != 1 or len(stops) != 1 or not readbacks or stops[0]["ack"]["result"] != root["activation"]["previous_stop"]:
        raise ValueError("One original stop ACK and exact recovered readback required")
    expected = {}
    reserved_commands = {}
    for record in records:
        activation = record["activation"]
        commands = [activation["claim"]["candidate"]]
        if activation["phase"] == "rolled_back":
            commands.append(activation["claim"]["rollback"])
        for command in commands:
            reserved_commands[command["operation_id"]] = dict(command,
                intent_sha256=activation["claim"]["intent_sha256"],predecessor_generation=(
                    activation["candidate"]["prepared"]["container"]["registration"]["generation"]
                    if command == activation["claim"]["rollback"] else
                    activation["claim"]["previous"]["prepared"]["container"]["registration"]["generation"]))
            for name in ("prepare_replacement","attach_replacement","start_replacement"):
                expected[name,command["operation_id"]] = command["generation"]
    expected["stop_replacement",root["activation"]["claim"]["candidate"]["operation_id"]] = root["activation"]["claim"]["candidate"]["generation"]
    expected["stop_replacement",second["activation"]["claim"]["candidate"]["operation_id"]] = second["activation"]["claim"]["candidate"]["generation"]
    expected["stop_replacement",failed["activation"]["claim"]["candidate"]["operation_id"]] = failed["activation"]["claim"]["candidate"]["generation"]
    expected["stop_replacement",failed["activation"]["claim"]["rollback"]["operation_id"]] = failed["activation"]["claim"]["rollback"]["generation"]
    if set(effects) != set(expected):
        raise ValueError("Exact native Base4 effect inventory required")
    for key, generation in expected.items():
        command = effects[key]
        if command["generation"] != generation or any(command[k] != v for k,v in reserved_commands[key[1]].items()):
            raise ValueError("Reserved original generation/stop/intent/predecessor mismatch")


def validate_report(value, armed, events):
    closed(value,("state","source_commit","matrix","cut","records","peer_before","peer_after",
                  "runtime_ready","sdlc_completion"))
    if (value["state"] != "scoped_cut_matrix_passed" or value["source_commit"] != SOURCE or value["matrix"] != MATRIX
            or value["runtime_ready"] is not False or value["sdlc_completion"] is not False):
        raise ValueError("Closed source-only cut matrix required")
    cut = value["cut"]
    closed(cut,("phase","previous_stop","candidate","rollback","readiness","claim","plan_sha256","physical_exit","draining"))
    if cut["phase"] != "stopping_previous" or any(cut[k] is not None for k in ("previous_stop","candidate","rollback","readiness")) or cut["physical_exit"] is not True or cut["draining"] is not True:
        raise ValueError("Exact ACK-before-phase-CAS cut required")
    sha(cut["plan_sha256"])
    records = value["records"]
    if not isinstance(records,list) or len(records) != 3:
        raise ValueError("Recovered, next and failed-next records required")
    root, second, failed = records
    if root["activation"]["claim"] != cut["claim"]:
        raise ValueError("Original immutable claim changed")
    original = root["activation"]["claim"]["previous"]
    if (digest(original["prepared"]["container"]["registration"]) != armed["registration_sha256"]
            or digest(original["snapshot"]) != armed["snapshot_sha256"] or original["stop_id"] != armed["stop_id"]):
        raise ValueError("Original registration/start snapshot/stop identity required")
    previous = None
    for index, record in enumerate(records):
        closed(record,("activation","effective_revision","launch_generation","plan_sha256","managed_sha256",
                       "lease","lease_receipt","lease_valid","health_running","authority_count"))
        activation = record["activation"]
        closed(activation,("claim","phase","previous_stop","candidate","candidate_stop","rollback","readiness"))
        claim = activation["claim"]
        lease = record["lease"]
        closed(lease,("request","epoch","lease_version","lease_expires_at"))
        if (lease["request"]["launch_id"] != armed["generation"] or lease["request"]["agent_id"] != armed["resource_id"]
                or lease["request"]["original_controller_id"] != original["controller_id"]
                or lease["request"]["controller_id"] == original["controller_id"]
                or lease["request"] != root["lease"]["request"] or lease["epoch"] != root["lease"]["epoch"]
                or type(lease["lease_version"]) is not int or lease["lease_version"] <= 0):
            raise ValueError("Same original anchor and current recovered logical controller required")
        if record["lease_valid"] is not True or record["health_running"] is not True or type(record["authority_count"]) is not int or record["authority_count"] != 1:
            raise ValueError("Fresh actual native lease/authority/readiness required")
        heartbeat = record["lease_receipt"]
        closed(heartbeat,("ack","receipt"))
        if heartbeat["ack"] != dict(state="controller_heartbeat",recovery_id=record["lease"]["request"]["id"],
                lease_version=record["lease"]["lease_version"],lease_expires_at=record["lease"]["lease_expires_at"]):
            raise ValueError("Latest lease heartbeat must be applied")
        observed = heartbeat["receipt"]
        if (observed["state"] != "observed" or observed["observation"] != "namespace_exited"
                or observed["generation"] != armed["generation"] or observed["container_id"] != armed["container_id"]
                or observed["snapshot"] != original["snapshot"]):
            raise ValueError("Original exited anchor readback required")
        if index == 0:
            if record["plan_sha256"] != cut["plan_sha256"] or claim.get("lineage") is not None or claim["previous"]["prepared"]["container"]["registration"]["generation"] != armed["generation"]:
                raise ValueError("Original plan/root required")
        else:
            lineage = claim["lineage"]
            if (lineage["family_id"] != root["activation"]["claim"]["id"]
                    or lineage["predecessor_activation_id"] != previous["activation"]["claim"]["id"]
                    or lineage["predecessor_intent_sha256"] != previous["activation"]["claim"]["intent_sha256"]
                    or lineage["anchor"] != root["activation"]["claim"]["previous"]
                    or claim["previous_revision"] != previous["effective_revision"]
                    or claim["previous"]["prepared"]["container"]["registration"]["generation"] != previous["launch_generation"]):
                raise ValueError("Exact current-effective predecessor lineage required")
        rollback = index == 2
        launch = activation["rollback" if rollback else "candidate"]
        reserved = claim["rollback" if rollback else "candidate"]
        closed(activation["readiness"],("generation","files_sha256","capabilities_sha256"))
        sha(activation["readiness"]["files_sha256"])
        sha(activation["readiness"]["capabilities_sha256"])
        if (activation["phase"] != ("rolled_back" if rollback else "committed")
                or record["effective_revision"] != (previous["effective_revision"] if rollback else claim["revision"])
                or record["launch_generation"] != launch["prepared"]["container"]["registration"]["generation"]
                or activation["readiness"]["generation"] != record["launch_generation"]
                or record["launch_generation"] != reserved["generation"]
                or launch["prepared"]["container"]["registration"]["operation_id"] != reserved["operation_id"]
                or launch["stop_id"] != reserved["stop_id"]
                or record["managed_sha256"] != (claim["previous_files_sha256"] if rollback else claim["files_sha256"])
                or record["managed_sha256"] != activation["readiness"]["files_sha256"]):
            raise ValueError("Physical readiness before effective revision required")
        if rollback and (record["managed_sha256"] != previous["managed_sha256"] or record["launch_generation"] in
                (previous["launch_generation"],activation["candidate"]["prepared"]["container"]["registration"]["generation"])):
            raise ValueError("Exact current working config on new rollback generation required")
        previous = record
    if value["peer_before"] != value["peer_after"]:
        raise ValueError("Peer launch/mounts/PID/files/POSTs changed")
    validate_trace(events,armed,records)
