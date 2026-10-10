import { useEffect, useRef, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, ArrowRight, Bot, MessageSquare, Plus, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button, Dialog, DialogContent, DialogHeader, DialogTitle, Input, Label } from '@sdlc/ui/ui'
import { createSession, getSession, listAgentDirectory } from '@/api/fleet'
import { apiBaseUrl } from '@/api/client'
import { getChatsDirectory } from '@/api/chats-directory'
import type { AgentDirectoryItem, AgentSession } from '@/api/types'
import { useSessionUserFilter, SessionUserFilter } from '@/shared/session-user-filter'
import { isCurrentAuth, ssoConfig, useAuthStore } from '@/shared/auth/store'
import {
  clearDispatch,
  commandService,
  dispatchHeld,
  markDispatch,
  payloadDigest,
  unknownOutcome,
  type DispatchMarker,
} from '../chat-detail/core'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { sdlcRoleLabel } from '@/shared/sdlc-roles'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

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
                        to={`/chats/${session.id}?backTo=${encodeURIComponent(returnTo)}`}
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
      {selected ? (
        <CreatePrivateChat
          key={selected.agent.id}
          agent={selected.agent}
          open={createOpen}
          onOpenChange={setCreateOpen}
          returnTo={returnTo}
        />
      ) : null}
    </>
  )
}

function CreatePrivateChat({
  returnTo,
  agent,
  open,
  onOpenChange,
}: {
  returnTo: string
  agent: AgentDirectoryItem
  open: boolean
  onOpenChange: (value: boolean) => void
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [title, setTitle] = useState('')
  const [key, setKey] = useState(() => crypto.randomUUID())
  const auth = useAuthStore()
  const scopeId = `create:${agent.id}`
  const [held, setHeld] = useState(() => dispatchHeld(scopeId))
  const original = useRef<{ title: string; marker: DispatchMarker; uncertain: boolean } | null>(
    null,
  )
  const live = useRef(true)
  const dispatching = useRef(false)
  useEffect(() => {
    live.current = true
    return () => {
      live.current = false
    }
  }, [])
  const mutation = useMutation({
    mutationFn: async () => {
      if (dispatching.current) throw new Error('Dispatch pending')
      dispatching.current = true
      const scope = useAuthStore.getState()
      try {
        const directory = await listAgentDirectory()
        const current = directory.find((entry) => entry.id === agent.id)
        if (
          !isCurrentAuth(scope) ||
          !scope.permissions.includes('sessions:write_own') ||
          current?.product_role !== 'executor' ||
          current.status === 'archived'
        )
          throw new Error('Read-only')
        if (!original.current) {
          if (dispatchHeld(scopeId)) throw new Error('Reconciliation required')
          const marker = {
            actor: scope.userId!,
            agent: agent.id,
            service: commandService(apiBaseUrl, ssoConfig.issuer),
            key,
            digest: await payloadDigest({
              title: title.trim(),
              primary_agent_id: agent.id,
              leader_agent_id: null,
            }),
          }
          if (!isCurrentAuth(scope) || !live.current) throw new Error('Authentication changed')
          markDispatch(scopeId, marker)
          original.current = { title: title.trim(), marker, uncertain: false }
          setHeld(true)
        }
        const command = original.current
        if (
          command.marker.actor !== scope.userId ||
          command.marker.agent !== agent.id ||
          command.marker.service !== commandService(apiBaseUrl, ssoConfig.issuer)
        )
          throw new Error('Authentication or command target changed')
        let created: AgentSession
        try {
          created = await createSession({
            primary_agent_id: command.marker.agent,
            title: command.title,
            leader_agent_id: null,
            idempotency_key: command.marker.key,
          })
        } catch (failure) {
          if (
            live.current &&
            isCurrentAuth(scope) &&
            !command.uncertain &&
            !unknownOutcome(failure) &&
            clearDispatch(scopeId, command.marker)
          ) {
            original.current = null
            setHeld(false)
          }
          if (unknownOutcome(failure)) command.uncertain = true
          throw failure
        }
        command.uncertain = true
        // Confirm the returned id against the protected session before releasing the key.
        const confirmed = await getSession(created.id)
        if (
          !isCurrentAuth(scope) ||
          !live.current ||
          confirmed.user_id !== scope.userId ||
          confirmed.primary_agent_id !== command.marker.agent ||
          confirmed.title !== command.title
        )
          throw new Error('Unconfirmed creation')
        return confirmed
      } finally {
        dispatching.current = false
      }
    },
    onError: () => {
      if (!live.current) return
      setHeld(dispatchHeld(scopeId))
    },
    onSuccess: async (session) => {
      if (
        !live.current ||
        !original.current ||
        original.current.marker.actor !== useAuthStore.getState().userId ||
        !clearDispatch(scopeId, original.current.marker)
      )
        return
      original.current = null
      setHeld(false)
      await client.invalidateQueries({ queryKey: ['chats-directory'] })
      onOpenChange(false)
      setTitle('')
      setKey(crypto.randomUUID())
      if (!live.current || useAuthStore.getState().signingOut) return
      navigate(`/chats/${session.id}?backTo=${encodeURIComponent(returnTo)}`)
    },
  })
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
            {t('chats.new')} · {agent.display_name}
          </DialogTitle>
        </DialogHeader>
        <form
          className="grid gap-4"
          aria-busy={mutation.isPending}
          onSubmit={(event) => {
            event.preventDefault()
            if (
              title.trim() &&
              !mutation.isPending &&
              auth.permissions.includes('sessions:write_own') &&
              !auth.signingOut &&
              (!held || original.current)
            )
              mutation.mutate()
          }}
        >
          <Label htmlFor="chat-title">{t('sessions.sessionTitle')}</Label>
          <Input
            id="chat-title"
            autoFocus
            value={title}
            required
            maxLength={200}
            disabled={mutation.isPending || held}
            onChange={(event) => {
              setTitle(event.target.value)
              setKey(crypto.randomUUID())
              mutation.reset()
            }}
          />
          {held ? (
            <p role="alert" className="text-sm text-text-secondary">
              {t('chatCore.createHeld')}
            </p>
          ) : null}
          {mutation.isError ? <ErrorState message={t('sessions.createError')} /> : null}
          <Button
            type="submit"
            disabled={
              mutation.isPending ||
              !title.trim() ||
              auth.signingOut ||
              !auth.permissions.includes('sessions:write_own') ||
              (held && !original.current)
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
