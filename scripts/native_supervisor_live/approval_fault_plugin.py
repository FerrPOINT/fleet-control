"""Owned native approval observations; optionally lose a real exact-action ACK."""
import json
import os
from pathlib import Path


def register(ctx):
    def wire(app, adapter):
        from aiohttp import web

        if os.environ.get('FLEET_NATIVE_SUPERVISOR_TEST') != '1':
            raise RuntimeError('Native approval observer requires explicit QA opt-in')
        root = Path(os.environ['FLEET_NATIVE_APPROVAL_FAULT_ROOT'])
        recovery = os.environ.get('FLEET_NATIVE_APPROVAL_RECOVERY_TEST') == '1'
        if (root != Path('/tmp/fleet-native-supervisor/approval-fault')
                or not root.is_dir() or root.is_symlink()):
            raise RuntimeError('Native approval observer requires its existing owned root')

        def record(**fields):
            body = (json.dumps(fields, separators=(',', ':'))+'\n').encode()
            fd = os.open(root/'native-events.jsonl', os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
            try:
                if os.write(fd, body) != len(body):
                    raise RuntimeError('Native approval observation was not fully saved')
            finally:
                os.close(fd)

        @web.middleware
        async def observe(request, handler):
            denied = adapter._check_auth(request)
            if denied is not None:
                return denied
            if recovery:
                parts = request.path.split('/')
                if request.method == 'POST' and request.path == '/v1/runs':
                    record(kind='run_post')
                if (request.method == 'GET' and len(parts) == 5
                        and parts[1:3] == ['v1', 'runs'] and parts[4] == 'events'):
                    record(kind='events_held', run_id=parts[3])
                    return web.Response(status=503)
                if (request.method == 'GET' and len(parts) == 4
                        and parts[1:3] == ['v1', 'runs']):
                    response = await handler(request)
                    if response.status == 200:
                        payload = json.loads(response.body)
                        approval = payload.get('approval') or {}
                        waiting = (payload.get('object') == 'hermes.run'
                                   and payload.get('run_id') == parts[3]
                                   and payload.get('status') == 'waiting_for_approval'
                                   and approval.get('event') == 'approval.request'
                                   and approval.get('run_id') == parts[3])
                        held = waiting and (root/'hold-approval-readback').is_file()
                        record(kind='waiting_held' if held else 'status_read',
                               run_id=parts[3], session_id=payload.get('session_id'),
                               request_id=approval.get('request_id'), status=payload.get('status'))
                        if held:
                            if request.transport is None:
                                raise RuntimeError('Native waiting snapshot has no transport')
                            request.transport.close()
                    return response
            if (request.method != 'POST' or not request.path.startswith('/v1/runs/')
                    or not request.path.endswith('/approval')):
                return await handler(request)
            body = await request.json()
            run_id = request.path.split('/')[3]
            record(kind='approval_post', run_id=run_id, request_id=body.get('request_id'),
                   choice=body.get('choice'), resolve_all=body.get('resolve_all'))
            response = await handler(request)
            if response.status != 200:
                return response
            ack = json.loads(response.body)
            if (ack.get('object') != 'hermes.run.approval_response'
                    or ack.get('run_id') != run_id
                    or ack.get('request_id') != body.get('request_id')
                    or ack.get('choice') != body.get('choice') or ack.get('resolved') != 1):
                return response
            record(kind='approval_ack', run_id=run_id, request_id=ack['request_id'], choice=ack['choice'])
            try:
                (root/'drop-next-approval').unlink()
            except FileNotFoundError:
                return response
            record(kind='approval_ack_dropped', run_id=run_id, request_id=ack['request_id'])
            if request.transport is None:
                raise RuntimeError('Native approval ACK has no transport')
            request.transport.close()
            return response

        app.middlewares.append(observe)

    ctx.register_platform_handler('api_server', wire)
