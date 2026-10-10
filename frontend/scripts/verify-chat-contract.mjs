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

const documentationKeys = new Set([
  'title',
  'description',
  'example',
  'examples',
  'externalDocs',
  'deprecated',
])
const schemaMaps = new Set([
  'properties',
  'patternProperties',
  '$defs',
  'definitions',
  'dependentSchemas',
])
const schemaValues = new Set([
  'items',
  'additionalProperties',
  'unevaluatedProperties',
  'contains',
  'not',
  'if',
  'then',
  'else',
  'propertyNames',
])

function canonicalValue(value) {
  if (Array.isArray(value)) return value.map(canonicalValue)
  if (value !== null && typeof value === 'object')
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, item]) => [key, canonicalValue(item)]),
    )
  return value
}

function sorted(values) {
  return [...values].sort((left, right) =>
    JSON.stringify(left).localeCompare(JSON.stringify(right)),
  )
}

// Discard documentation only; validation constraints remain part of the wire contract.
export function wireSchema(schema, document, references = new Set()) {
  if (typeof schema === 'boolean') return schema
  if (schema === null || typeof schema !== 'object' || Array.isArray(schema))
    throw new Error('Invalid wire schema')
  if (schema.$ref) {
    if (!schema.$ref.startsWith('#/components/schemas/'))
      throw new Error('External schema reference')
    const name = schema.$ref.split('/').at(-1)
    if (!Object.hasOwn(document.components?.schemas ?? {}, name))
      throw new Error(`Missing schema ${name}`)
    const component = document.components?.schemas?.[name]
    if (component === undefined) throw new Error(`Missing schema ${name}`)
    if (references.has(name)) throw new Error(`Recursive schema ${name}`)
    const next = new Set(references).add(name)
    const resolved = wireSchema(component, document, next)
    const siblings = Object.fromEntries(
      Object.entries(schema).filter(([key]) => key !== '$ref' && !documentationKeys.has(key)),
    )
    return Object.keys(siblings).length
      ? { allOf: sorted([resolved, wireSchema(siblings, document, references)]) }
      : resolved
  }
  const output = []
  for (const [key, value] of Object.entries(schema).sort(([left], [right]) =>
    left.localeCompare(right),
  )) {
    if (documentationKeys.has(key)) continue
    if (schemaMaps.has(key))
      output.push([
        key,
        Object.fromEntries(
          Object.entries(value)
            .sort(([left], [right]) => left.localeCompare(right))
            .map(([name, item]) => [name, wireSchema(item, document, references)]),
        ),
      ])
    else if (schemaValues.has(key))
      output.push([
        key,
        Array.isArray(value)
          ? value.map((item) => wireSchema(item, document, references))
          : wireSchema(value, document, references),
      ])
    else if (['oneOf', 'anyOf', 'allOf'].includes(key))
      output.push([key, sorted(value.map((item) => wireSchema(item, document, references)))])
    else if (['required', 'enum', 'type'].includes(key) && Array.isArray(value))
      output.push([key, sorted(value.map(canonicalValue))])
    else output.push([key, canonicalValue(value)])
  }
  return Object.fromEntries(output)
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
  const fleet = JSON.parse(readFileSync(resolve(repo, 'openapi/openapi.json'), 'utf8'))
  const record = process.argv.indexOf('--record')
  const tracker = process.argv.indexOf('--tracker')
  if (record !== -1) {
    const source = JSON.parse(readFileSync(process.argv[record + 1], 'utf8'))
    const candidate = trackerContract(source)
    verifyContract(fleet, candidate)
    writeFileSync(snapshotPath, `${JSON.stringify(candidate, null, 2)}\n`)
  }
  const contract = JSON.parse(readFileSync(snapshotPath, 'utf8'))
  verifyContract(fleet, contract)
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
