"""QA-only native middleware: discard a real accepted response, never fake a run."""
import hashlib
import json
import os
from pathlib import Path


def register(ctx):
    def wire(app, adapter):
        from aiohttp import web

        if os.environ.get('FLEET_NATIVE_SUPERVISOR_TEST') != '1':
            raise RuntimeError('Native fault fixture requires explicit QA opt-in')
        root = Path(os.environ['FLEET_NATIVE_FAULT_ROOT'])
        if not root.is_absolute() or not root.is_dir() or root.is_symlink():
            raise RuntimeError('Native fault fixture requires its existing owned root')

        def record(kind, **fields):
            body = (json.dumps({'kind':kind, **fields}, separators=(',',':'))+'\n').encode()
            fd = os.open(root/'native-events.jsonl', os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
            try:
                if os.write(fd, body) != len(body):
                    raise RuntimeError('Native fault observation was not fully saved')
            finally:
                os.close(fd)

        @web.middleware
        async def fault(request, handler):
            denied = adapter._check_auth(request)
            if denied is not None:
                return denied
            if request.path == '/fleet/v1/recovery/lookup':
                blocked = (root/'hold-lookup').exists()
                record('lookup', blocked=blocked)
                if blocked:
                    return web.json_response({'error':'owned_qa_recovery_hold'}, status=503)
            if request.path.startswith('/v1/runs/') and request.path.endswith('/events'):
                record('events')
            if request.method == 'POST' and request.path == '/v1/runs':
                raw = await request.read()
                key = request.headers.get('Idempotency-Key')
                digest = hashlib.sha256(raw).hexdigest()
                record('post', key=key, sha256=digest)
                response = await handler(request)
                if response.status != 202:
                    return response
                accepted = json.loads(response.body)
                record('accepted', key=key, sha256=digest, run_id=accepted['run_id'])
                # Only the client acknowledgement is lost; native reservation/inference already ran.
                if request.transport is None:
                    raise RuntimeError('Native accepted connection has no transport')
                request.transport.close()
                return response
            return await handler(request)

        app.middlewares.append(fault)

    ctx.register_platform_handler('api_server', wire)
