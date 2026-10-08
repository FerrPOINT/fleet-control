"""Owned QA only: delay a genuine final observer caps response, never replace it."""
import asyncio
import os
from pathlib import Path
import re

CAPS = '/fleet/v1/request-observations/capabilities'
READ = '/fleet/v1/request-observations/'
PAUSE_SECONDS = 2
NOFOLLOW = getattr(os, 'O_NOFOLLOW', 0)


def owned_root(environment):
    if environment.get('FLEET_NATIVE_SUPERVISOR_TEST') != '1':
        raise RuntimeError('Observer fault fixture requires explicit QA opt-in')
    root = Path(environment['FLEET_NATIVE_OBSERVER_FAULT_ROOT'])
    if not root.is_absolute() or root.is_symlink() or not root.is_dir():
        raise RuntimeError('Observer fault fixture requires its owned existing root')
    return root


def marker(root, name, body):
    temporary = root / (name + '.tmp')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | NOFOLLOW, 0o600)
    try:
        if os.write(fd, body) != len(body):
            raise RuntimeError('Observer fault marker was not fully saved')
        os.fsync(fd)
    finally:
        os.close(fd)
    os.rename(temporary, root / name)


class FinalCapsPause:
    def __init__(self, root):
        self.root = root
        self.observed_run = None

    def armed_run(self):
        try:
            path = self.root / 'armed-run'
            if path.is_symlink():
                raise RuntimeError('Observer arm must be an owned regular file')
            fd = os.open(path, os.O_RDONLY | NOFOLLOW)
        except FileNotFoundError:
            return None
        try:
            run = os.read(fd, 65).decode('ascii')
        finally:
            os.close(fd)
        if not re.fullmatch(r'run_[a-f0-9]{32}', run):
            raise RuntimeError('Observer fault fixture requires an exact original run')
        return run

    async def after(self, request, response):
        if request.method != 'GET' or response.status != 200:
            return response
        run = self.armed_run()
        if run is None:
            self.observed_run = None
            return response
        if request.path == READ + run:
            self.observed_run = run
        elif request.path == CAPS and self.observed_run == run:
            self.observed_run = None
            if type(response.body) is not bytes or len(response.body) > 16384:
                raise RuntimeError('Observer fixture requires a genuine bounded caps response')
            # Publish only after the real authenticated handler has built its response.
            marker(self.root, 'ready.json', response.body)

            async def release():
                while not (self.root / 'release').exists():
                    await asyncio.sleep(0.005)

            try:
                await asyncio.wait_for(release(), timeout=PAUSE_SECONDS)
            except TimeoutError:
                raise RuntimeError('Owned observer pause expired without release') from None
            marker(self.root, 'returned', b'genuine-200')
        return response


def middleware(adapter, environment):
    pause = FinalCapsPause(owned_root(environment))

    async def fault(request, handler):
        denied = adapter._check_auth(request)
        if denied is not None:
            return denied
        response = await handler(request)
        return await pause.after(request, response)

    return fault


def register(ctx):
    def wire(app, adapter):
        from aiohttp import web

        app.middlewares.append(web.middleware(middleware(adapter, os.environ)))

    ctx.register_platform_handler('api_server', wire)
