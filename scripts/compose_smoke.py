"""Authenticated acceptance against a disposable, clean HMAC Compose stack."""

import argparse
import json
import subprocess
import time
import urllib.error
import urllib.request
import uuid


def request(base, path, token=None, payload=None, expected=200):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    body = None if payload is None else json.dumps(payload).encode()
    req = urllib.request.Request(base + path, data=body, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            status, data = response.status, response.read()
    except urllib.error.HTTPError as error:
        status, data = error.code, error.read()
    assert status == expected, f"{path}: expected {expected}, got {status}"
    return json.loads(data) if data else None


def register(base, suffix):
    return request(base, "/api/v1/auth/register", payload={
        "email": f"smoke-{suffix}@example.test",
        "username": f"smoke_{suffix}",
        "display_name": "Compose smoke",
        "password": uuid.uuid4().hex + "aA1!",
    })


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--compose-project", required=True)
    args = parser.parse_args()
    base = args.url.rstrip("/")
    suffix = uuid.uuid4().hex[:12]
    request(base, "/api/v1/health")
    request(base, "/api/v1/sessions", expected=401)
    with urllib.request.urlopen(base + "/chats", timeout=10) as response:
        assert b'<div id="root">' in response.read(), "SPA route unavailable"

    admin = register(base, suffix + "a")
    owner = register(base, suffix + "b")
    stranger = register(base, suffix + "c")
    assert admin["system_role"] == "admin", "smoke requires an empty database"
    assert owner["system_role"] == stranger["system_role"] == "user"
    agents = request(base, "/api/v1/agent-directory", owner["access_token"])
    executor = next(agent for agent in agents if agent["product_role"] == "executor")
    request(base, "/api/v1/agents", owner["access_token"], expected=403)
    request(base, "/api/v1/sessions?user_id=all", owner["access_token"], expected=403)
    payload = {
        "primary_agent_id": executor["id"], "title": "Private compose smoke",
        "idempotency_key": suffix,
    }
    session = request(base, "/api/v1/sessions", owner["access_token"], payload)
    assert session["visibility"] == "private" and session["leader_agent_id"] is None
    replay = request(base, "/api/v1/sessions", owner["access_token"], payload)
    assert replay["id"] == session["id"], "replay created another session"
    request(base, "/api/v1/sessions", owner["access_token"],
            {**payload, "title": "Changed payload"}, expected=409)
    path = f'/api/v1/sessions/{session["id"]}'
    request(base, path, stranger["access_token"], expected=403)
    request(base, path + "/stream", stranger["access_token"], expected=403)
    assert request(base, "/api/v1/sessions", stranger["access_token"]) == []
    all_sessions = request(base, "/api/v1/sessions?user_id=all", admin["access_token"])
    assert any(item["id"] == session["id"] for item in all_sessions)

    req = urllib.request.Request(base + path + "/stream?cursor=0", headers={
        "Authorization": f'Bearer {owner["access_token"]}',
    })
    started = time.monotonic()
    with urllib.request.urlopen(req, timeout=5) as response:
        assert "text/event-stream" in response.headers["Content-Type"]
        assert response.headers["X-Accel-Buffering"] == "no"
        while True:
            line = response.readline()
            if line.startswith(b"data:"):
                assert json.loads(line[5:])["type"] == "snapshot"
                break
            assert line, "SSE closed without a snapshot"
    assert time.monotonic() - started < 5, "Nginx buffered the first SSE event"

    # The caller owns this disposable project; never restart an ambient stack.
    subprocess.run(["docker", "compose", "-p", args.compose_project,
                    "restart", "backend"], check=True)
    for attempt in range(30):
        try:
            restored = request(base, path, owner["access_token"])
            assert restored["id"] == session["id"]
            break
        except (urllib.error.URLError, AssertionError):
            if attempt == 29:
                raise
            time.sleep(1)
    replay = request(base, "/api/v1/sessions", owner["access_token"], payload)
    assert replay["id"] == session["id"], "restart lost idempotency state"
    print("Compose smoke passed: auth/RBAC, private chat, replay/conflict, "
          "proxied SSE, restart persistence")


if __name__ == "__main__":
    main()
