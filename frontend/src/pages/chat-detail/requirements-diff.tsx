import type { RequirementsRevision } from '@/api/task-chats'

const labels = {
  goal: 'Цель',
  scope: 'Входит в объём',
  exclusions: 'Не входит в объём',
  scenarios: 'Сценарии',
  acceptance_criteria: 'Критерии приёмки',
  constraints: 'Ограничения',
  dependencies: 'Зависимости',
  assumptions: 'Допущения',
  checklist: 'Checklist',
  prerequisites: 'Технические prerequisites',
}
function Value({ value }: { value: string | string[] }) {
  return Array.isArray(value) ? (
    value.length ? (
      <ul>
        {value.map((item, index) => (
          <li key={index}>{item}</li>
        ))}
      </ul>
    ) : (
      <p>Пусто</p>
    )
  ) : (
    <p>{value || 'Пусто'}</p>
  )
}
export function RequirementsDiff({
  before,
  after,
}: {
  before: RequirementsRevision
  after: RequirementsRevision
}) {
  const changed = (Object.keys(labels) as (keyof typeof labels)[]).filter(
    (key) => JSON.stringify(before[key]) !== JSON.stringify(after[key]),
  )
  return (
    <section aria-label="Изменения требований" className="fc-chat-requirements">
      <h3>
        Изменения редакции {before.revision} → {after.revision}
      </h3>
      {!changed.length && <p>Содержимое документа не изменилось.</p>}
      {changed.map((key) => (
        <section key={key}>
          <h4>{labels[key]}</h4>
          <div className="fc-chat-revision-comparison">
            <div>
              <h5>Было</h5>
              <Value value={before[key]} />
            </div>
            <div>
              <h5>Стало</h5>
              <Value value={after[key]} />
            </div>
          </div>
        </section>
      ))}
    </section>
  )
}
