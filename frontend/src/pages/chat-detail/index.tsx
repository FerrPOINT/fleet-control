import { useCallback, useEffect, useRef, useState } from 'react'
import { Link, useBlocker, useParams, useSearchParams } from 'react-router'
import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
  type UseMutationResult,
} from '@tanstack/react-query'
import {
  ArrowDown,
  ArrowLeft,
  Bot,
  Check,
  ClipboardList,
  FileText,
  Info,
  MessageSquare,
  Send,
  ShieldCheck,
  Square,
} from 'lucide-react'
import {
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
import { connectAuthenticatedEventStream } from '@sdlc/ui/lib'
import { ApiError } from '@sdlc/ui/lib'
import { apiBaseUrl } from '@/api/client'
import {
  lookupRuntimeControlByDigest,
  runtimeControlPayloadSha256,
  trimRuntimeControlInput,
  type OriginalRuntimeControl,
} from '@/api/runtime-control-lookup'
import type { RuntimeControlReceipt, RuntimeRunControlResponse } from '@/api/types'
import {
  createSessionMessage,
  getSession,
  listAgentDirectory,
  listSessionAgentRuns,
  steerSessionRun,
  stopSessionRun,
} from '@/api/fleet'
import {
  answerClarification,
  blockedLabels,
  canSubmitAnswer,
  confirmRequirements,
  getChatControls,
  getChatHistory,
  getClarifications,
  getRequirements,
  getTaskContext,
  type RequirementsRevision,
  type AnswerInput,
} from '@/api/task-chats'
import { ssoConfig, useAuthStore } from '@/shared/auth/store'
import {
  ControlPreparationError,
  clearControlRecovery,
  controlRecoveryService,
  readControlRecovery,
  writeControlRecovery,
  type ControlRecoveryRecord,
} from '@/shared/chat-control-recovery'
import { TaskApprovalsPanel } from './approvals'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, StatusBadge, formatDate } from '../common'
import {
  RuntimeControlHistory,
  isUnresolvedControl,
  useRuntimeControls,
} from '../runtime-control-history'
import './chat.css'

function requestKey() {
  return crypto.randomUUID()
}
type ConfirmationCommand = { revision: number; hash: string; key: string }
type PmCommandScope = { actorId: string; service: string; key: string }
type OriginalControlScope = { runId: string; agentId: string; actorId: string; key: string }
type AnswerDraft = {
  selected: string[]
  text: string
  comment: string
  key: string
  optionLabels: Record<string, string>
}
type ConfirmationResult = Awaited<ReturnType<typeof confirmRequirements>>
type ConfirmationMutation = UseMutationResult<ConfirmationResult, Error, ConfirmationCommand>
function unknownOutcome(error: unknown) {
  return (
    Boolean(error) &&
    !(error instanceof ControlPreparationError) &&
    (!(error instanceof ApiError) ||
      (error.status >= 200 && error.status < 300) ||
      error.status === 408 ||
      error.status >= 500)
  )
}
function acceptedControl(receipt: RuntimeControlReceipt | undefined) {
  return Boolean(
    receipt &&
    (receipt.state === 'acknowledged' ||
      (receipt.state === 'terminal_observed' &&
        typeof receipt.acknowledgement === 'string' &&
        Boolean(receipt.acknowledgement.trim()))),
  )
}
function matchingControl(
  receipt: RuntimeControlReceipt | undefined,
  sessionId: string,
  command: OriginalControlScope,
  operation: 'steer' | 'stop',
) {
  return Boolean(
    receipt &&
    typeof receipt.id === 'string' &&
    receipt.id &&
    receipt.session_id === sessionId &&
    receipt.session_run_id === command.runId &&
    receipt.agent_id === command.agentId &&
    receipt.actor_user_id === command.actorId &&
    receipt.operation === operation,
  )
}
function acceptedControlResponse(
  result: RuntimeRunControlResponse | undefined,
  sessionId: string,
  command: OriginalControlScope,
  operation: 'steer' | 'stop',
) {
  return Boolean(
    result &&
    result.accepted === true &&
    result.session_id === sessionId &&
    result.run_id === command.runId &&
    (!result.command ||
      (matchingControl(result.command, sessionId, command, operation) &&
        acceptedControl(result.command))),
  )
}
async function refreshHistory(client: QueryClient, id: string) {
  const queryKey = ['chat-history', id]
  const alreadyFetching = client.isFetching({ queryKey, exact: true }) > 0
  await client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false })
  // A pending page began before this event: read again after it without cancelling it.
  if (alreadyFetching) {
    await client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false })
  }
}
function ReadableError({ error }: { error: unknown }) {
  return <ErrorState message={error instanceof Error ? error.message : 'Данные недоступны'} />
}

export function ChatDetailPage() {
  const { sessionId = '' } = useParams()
  return <ChatWorkspace key={sessionId} id={sessionId} />
}

function ChatWorkspace({ id }: { id: string }) {
  const [params, setParams] = useSearchParams()
  const client = useQueryClient()
  const token = useAuthStore((state) => state.token)
  const signingOut = useAuthStore((state) => state.signingOut)
  const userId = useAuthStore((state) => state.userId)
  const [stopUncertain, setStopUncertain] = useState(false)
  const controlService = controlRecoveryService(apiBaseUrl, ssoConfig.issuer)
  const [controlRecovery, setControlRecovery] = useState(() => readControlRecovery(id))
  const recoveryRef = useRef(controlRecovery)
  recoveryRef.current = controlRecovery
  const session = useQuery({ queryKey: ['session', id], queryFn: () => getSession(id) })
  const sessionDenied =
    session.isError &&
    session.error instanceof ApiError &&
    [401, 403, 404].includes(session.error.status)
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const task = useQuery({
    queryKey: ['task-context', id],
    queryFn: () => getTaskContext(id),
    refetchInterval: 10000,
  })
  const controls = useQuery({
    queryKey: ['chat-controls', id],
    queryFn: () => getChatControls(id),
    refetchInterval: 10000,
  })
  const controlsFresh = controls.isSuccess && !controls.isFetching && controls.failureCount === 0
  const runs = useQuery({
    queryKey: ['session-runs', id],
    queryFn: () => listSessionAgentRuns(id),
    refetchInterval: 10000,
  })
  const controlRunId =
    controls.data?.active_run_id ??
    [...(runs.data ?? [])].sort((a, b) => b.created_at.localeCompare(a.created_at))[0]?.id
  const runtimeCommands = useRuntimeControls(id, controlRunId)
  const history = useInfiniteQuery({
    queryKey: ['chat-history', id],
    queryFn: ({ pageParam }) => getChatHistory(id, pageParam),
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (last) => last.next_before ?? undefined,
  })
  const bound = Boolean(task.data?.binding)
  const questions = useQuery({
    queryKey: ['clarifications', id],
    queryFn: () => getClarifications(id),
    enabled: bound,
    refetchInterval: 10000,
  })
  const requirements = useQuery({
    queryKey: ['requirements', id],
    queryFn: () => getRequirements(id),
    enabled: bound,
    refetchInterval: 10000,
  })
  const [contextOpen, setContextOpen] = useState(false)
  const contextTrigger = useRef<HTMLButtonElement>(null)
  const [drafts, setDrafts] = useState<Record<string, AnswerDraft>>({})
  const [body, setBody] = useState('')
  const [messageKey, setMessageKey] = useState(requestKey)
  const [delta, setDelta] = useState<Record<string, string>>({})
  const [receipt, setReceipt] = useState<string | null>(null)
  const [confirmationUncertain, setConfirmationUncertain] = useState(false)
  const [answerUncertain, setAnswerUncertain] = useState(false)
  const [messageUncertain, setMessageUncertain] = useState(false)
  const confirmationScope = useRef<PmCommandScope | undefined>(undefined)
  const answerScope = useRef<PmCommandScope | undefined>(undefined)
  const confirmation = useMutation({
    mutationFn: (command: ConfirmationCommand) => {
      preparePmCommand(confirmationScope, command.key)
      return confirmRequirements(id, command.revision, command.hash, command.key)
    },
    onSuccess: async (_result, command) => {
      if (!matchesPmActor(confirmationScope.current, command.key)) {
        setConfirmationUncertain(true)
        await invalidate()
        return
      }
      setConfirmationUncertain(false)
      setReceipt(
        `Подтверждение редакции ${command.revision} сохранено. Следующее назначение проверяется отдельно.`,
      )
      await invalidate()
    },
    onError: (error) => {
      if (unknownOutcome(error)) setConfirmationUncertain(true)
      void task.refetch()
      void requirements.refetch()
    },
  })
  const transcript = useRef<HTMLDivElement>(null)
  const following = useRef(true)
  const readingPosition = useRef(0)
  const attachTranscript = useCallback((node: HTMLDivElement | null) => {
    transcript.current = node
    if (node) node.scrollTop = following.current ? node.scrollHeight : readingPosition.current
  }, [])
  const [newMessages, setNewMessages] = useState(false)
  const tab = ['dialogue', 'clarification', 'requirements'].includes(params.get('tab') ?? '')
    ? params.get('tab')!
    : 'dialogue'
  const sessionFresh = session.isSuccess && !session.isFetching && session.failureCount === 0
  const owner = !signingOut && Boolean(token) && sessionFresh && session.data?.user_id === userId
  const authority = useRef({ owner, userId, token, service: controlService })
  authority.current = { owner, userId, token, service: controlService }
  function matchesPmActor(scope: PmCommandScope | undefined, key: string | undefined) {
    const current = useAuthStore.getState()
    return Boolean(
      scope &&
      scope.key === key &&
      current.token &&
      !current.signingOut &&
      current.userId === scope.actorId &&
      authority.current.owner &&
      authority.current.service === scope.service,
    )
  }
  function preparePmCommand(scope: { current: PmCommandScope | undefined }, key: string) {
    if (!scope.current || scope.current.key !== key) {
      const current = useAuthStore.getState()
      if (!authority.current.owner || !current.userId || !current.token || current.signingOut)
        throw new ControlPreparationError('Доступ изменился. Команда не отправлена.')
      scope.current = { actorId: current.userId, service: authority.current.service, key }
    }
    if (!matchesPmActor(scope.current, key))
      throw new ControlPreparationError('Для исходной команды нужен тот же пользователь и сервис.')
  }
  const finishControl = useCallback(
    (command: OriginalControlScope, operation: 'steer' | 'stop') => {
      const stored = recoveryRef.current
      if (
        stored.state !== 'pending' ||
        !authority.current.owner ||
        useAuthStore.getState().signingOut ||
        useAuthStore.getState().userId !== command.actorId ||
        stored.command.service !== controlService ||
        stored.command.operation !== operation ||
        stored.command.key !== command.key ||
        stored.command.runId !== command.runId ||
        stored.command.actorId !== command.actorId ||
        stored.command.agentId !== command.agentId
      )
        return false
      if (!clearControlRecovery(stored.command)) {
        recoveryRef.current = { state: 'blocked' }
        setControlRecovery({ state: 'blocked' })
        return false
      }
      recoveryRef.current = { state: 'none' }
      setControlRecovery({ state: 'none' })
      return true
    },
    [controlService],
  )
  const canManageApprovals = useAuthStore((state) => state.permissions.includes('agents:manage'))
  const canResolveApprovals =
    !signingOut && Boolean(token) && sessionFresh && (canManageApprovals || owner)
  const context = task.data?.tracker
  const taskFresh = task.isSuccess && !task.isFetching && task.failureCount === 0
  const questionsFresh =
    questions.isSuccess && !questions.isFetching && questions.failureCount === 0
  const requirementsFresh =
    requirements.isSuccess && !requirements.isFetching && requirements.failureCount === 0
  const questionList = questions.data?.questions ?? []
  const selectedQuestion =
    questionList.find((question) => question.id === params.get('question')) ??
    questionList.find((question) => question.state === 'open') ??
    questionList[0]
  const questionKey = selectedQuestion ? `${selectedQuestion.id}:${selectedQuestion.version}` : ''
  const draft = drafts[questionKey] ?? {
    selected: [],
    text: '',
    comment: '',
    key: '',
    optionLabels: {},
  }
  const dirty =
    confirmation.isPending ||
    confirmationUncertain ||
    messageUncertain ||
    stopUncertain ||
    controlRecovery.state !== 'none' ||
    Boolean(body.trim()) ||
    Object.values(drafts).some((value) =>
      Boolean(value.selected.length || value.text.trim() || value.comment.trim()),
    )
  const staleDrafts = Object.entries(drafts).filter(
    ([key]) => selectedQuestion && key.startsWith(`${selectedQuestion.id}:`) && key !== questionKey,
  )
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      dirty && currentLocation.pathname !== nextLocation.pathname,
  )
  useEffect(() => {
    const warn = (event: BeforeUnloadEvent) => {
      if (dirty) {
        event.preventDefault()
        event.returnValue = ''
      }
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty])
  const invalidate = async () => {
    await Promise.all(
      [
        'session',
        'chat-history',
        'session-runs',
        'chat-controls',
        'task-context',
        'clarifications',
        'requirements',
        'task-approvals',
        'task-approval-decision',
        'runtime-controls',
        'runtime-control-lookup',
      ].map((key) =>
        key === 'chat-history'
          ? refreshHistory(client, id)
          : client.invalidateQueries({ queryKey: [key, id] }),
      ),
    )
  }
  useEffect(() => {
    if (!token || signingOut || sessionDenied) return
    return connectAuthenticatedEventStream({
      url: `${apiBaseUrl}/api/v1/sessions/${id}/stream`,
      token,
      eventTypes: ['session'],
      onOpen: () => {
        setDelta({})
        ;[
          'session',
          'chat-history',
          'session-runs',
          'chat-controls',
          'task-context',
          'clarifications',
          'requirements',
          'task-approvals',
          'runtime-controls',
          'runtime-control-lookup',
        ].forEach((key) => {
          if (key === 'chat-history') void refreshHistory(client, id)
          else void client.invalidateQueries({ queryKey: [key, id] })
        })
      },
      onEvent: (_type, data) => {
        if (
          data &&
          typeof data === 'object' &&
          'type' in data &&
          data.type === 'session_run_delta' &&
          'run_id' in data &&
          typeof data.run_id === 'string'
        ) {
          const runId = data.run_id
          if ('text' in data && typeof data.text === 'string') {
            const text = data.text
            setDelta((current) => ({ ...current, [runId]: text }))
          } else if ('delta' in data && typeof data.delta === 'string') {
            const text = data.delta
            setDelta((current) => ({ ...current, [runId]: (current[runId] ?? '') + text }))
          }
        } else {
          ;[
            'session',
            'chat-history',
            'session-runs',
            'chat-controls',
            'task-context',
            'clarifications',
            'requirements',
            'task-approvals',
            'task-approval-decision',
            'runtime-controls',
            'runtime-control-lookup',
          ].forEach((key) => {
            if (key === 'chat-history') void refreshHistory(client, id)
            else void client.invalidateQueries({ queryKey: [key, id] })
          })
        }
      },
    })
  }, [client, id, token, signingOut, sessionDenied])
  const messageMap = new Map(
    [...(history.data?.pages ?? [])]
      .reverse()
      .flatMap((page) => page.items)
      .map((message) => [message.id, message]),
  )
  const messages = [...messageMap.values()]
  const lastMessage = messages.at(-1)?.id
  const activeRun = runs.data?.find((run) => run.id === controls.data?.active_run_id)
  const displayedDelta = activeRun ? delta[activeRun.id] : undefined
  const previousContent = useRef({ lastMessage, delta: displayedDelta })
  useEffect(() => {
    const changed =
      (Boolean(lastMessage) && lastMessage !== previousContent.current.lastMessage) ||
      (Boolean(displayedDelta) && displayedDelta !== previousContent.current.delta)
    previousContent.current = { lastMessage, delta: displayedDelta }
    if (!transcript.current) return
    if (following.current) transcript.current.scrollTop = transcript.current.scrollHeight
    else if (changed) setNewMessages(true)
  }, [lastMessage, displayedDelta])
  const prepareControl = async (command: OriginalControlScope, payload: OriginalRuntimeControl) => {
    const originalToken = useAuthStore.getState().token
    try {
      const payloadSha256 = await runtimeControlPayloadSha256(payload)
      if (
        !authority.current.owner ||
        useAuthStore.getState().signingOut ||
        authority.current.service !== controlService ||
        useAuthStore.getState().userId !== command.actorId ||
        useAuthStore.getState().token !== originalToken
      )
        throw new ControlPreparationError('Доступ изменился. Команда не отправлена.')
      const original: ControlRecoveryRecord = {
        version: 1,
        service: controlService,
        sessionId: id,
        runId: command.runId,
        agentId: command.agentId,
        actorId: command.actorId,
        key: command.key,
        operation: payload.operation,
        payloadSha256,
      }
      writeControlRecovery(original)
      recoveryRef.current = { state: 'pending', command: original }
      setControlRecovery(recoveryRef.current)
    } catch (error) {
      if (readControlRecovery(id).state === 'blocked') setControlRecovery({ state: 'blocked' })
      throw error instanceof ControlPreparationError
        ? error
        : new ControlPreparationError('Не удалось подготовить ключ сверки. Команда не отправлена.')
    }
  }
  const message = useMutation({
    mutationFn: async (
      command:
        | (OriginalControlScope & { kind: 'steer'; input: string })
        | { kind: 'prompt'; input: string; key: string },
    ) => {
      if (command.kind === 'prompt')
        return createSessionMessage(id, { body: command.input, idempotency_key: command.key })
      await prepareControl(command, { operation: 'steer', input: command.input })
      return steerSessionRun(id, command.runId, { input: command.input }, command.key)
    },
    onSuccess: async (result, command) => {
      if (
        command.kind === 'steer' &&
        (!acceptedControlResponse(result as RuntimeRunControlResponse, id, command, 'steer') ||
          !finishControl(command, 'steer'))
      ) {
        setMessageUncertain(true)
        setReceipt('Исход команды неизвестен. Проверяется сохранённая запись.')
        await invalidate()
        return
      }
      setMessageUncertain(false)
      setBody('')
      setMessageKey(requestKey())
      setReceipt(
        'delivery_state' in result && result.delivery_state === 'pending'
          ? 'Сообщение сохранено. Доставка ожидается.'
          : 'Команда принята. Выполнение проверяется по статусу.',
      )
      await invalidate()
    },
    onError: (error) => {
      if (unknownOutcome(error)) setMessageUncertain(true)
      void invalidate()
    },
  })
  const stopKeys = useRef(new Map<string, string>())
  const stop = useMutation({
    mutationFn: async (command: OriginalControlScope) => {
      await prepareControl(command, { operation: 'stop', input: null })
      return stopSessionRun(id, command.runId, command.key)
    },
    onSuccess: async (result, command) => {
      setStopUncertain(
        !acceptedControlResponse(result, id, command, 'stop') || !finishControl(command, 'stop'),
      )
      await invalidate()
    },
    onError: (error) => {
      if (unknownOutcome(error)) setStopUncertain(true)
      void invalidate()
    },
  })
  const resetMessage = message.reset
  const resetStop = stop.reset
  const originalSteer =
    messageUncertain && message.variables?.kind === 'steer' ? message.variables : undefined
  const knownSteerReceipt =
    originalSteer &&
    message.data &&
    'command' in message.data &&
    message.data.accepted === false &&
    message.data.session_id === id &&
    message.data.run_id === originalSteer.runId &&
    message.data.command?.id &&
    message.data.command.session_id === id &&
    message.data.command.session_run_id === originalSteer.runId &&
    originalSteer.actorId === userId &&
    message.data.command.agent_id === originalSteer.agentId &&
    message.data.command.actor_user_id === originalSteer.actorId &&
    message.data.command.operation === 'steer'
      ? message.data.command
      : undefined
  const originalSteerCommands = useRuntimeControls(
    id,
    owner ? knownSteerReceipt?.session_run_id : undefined,
  )
  useEffect(() => {
    if (
      !owner ||
      !knownSteerReceipt ||
      !originalSteerCommands.isSuccess ||
      originalSteerCommands.isFetching ||
      originalSteerCommands.failureCount > 0
    )
      return
    const confirmed = originalSteerCommands.data?.some(
      (command) =>
        command.id === knownSteerReceipt.id &&
        command.session_id === id &&
        command.session_run_id === knownSteerReceipt.session_run_id &&
        command.agent_id === knownSteerReceipt.agent_id &&
        command.actor_user_id === userId &&
        command.operation === 'steer' &&
        acceptedControl(command),
    )
    if (!confirmed || !originalSteer || !finishControl(originalSteer, 'steer')) return
    resetMessage()
    setMessageUncertain(false)
    setBody('')
    setMessageKey(requestKey())
    setReceipt('Runtime подтвердил исходное уточнение. Состояние запуска проверяется отдельно.')
  }, [
    id,
    userId,
    owner,
    knownSteerReceipt,
    originalSteerCommands.data,
    originalSteerCommands.isSuccess,
    originalSteerCommands.isFetching,
    originalSteerCommands.failureCount,
    resetMessage,
    originalSteer,
    finishControl,
  ])
  const originalControl = controlRecovery.state === 'pending' ? controlRecovery.command : undefined
  const recoveryContextMatches = Boolean(
    originalControl &&
    originalControl.service === controlService &&
    originalControl.actorId === userId,
  )
  const originalControlLookup = useQuery({
    queryKey: [
      'runtime-control-lookup',
      id,
      originalControl?.service,
      originalControl?.actorId,
      originalControl?.runId,
      originalControl?.key,
    ],
    queryFn: () => {
      const original = originalControl!
      return lookupRuntimeControlByDigest(id, original.runId, original.key, {
        operation: original.operation,
        payload_sha256: original.payloadSha256,
      })
    },
    enabled:
      owner &&
      recoveryContextMatches &&
      !message.isPending &&
      !stop.isPending &&
      !knownSteerReceipt,
    retry: false,
    refetchInterval: (query) =>
      query.state.status === 'success' && query.state.data && isUnresolvedControl(query.state.data)
        ? 3000
        : false,
  })
  useEffect(() => {
    if (
      !owner ||
      !originalControl ||
      !recoveryContextMatches ||
      originalControl.actorId !== userId ||
      !originalControlLookup.isSuccess ||
      originalControlLookup.isFetching ||
      originalControlLookup.failureCount > 0
    )
      return
    const receipt = originalControlLookup.data
    const expectedId = originalControl.operation === 'stop' ? stop.data?.command?.id : undefined
    if (
      !matchingControl(receipt, id, originalControl, originalControl.operation) ||
      !acceptedControl(receipt) ||
      (expectedId && receipt?.id !== expectedId)
    )
      return
    if (!finishControl(originalControl, originalControl.operation)) return
    if (originalControl.operation === 'steer') {
      resetMessage()
      setMessageUncertain(false)
      setBody('')
      setMessageKey(requestKey())
      setReceipt('Runtime подтвердил исходное уточнение. Состояние запуска проверяется отдельно.')
    } else {
      resetStop()
      setStopUncertain(false)
      setReceipt('Runtime подтвердил исходную остановку. Завершение запуска проверяется отдельно.')
    }
  }, [
    owner,
    userId,
    originalControl,
    recoveryContextMatches,
    finishControl,
    originalControlLookup.isSuccess,
    originalControlLookup.isFetching,
    originalControlLookup.failureCount,
    originalControlLookup.data,
    stop.data,
    resetMessage,
    resetStop,
    id,
  ])
  const answer = useMutation({
    mutationFn: (command: { questionId: string; questionKey: string; payload: AnswerInput }) => {
      preparePmCommand(answerScope, command.payload.idempotency_key)
      return answerClarification(id, command.questionId, command.payload)
    },
    onSuccess: async (_result, command) => {
      if (!matchesPmActor(answerScope.current, command.payload.idempotency_key)) {
        setAnswerUncertain(true)
        await invalidate()
        return
      }
      setAnswerUncertain(false)
      setReceipt('Ответ сохранён. Требования ещё не опубликованы.')
      setDrafts((current) => {
        const next = { ...current }
        delete next[command.questionKey]
        return next
      })
      await invalidate()
    },
    onError: (error) => {
      if (unknownOutcome(error)) setAnswerUncertain(true)
      void questions.refetch()
      void task.refetch()
    },
  })
  const controlHeld =
    controlRecovery.state !== 'none' ||
    stop.isPending ||
    (message.isPending && message.variables?.kind === 'steer') ||
    stopUncertain ||
    Boolean(originalSteer) ||
    (Boolean(controls.data?.active_run_id) &&
      (!runtimeCommands.isSuccess ||
        runtimeCommands.isFetching ||
        runtimeCommands.failureCount > 0 ||
        runtimeCommands.data?.some(isUnresolvedControl)))
  const canStop =
    owner &&
    controlsFresh &&
    Boolean(controls.data?.can_stop && controls.data.active_run_id) &&
    !stop.isPending &&
    !controlHeld
  const uncertainSteer = messageUncertain && message.variables?.kind === 'steer'
  const canSubmitMessage =
    owner &&
    controlsFresh &&
    !message.isPending &&
    !uncertainSteer &&
    !controlHeld &&
    Boolean(controls.data?.can_steer ? trimRuntimeControlInput(body) : body.trim()) &&
    (messageUncertain || controls.data?.can_send || controls.data?.can_steer)
  const submitMessage = () => {
    if (!canSubmitMessage) return
    if (messageUncertain && message.variables) {
      message.mutate(message.variables)
      return
    }
    message.mutate(
      controls.data?.can_steer && controls.data.active_run_id
        ? {
            kind: 'steer',
            runId: controls.data.active_run_id,
            agentId: session.data!.primary_agent_id,
            actorId: session.data!.user_id,
            input: body,
            key: messageKey,
          }
        : { kind: 'prompt', input: body.trim(), key: messageKey },
    )
  }
  const updateDraft = (change: Partial<typeof draft>) =>
    setDrafts((current) => ({
      ...current,
      [questionKey]: {
        ...(current[questionKey] ?? { selected: [], text: '', comment: '', key: requestKey() }),
        key: requestKey(),
        ...change,
        optionLabels: Object.fromEntries(
          (selectedQuestion?.options ?? []).map((option) => [option.id, option.label]),
        ),
      },
    }))
  const switchTab = (value: string) =>
    setParams((current) => {
      const next = new URLSearchParams(current)
      next.set('tab', value)
      return next
    })
  if (session.isPending) return <EmptyState title="Загрузка чата" />
  if (session.isError || !session.data)
    return (
      <div className="space-y-4">
        <ReadableError error={session.error} />
        <Button
          variant="outline"
          disabled={session.isFetching}
          onClick={() => void session.refetch()}
        >
          Проверить доступ к чату
        </Button>
      </div>
    )
  const agent = agents.data?.find((item) => item.id === session.data.primary_agent_id)
  const waiting = context?.waiting_reason
  const returnTo = params.get('returnTo')
  const backTo =
    returnTo === '/chats' || returnTo?.startsWith('/chats?')
      ? returnTo
      : `/chats${params.get('agent') ? `?agent=${encodeURIComponent(params.get('agent')!)}` : ''}`
  const contextContent = (
    <>
      <h2>Контекст задачи</h2>
      {controlRunId && <RuntimeControlHistory sessionId={id} runId={controlRunId} />}
      <dl className="fc-chat-fields">
        <div>
          <dt>Задача</dt>
          <dd>
            {task.isPending
              ? 'Загрузка контекста'
              : task.isError
                ? 'Контекст не обновлён'
                : (task.data?.binding?.task_id ?? 'Свободный чат')}
          </dd>
        </div>
        <div>
          <dt>Владелец</dt>
          <dd>
            <UserAvatar userId={session.data.user_id} name={session.data.user_display_name} />
            {session.data.user_display_name}
          </dd>
        </div>
        <div>
          <dt>Требования</dt>
          <dd>
            {context?.requirement_revision
              ? `Редакция ${context.requirement_revision}`
              : 'Пока нет редакции'}
          </dd>
        </div>
        <div>
          <dt>Выполнение</dt>
          <dd>
            {controls.isPending || runs.isPending ? (
              'Загрузка запусков'
            ) : controls.isError || runs.isError ? (
              'Состояние запуска не обновлено'
            ) : activeRun ? (
              <StatusBadge value={activeRun.state} />
            ) : (
              'Нет активного запуска'
            )}
          </dd>
        </div>
        <div>
          <dt>Execution</dt>
          <dd>{context?.assignment?.execution_id ?? 'Не назначен'}</dd>
        </div>
        <div>
          <dt>Checkpoint</dt>
          <dd>Нет подтверждённых данных</dd>
        </div>
      </dl>
      <h3>Уточнения</h3>
      <p>
        {questions.isError
          ? 'Источник недоступен'
          : questions.isPending && bound
            ? 'Загрузка уточнений'
            : bound
              ? `${questionList.filter((question) => question.state === 'answered').length} / ${questionList.length}`
              : 'Нет привязки к SDLC'}
      </p>
      <h3>Запуски</h3>
      {runs.isError ? (
        <ReadableError error={runs.error} />
      ) : (
        (runs.data ?? []).map((run) => (
          <details key={run.id}>
            <summary>
              {formatDate(run.created_at)} · <StatusBadge value={run.state} />
            </summary>
            <dl>
              <dt>Run</dt>
              <dd>{run.id}</dd>
              <dt>Runtime</dt>
              <dd>{run.runtime_run_id ?? 'Не подтверждён'}</dd>
              <dt>Модель</dt>
              <dd>
                {run.model ?? 'Неизвестно'} · {run.provider ?? 'Неизвестно'}
              </dd>
            </dl>
            {run.last_error && <p role="alert">{run.last_error}</p>}
          </details>
        ))
      )}
    </>
  )
  return (
    <section className="fc-chat-workbench">
      <header className="fc-chat-header">
        <div>
          <Link to={backTo} aria-label="Вернуться к чатам">
            <ArrowLeft size={18} />
          </Link>
          <span>{session.data.task_key}</span>
          <h1>{session.data.title}</h1>
          <StatusBadge
            value={session.data.visibility === 'private' ? 'private' : 'leader_scoped'}
          />
        </div>
        <div>
          {context?.stage && <StatusBadge value={context.stage} />}
          {waiting && <span className="fc-chat-wait">{waiting}</span>}
          <Button
            ref={contextTrigger}
            variant="ghost"
            className="fc-chat-context-button"
            aria-label="Контекст задачи"
            title="Контекст задачи"
            onClick={() => setContextOpen(true)}
          >
            <Info size={18} />
          </Button>
        </div>
      </header>
      <div className="fc-chat-agent">
        <Bot size={16} />
        <strong>{agent?.display_name ?? session.data.primary_agent_name}</strong>
        <span>{agent?.kind ?? 'Runtime неизвестен'}</span>
        <StatusBadge value={agent?.status} />
        {controls.data?.blocked_reason && (
          <span>{blockedLabels[controls.data.blocked_reason] ?? controls.data.blocked_reason}</span>
        )}
      </div>
      <div className="fc-chat-grid">
        <div className="fc-chat-main">
          <Tabs value={tab} onValueChange={switchTab} className="fc-chat-tabs">
            <TabsList className="fc-chat-tab-list" aria-label="Разделы задачи">
              <TabsTrigger value="dialogue">
                <MessageSquare size={15} />
                Диалог
              </TabsTrigger>
              <TabsTrigger value="clarification">
                <ClipboardList size={15} />
                Уточнения
              </TabsTrigger>
              <TabsTrigger value="requirements">
                <FileText size={15} />
                Требования
              </TabsTrigger>
            </TabsList>
            {task.isError && <ReadableError error={task.error} />}
            {receipt && (
              <p role="status" className="fc-chat-notice">
                {receipt}
              </p>
            )}
            <TabsContent value="dialogue" className="fc-chat-panel">
              <div
                ref={attachTranscript}
                className="fc-chat-scroll"
                onScroll={() => {
                  const node = transcript.current
                  if (node) {
                    readingPosition.current = node.scrollTop
                    following.current = node.scrollHeight - node.scrollTop - node.clientHeight < 48
                  }
                }}
              >
                {history.isError && <ReadableError error={history.error} />}
                {history.isPending && <p>Загрузка истории</p>}
                {history.hasNextPage && (
                  <Button
                    variant="outline"
                    disabled={history.isFetchingNextPage}
                    onClick={() => {
                      const node = transcript.current
                      const before = node?.scrollHeight ?? 0
                      void history.fetchNextPage().then(() => {
                        requestAnimationFrame(() => {
                          if (node) node.scrollTop += node.scrollHeight - before
                        })
                      })
                    }}
                  >
                    Предыдущие сообщения
                  </Button>
                )}
                {history.isSuccess && !messages.length && <EmptyState title="Сообщений пока нет" />}
                {messages.map((item) => (
                  <article className="fc-chat-message" key={item.id}>
                    <div>
                      <UserAvatar
                        userId={item.author_user_id ?? item.author_agent_id ?? 'system'}
                        name={item.author_display_name}
                      />
                      <strong>{item.author_display_name}</strong>
                      <time>{formatDate(item.created_at)}</time>
                      {item.author_type === 'user' && <StatusBadge value={item.delivery_state} />}
                    </div>
                    <p>{item.body}</p>
                    {item.delivery_error && <p role="alert">{item.delivery_error}</p>}
                  </article>
                ))}
                {activeRun && delta[activeRun.id] && (
                  <article className="fc-chat-message">
                    <strong>{activeRun.agent_name}</strong>
                    <p>{delta[activeRun.id]}</p>
                  </article>
                )}
                <TaskApprovalsPanel sessionId={id} canResolve={canResolveApprovals} hideWhenEmpty />
              </div>
              {newMessages && (
                <Button
                  variant="outline"
                  onClick={() => {
                    following.current = true
                    setNewMessages(false)
                    if (transcript.current)
                      transcript.current.scrollTop = transcript.current.scrollHeight
                  }}
                >
                  <ArrowDown size={15} />
                  Новые сообщения
                </Button>
              )}
              {bound && questionList.some((question) => question.state === 'open') && (
                <Button variant="outline" onClick={() => switchTab('clarification')}>
                  <ClipboardList size={15} />
                  Ответить на уточнение
                </Button>
              )}
              <form
                className="fc-chat-composer"
                onSubmit={(event) => {
                  event.preventDefault()
                  submitMessage()
                }}
              >
                <Label htmlFor="chat-prompt">
                  {controls.data?.can_steer ? 'Уточнение активному запуску' : 'Сообщение агенту'}
                </Label>
                <Textarea
                  id="chat-prompt"
                  value={body}
                  disabled={
                    !owner ||
                    (!controls.data?.can_send && !controls.data?.can_steer) ||
                    message.isPending ||
                    messageUncertain ||
                    stopUncertain ||
                    controlRecovery.state !== 'none'
                  }
                  onChange={(event) => {
                    setBody(event.target.value)
                    setMessageKey(requestKey())
                  }}
                  rows={2}
                />
                {message.isError && <ReadableError error={message.error} />}
                {messageUncertain && (
                  <p role="status">
                    {uncertainSteer
                      ? 'Исход уточнения запуску неизвестен. Нельзя повторить его как новый prompt; требуется сверка runtime.'
                      : 'Исход команды проверяется. Повтор сообщения использует прежний ключ; текст пока нельзя менять.'}
                  </p>
                )}
                {controls.isError && (
                  <>
                    <ReadableError error={controls.error} />
                    <Button
                      type="button"
                      variant="outline"
                      disabled={controls.isFetching}
                      onClick={() => void controls.refetch()}
                    >
                      Проверить доступность отправки
                    </Button>
                  </>
                )}
                <div>
                  <span>
                    <ShieldCheck size={14} />
                    {owner ? 'Ваш чат' : 'Только чтение'}
                  </span>
                  <Button
                    type="submit"
                    aria-label={
                      controls.data?.can_steer
                        ? 'Передать уточнение запуску'
                        : 'Отправить сообщение'
                    }
                    title={
                      controls.data?.can_steer
                        ? 'Передать уточнение запуску'
                        : 'Отправить сообщение'
                    }
                    disabled={!canSubmitMessage}
                  >
                    <Send size={16} />
                  </Button>
                  {controls.data?.can_stop && (
                    <Button
                      type="button"
                      variant="outline"
                      aria-label="Остановить запуск"
                      title="Остановить запуск"
                      disabled={!canStop}
                      onClick={() => {
                        const runId = controls.data?.active_run_id
                        if (!canStop || !runId) return
                        const key = stopKeys.current.get(runId) ?? requestKey()
                        stopKeys.current.set(runId, key)
                        stop.mutate({
                          runId,
                          key,
                          agentId: session.data.primary_agent_id,
                          actorId: session.data.user_id,
                        })
                      }}
                    >
                      <Square size={15} />
                    </Button>
                  )}
                </div>
                {stop.isError && <ReadableError error={stop.error} />}
                {(stopUncertain || originalControl?.operation === 'stop') && (
                  <p role="status">Исход остановки неизвестен. Проверяется исходная запись.</p>
                )}
                {originalControl?.operation === 'steer' && !messageUncertain && (
                  <p role="status">
                    Исход уточнения запуску неизвестен. Проверяется исходная запись.
                  </p>
                )}
                {controlRecovery.state === 'blocked' && (
                  <p role="alert">Ключ сверки недоступен. Новые команды заблокированы.</p>
                )}
                {originalControl && !recoveryContextMatches && (
                  <p role="status">
                    Для сверки исходной команды нужен тот же пользователь и сервис Fleet Control.
                  </p>
                )}
                {originalControl && (
                  <div className="space-y-2">
                    {originalControlLookup.isError && (
                      <ReadableError error={originalControlLookup.error} />
                    )}
                    <Button
                      variant="outline"
                      type="button"
                      disabled={
                        !owner ||
                        !recoveryContextMatches ||
                        message.isPending ||
                        stop.isPending ||
                        originalControlLookup.isFetching
                      }
                      onClick={() => void originalControlLookup.refetch()}
                    >
                      {originalControl.operation === 'steer'
                        ? 'Проверить исходное уточнение'
                        : 'Проверить исходную остановку'}
                    </Button>
                  </div>
                )}
              </form>
            </TabsContent>
            <TabsContent value="clarification" className="fc-chat-panel">
              <div className="fc-chat-scroll">
                {task.isPending ? (
                  <p>Загрузка контекста</p>
                ) : !bound ? (
                  <EmptyState
                    title={
                      task.isError ? 'Контекст недоступен' : 'Свободный чат: уточнения не привязаны'
                    }
                  />
                ) : questions.isPending ? (
                  <p>Загрузка уточнений</p>
                ) : questions.isError && !questionList.length ? (
                  <ReadableError error={questions.error} />
                ) : !questionList.length ? (
                  <EmptyState title="Уточнений пока нет" />
                ) : (
                  <>
                    {questions.isError && (
                      <>
                        <ReadableError error={questions.error} />
                        <p role="status">
                          Показаны последние полученные вопросы. Ответы заблокированы до обновления.
                        </p>
                      </>
                    )}
                    <nav className="fc-chat-question-list" aria-label="Вопросы">
                      {questionList.map((question, index) => (
                        <button
                          key={question.id}
                          aria-current={question.id === selectedQuestion?.id ? 'true' : undefined}
                          onClick={() =>
                            setParams((current) => {
                              const next = new URLSearchParams(current)
                              next.set('question', question.id)
                              return next
                            })
                          }
                        >
                          {index + 1}. {question.text}
                          <StatusBadge value={question.state} />
                        </button>
                      ))}
                    </nav>
                    {selectedQuestion && (
                      <fieldset
                        className="fc-chat-question"
                        disabled={
                          !owner ||
                          !taskFresh ||
                          !questionsFresh ||
                          !context?.permissions.can_answer ||
                          selectedQuestion.state !== 'open' ||
                          answer.isPending ||
                          answerUncertain
                        }
                      >
                        <legend>{selectedQuestion.text}</legend>
                        <p>{selectedQuestion.rationale}</p>
                        <p className="fc-chat-muted">
                          {selectedQuestion.required
                            ? 'Обязательный вопрос'
                            : 'Необязательный вопрос'}{' '}
                          · Требования: ред. {selectedQuestion.requirement_revision} ·{' '}
                          {selectedQuestion.requirement_reference}
                        </p>
                        {selectedQuestion.mode !== 'text' &&
                          selectedQuestion.options.map((option) => (
                            <label className="fc-chat-option" key={option.id}>
                              <input
                                type={selectedQuestion.mode === 'multiple' ? 'checkbox' : 'radio'}
                                name={`question-${selectedQuestion.id}`}
                                checked={
                                  selectedQuestion.state === 'answered'
                                    ? (selectedQuestion.answer?.selected_option_ids.includes(
                                        option.id,
                                      ) ?? false)
                                    : draft.selected.includes(option.id)
                                }
                                onChange={(event) =>
                                  updateDraft({
                                    selected:
                                      selectedQuestion.mode === 'single'
                                        ? [option.id]
                                        : event.target.checked
                                          ? [...draft.selected, option.id]
                                          : draft.selected.filter((value) => value !== option.id),
                                  })
                                }
                              />
                              <span>
                                <strong>{option.label}</strong>
                                {selectedQuestion.recommended_option_id === option.id && (
                                  <small>Рекомендация PM</small>
                                )}
                                <span>{option.consequences}</span>
                              </span>
                            </label>
                          ))}
                        <Label htmlFor="clarification-text">
                          {selectedQuestion.mode === 'text'
                            ? 'Ваш ответ'
                            : 'Свой вариант или детали'}
                        </Label>
                        <Textarea
                          id="clarification-text"
                          value={
                            selectedQuestion.state === 'answered'
                              ? (selectedQuestion.answer?.text ?? '')
                              : draft.text
                          }
                          onChange={(event) => updateDraft({ text: event.target.value })}
                        />
                        <Label htmlFor="clarification-comment">Комментарий</Label>
                        <Textarea
                          id="clarification-comment"
                          value={
                            selectedQuestion.state === 'answered'
                              ? (selectedQuestion.answer?.comment ?? '')
                              : draft.comment
                          }
                          onChange={(event) => updateDraft({ comment: event.target.value })}
                        />
                      </fieldset>
                    )}
                    {staleDrafts.map(([key, previous]) => (
                      <section key={key} className="fc-chat-question" role="alert">
                        <h3>Вопрос изменился. Несохранённый ответ сохранён отдельно.</h3>
                        <p>{previous.text}</p>
                        <p>{previous.comment}</p>
                        <p>
                          {previous.selected
                            .map((optionId) => previous.optionLabels[optionId] ?? optionId)
                            .join(', ')}
                        </p>
                        <Button
                          variant="outline"
                          disabled={
                            !owner ||
                            !taskFresh ||
                            !questionsFresh ||
                            !context?.permissions.can_answer ||
                            selectedQuestion?.state !== 'open' ||
                            answer.isPending ||
                            answerUncertain ||
                            Boolean(
                              draft.text.trim() || draft.comment.trim() || draft.selected.length,
                            )
                          }
                          onClick={() =>
                            setDrafts((current) => {
                              const next = { ...current }
                              delete next[key]
                              next[questionKey] = {
                                ...previous,
                                key: requestKey(),
                                optionLabels: Object.fromEntries(
                                  (selectedQuestion?.options ?? []).map((option) => [
                                    option.id,
                                    option.label,
                                  ]),
                                ),
                                selected: previous.selected.filter((id) =>
                                  selectedQuestion?.options.some((option) => option.id === id),
                                ),
                              }
                              return next
                            })
                          }
                        >
                          Перенести черновик и проверить новый вопрос
                        </Button>
                      </section>
                    ))}
                  </>
                )}
                {answer.isError && <ReadableError error={answer.error} />}
                {answerUncertain && (
                  <div>
                    <p role="status">
                      Неизвестен исход сохранения. Проверьте состояние вопроса или повторите тот же
                      ответ без изменения ключа.
                    </p>
                    <Button
                      variant="outline"
                      disabled={
                        !owner ||
                        !taskFresh ||
                        !questionsFresh ||
                        !matchesPmActor(
                          answerScope.current,
                          answer.variables?.payload.idempotency_key,
                        ) ||
                        answer.isPending ||
                        !answer.variables
                      }
                      onClick={() => answer.variables && answer.mutate(answer.variables)}
                    >
                      Повторить исходный ответ
                    </Button>
                  </div>
                )}
              </div>
              <footer className="fc-chat-answer-footer">
                <p>Ответ закрывает вопрос, но не публикует требования.</p>
                <Button
                  disabled={
                    !selectedQuestion ||
                    !owner ||
                    !taskFresh ||
                    !questionsFresh ||
                    !context?.permissions.can_answer ||
                    (answerUncertain &&
                      !matchesPmActor(
                        answerScope.current,
                        answer.variables?.payload.idempotency_key,
                      )) ||
                    (answerUncertain && answer.variables?.questionKey !== questionKey) ||
                    !draft.key ||
                    !canSubmitAnswer(selectedQuestion, draft.selected, draft.text) ||
                    answer.isPending
                  }
                  onClick={() => {
                    if (answerUncertain && answer.variables) {
                      answer.mutate(answer.variables)
                      return
                    }
                    if (!selectedQuestion) return
                    answer.mutate({
                      questionId: selectedQuestion.id,
                      questionKey,
                      payload: {
                        expected_question_version: selectedQuestion.version,
                        requirement_revision: selectedQuestion.requirement_revision,
                        selected_option_ids: draft.selected,
                        text: draft.text.trim() || null,
                        comment: draft.comment.trim() || null,
                        idempotency_key: draft.key,
                      },
                    })
                  }}
                >
                  <Check size={15} />
                  Сохранить ответ
                </Button>
              </footer>
            </TabsContent>
            <TabsContent value="requirements" className="fc-chat-panel">
              <div className="fc-chat-scroll">
                {task.isPending ? (
                  <p>Загрузка контекста</p>
                ) : !bound ? (
                  <EmptyState title="Нет привязки к SDLC" />
                ) : requirements.isPending ? (
                  <p>Загрузка требований</p>
                ) : requirements.isError && !requirements.data ? (
                  <ReadableError error={requirements.error} />
                ) : (
                  <>
                    {requirements.isError && (
                      <>
                        <ReadableError error={requirements.error} />
                        <p role="status">
                          Показана последняя полученная редакция. Подтверждение заблокировано до
                          обновления.
                        </p>
                      </>
                    )}
                    <RequirementsView
                      revisions={requirements.data?.revisions ?? []}
                      currentRevision={context?.requirement_revision ?? null}
                      canConfirm={
                        owner &&
                        taskFresh &&
                        requirementsFresh &&
                        Boolean(context?.permissions.can_confirm)
                      }
                      confirmation={confirmation}
                      confirmationUncertain={confirmationUncertain}
                      canReplayConfirmation={
                        owner &&
                        taskFresh &&
                        requirementsFresh &&
                        matchesPmActor(confirmationScope.current, confirmation.variables?.key)
                      }
                    />
                  </>
                )}
              </div>
            </TabsContent>
          </Tabs>
        </div>
        <aside className="fc-chat-context">{contextContent}</aside>
      </div>
      <Dialog open={contextOpen} onOpenChange={setContextOpen}>
        <DialogContent
          onCloseAutoFocus={(event) => {
            event.preventDefault()
            contextTrigger.current?.focus()
          }}
        >
          <DialogHeader>
            <DialogTitle>Контекст задачи</DialogTitle>
          </DialogHeader>
          <div className="fc-chat-drawer-content">{contextContent}</div>
        </DialogContent>
      </Dialog>
      <Dialog
        open={blocker.state === 'blocked'}
        onOpenChange={(open) => {
          if (!open && blocker.state === 'blocked') blocker.reset()
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Остались несохранённые изменения</DialogTitle>
          </DialogHeader>
          <p>Черновик хранится только в этом чате и будет потерян при переходе.</p>
          <Button variant="outline" onClick={() => blocker.state === 'blocked' && blocker.reset()}>
            Остаться
          </Button>
          <Button onClick={() => blocker.state === 'blocked' && blocker.proceed()}>
            Уйти без сохранения
          </Button>
        </DialogContent>
      </Dialog>
    </section>
  )
}

const documentLabels = {
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
function RequirementsBody({ revision }: { revision: RequirementsRevision }) {
  return (
    <article className="fc-chat-requirements">
      <h2>Редакция {revision.revision}</h2>
      <p>{revision.goal}</p>
      {Object.entries(documentLabels).map(([key, label]) => (
        <section key={key}>
          <h3>{label}</h3>
          <ul>
            {(revision[key as keyof typeof documentLabels] ?? []).map((item, index) => (
              <li key={index}>{item}</li>
            ))}
          </ul>
        </section>
      ))}
    </article>
  )
}
function RequirementsView({
  revisions,
  currentRevision,
  canConfirm,
  confirmation,
  confirmationUncertain,
  canReplayConfirmation,
}: {
  revisions: RequirementsRevision[]
  currentRevision: number | null
  canConfirm: boolean
  confirmation: ConfirmationMutation
  confirmationUncertain: boolean
  canReplayConfirmation: boolean
}) {
  const [params, setParams] = useSearchParams()
  const selected =
    revisions.find((revision) => String(revision.revision) === params.get('revision')) ??
    revisions.find((revision) => revision.revision === currentRevision) ??
    revisions.at(-1)
  const previous = revisions
    .filter((revision) => selected && revision.revision < selected.revision)
    .sort((a, b) => b.revision - a.revision)[0]
  if (!selected) return <EmptyState title="PM пока не подготовил требования" />
  return (
    <>
      <Label htmlFor="requirement-revision">Редакция требований</Label>
      <select
        id="requirement-revision"
        value={selected.revision}
        onChange={(event) =>
          setParams((current) => {
            const next = new URLSearchParams(current)
            next.set('revision', event.target.value)
            return next
          })
        }
      >
        {revisions.map((revision) => (
          <option key={revision.revision} value={revision.revision}>
            Редакция {revision.revision}
          </option>
        ))}
      </select>
      <RequirementsBody revision={selected} />
      {previous && (
        <details>
          <summary>Сравнить с редакцией {previous.revision}</summary>
          <div className="fc-chat-revision-comparison">
            <RequirementsBody revision={previous} />
            <RequirementsBody revision={selected} />
          </div>
        </details>
      )}
      <RevisionConfirmation
        key={`${selected.revision}:${selected.content_hash}`}
        revision={selected}
        enabled={canConfirm && selected.revision === currentRevision}
        mutation={confirmation}
        uncertain={confirmationUncertain}
        canReplay={canReplayConfirmation}
      />
    </>
  )
}
function RevisionConfirmation({
  revision,
  enabled,
  mutation,
  uncertain,
  canReplay,
}: {
  revision: RequirementsRevision
  enabled: boolean
  mutation: ConfirmationMutation
  uncertain: boolean
  canReplay: boolean
}) {
  const [checked, setChecked] = useState(false)
  const [key] = useState(requestKey)
  const sameTarget =
    mutation.variables?.revision === revision.revision &&
    mutation.variables?.hash === revision.content_hash
  const held = mutation.isPending || uncertain
  const saved = sameTarget && mutation.isSuccess && !uncertain
  return (
    <div className="fc-chat-confirm">
      <label>
        <input
          type="checkbox"
          checked={checked || (sameTarget && (held || saved))}
          disabled={!enabled || held || saved}
          onChange={(event) => setChecked(event.target.checked)}
        />
        Подтверждаю цель, границы и критерии приёмки редакции {revision.revision}
      </label>
      {!enabled && <p>Подтверждение недоступно: проверьте актуальную редакцию и prerequisites.</p>}
      <Button
        disabled={
          !enabled ||
          (!checked && !(sameTarget && uncertain)) ||
          mutation.isPending ||
          saved ||
          (uncertain && !canReplay) ||
          (uncertain && !sameTarget)
        }
        onClick={() =>
          mutation.mutate(
            uncertain && sameTarget && mutation.variables
              ? mutation.variables
              : { revision: revision.revision, hash: revision.content_hash, key },
          )
        }
      >
        <Check size={15} />
        Подтвердить редакцию {revision.revision}
      </Button>
      {mutation.isError && sameTarget && <ReadableError error={mutation.error} />}
      {uncertain && (
        <div>
          <p role="status">
            Исход подтверждения редакции {mutation.variables?.revision} неизвестен. Сверьте
            состояние; повтор возможен только для исходной редакции и с прежним ключом.
          </p>
          <Button
            variant="outline"
            disabled={!canReplay || mutation.isPending || !mutation.variables}
            onClick={() => mutation.variables && mutation.mutate(mutation.variables)}
          >
            Повторить исходное подтверждение
          </Button>
        </div>
      )}
      {saved && (
        <p role="status">Подтверждение сохранено. Следующее назначение проверяется отдельно.</p>
      )}
    </div>
  )
}
