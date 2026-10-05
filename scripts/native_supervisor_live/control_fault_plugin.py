"""QA-only middleware: lose real steer/stop ACKs and hold original-context GET."""
import hashlib
import json
import os
from pathlib import Path


def register(ctx):
    def wire(app, adapter):
        from aiohttp import web

        if os.environ.get('FLEET_NATIVE_SUPERVISOR_TEST') != '1':
            raise RuntimeError('Control fault fixture requires explicit QA opt-in')
        root = Path(os.environ['FLEET_NATIVE_CONTROL_FAULT_ROOT'])
        if not root.is_absolute() or not root.is_dir() or root.is_symlink():
            raise RuntimeError('Control fault fixture requires its owned existing root')

        def record(kind, **fields):
            body = (json.dumps({'kind': kind, **fields}, separators=(',', ':')) + '\n').encode()
            fd = os.open(root / 'control-events.jsonl', os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
            try:
                if os.write(fd, body) != len(body):
                    raise RuntimeError('Control observation was not fully saved')
            finally:
                os.close(fd)

        @web.middleware
        async def fault(request, handler):
            denied = adapter._check_auth(request)
            if denied is not None:
                return denied
            if request.path == '/fleet/v1/controls/lookup':
                held = (root / 'hold-lookup').exists()
                record('lookup', held=held, key=request.query.get('command_id'))
                if held:
                    return web.json_response({'error': 'owned_qa_control_hold'}, status=503)
            operation = request.path.rsplit('/', 1)[-1]
            if request.method == 'POST' and request.path.startswith('/v1/runs/') and operation in {'steer', 'stop'}:
                raw = await request.read()
                key = request.headers.get('Idempotency-Key')
                record('post', operation=operation, key=key, sha256=hashlib.sha256(raw).hexdigest())
                response = await handler(request)
                if response.status == 200:
                    ack = json.loads(response.body)
                    record('native_ack', operation=operation, key=key, run_id=ack['run_id'])
                    if request.transport is None:
                        raise RuntimeError('Control ACK has no transport')
                    request.transport.close()
                return response
            return await handler(request)

        # Observe the wrapper's cached bounded body; do not read before its request clone.
        app.middlewares.append(fault)

    ctx.register_platform_handler('api_server', wire)
