import { spawn } from 'node:child_process'
import process from 'node:process'

const previewPort = process.env.SCREENSHOT_PREVIEW_PORT ?? '4173'
const baseUrl = process.env.SCREENSHOT_BASE_URL ?? `http://127.0.0.1:${previewPort}`

function run(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: 'inherit',
      shell: false,
      ...options,
    })
    child.on('error', reject)
    child.on('exit', (code) => {
      if (code === 0) resolve()
      else reject(new Error(`${command} ${args.join(' ')} exited with code ${code}`))
    })
  })
}

async function waitForServer() {
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      const response = await fetch(baseUrl)
      if (response.ok) return
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 1000))
    }
  }
  throw new Error(`Preview server did not become ready at ${baseUrl}`)
}

const preview = spawn(
  process.execPath,
  ['node_modules/vite/bin/vite.js', 'preview', '--host', '127.0.0.1', '--port', previewPort],
  {
    stdio: 'inherit',
    shell: false,
  },
)

try {
  await waitForServer()
  await run(process.execPath, ['scripts/capture-screenshots.mjs'], {
    env: {
      ...process.env,
      SCREENSHOT_BASE_URL: baseUrl,
    },
  })
} finally {
  preview.kill('SIGTERM')
}
