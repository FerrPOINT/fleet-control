import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '../..')
const markdownRoots = [path.join(repoRoot, 'README.md'), path.join(repoRoot, 'docs')]
const ignoredSchemes = /^(https?:|mailto:|#)/
const linkPattern = /\[[^\]]+\]\(([^)]+)\)/g

function collectMarkdownFiles(targetPath, files = []) {
  if (!existsSync(targetPath)) return files
  const stats = statSync(targetPath)
  if (stats.isFile() && targetPath.endsWith('.md')) {
    files.push(targetPath)
  }
  if (stats.isDirectory()) {
    for (const entry of readdirSync(targetPath)) {
      collectMarkdownFiles(path.join(targetPath, entry), files)
    }
  }
  return files
}

function normalizeTarget(rawTarget) {
  const unwrapped = rawTarget.trim().replace(/^<(.+)>$/, '$1')
  const [withoutFragment] = unwrapped.split('#')
  return withoutFragment
}

const failures = []
const markdownFiles = markdownRoots.flatMap((root) => collectMarkdownFiles(root))

for (const filePath of markdownFiles) {
  const content = readFileSync(filePath, 'utf8')
  for (const match of content.matchAll(linkPattern)) {
    const target = normalizeTarget(match[1])
    if (!target || ignoredSchemes.test(target)) continue

    const absoluteTarget = path.resolve(path.dirname(filePath), decodeURI(target))
    if (!absoluteTarget.startsWith(repoRoot)) {
      failures.push(`${path.relative(repoRoot, filePath)} links outside repo: ${target}`)
      continue
    }

    if (!existsSync(absoluteTarget)) {
      failures.push(`${path.relative(repoRoot, filePath)} has missing link: ${target}`)
    }
  }
}

if (failures.length) {
  console.error(failures.join('\n'))
  throw new Error(`Markdown link check failed with ${failures.length} missing or unsafe links.`)
}

console.log(`Checked ${markdownFiles.length} Markdown files.`)
