import { useEffect, useState } from 'react'
import { useLocation } from 'react-router'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { parseNamespaceLocation, useSessionCommand, withNamespaceLocation } from '@sdlc/ui/lib'
import { Button, usePlatformServices } from '@sdlc/ui/ui'
import { apiRequest } from '@/api/client'
import { useAuthStore } from '@/shared/auth/store'
import type { components } from '@/api/generated'

type Context = components['schemas']['ExecutionContextV2']
type Projection = components['schemas']['SessionExecutionContext']
export const namespaceContextEnabled = import.meta.env.VITE_NAMESPACE_ENABLED === 'true'
const immutableId = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
function identity(params: URLSearchParams, key: string) {
  const values = params.getAll(key)
  const value = values[0]
  return values.length === 1 &&
    value &&
    immutableId.test(value) &&
    value !== '00000000-0000-0000-0000-000000000000'
    ? value
    : null
}
export function ExecutionContextPanel({
  sessionId,
  onBlocked,
}: {
  sessionId: string
  onBlocked: (value: boolean) => void
}) {
  const { search } = useLocation()
  const namespace = parseNamespaceLocation(search)
  const params = new URLSearchParams(search)
  const task = identity(params, 'task_id')
  const tracker = identity(params, 'tracker_instance_id')
  const malformed =
    ((params.has('registry_instance_id') || params.has('namespace_id')) && !namespace) ||
    ((params.has('task_id') || params.has('tracker_instance_id')) &&
      (!task || !tracker || !namespace))
  const actor = useAuthStore((state) => state.userId)
  const { services } = usePlatformServices()
  const cache = useQueryClient()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const pendingRef = useSessionCommand<Context>(
    `fleet:execution-context:${actor}:${sessionId}:${namespace?.registry_instance_id}:${namespace?.namespace_id}`,
  )
  const query = useQuery({
    queryKey: [
      'execution-context-v2',
      actor,
      sessionId,
      namespace?.registry_instance_id,
      namespace?.namespace_id,
    ],
    enabled: namespaceContextEnabled,
    queryFn: ({ signal }) =>
      apiRequest<Projection | null>(`/api/v2/sessions/${sessionId}/execution-context`, { signal }),
  })
  useEffect(() => {
    onBlocked(
      namespaceContextEnabled &&
        (malformed || query.isPending || query.isError || Boolean(query.data)),
    )
  }, [malformed, query.isPending, query.isError, query.data, onBlocked])
  async function bind() {
    if (malformed || !namespace || !tracker || !task) return
    pendingRef.current ??= {
      schema_version: 2,
      operation_id: crypto.randomUUID(),
      namespace,
      task: { tracker_instance_id: tracker, task_id: task },
      repositories: [],
    }
    setBusy(true)
    setError(undefined)
    try {
      await apiRequest<Projection>(`/api/v2/sessions/${sessionId}/execution-context`, {
        method: 'PUT',
        body: JSON.stringify(pendingRef.current),
      })
      pendingRef.current = null
      await cache.invalidateQueries({ queryKey: ['execution-context-v2', actor, sessionId] })
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Не удалось сохранить контекст')
    } finally {
      setBusy(false)
    }
  }
  if (!namespaceContextEnabled) return null
  if (malformed)
    return (
      <p role="alert" className="mt-4 text-danger">
        Некорректная ссылка на проект или задачу. Запуск закрыт до исправления ссылки.
      </p>
    )
  if (query.isPending)
    return (
      <p role="status" className="mt-4">
        Проверяем контекст исполнения…
      </p>
    )
  if (query.isError)
    return (
      <div className="mt-4 space-y-2">
        <p role="alert" className="text-danger">
          Контекст исполнения недоступен. Запуск закрыт до проверки.
        </p>
        <Button variant="outline" onClick={() => void query.refetch()}>
          Повторить проверку
        </Button>
      </div>
    )
  if (query.data) {
    const context = query.data.context
    if (
      namespace &&
      (context.namespace.namespace_id !== namespace.namespace_id ||
        context.namespace.registry_instance_id !== namespace.registry_instance_id)
    )
      return (
        <p role="alert" className="mt-4 text-danger">
          Сессия привязана к другому проекту.
        </p>
      )
    const trackerUrl = services.find((service) => service.key === 'task-tracker')?.ui_url
    const forgeUrl = services.find((service) => service.key === 'ci-cd')?.ui_url
    return (
      <section className="mt-4 space-y-3 rounded-md border border-border bg-surface p-4">
        <h2 className="font-semibold">Контекст проекта</h2>
        <p className="text-sm text-text-muted">
          Контекст сохранён. Адаптер автономного исполнения ещё не включён; новые запуски закрыты.
        </p>
        {trackerUrl && (
          <a
            className="text-accent"
            href={withNamespaceLocation(
              `${trackerUrl}/issues/${context.task.task_id}`,
              context.namespace,
            )}
          >
            Задача в Tracker
          </a>
        )}
        {forgeUrl &&
          context.repositories.map((repository) => (
            <p key={`${repository.forge_instance_id}:${repository.repository_id}`}>
              <a
                className="text-accent"
                href={withNamespaceLocation(
                  `${forgeUrl}/catalog/repositories/${repository.repository_id}`,
                  context.namespace,
                )}
              >
                Репозиторий в Forge
              </a>
            </p>
          ))}
      </section>
    )
  }
  if (!namespace || !tracker || !task) return null
  return (
    <section className="mt-4 space-y-3 rounded-md border border-border bg-surface p-4">
      <h2 className="font-semibold">Связать с задачей проекта</h2>
      <p className="text-sm text-text-muted">
        Будет проверена задача из ссылки Tracker. Контекст сохраняется один раз; сессия должна
        завершить активные запуски.
      </p>
      {error && (
        <p role="alert" className="text-danger">
          {error}. Повтор использует исходную операцию.
        </p>
      )}
      <Button disabled={busy} onClick={() => void bind()}>
        {busy
          ? 'Проверяем…'
          : pendingRef.current
            ? 'Повторить исходную операцию'
            : 'Сохранить контекст'}
      </Button>
    </section>
  )
}
