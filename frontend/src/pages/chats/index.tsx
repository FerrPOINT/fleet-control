import { useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, ArrowRight, Bot, MessageSquare, Plus, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { ApiError, parseNamespaceLocation } from '@sdlc/ui/lib'
import type { components } from '@/api/generated'
import {
  payloadDigest,
  markDispatch,
  clearDispatch,
  dispatchHeld,
  commandService,
  type DispatchMarker,
} from '../chat-detail/core'
import { Button, Dialog, DialogContent, DialogHeader, DialogTitle, Input, Label } from '@sdlc/ui/ui'
import { createSession, createContextSession, getSession, listAgentDirectory } from '@/api/fleet'
import { getChatsDirectory } from '@/api/chats-directory'
import type { AgentDirectoryItem, CreateSessionRequest } from '@/api/types'
import { useSessionUserFilter, SessionUserFilter } from '@/shared/session-user-filter'
import { isCurrentAuth, ssoConfig, useAuthStore } from '@/shared/auth/store'
import { apiBaseUrl } from '@/api/client'
import { ControlPreparationError, controlRecoveryService } from '@/shared/chat-control-recovery'
import { useDispatchRecovery } from '../chat-detail/dispatch-recovery'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { sdlcRoleLabel } from '@/shared/sdlc-roles'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

function contextPath(path: string, params: URLSearchParams) {
  const target = new URL(path, 'https://relative.invalid')
  const context = target.searchParams
  for (const key of ['registry_instance_id', 'namespace_id', 'tracker_instance_id', 'task_id'])
    for (const value of params.getAll(key)) context.append(key, value)
  return target.pathname + target.search + target.hash
}

export function ChatsPage() {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const search = params.get('q') ?? ''
  const [createOpen, setCreateOpen] = useState(false)
  const currentUserId = useAuthStore((state) => state.userId)
  const userScope = params.get('users')
  const filter = useSessionUserFilter()
  const selectedUserIds =
    filter.isSystemAdmin && userScope !== null
      ? userScope.trim().toLowerCase() === 'all' || !userScope.trim()
        ? []
        : userScope
            .split(',')
            .map((value) => value.trim())
            .filter(Boolean)
      : currentUserId
        ? [currentUserId]
        : []
  const agentId = params.get('agent') ?? undefined
  const before = params.get('before') ?? undefined
  const directory = useQuery({
    queryKey: [
      'chats-directory',
      currentUserId,
      filter.isSystemAdmin,
      agentId,
      selectedUserIds,
      search,
      before,
    ],
    queryFn: () =>
      getChatsDirectory({ agentId, userIds: selectedUserIds, search, before, limit: 50 }),
    enabled: Boolean(currentUserId),
  })
  const page = directory.isError ? undefined : directory.data
  const groups = page?.agents ?? []
  const selectedId = page?.selected_agent_id
  const selected = groups.find((group) => group.agent.id === selectedId)
  const returnParams = new URLSearchParams(params)
  if (selectedId) returnParams.set('agent', selectedId)
  returnParams.set('users', selectedUserIds.join(',') || 'all')
  const returnTo = `/chats?${returnParams}`
  const setUsers = (ids: string[]) => {
    setParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.set('users', ids.join(',') || 'all')
        next.delete('before')
        return next
      },
      { replace: true },
    )
  }
  const persistedFilter = {
    ...filter,
    selectedUserIds,
    selectedUsers: selectedUserIds
      .map((id) => filter.allUsers.find((user) => user.id === id))
      .filter((user): user is NonNullable<typeof user> => Boolean(user)),
    addUser: (userId: string) => {
      if (filter.isSystemAdmin) setUsers([...new Set([...selectedUserIds, userId])])
    },
    removeUser: (userId: string) => {
      if (filter.isSystemAdmin) setUsers(selectedUserIds.filter((id) => id !== userId))
    },
  }
  const visibleSessions = page?.items ?? []
  const firstPage = () =>
    setParams((current) => {
      const next = new URLSearchParams(current)
      next.delete('before')
      return next
    })

  return (
    <>
      <PageHeader
        title={t('chats.title')}
        actions={
          selected?.agent.product_role === 'executor' && selected.agent.status !== 'archived' ? (
            <Button
              onClick={() => setCreateOpen(true)}
              disabled={directory.isPending || directory.isError}
            >
              <Plus className="h-4 w-4" />
              {t('chats.new')}
            </Button>
          ) : null
        }
      />
      <SessionUserFilter filter={persistedFilter} className="mb-4" />
      <label className="relative mb-4 block w-full min-w-0 sm:max-w-md">
        <Search className="absolute left-3 top-3 h-4 w-4 text-text-muted" aria-hidden />
        <Input
          value={search}
          onChange={(event) =>
            setParams(
              (current) => {
                const next = new URLSearchParams(current)
                if (event.target.value) next.set('q', event.target.value)
                else next.delete('q')
                next.delete('before')
                return next
              },
              { replace: true },
            )
          }
          className="h-10 pl-9"
          maxLength={200}
          aria-label={t('chats.search')}
          placeholder={t('chats.search')}
        />
      </label>
      {directory.isError ? (
        <div className="mb-4 space-y-2">
          <ErrorState message={t('chats.loadError')} />
          <Button
            variant="outline"
            onClick={() => {
              void directory.refetch()
            }}
          >
            {t('sessions.retry')}
          </Button>
          {before && (
            <Button variant="outline" onClick={firstPage}>
              {t('chats.firstPage', { defaultValue: 'Первая страница' })}
            </Button>
          )}
          {agentId && (
            <Button
              variant="outline"
              onClick={() =>
                setParams((current) => {
                  const next = new URLSearchParams(current)
                  next.delete('agent')
                  next.delete('before')
                  return next
                })
              }
            >
              {t('chats.selectAgent')}
            </Button>
          )}
        </div>
      ) : null}
      {directory.isPending ? (
        <EmptyState title={t('chats.loading')} />
      ) : !directory.isError ? (
        <div className="grid min-w-0 gap-4 md:grid-cols-[260px_minmax(0,1fr)]">
          <nav
            aria-label={t('chats.agents')}
            className="min-w-0 border-b border-border md:border-b-0 md:border-r md:pr-4"
          >
            <ul className="space-y-1 pb-4">
              {groups.map(({ agent, matching_session_count }) => (
                <li key={agent.id}>
                  <button
                    type="button"
                    aria-current={agent.id === selectedId ? 'true' : undefined}
                    onClick={() =>
                      setParams((current) => {
                        const next = new URLSearchParams(current)
                        next.set('agent', agent.id)
                        next.delete('before')
                        return next
                      })
                    }
                    className={`flex min-h-14 w-full min-w-0 items-center gap-3 rounded-md px-3 py-2 text-left ${agent.id === selectedId ? 'bg-surface-raised' : 'hover:bg-surface-raised'}`}
                  >
                    <Bot className="h-4 w-4 shrink-0 text-text-muted" />
                    <span className="min-w-0 flex-1">
                      <span className="block break-words text-sm font-medium text-text-primary">
                        {agent.display_name}
                      </span>
                      <span className="block text-xs text-text-muted">
                        {agent.name} · {sdlcRoleLabel(agent.sdlc_role)}
                      </span>
                    </span>
                    <span
                      className="text-xs tabular-nums text-text-muted"
                      aria-label={t('chats.count')}
                    >
                      {matching_session_count}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
            {!groups.length ? <EmptyState title={t('sessions.noAgents')} /> : null}
          </nav>
          <section className="min-w-0" aria-label={t('chats.sessions')}>
            {selected ? (
              <>
                <div className="mb-4 flex flex-wrap items-center gap-3 border-b border-border pb-4">
                  <h2 className="min-w-0 break-words text-base font-semibold text-text-primary">
                    {selected.agent.display_name}
                  </h2>
                  <StatusBadge value={selected.agent.status} />
                </div>
                <ul className="divide-y divide-border">
                  {visibleSessions.map((session) => (
                    <li key={session.id}>
                      <Link
                        to={contextPath(
                          `/chats/${session.id}?returnTo=${encodeURIComponent(returnTo)}`,
                          params,
                        )}
                        className="flex min-w-0 items-start gap-3 rounded-sm px-2 py-4 hover:bg-surface-raised focus-visible:outline-focus"
                      >
                        <UserAvatar
                          userId={session.user_id}
                          name={session.user_display_name}
                          size="md"
                        />
                        <span className="min-w-0 flex-1">
                          <span className="flex flex-wrap items-center gap-2">
                            <span className="break-words text-sm font-medium text-text-primary">
                              {session.title}
                            </span>
                            <StatusBadge value={session.state} />
                            <StatusBadge value={session.visibility} />
                          </span>
                          <span className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-text-muted">
                            <span>{session.task_key || t('chats.freeChat')}</span>
                            <span>{session.user_display_name}</span>
                            <span>{formatDate(session.updated_at)}</span>
                          </span>
                          {session.last_message_preview ? (
                            <span className="mt-2 line-clamp-2 block break-words text-sm text-text-secondary">
                              {session.last_message_preview}
                            </span>
                          ) : null}
                        </span>
                        <MessageSquare
                          className="mt-1 h-4 w-4 shrink-0 text-text-muted"
                          aria-hidden
                        />
                      </Link>
                    </li>
                  ))}
                </ul>
                {!visibleSessions.length ? <EmptyState title={t('chats.empty')} /> : null}
                <nav
                  className="mt-4 flex items-center justify-end gap-2"
                  aria-label={t('chats.pagination', { defaultValue: 'Страницы чатов' })}
                >
                  <Button
                    variant="outline"
                    className="h-10 w-10 p-0"
                    title={t('chats.firstPage', { defaultValue: 'Первая страница' })}
                    aria-label={t('chats.firstPage', { defaultValue: 'Первая страница' })}
                    disabled={!before || directory.isFetching}
                    onClick={firstPage}
                  >
                    <ArrowLeft className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="outline"
                    className="h-10 w-10 p-0"
                    title={t('chats.nextPage', { defaultValue: 'Следующая страница' })}
                    aria-label={t('chats.nextPage', { defaultValue: 'Следующая страница' })}
                    disabled={!page?.next_before || directory.isFetching}
                    onClick={() => {
                      if (!page?.next_before || !selectedId) return
                      setParams((current) => {
                        const next = new URLSearchParams(current)
                        next.set('agent', selectedId)
                        next.set('before', page.next_before!)
                        return next
                      })
                    }}
                  >
                    <ArrowRight className="h-4 w-4" />
                  </Button>
                </nav>
              </>
            ) : (
              <EmptyState title={t('chats.selectAgent')} />
            )}
          </section>
        </div>
      ) : null}
      <CreatePrivateChat
        agent={selected?.agent}
        open={createOpen}
        onOpenChange={setCreateOpen}
        returnTo={returnTo}
      />
    </>
  )
}

function CreatePrivateChat({
  agent,
  open,
  onOpenChange,
  returnTo,
}: {
  agent?: AgentDirectoryItem
  open: boolean
  onOpenChange: (value: boolean) => void
  returnTo: string
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [title, setTitle] = useState('')
  const [key, setKey] = useState(() => crypto.randomUUID())
  const [uncertain, setUncertain] = useState(false)
  const dispatch = useDispatchRecovery('create:chats')
  const [contextParams] = useSearchParams()
  const namespaceCreation =
    import.meta.env.VITE_NAMESPACE_ENABLED === 'true' &&
    ['registry_instance_id', 'namespace_id', 'tracker_instance_id', 'task_id'].some((name) =>
      contextParams.has(name),
    )
  const namespaceScope = `create:namespace:${agent?.id ?? ''}`
  const [namespaceHeld, setNamespaceHeld] = useState(() => dispatchHeld(namespaceScope))
  const mutation = useMutation({
    mutationFn: async (command: {
      input: CreateSessionRequest
      agentName: string
      context?: components['schemas']['ExecutionContextV2']
      marker?: DispatchMarker
    }) => {
      if (namespaceCreation || command.context) {
        const scope = useAuthStore.getState()
        const namespace = parseNamespaceLocation(contextParams.toString())
        const identity = (name: string) => {
          const values = contextParams.getAll(name)
          return values.length === 1 &&
            /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(
              values[0] ?? '',
            ) &&
            values[0] !== '00000000-0000-0000-0000-000000000000'
            ? values[0]
            : null
        }
        const tracker = identity('tracker_instance_id'),
          task = identity('task_id')
        if (!command.context && (!namespace || !tracker || !task))
          throw new ControlPreparationError('Invalid Namespace Task reference')
        const context: components['schemas']['ExecutionContextV2'] = command.context ?? {
          schema_version: 2,
          operation_id: command.input.idempotency_key!,
          namespace: namespace!,
          task: { tracker_instance_id: tracker!, task_id: task! },
          repositories: [],
        }
        const current = (await listAgentDirectory()).find(
          (entry) => entry.id === command.input.primary_agent_id,
        )
        if (
          !isCurrentAuth(scope) ||
          !scope.permissions.includes('sessions:write_own') ||
          current?.product_role !== 'executor' ||
          current.status === 'archived'
        )
          throw new ControlPreparationError('Read-only')
        const scopeId = `create:namespace:${command.input.primary_agent_id}`
        if (!command.marker && dispatchHeld(scopeId))
          throw new ControlPreparationError('Reconcile original Namespace creation')
        command.context = context
        if (!command.marker) {
          const marker = {
            actor: scope.userId!,
            agent: command.input.primary_agent_id!,
            service: commandService(apiBaseUrl, ssoConfig.issuer),
            key: command.input.idempotency_key!,
            digest: await payloadDigest({
              primary_agent_id: command.input.primary_agent_id,
              title: command.input.title,
              context,
            }),
          }
          markDispatch(scopeId, marker)
          command.marker = marker
        }
        if (command.marker.actor !== scope.userId)
          throw new ControlPreparationError('Authentication changed')
        setNamespaceHeld(true)
        const receipt = await createContextSession({
          primary_agent_id: command.input.primary_agent_id!,
          title: command.input.title!,
          context,
        })
        const confirmed = await getSession(receipt.session.id)
        if (
          !isCurrentAuth(scope) ||
          confirmed.user_id !== scope.userId ||
          confirmed.primary_agent_id !== command.input.primary_agent_id ||
          confirmed.title !== command.input.title ||
          (await payloadDigest(receipt.execution_context.context)) !==
            (await payloadDigest(context))
        )
          throw new Error('Unconfirmed Namespace creation')
        return confirmed
      }
      await dispatch.prepare(
        command.input,
        command.input.idempotency_key!,
        command.input.primary_agent_id!,
        controlRecoveryService(apiBaseUrl, ssoConfig.issuer),
        () =>
          Boolean(useAuthStore.getState().token) &&
          useAuthStore.getState().permissions.includes('sessions:write_own'),
      )
      return createSession(command.input)
    },
    onSuccess: async (session, command) => {
      const matching = Boolean(
        session?.id &&
        session.user_id === useAuthStore.getState().userId &&
        session.primary_agent_id === command.input.primary_agent_id,
      )
      if (!matching || (!command.context && !dispatch.finish(matching))) {
        setUncertain(true)
        return
      }
      if (
        command.context &&
        (!command.marker ||
          !clearDispatch(`create:namespace:${command.input.primary_agent_id}`, command.marker))
      ) {
        setUncertain(true)
        return
      }
      setNamespaceHeld(false)
      setUncertain(false)
      await client.invalidateQueries({ queryKey: ['chats-directory'] })
      onOpenChange(false)
      setTitle('')
      setKey(crypto.randomUUID())
      navigate(
        contextPath(`/chats/${session.id}?returnTo=${encodeURIComponent(returnTo)}`, contextParams),
      )
    },
    onError: (error, command) => {
      if (!command.context) dispatch.fail(error)
      if (
        !(error instanceof ControlPreparationError) &&
        (!(error instanceof ApiError) ||
          error.status < 400 ||
          error.status === 408 ||
          error.status >= 500)
      )
        setUncertain(true)
    },
  })
  const held =
    mutation.isPending || uncertain || (namespaceCreation ? namespaceHeld : dispatch.restored)
  if (!agent && !held) return null
  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        if (!mutation.isPending) onOpenChange(value)
      }}
    >
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {t('chats.new')} · {held ? mutation.variables?.agentName : agent?.display_name}
          </DialogTitle>
        </DialogHeader>
        <form
          className="grid gap-4"
          aria-busy={mutation.isPending}
          onSubmit={(event) => {
            event.preventDefault()
            if (
              mutation.isPending ||
              (!namespaceCreation && dispatch.restored) ||
              (namespaceCreation && namespaceHeld && !mutation.variables)
            )
              return
            if (uncertain && mutation.variables) mutation.mutate(mutation.variables)
            else if (
              title.trim() &&
              agent?.product_role === 'executor' &&
              agent.status !== 'archived'
            )
              mutation.mutate({
                agentName: agent.display_name,
                input: {
                  primary_agent_id: agent.id,
                  title: title.trim(),
                  leader_agent_id: null,
                  idempotency_key: key,
                },
              })
          }}
        >
          <Label htmlFor="chat-title">{t('sessions.sessionTitle')}</Label>
          <Input
            id="chat-title"
            autoFocus
            value={title}
            required
            maxLength={200}
            disabled={held}
            onChange={(event) => {
              setTitle(event.target.value)
              setKey(crypto.randomUUID())
              mutation.reset()
            }}
          />
          {mutation.isError && !uncertain ? (
            <ErrorState message={t('sessions.createError')} />
          ) : null}
          {uncertain ? <ErrorState message={t('chats.creationUnknown')} /> : null}
          {!namespaceCreation && dispatch.restored && (
            <p role="status">
              Создание исходного чата требует сверки после перезагрузки. Новый чат заблокирован;
              приватный текст не сохранён.
            </p>
          )}
          <Button
            type="submit"
            disabled={
              mutation.isPending ||
              (!namespaceCreation && dispatch.restored) ||
              !title.trim() ||
              (namespaceCreation && namespaceHeld && !mutation.variables)
            }
          >
            <Plus className="h-4 w-4" />
            {mutation.isPending ? t('sessions.creating') : t('sessions.create')}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
