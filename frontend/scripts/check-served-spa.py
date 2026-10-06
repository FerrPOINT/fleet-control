"""Check real HTML/cache/bundle responses after a frontend release; no auth data."""
import argparse
import re
import urllib.error
import urllib.parse
import urllib.request


def read(url):
    try:
        with urllib.request.urlopen(url, timeout=10) as response:
            return response.status, response.headers, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.headers, error.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base-url', required=True)
    args = parser.parse_args()
    base = args.base_url.rstrip('/') + '/'
    status, headers, body = read(base + 'login?logged_out=1')
    assert status == 200 and 'text/html' in headers.get('Content-Type', '')
    assert 'no-store' in headers.get('Cache-Control', ''), 'SPA document can be cached across releases'
    bundles = re.findall(r'src="([^"]+\.js)"', body.decode())
    assert bundles, 'Missing JS entry bundle'
    for bundle in bundles:
        target = urllib.parse.urljoin(base, bundle)
        assert target.startswith(base + 'assets/'), 'Bundle escaped deployment scope'
        status, headers, _ = read(target)
        assert status == 200 and 'text/html' not in headers.get('Content-Type', '')
        assert 'immutable' in headers.get('Cache-Control', ''), 'Hashed bundle is not immutable'
    status, headers, _ = read(base + 'assets/missing-bundle-release-smoke.js')
    assert status == 404, 'Missing bundle falls back to SPA HTML'
    assert 'immutable' not in headers.get('Cache-Control', ''), 'Missing bundle must not be cached for a year'
    print('SPA HTTP smoke PASS: no-store documents, immutable bundles, missing bundle 404')


if __name__ == '__main__':
    main()
