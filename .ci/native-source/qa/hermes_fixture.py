"""QA transport faults around genuine pinned Hermes; never substitutes its API/model runs."""
import argparse
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time

LOCK = threading.Lock()
COUNTS = Path("/logs/qa-native-posts.json")
ACK_LOSS = Path("/logs/qa-native-ack-loss.json")
HOPS = {"connection", "keep-alive", "proxy-authenticate", "proxy-authorization", "te", "trailer", "transfer-encoding", "upgrade"}


def unknown(body):
    value = json.loads(body)
    return isinstance(value.get("input"), str) and value["input"].startswith("unknown-live-")


def increment(path, body):
    key = hashlib.sha256(body).hexdigest()
    with LOCK:
        values = json.loads(path.read_text()) if path.exists() else {}
        values[key] = values.get(key, 0) + 1
        temporary = path.with_suffix(".tmp")
        with temporary.open("w", encoding="utf-8", newline="\n") as stream:
            json.dump(values, stream, sort_keys=True)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.replace(path)


def count_post(body):
    increment(COUNTS,body)


def count_ack_loss(body):
    increment(ACK_LOSS,body)


class Proxy(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    gateway_port = None

    def log_message(self, *_):
        pass

    def relay(self):
        connection = http.client.HTTPConnection("127.0.0.1", self.gateway_port, timeout=150)
        try:
            size = int(self.headers.get("Content-Length", "0"))
            if size > 1024 * 1024 or self.headers.get("Transfer-Encoding"):
                self.send_error(413)
                return
            body = self.rfile.read(size)
            create = self.command == "POST" and self.path == "/v1/runs"
            if create:
                count_post(body)
            headers = {k: v for k, v in self.headers.items() if k.lower() not in HOPS | {"host"}}
            connection.request(self.command, self.path, body=body, headers=headers)
            response = connection.getresponse()
            # Consume a real native ACK, then lose it. Never retry/issue a replacement POST.
            if create and unknown(body) and 200 <= response.status < 300:
                response.read(1024 * 1024 + 1)
                count_ack_loss(body)
                self.close_connection = True
                self.connection.shutdown(2)
                return
            self.send_response(response.status)
            for name, value in response.getheaders():
                if name.lower() not in HOPS:
                    self.send_header(name, value)
            self.send_header("Connection", "close")
            self.end_headers()
            while raw := response.read1(65536):
                self.wfile.write(raw)
                self.wfile.flush()
        except (OSError, http.client.HTTPException, ValueError):
            # No raw auth/header/body logging, fabricated ACK or successful fallback.
            self.close_connection = True
        finally:
            connection.close()
            self.close_connection = True

    do_GET = do_POST = do_DELETE = relay


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["serve"])
    parser.add_argument("--host", choices=["0.0.0.0"], required=True)
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args()
    if not 1024 <= args.port < 65534 or os.environ.get("HERMES_HOME") != "/config":
        raise SystemExit("QA contract mismatch")
    # Same opt-in fault used by earlier genuine readiness rollback acceptance.
    if "QA_READINESS_DELAY_18" in Path("/config/SOUL.md").read_text():
        time.sleep(120)
    os.environ.setdefault("OPENAI_API_KEY", "owned-local-model-fixture")
    child = subprocess.Popen([sys.executable, "/runtime/hermes-container.py", "serve", "--host", args.host,
                              "--port", str(args.port + 1)])
    Proxy.gateway_port = args.port + 1
    server = ThreadingHTTPServer((args.host, args.port), Proxy)
    server.daemon_threads = True
    def stop(*_):
        child.terminate()
        raise SystemExit(1)
    signal.signal(signal.SIGTERM, stop)
    try:
        server.serve_forever()
    finally:
        server.server_close()
        child.terminate()
        try:
            child.wait(timeout=5)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait()


if __name__ == "__main__":
    main()
