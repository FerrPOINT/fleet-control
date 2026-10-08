import type { Page } from '@playwright/test'
import { generateKeyPairSync, sign } from 'node:crypto'

export async function installSsoMocks(page: Page, actor: () => string) {
  const { privateKey, publicKey } = generateKeyPairSync('ec', { namedCurve: 'P-256' })
  const jwk = { ...publicKey.export({ format: 'jwk' }), kid: 'qa', alg: 'ES256', use: 'sig' }
  let issuer = 'http://localhost:7701'
  let nonce = ''
  await page.route('**/oidc/authorize**', async (route) => {
    const url = new URL(route.request().url())
    issuer = url.origin
    nonce = url.searchParams.get('nonce') ?? ''
    const callback = new URL(url.searchParams.get('redirect_uri') ?? '/')
    callback.searchParams.set('code', 'qa-code')
    callback.searchParams.set('state', url.searchParams.get('state') ?? '')
    await route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: `<!doctype html><script>location.replace(${JSON.stringify(callback.toString())})</script>`,
    })
  })
  await page.route('**/oidc/token', async (route) => {
    const header = Buffer.from(JSON.stringify({ alg: 'ES256', typ: 'JWT', kid: 'qa' })).toString(
      'base64url',
    )
    const payload = Buffer.from(
      JSON.stringify({
        iss: issuer,
        aud: 'fleet-control',
        sub: actor(),
        email: 'admin@fleet-control.local',
        nonce,
        iat: Math.floor(Date.now() / 1000),
        exp: Math.floor(Date.now() / 1000) + 3600,
      }),
    ).toString('base64url')
    const content = `${header}.${payload}`
    const signature = sign('sha256', Buffer.from(content), {
      key: privateKey,
      dsaEncoding: 'ieee-p1363',
    }).toString('base64url')
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'access-control-allow-origin': '*' },
      body: JSON.stringify({
        access_token: 'qa-access-token',
        id_token: `${content}.${signature}`,
        expires_in: 3600,
      }),
    })
  })
  await page.route('**/oidc/jwks', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'access-control-allow-origin': '*' },
      body: JSON.stringify({ keys: [jwk] }),
    }),
  )
}
