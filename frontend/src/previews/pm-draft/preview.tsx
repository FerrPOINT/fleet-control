import { useEffect, useState } from 'react'
import { useSearchParams } from 'react-router'
import { ArrowLeft, Bot, FilePlus2, MessageSquare, RefreshCw } from 'lucide-react'
import { AppShell, Button, Input, Label, Textarea } from '@sdlc/ui/ui'
import type { CreatePmDraft, PmDraftCreation } from '@/api/pm-drafts'
import {
  agents,
  initialRequest,
  operationAt,
  projects,
  steps,
  type Command,
  type Step,
} from './fixtures'

type Phase = 'form' | 'uncertain' | 'incomplete' | 'awaiting' | 'absent'
const phases: Phase[] = ['form', 'uncertain', 'incomplete', 'awaiting', 'absent']
const labels: Record<Step, string> = {
  draft: 'Сохранение задачи',
  input: 'Сохранение исходного запроса',
  reservation: 'Подготовка назначения PM',
  chat: 'Сохранение личного чата',
  admission: 'Допуск PM к запуску',
}

export function PmDraftPreview({ observe }: { observe?: (command: Command) => void }) {
  const [params, setParams] = useSearchParams()
  const requested = params.get('state') as Phase
  const initialPhase = phases.includes(requested) ? requested : 'form'
  const restored = initialPhase !== 'form'
  const requestedStep = params.get('step') as Step
  const initialStep =
    steps.includes(requestedStep) && requestedStep !== 'admission' ? requestedStep : 'reservation'
  const [phase, setPhase] = useState<Phase>(initialPhase)
  const [project, setProject] = useState(restored ? projects.projects[0]!.id : '')
  const [agent, setAgent] = useState(restored ? initialRequest.agent_id : '')
  const [title, setTitle] = useState(restored ? initialRequest.title : '')
  const [description, setDescription] = useState(restored ? initialRequest.description : '')
  const [original, setOriginal] = useState<{ projectId: string; request: CreatePmDraft } | null>(
    restored ? { projectId: projects.projects[0]!.id, request: initialRequest } : null,
  )
  const [operation, setOperation] = useState<PmDraftCreation | null>(() =>
    initialPhase === 'incomplete' || initialPhase === 'awaiting'
      ? operationAt(
          initialPhase === 'awaiting' ? 'admission' : initialStep,
          projects.projects[0]!.id,
          initialRequest.agent_id,
        )
      : null,
  )
  const [pendingReadback, setPendingReadback] = useState<PmDraftCreation | null>(null)
  const [checking, setChecking] = useState(false)
  const [notice, setNotice] = useState('')
  const [nextPageLoaded, setNextPageLoaded] = useState(false)
  const view =
    params.get('view') === 'list' ? 'list' : params.get('view') === 'chat' ? 'chat' : 'create'
  const denied = params.get('access') === 'denied'
  const readOnly = params.get('access') === 'readonly'
  const unavailable = params.get('access') === 'unavailable'
  const disabled = params.get('access') === 'disabled'
  const filteredPage = params.get('access') === 'empty-page' && !nextPageLoaded
  const empty = params.get('access') === 'empty'
  const directory = disabled
    ? { ...projects, enabled: false, projects: [], next_cursor: null }
    : denied
      ? { ...projects, projects: [], next_cursor: null }
      : filteredPage
        ? { ...projects, projects: [], next_cursor: '11111111-1111-4111-8111-000000000001' }
        : empty
          ? { ...projects, projects: [] }
          : projects
  const locked = original !== null
  const writable = !denied && !readOnly && !unavailable && directory.enabled
  const valid =
    Boolean(project && agent && title.trim()) &&
    !Array.from(title).some((char) => char.charCodeAt(0) < 32 || char.charCodeAt(0) === 127) &&
    !description.includes('\0')
  const dirty = Boolean(title || description)
  useEffect(() => {
    if (!dirty) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty])
  const changeView = (next: string) => {
    setParams((current) => {
      const nextParams = new URLSearchParams(current)
      nextParams.set('view', next)
      return nextParams
    })
  }
  const accept = (result: PmDraftCreation) => {
    setOperation(result)
    setPhase(result.state === 'awaiting_admission' ? 'awaiting' : 'incomplete')
    setNotice('')
  }
  const reconcile = async () => {
    if (!original || denied || readOnly || !directory.enabled || checking) return
    setChecking(true)
    setNotice('')
    if (operation) observe?.({ kind: 'readback', operationId: operation.operation_id })
    else
      observe?.({
        kind: 'lookup',
        projectId: original.projectId,
        key: original.request.idempotency_key,
      })
    await Promise.resolve()
    if (unavailable)
      setNotice(
        'Не удалось проверить состояние. Исход операции остаётся неизвестным; повторное создание заблокировано.',
      )
    else if (!operation && params.get('result') === 'absent') setPhase('absent')
    else
      accept(
        pendingReadback ??
          operation ??
          operationAt(initialStep, original.projectId, original.request.agent_id),
      )
    setChecking(false)
  }
  const continuePreparation = () => {
    if (!operation || !original || !writable || phase !== 'incomplete') return
    observe?.({ kind: 'continue', operationId: operation.operation_id })
    const next = operationAt(
      steps[steps.indexOf(operation.next_step) + 1] ?? 'admission',
      original.projectId,
      original.request.agent_id,
    )
    if (params.get('continue') === 'unknown') {
      setPendingReadback(next)
      setPhase('uncertain')
    } else accept(next)
  }
  const status =
    phase === 'uncertain'
      ? 'Ответ не получен'
      : phase === 'absent'
        ? 'Операция не найдена'
        : phase === 'awaiting'
          ? 'Задача и чат сохранены'
          : operation?.task_id
            ? 'Задача создана, подготовка не завершена'
            : 'Подготовка задачи не завершена'
  const accessNotice = denied
    ? 'Нет доступа к проекту. Создание и продолжение подготовки недоступны.'
    : readOnly
      ? 'Только просмотр. Создание и продолжение подготовки доступны владельцу.'
      : disabled
        ? 'Создание задач с PM пока недоступно.'
        : unavailable && phase === 'form'
          ? 'Не удалось загрузить проекты. Попробуйте проверить доступность позже.'
          : ''
  return (
    <AppShell
      currentServiceKey="fleet-control"
      title="Fleet Control"
      context={<span className="text-sm">Макет · вымышленные данные</span>}
      navigation={[{ to: '/?view=list', label: 'Чаты', icon: MessageSquare, active: true }]}
    >
      <header className="mb-6 flex min-w-0 flex-wrap items-center gap-3 border-b border-border pb-4">
        {view !== 'list' && (
          <Button
            variant="ghost"
            size="icon"
            aria-label="Вернуться в чаты"
            onClick={() => changeView('list')}
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
        )}
        <h1 className="text-xl font-semibold">
          {view === 'list' ? 'Чаты' : view === 'chat' ? title || 'PM-чат' : 'Новая задача с PM'}
        </h1>
        <span className="text-xs text-text-muted">Личная</span>
      </header>
      {accessNotice && (
        <p role="alert" className="mb-5 max-w-3xl text-sm">
          {accessNotice}
        </p>
      )}
      {view === 'list' ? (
        <section aria-label="Чаты Project Manager" className="max-w-3xl">
          <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <Bot className="h-5 w-5" />
              <strong>Project Manager</strong>
            </div>
            <Button
              disabled={!original && !dirty && !writable}
              onClick={() => changeView('create')}
            >
              <FilePlus2 className="h-4 w-4" />
              {original
                ? 'Вернуться к подготовке задачи'
                : dirty
                  ? 'Вернуться к черновику'
                  : 'Создать задачу с PM'}
            </Button>
          </div>
          {operation?.session_id ? (
            <Button
              variant="outline"
              className="w-full justify-start"
              onClick={() => changeView('chat')}
            >
              {title} · Ожидает допуска PM
            </Button>
          ) : (
            <p className="border-y border-border py-4 text-sm text-text-muted">
              Сохранённых PM-чатов пока нет.{original && ' Исходный запрос сохранён в форме.'}
            </p>
          )}
        </section>
      ) : view === 'chat' ? (
        <section aria-label="Сохранённый PM-чат" className="max-w-3xl space-y-5">
          {!operation?.session_id ? (
            <p role="alert">Сохранённый чат пока не подтверждён.</p>
          ) : (
            <>
              <p role="status" className="border-b border-border pb-4">
                Ожидает допуска PM к запуску. Агент ещё не запущен.
              </p>
              <h2 className="font-semibold">Исходный запрос</h2>
              <p className="whitespace-pre-wrap break-words">
                {description || 'Запрос без описания'}
              </p>
              <p className="text-sm text-text-muted">Сообщений агента пока нет.</p>
              <Label htmlFor="draft-chat-input">Сообщение агенту</Label>
              <Textarea
                id="draft-chat-input"
                disabled
                placeholder="Отправка станет доступна после подтверждённого допуска."
              />
              <Button disabled>Отправить сообщение</Button>
              <Button variant="outline" onClick={() => changeView('create')}>
                Вернуться к состоянию подготовки
              </Button>
            </>
          )}
        </section>
      ) : (
        <section className="max-w-3xl" aria-label="Создание задачи">
          <form
            className="grid min-w-0 gap-5"
            onSubmit={(event) => {
              event.preventDefault()
              if (!writable || !valid || (locked && phase !== 'absent')) return
              const command = original ?? {
                projectId: project,
                request: {
                  agent_id: agent,
                  title,
                  description,
                  idempotency_key: crypto.randomUUID(),
                },
              }
              setOriginal(command)
              observe?.({ kind: 'create', projectId: command.projectId, request: command.request })
              if (!original && ['unknown', 'absent'].includes(params.get('result') ?? ''))
                setPhase('uncertain')
              else
                accept(
                  operationAt(
                    original ? 'draft' : initialStep,
                    command.projectId,
                    command.request.agent_id,
                  ),
                )
            }}
          >
            <div className="grid min-w-0 gap-5 sm:grid-cols-2">
              <div className="min-w-0 space-y-2">
                <Label htmlFor="draft-project">Проект</Label>
                <select
                  id="draft-project"
                  value={project}
                  disabled={locked || !writable}
                  onChange={(event) => setProject(event.target.value)}
                  className="h-10 w-full min-w-0 rounded-md border border-border bg-surface px-3 text-sm"
                >
                  <option value="">Выберите проект</option>
                  {directory.projects.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.key} · {item.name}
                    </option>
                  ))}
                </select>
                {!directory.projects.length && directory.enabled && !denied && !unavailable && (
                  <p className="text-xs text-text-muted">
                    На этой странице нет доступных проектов.
                  </p>
                )}
                {directory.next_cursor && (
                  <Button
                    type="button"
                    variant="outline"
                    disabled={locked || !writable}
                    onClick={() => setNextPageLoaded(true)}
                  >
                    Ещё проекты
                  </Button>
                )}
              </div>
              <div className="min-w-0 space-y-2">
                <Label htmlFor="draft-agent">Project Manager</Label>
                <select
                  id="draft-agent"
                  value={agent}
                  disabled={locked || !writable}
                  onChange={(event) => setAgent(event.target.value)}
                  className="h-10 w-full min-w-0 rounded-md border border-border bg-surface px-3 text-sm"
                >
                  <option value="">Выберите агента</option>
                  {agents.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.name}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="draft-title">Название задачи</Label>
              <Input
                id="draft-title"
                value={title}
                maxLength={500}
                readOnly={locked || !writable}
                onChange={(event) => setTitle(event.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="draft-description">Исходный запрос</Label>
              <Textarea
                id="draft-description"
                value={description}
                maxLength={100000}
                rows={8}
                readOnly={locked || !writable}
                onChange={(event) => setDescription(event.target.value)}
                className="min-h-40 resize-y"
              />
            </div>
            {(phase === 'form' || phase === 'absent') && (
              <div className="border-t border-border pt-4">
                <Button type="submit" disabled={!valid || !writable}>
                  <FilePlus2 className="h-4 w-4" />
                  {phase === 'absent' ? 'Повторить исходное создание' : 'Создать задачу'}
                </Button>
              </div>
            )}
          </form>
          {phase !== 'form' && (
            <section className="mt-5 space-y-3 border-t border-border pt-5" aria-live="polite">
              <h2 className="text-base font-semibold">{status}</h2>
              <p className="text-sm text-text-secondary">
                {phase === 'uncertain'
                  ? 'Исход операции неизвестен. Сначала проверьте её состояние.'
                  : phase === 'absent'
                    ? 'Проверка не нашла операцию по исходному ключу. Можно повторить тот же запрос без изменений.'
                    : phase === 'awaiting'
                      ? 'Ожидает допуска PM к запуску. Агент ещё не запущен.'
                      : `${operation?.task_id ? 'UX-102 · ' : ''}${labels[operation!.next_step]}. Продолжение использует сохранённую операцию.`}
              </p>
              {notice && (
                <p role="alert" className="text-sm">
                  {notice}
                </p>
              )}
              {phase === 'uncertain' && (
                <Button
                  type="button"
                  variant="outline"
                  disabled={checking || denied || readOnly || !directory.enabled}
                  onClick={() => void reconcile()}
                >
                  <RefreshCw className="h-4 w-4" />
                  {checking ? 'Проверяем состояние' : 'Проверить состояние'}
                </Button>
              )}
              {phase === 'incomplete' && (
                <Button type="button" disabled={!writable} onClick={continuePreparation}>
                  <RefreshCw className="h-4 w-4" />
                  Продолжить подготовку
                </Button>
              )}
              {phase === 'awaiting' && (
                <Button
                  type="button"
                  variant="outline"
                  disabled={denied}
                  onClick={() => changeView('chat')}
                >
                  <MessageSquare className="h-4 w-4" />
                  Открыть чат
                </Button>
              )}
            </section>
          )}
        </section>
      )}
    </AppShell>
  )
}
