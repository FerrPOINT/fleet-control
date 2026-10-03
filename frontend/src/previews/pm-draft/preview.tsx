import { useEffect, useState } from 'react'
import { useSearchParams } from 'react-router'
import {
  ArrowLeft,
  Bot,
  Check,
  FilePlus2,
  MessageSquare,
  RefreshCw,
  ShieldCheck,
} from 'lucide-react'
import { AppShell, Button, Input, Label, Textarea } from '@sdlc/ui/ui'

type Phase = 'form' | 'uncertain' | 'incomplete' | 'awaiting' | 'absent'
const phases: Phase[] = ['form', 'uncertain', 'incomplete', 'awaiting', 'absent']

export function PmDraftPreview() {
  const [params, setParams] = useSearchParams()
  const requested = params.get('state') as Phase
  const [phase, setPhase] = useState<Phase>(phases.includes(requested) ? requested : 'form')
  const restored = ['uncertain', 'incomplete', 'awaiting', 'absent'].includes(requested)
  const [project, setProject] = useState(restored ? 'portal' : '')
  const [agent, setAgent] = useState(restored ? 'agent3' : '')
  const [title, setTitle] = useState(restored ? 'Портал заявок сотрудников' : '')
  const [description, setDescription] = useState(
    restored
      ? 'Нужен внутренний портал: создание заявки, назначение исполнителя, история изменений. Доступ только участникам проекта. Уточнить критерии приёмки перед разработкой.'
      : '',
  )
  const [submitted, setSubmitted] = useState(restored)
  const [checking, setChecking] = useState(false)
  const [notice, setNotice] = useState('')
  const view = params.get('view') === 'list' ? 'list' : 'create'
  const denied = params.get('access') === 'denied'
  const unavailable = params.get('access') === 'unavailable'
  const locked = phase !== 'form'
  const dirty = Boolean(title || description)
  useEffect(() => {
    if (!dirty || submitted) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty, submitted])
  const changeView = (next: string) => {
    if (next === 'list' && dirty && !submitted && !window.confirm('Выйти без сохранения запроса?'))
      return
    setParams((current) => {
      const nextParams = new URLSearchParams(current)
      nextParams.set('view', next)
      return nextParams
    })
  }
  const reconcile = async () => {
    setChecking(true)
    setNotice('')
    await Promise.resolve()
    if (unavailable) setNotice('Tracker недоступен. Исходный запрос сохранён в этой форме.')
    else setPhase(params.get('result') === 'absent' ? 'absent' : 'incomplete')
    setChecking(false)
  }
  return (
    <AppShell
      currentServiceKey="fleet-control"
      title="Fleet Control"
      context={<span className="text-sm">Макет</span>}
      navigation={[{ to: '/?view=list', label: 'Чаты', icon: MessageSquare, active: true }]}
    >
      <header className="mb-6 flex min-w-0 flex-wrap items-center gap-3 border-b border-border pb-4">
        {view === 'create' && (
          <Button
            variant="ghost"
            size="icon"
            title="Вернуться в чаты"
            aria-label="Вернуться в чаты"
            onClick={() => changeView('list')}
          >
            <ArrowLeft className="h-4 w-4" />
          </Button>
        )}
        <h1 className="text-xl font-semibold">{view === 'list' ? 'Чаты' : 'Новая задача с PM'}</h1>
        <span className="flex items-center gap-1 text-xs text-text-muted">
          <ShieldCheck className="h-4 w-4" /> Личная
        </span>
      </header>
      {view === 'list' ? (
        <section aria-label="Чаты Project Manager" className="max-w-3xl">
          <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <Bot className="h-5 w-5" />
              <strong>Project Manager</strong>
              <span className="text-sm text-text-muted">agent3 · Hermes</span>
            </div>
            <Button onClick={() => changeView('create')}>
              <FilePlus2 className="h-4 w-4" /> Создать задачу с PM
            </Button>
          </div>
          <div className="border-y border-border py-4">
            <span className="mr-3 text-sm text-text-muted">UX-101</span>Портал заявок сотрудников
            <span className="mt-2 block text-xs text-text-muted">Draft · Требуется ответ</span>
          </div>
        </section>
      ) : (
        <section className="max-w-3xl" aria-label="Создание задачи">
          {denied ? (
            <p role="alert" className="mb-4 text-sm">
              Нет доступа к SDLC-проектам. Создание задачи недоступно.
            </p>
          ) : unavailable && phase === 'form' ? (
            <p role="alert" className="mb-4 text-sm">
              Не удалось загрузить проекты. Создание задачи недоступно.
            </p>
          ) : null}
          <form
            className="grid min-w-0 gap-5"
            onSubmit={(event) => {
              event.preventDefault()
              if (
                !project ||
                !agent ||
                !title.trim() ||
                denied ||
                unavailable ||
                (locked && phase !== 'absent')
              )
                return
              setSubmitted(true)
              setPhase('uncertain')
            }}
          >
            <div className="grid min-w-0 gap-5 sm:grid-cols-2">
              <div className="min-w-0 space-y-2">
                <Label htmlFor="draft-project">Проект</Label>
                <select
                  id="draft-project"
                  value={project}
                  disabled={locked || denied || unavailable}
                  onChange={(event) => setProject(event.target.value)}
                  className="h-10 w-full min-w-0 rounded-md border border-border bg-surface px-3 text-sm"
                >
                  <option value="">Выберите проект</option>
                  <option value="portal">UX · Портал заявок</option>
                  <option value="operations">OPS · Внутренние сервисы</option>
                </select>
              </div>
              <div className="min-w-0 space-y-2">
                <Label htmlFor="draft-agent">Project Manager</Label>
                <select
                  id="draft-agent"
                  value={agent}
                  disabled={locked || denied || unavailable}
                  onChange={(event) => setAgent(event.target.value)}
                  className="h-10 w-full min-w-0 rounded-md border border-border bg-surface px-3 text-sm"
                >
                  <option value="">Выберите агента</option>
                  <option value="agent3">agent3 · Project Manager · Hermes</option>
                  <option value="agent7">agent7 · PM внутренних сервисов · Hermes</option>
                </select>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="draft-title">Название задачи</Label>
              <Input
                id="draft-title"
                value={title}
                maxLength={500}
                readOnly={locked}
                disabled={denied}
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
                readOnly={locked}
                disabled={denied}
                onChange={(event) => setDescription(event.target.value)}
                className="min-h-40 resize-y"
              />
            </div>
            {(phase === 'form' || phase === 'absent') && (
              <div className="flex flex-wrap gap-3 border-t border-border pt-4">
                <Button
                  type="submit"
                  disabled={!project || !agent || !title.trim() || denied || unavailable}
                >
                  <FilePlus2 className="h-4 w-4" /> Создать задачу
                </Button>
              </div>
            )}
          </form>
          {phase !== 'form' && (
            <section className="mt-5 space-y-3 border-t border-border pt-5" aria-live="polite">
              <h2 className="flex items-center gap-2 text-base font-semibold">
                {phase === 'awaiting' && <Check className="h-4 w-4" />}
                {phase === 'uncertain'
                  ? 'Ответ не получен'
                  : phase === 'incomplete'
                    ? 'Задача создана, подготовка не завершена'
                    : phase === 'absent'
                      ? 'Операция не найдена'
                      : 'Задача и чат сохранены'}
              </h2>
              <p className="text-sm text-text-secondary">
                {phase === 'uncertain'
                  ? 'Исход создания пока неизвестен. Сначала проверьте сохранённую операцию.'
                  : phase === 'incomplete'
                    ? 'UX-102 · Продолжение использует исходный запрос. Новая задача не создаётся.'
                    : phase === 'absent'
                      ? 'Проверка не нашла сохранённую операцию. Можно повторить тот же запрос.'
                      : 'UX-102 · Ожидает допуска PM к запуску. Агент ещё не запущен.'}
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
                  disabled={checking}
                  onClick={() => void reconcile()}
                >
                  <RefreshCw className="h-4 w-4" /> Проверить состояние
                </Button>
              )}
              {phase === 'incomplete' && (
                <Button type="button" onClick={() => setPhase('awaiting')}>
                  <RefreshCw className="h-4 w-4" /> Продолжить подготовку
                </Button>
              )}
              {phase === 'awaiting' && (
                <Button
                  type="button"
                  variant="outline"
                  onClick={() =>
                    setNotice('UX-102 · PM-чат сохранён. Допуск агента к запуску не подтверждён.')
                  }
                >
                  <MessageSquare className="h-4 w-4" /> Открыть чат
                </Button>
              )}
            </section>
          )}
        </section>
      )}
    </AppShell>
  )
}
