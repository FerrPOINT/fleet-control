import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { isDeepStrictEqual } from 'node:util'

const schemaPairs = {
  TrackerTaskContext: 'SdlcContext',
  TrackerQuestion: 'Question',
  TrackerAnswer: 'Answer',
  TrackerRequirementsRevision: 'RequirementsRevision',
  TrackerConfirmation: 'Confirmation',
  ClarificationAnswerRequest: 'AnswerCommand',
  ConfirmRequirementsRequest: 'ConfirmCommand',
}

// Resolve component names so independently named DTOs compare by their wire shape.
export function wireSchema(schema, document) {
  if (schema.$ref) {
    const name = schema.$ref.split('/').at(-1)
    const component = document.components?.schemas?.[name]
    if (!component) throw new Error(`Missing schema ${name}`)
    return wireSchema(component, document)
  }
  const output = {}
  for (const key of ['type', 'format', 'enum', 'required']) {
    if (schema[key] !== undefined)
      output[key] = Array.isArray(schema[key]) ? [...schema[key]].sort() : schema[key]
  }
  if (schema.properties) {
    output.properties = Object.fromEntries(
      Object.entries(schema.properties)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, value]) => [key, wireSchema(value, document)]),
    )
  }
  if (schema.items) output.items = wireSchema(schema.items, document)
  for (const key of ['oneOf', 'anyOf', 'allOf']) {
    if (schema[key])
      output[key] = schema[key]
        .map((value) => wireSchema(value, document))
        .sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)))
  }
  return output
}

export function trackerContract(document) {
  return Object.fromEntries(
    Object.entries(schemaPairs).map(([fleet, tracker]) => [
      fleet,
      wireSchema({ $ref: `#/components/schemas/${tracker}` }, document),
    ]),
  )
}

export function verifyContract(fleet, contract) {
  for (const name of Object.keys(schemaPairs)) {
    const actual = wireSchema({ $ref: `#/components/schemas/${name}` }, fleet)
    if (!isDeepStrictEqual(actual, contract[name]))
      throw new Error(`Chat contract drift: ${name}. Coordinate Tracker and Fleet before rollout.`)
  }
}

function main() {
  const repo = resolve(fileURLToPath(new URL('../..', import.meta.url)))
  const snapshotPath = resolve(repo, 'docs/contracts/tracker-clarification-v1.json')
  const record = process.argv.indexOf('--record')
  const tracker = process.argv.indexOf('--tracker')
  if (record !== -1) {
    const source = JSON.parse(readFileSync(process.argv[record + 1], 'utf8'))
    writeFileSync(snapshotPath, `${JSON.stringify(trackerContract(source), null, 2)}\n`)
  }
  const contract = JSON.parse(readFileSync(snapshotPath, 'utf8'))
  verifyContract(JSON.parse(readFileSync(resolve(repo, 'openapi/openapi.json'), 'utf8')), contract)
  if (tracker !== -1) {
    const current = trackerContract(JSON.parse(readFileSync(process.argv[tracker + 1], 'utf8')))
    if (!isDeepStrictEqual(current, contract))
      throw new Error('Tracker differs from the accepted v1 snapshot')
  }
  console.log(
    tracker !== -1
      ? 'Verified seven Fleet DTOs against the accepted snapshot and provided Tracker source.'
      : 'Verified seven Fleet DTOs against the accepted local snapshot; published Tracker source was not provided.',
  )
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main()
