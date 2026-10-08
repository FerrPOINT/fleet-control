import { apiBaseUrl } from '@/api/client'
import { useEffect, useRef, useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Bot, MessageSquare, Plus, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button, Dialog, DialogContent, DialogHeader, DialogTitle, Input, Label } from '@sdlc/ui/ui'
import { getSession, createSession, listAgentDirectory, listSessions } from '@/api/fleet'
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

function contextPath(path: string, params: URLSearchParams) {
  const target = new URL(path, 'https://relative.invalid')
  const context = target.searchParams
  for (const key of ['registry_instance_id', 'namespace_id', 'tracker_instance_id', 'task_id'])
    for (const value of params.getAll(key)) context.append(key, value)
  return target.pathname + target.search + target.hash
}

export function groupChats(agents: AgentDirectoryItem[], sessions: AgentSession[]) {
  return agents.map((agent) => ({
    agent,
    sessions: sessions
      .filter((session) => session.primary_agent_id === agent.id)
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at) || a.id.localeCompare(b.id)),
  }))
}

export function ChatsPage() {
  const { t } = useTranslation()
  const [params, setParams] = useSearchParams()
  const search = params.get('q') ?? ''
  const auth = useAuthStore()
  const [createOpen, setCreateOpen] = useState(false)
  const filter = useSessionUserFilter()
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const selectedId = params.get('agent') ?? agents.data?.[0]?.id
  const selectedUsers =
    params.has('users') && filter.isSystemAdmin
      ? params.get('users')!.split(',').filter(Boolean)
      : filter.selectedUserIds
  const sessions = useQuery({
    queryKey: ['sessions', 'chats', selectedId, selectedUsers],
    queryFn: () => listSessions(selectedId, selectedUsers),
    enabled: Boolean(selectedId),
    retry: false,
  })
  const groups = groupChats(agents.data ?? [], sessions.data ?? [])
  const selected = groups.find((group) => group.agent.id === selectedId)
  const urlFilter = {
    ...filter,
    selectedUserIds: selectedUsers,
    selectedUsers: selectedUsers
      .map((id) => filter.allUsers.find((user) => user.id === id))
      .filter((user): user is (typeof filter.allUsers)[number] => Boolean(user)),
    addUser: (id: string) => updateUsers([...new Set([...selectedUsers, id])]),
    removeUser: (id: string) => {
      if (filter.isSystemAdmin) updateUsers(selectedUsers.filter((value) => value !== id))
    },
  }
  function updateUsers(ids: string[]) {
    if (!filter.isSystemAdmin) return
    setParams((current) => {
      const next = new URLSearchParams(current)
      next.set('users', ids.join(','))
      return next
    })
  }
  const needle = search.trim().toLocaleLowerCase()
  const visibleSessions =
    selected?.sessions.filter((session) =>
      [session.title, session.task_key ?? '', session.user_display_name].some((value) =>
        value.toLocaleLowerCase().includes(needle),
      ),
    ) ?? []

  return (
    <>
      <PageHeader
        title={t('chats.title')}
        actions={
          selected?.agent.product_role === 'executor' &&
          selected.agent.status !== 'archived' &&
          auth.permissions.includes('sessions:write_own') ? (
            <Button
              onClick={() => setCreateOpen(true)}
              disabled={
                agents.isPending ||
                sessions.isPending ||
                agents.isFetching ||
                agents.isError ||
                sessions.isError ||
                auth.signingOut
              }
            >
              <Plus className="h-4 w-4" />
              {t('chats.new')}
            </Button>
          ) : null
        }
      />
      <SessionUserFilter filter={urlFilter} className="mb-4" />
      {agents.isError || sessions.isError ? (
        <div className="mb-4 space-y-2">
          <ErrorState message={t('chats.loadError')} />
          <Button
            variant="outline"
            onClick={() => {
              void agents.refetch()
              void sessions.refetch()
            }}
          >
            {t('sessions.retry')}
          </Button>
        </div>
      ) : null}
      {agents.isError || sessions.isError ? null : agents.isPending ||
        (Boolean(selectedId) && sessions.isPending) ? (
        <EmptyState title={t('chats.loading')} />
      ) : (
        <div className="grid min-w-0 gap-4 md:grid-cols-[260px_minmax(0,1fr)]">
          <nav
            aria-label={t('chats.agents')}
            className="min-w-0 border-b border-border md:border-b-0 md:border-r md:pr-4"
          >
            <ul className="space-y-1 pb-4">
              {groups.map(({ agent }) => (
                <li key={agent.id}>
                  <button
                    type="button"
                    aria-current={agent.id === selectedId ? 'true' : undefined}
                    onClick={() =>
                      setParams((current) => {
                        const next = new URLSearchParams(current)
                        next.set('agent', agent.id)
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
                  </button>
                </li>
              ))}
            </ul>
            {!groups.length && !agents.isError ? (
              <EmptyState title={t('sessions.noAgents')} />
            ) : null}
          </nav>
          <section className="min-w-0" aria-label={t('chats.sessions')}>
            {selected ? (
              <>
                <div className="mb-4 flex flex-wrap items-center gap-3 border-b border-border pb-4">
                  <h2 className="min-w-0 break-words text-base font-semibold text-text-primary">
                    {selected.agent.display_name}
                  </h2>
                  <StatusBadge value={selected.agent.status} />
                  <label className="relative ml-auto w-full min-w-0 sm:w-72">
                    <Search className="absolute left-3 top-3 h-4 w-4 text-text-muted" aria-hidden />
                    <Input
                      value={search}
                      onChange={(event) =>
                        setParams(
                          (current) => {
                            const next = new URLSearchParams(current)
                            if (event.target.value) next.set('q', event.target.value)
                            else next.delete('q')
                            return next
                          },
                          { replace: true },
                        )
                      }
                      className="h-10 pl-9"
                      aria-label={t('chats.search')}
                      placeholder={t('chats.search')}
                    />
                  </label>
                </div>
                <p className="mb-3 text-xs text-text-muted">
                  {t('chatCore.loaded', { count: selected.sessions.length })}
                </p>
                <ul className="divide-y divide-border">
                  {visibleSessions.map((session) => (
                    <li key={session.id}>
                      <Link
                        to={contextPath(
                          `/chats/${session.id}?backTo=${encodeURIComponent(`/chats?${params}`)}`,
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
                {!visibleSessions.length && !sessions.isError ? (
                  <EmptyState title={t('chats.empty')} />
                ) : null}
              </>
            ) : !agents.isError ? (
              <EmptyState title={t('chats.selectAgent')} />
            ) : null}
          </section>
        </div>
      )}
      {selected ? (
        <CreatePrivateChat
          key={selected.agent.id}
          backTo={`/chats?${params}`}
          agent={selected.agent}
          open={createOpen}
          onOpenChange={setCreateOpen}
        />
      ) : null}
    </>
  )
}

function CreatePrivateChat({
  backTo,
  agent,
  open,
  onOpenChange,
}: {
  backTo: string
  agent: AgentDirectoryItem
  open: boolean
  onOpenChange: (value: boolean) => void
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [contextParams] = useSearchParams()
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
        if (command.marker.actor !== scope.userId) throw new Error('Authentication changed')
        let created: AgentSession
        try {
          created = await createSession({
            primary_agent_id: agent.id,
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
          confirmed.primary_agent_id !== agent.id ||
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
      await client.invalidateQueries({ queryKey: ['sessions'] })
      onOpenChange(false)
      setTitle('')
      setKey(crypto.randomUUID())
      if (!live.current || useAuthStore.getState().signingOut) return
      navigate(
        contextPath(`/chats/${session.id}?backTo=${encodeURIComponent(backTo)}`, contextParams),
      )
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
