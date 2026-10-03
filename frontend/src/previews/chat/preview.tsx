import { useState } from 'react'
import { useSearchParams } from 'react-router'
import {
  ArrowLeft,
  ArrowRight,
  Bot,
  Check,
  CheckCircle2,
  ChevronRight,
  ClipboardList,
  Clock3,
  FileText,
  Info,
  MessageSquare,
  Send,
  ShieldCheck,
  UserRound,
  X,
} from 'lucide-react'
import {
  AppShell,
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Label,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
  Textarea,
} from '@sdlc/ui/ui'

type Answer = { choice: string; comment: string }
type Message = { author: 'human' | 'pm'; text: string }
const options = [
  {
    id: 'project',
    title: 'Участники проекта',
    detail: 'Задачи видят только участники своего проекта. Администратор имеет общий доступ.',
    recommendation: true,
  },
  {
    id: 'owner',
    title: 'Только автор задачи',
    detail:
      'Каждый сотрудник видит только свои задачи. Для совместной работы потребуется явный доступ.',
    recommendation: false,
  },
  {
    id: 'company',
    title: 'Все сотрудники',
    detail:
      'Задачи доступны всей компании. Этот вариант не подходит для конфиденциальных проектов.',
    recommendation: false,
  },
  {
    id: 'custom',
    title: 'Свой вариант',
    detail: 'Опишите, кому и при каких условиях нужен доступ.',
    recommendation: false,
  },
] as const
const initialMessages: Message[] = [
  {
    author: 'human',
    text: 'Нужен внутренний портал задач: сотрудники создают заявки, отслеживают статус и получают уведомления. Начнём с одного проекта.',
  },
  {
    author: 'pm',
    text: 'Зафиксировал цель, роли и критерии приёмки. Для MVP оставляем создание заявки, назначение исполнителя и историю изменений. Уведомления идут в приложении; интеграцию с почтой отложим.',
  },
  {
    author: 'human',
    text: 'Да. Задачу закрывает её автор после проверки результата. Нужен журнал действий.',
  },
  {
    author: 'pm',
    text: 'Добавил проверку результата автором и аудит. Осталось согласовать границы доступа к задачам. После ответа покажу всю редакцию требований для подтверждения.',
  },
]

function Badge({ children, tone = '' }: { children: React.ReactNode; tone?: string }) {
  return <span className={`preview-badge ${tone}`}>{children}</span>
}

function Context({ complete, published }: { complete: boolean; published: boolean }) {
  return (
    <div className="context-content">
      <div className="context-title">
        <FileText size={16} />
        <h2>Требования</h2>
        <Badge>Ред. 3</Badge>
      </div>
      <p className="context-goal">Внутренний портал заявок для одного проекта</p>
      <dl className="context-fields">
        <div>
          <dt>Основная задача</dt>
          <dd>UX-101</dd>
        </div>
        <div>
          <dt>Владелец</dt>
          <dd>
            <UserRound size={14} /> Алексей · вы
          </dd>
        </div>
        <div>
          <dt>Агент</dt>
          <dd>Project Manager · agent3</dd>
        </div>
        <div>
          <dt>Workflow</dt>
          <dd>PM draft · v2</dd>
        </div>
        <div>
          <dt>Запуск</dt>
          <dd>{published ? 'Завершён' : 'Ожидает ответа человека'}</dd>
        </div>
      </dl>
      <h3>
        Уточнения <span>{complete ? '5' : '4'} / 5</span>
      </h3>
      <ul className="checklist">
        {[
          'Цель и границы MVP',
          'Роли сотрудников',
          'Сценарии и приёмка',
          'Уведомления и аудит',
        ].map((label) => (
          <li key={label}>
            <CheckCircle2 size={15} />
            {label}
          </li>
        ))}
        <li className={complete ? '' : 'unresolved'}>
          {complete ? <CheckCircle2 size={15} /> : <Clock3 size={15} />}Доступ к задачам
        </li>
      </ul>
      <h3>Критерии приёмки</h3>
      <ol className="criteria">
        <li>Сотрудник создаёт заявку и видит статус.</li>
        <li>Исполнитель получает назначенную задачу.</li>
        <li>Автор подтверждает результат.</li>
        <li>Изменения сохраняются в журнале.</li>
      </ol>
      <div className="next-step">
        <ClipboardList size={17} />
        <div>
          <strong>Следующий этап: Analysis</strong>
          <p>{published ? 'Требования опубликованы · ред. 3' : 'После подтверждения редакции 3'}</p>
        </div>
      </div>
      <p className="context-private">
        <ShieldCheck size={14} />
        Личный чат · без лида
      </p>
    </div>
  )
}

export function Preview() {
  const [params, setParams] = useSearchParams()
  const view = params.get('view') ?? 'chat'
  const readOnly = params.get('state') === 'read-only'
  const [choice, setChoice] = useState('')
  const [comment, setComment] = useState('')
  const [answer, setAnswer] = useState<Answer | null>(null)
  const [messages, setMessages] = useState(initialMessages)
  const [draft, setDraft] = useState('')
  const [contextOpen, setContextOpen] = useState(false)
  const [reviewOpen, setReviewOpen] = useState(false)
  const [published, setPublished] = useState(false)
  const [confirmed, setConfirmed] = useState(false)
  const [conflict, setConflict] = useState(false)
  const status = published ? 'Опубликовано' : answer ? 'На подтверждении' : 'Требуется ваш ответ'
  function switchView(next: string) {
    setParams((current) => {
      const nextParams = new URLSearchParams(current)
      nextParams.set('view', next)
      return nextParams
    })
  }
  function submitAnswer() {
    if (!choice || (choice === 'custom' && !comment.trim()) || readOnly || answer) return
    if (params.get('state') === 'conflict') {
      setConflict(true)
      return
    }
    setAnswer({ choice, comment: comment.trim() })
    const label = options.find((option) => option.id === choice)?.title ?? choice
    setMessages((current) => [
      ...current,
      {
        author: 'human',
        text: `Доступ к задачам: ${label}.${comment.trim() ? ` ${comment.trim()}` : ''}`,
      },
      {
        author: 'pm',
        text: 'Ответ сохранён в редакции 3. Все пять уточнений закрыты. Проверьте требования перед публикацией; технические этапы ещё не запущены.',
      },
    ])
  }
  return (
    <AppShell
      currentServiceKey="fleet-control"
      title="Fleet Control"
      context={
        <span className="preview-context">
          Fleet Control <Badge>Макет</Badge>
        </span>
      }
      navigation={[
        { to: '/?view=chat', label: 'Чат', icon: MessageSquare, active: view === 'chat' },
        {
          to: '/?view=clarify',
          label: 'Уточнения',
          icon: ClipboardList,
          active: view === 'clarify',
        },
      ]}
    >
      <div className="chat-workbench">
        <header className="task-header">
          <div className="task-heading">
            <span className="task-key">UX-101</span>
            <h1>Портал заявок сотрудников</h1>
            <Badge tone="private">
              <ShieldCheck size={12} />
              Личная
            </Badge>
          </div>
          <div className="task-status">
            <Badge tone={published ? 'success' : 'waiting'}>{status}</Badge>
            <span>
              {published ? 'Backlog' : 'Draft'}
              <ChevronRight size={12} />
              {published ? 'Ожидает назначения Analyst' : 'Уточнение требований'}
            </span>
            <Button
              variant="ghost"
              size="icon"
              title="Контекст задачи"
              aria-label="Контекст задачи"
              className="context-trigger"
              onClick={() => setContextOpen(true)}
            >
              <Info size={18} />
            </Button>
          </div>
        </header>
        <div className="agent-toolbar">
          <div>
            <Bot size={18} />
            <strong>Project Manager</strong>
            <span>agent3 · Hermes</span>
          </div>
          <div className="toolbar-end">
            <Clock3 size={14} />
            <span>{published ? 'Запуск завершён' : 'Ожидает уточнения'}</span>
          </div>
        </div>
        <div className="conversation-grid">
          <section className="conversation" aria-label="Рабочая область чата">
            <Tabs value={view} onValueChange={switchView} className="preview-tabs-root">
              <TabsList className="view-tabs" aria-label="Разделы задачи">
                <TabsTrigger value="chat">
                  <MessageSquare size={15} />
                  Диалог
                </TabsTrigger>
                <TabsTrigger value="clarify">
                  <ClipboardList size={15} />
                  Уточнения <span>{answer ? '5/5' : '4/5'}</span>
                </TabsTrigger>
                <span className="revision-label">Требования · ред. 3</span>
              </TabsList>
              {view === 'chat' ? (
                <TabsContent value="chat" className="chat-panel">
                  <div className="transcript" role="log" aria-label="История сообщений">
                    <div className="date-separator">1 октября 2026</div>
                    {messages.map((message, index) => (
                      <article className={`message ${message.author}`} key={index}>
                        <div className="message-avatar">
                          {message.author === 'pm' ? <Bot size={18} /> : <UserRound size={18} />}
                        </div>
                        <div className="message-content">
                          <div className="message-label">
                            <strong>{message.author === 'pm' ? 'Project Manager' : 'Вы'}</strong>
                            <time>{`14:${String(20 + index).padStart(2, '0')}`}</time>
                            {message.author === 'pm' && <span>PM draft</span>}
                          </div>
                          <p>{message.text}</p>
                        </div>
                      </article>
                    ))}
                    {!answer ? (
                      <div className="question-callout">
                        <Clock3 size={18} />
                        <div>
                          <strong>Кто должен видеть задачи проекта?</strong>
                          <p>Последнее уточнение · права доступа · ред. 3</p>
                          <Button variant="outline" onClick={() => switchView('clarify')}>
                            Ответить на уточнение
                            <ArrowRight size={15} />
                          </Button>
                        </div>
                      </div>
                    ) : (
                      <div className="question-callout answered">
                        <CheckCircle2 size={18} />
                        <div>
                          <strong>
                            {published ? 'Редакция 3 опубликована' : 'Уточнения завершены'}
                          </strong>
                          <p>
                            {published
                              ? 'Следующий этап ожидает назначения, а не выполнен автоматически.'
                              : 'Перед Analysis требуется ваше подтверждение требований.'}
                          </p>
                          {!published && (
                            <Button variant="outline" onClick={() => setReviewOpen(true)}>
                              Проверить требования
                              <ArrowRight size={15} />
                            </Button>
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                  <form
                    className="composer"
                    onSubmit={(event) => {
                      event.preventDefault()
                      if (draft.trim() && !readOnly && !published) {
                        setMessages((current) => [
                          ...current,
                          { author: 'human', text: draft.trim() },
                        ])
                        setDraft('')
                      }
                    }}
                  >
                    <Label htmlFor="preview-message" className="sr-only">
                      Сообщение Project Manager
                    </Label>
                    <Textarea
                      id="preview-message"
                      placeholder="Сообщение Project Manager…"
                      value={draft}
                      onChange={(event) => setDraft(event.target.value)}
                      disabled={readOnly || published}
                      rows={2}
                    />
                    <div className="composer-footer">
                      <span>
                        <ShieldCheck size={13} />
                        {readOnly
                          ? 'Только чтение'
                          : published
                            ? 'Редакция опубликована'
                            : 'Личное обсуждение задачи'}
                      </span>
                      <Button
                        type="submit"
                        size="icon"
                        aria-label="Отправить сообщение"
                        title="Отправить сообщение"
                        disabled={!draft.trim() || readOnly || published}
                      >
                        <Send size={17} />
                      </Button>
                    </div>
                  </form>
                </TabsContent>
              ) : (
                <TabsContent value="clarify" className="clarify-panel">
                  <div className="clarify-scroll">
                    <div className="clarify-heading">
                      <span className="question-number">Уточнение 5 из 5</span>
                      <Badge tone="waiting">{answer ? 'Ответ сохранён' : 'Обязательное'}</Badge>
                    </div>
                    <h2>Кто должен видеть задачи проекта?</h2>
                    <p className="question-reason">
                      От выбора зависит модель доступа. Это бизнес-решение нужно зафиксировать до
                      начала Analysis.
                    </p>
                    <div className="question-source">
                      <FileText size={14} />
                      Требование REQ-04 · Доступ к задачам · ред. 3
                    </div>
                    <fieldset disabled={readOnly || Boolean(answer)} className="answer-options">
                      <legend className="sr-only">Выберите вариант доступа</legend>
                      {options.map((option) => (
                        <label
                          key={option.id}
                          className={`answer-option ${choice === option.id ? 'selected' : ''}`}
                        >
                          <input
                            type="radio"
                            name="access-choice"
                            value={option.id}
                            checked={choice === option.id}
                            onChange={() => setChoice(option.id)}
                          />
                          <span>
                            <span className="option-title">
                              {option.title}
                              {option.recommendation && (
                                <Badge tone="success">Рекомендация PM</Badge>
                              )}
                            </span>
                            <span className="option-detail">{option.detail}</span>
                          </span>
                        </label>
                      ))}
                    </fieldset>
                    <Label htmlFor="answer-comment">
                      {choice === 'custom' ? 'Ваше решение' : 'Комментарий к выбору'}{' '}
                      <span>{choice === 'custom' ? '· обязательно' : '· необязательно'}</span>
                    </Label>
                    <Textarea
                      id="answer-comment"
                      value={comment}
                      onChange={(event) => setComment(event.target.value)}
                      placeholder={
                        choice === 'custom'
                          ? 'Опишите правила доступа…'
                          : 'Добавьте ограничения или детали…'
                      }
                      disabled={readOnly || Boolean(answer)}
                      rows={2}
                    />
                    {conflict && (
                      <div className="conflict" role="alert">
                        <strong>Вопрос изменился в редакции 4</strong>
                        <p>Ваш ответ не отправлен. Выбор и комментарий сохранены в этом макете.</p>
                        <Button variant="outline" onClick={() => setConflict(false)}>
                          Понятно
                        </Button>
                      </div>
                    )}
                  </div>
                  <div className="clarify-actions">
                    <Button variant="ghost" onClick={() => switchView('chat')}>
                      <ArrowLeft size={16} />
                      Вернуться в диалог
                    </Button>
                    {answer ? (
                      <Button onClick={() => setReviewOpen(true)} disabled={readOnly || published}>
                        Проверить требования
                        <ArrowRight size={16} />
                      </Button>
                    ) : (
                      <Button
                        onClick={submitAnswer}
                        disabled={!choice || (choice === 'custom' && !comment.trim()) || readOnly}
                      >
                        <Check size={16} />
                        Подтвердить ответ
                      </Button>
                    )}
                  </div>
                  <p className="clarify-note">
                    Ответ закрывает этот вопрос, но не публикует требования и не запускает следующие
                    стадии.
                  </p>
                </TabsContent>
              )}
            </Tabs>
          </section>
          <aside className="task-context" aria-label="Контекст задачи">
            <Context complete={Boolean(answer)} published={published} />
          </aside>
        </div>
      </div>
      <Dialog open={contextOpen} onOpenChange={setContextOpen}>
        <DialogContent aria-describedby={undefined} className="context-dialog">
          <DialogHeader>
            <DialogTitle>Контекст задачи UX-101</DialogTitle>
          </DialogHeader>
          <Context complete={Boolean(answer)} published={published} />
        </DialogContent>
      </Dialog>
      <Dialog open={reviewOpen} onOpenChange={setReviewOpen}>
        <DialogContent aria-describedby={undefined} className="requirements-dialog">
          <DialogHeader>
            <DialogTitle>Подтвердить требования · редакция 3</DialogTitle>
          </DialogHeader>
          <p>UX-101 · Портал заявок сотрудников</p>
          <ul>
            <li>Один проект, роли автора, исполнителя и администратора.</li>
            <li>Создание заявки, назначение и история изменений.</li>
            <li>Результат подтверждает автор заявки.</li>
            <li>Уведомления в приложении; почта вне MVP.</li>
            <li>
              Доступ: {options.find((option) => option.id === answer?.choice)?.title}.{' '}
              {answer?.comment}
            </li>
          </ul>
          <label className="confirmation">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(event) => setConfirmed(event.target.checked)}
            />
            Подтверждаю цель, границы и критерии приёмки редакции 3
          </label>
          <div className="dialog-actions">
            <Button variant="outline" onClick={() => setReviewOpen(false)}>
              <X size={15} />
              Вернуться к уточнениям
            </Button>
            <Button
              disabled={!confirmed || readOnly || !answer}
              onClick={() => {
                setPublished(true)
                setReviewOpen(false)
                switchView('chat')
              }}
            >
              <Check size={15} />
              Подтвердить редакцию 3
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </AppShell>
  )
}
