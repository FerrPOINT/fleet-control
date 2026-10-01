import { useState } from 'react'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Bot, MessageSquare, Plus, Search } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button, Dialog, DialogContent, DialogHeader, DialogTitle, Input, Label } from '@sdlc/ui/ui'
import { createSession, listAgentDirectory, listSessions } from '@/api/fleet'
import type { AgentDirectoryItem, AgentSession } from '@/api/types'
import { useSessionUserFilter, SessionUserFilter } from '@/shared/session-user-filter'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { sdlcRoleLabel } from '@/shared/sdlc-roles'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

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
  const [createOpen, setCreateOpen] = useState(false)
  const userScope = params.get('users')
  const filter = useSessionUserFilter(
    userScope === null ? undefined : userScope === 'all' ? [] : userScope.split(','),
  )
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  const sessions = useQuery({
    queryKey: ['sessions', 'chats', filter.selectedUserIds],
    queryFn: () => listSessions(undefined, filter.selectedUserIds),
  })
  const groups = groupChats(agents.data ?? [], sessions.data ?? [])
  const selectedId =
    params.get('agent') ??
    groups.find((group) => group.sessions.length)?.agent.id ??
    groups[0]?.agent.id
  const selected = groups.find((group) => group.agent.id === selectedId)
  const returnParams = new URLSearchParams(params)
  if (selectedId) returnParams.set('agent', selectedId)
  returnParams.set('users', filter.selectedUserIds.join(',') || 'all')
  const returnTo = `/chats?${returnParams}`
  const setUsers = (ids: string[]) => {
    filter.setSelectedUserIds(ids)
    setParams(
      (current) => {
        const next = new URLSearchParams(current)
        next.set('users', ids.join(',') || 'all')
        return next
      },
      { replace: true },
    )
  }
  const persistedFilter = {
    ...filter,
    addUser: (userId: string) => {
      if (filter.isSystemAdmin) setUsers([...new Set([...filter.selectedUserIds, userId])])
    },
    removeUser: (userId: string) => {
      if (filter.isSystemAdmin) setUsers(filter.selectedUserIds.filter((id) => id !== userId))
    },
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
          selected?.agent.product_role === 'executor' && selected.agent.status !== 'archived' ? (
            <Button
              onClick={() => setCreateOpen(true)}
              disabled={agents.isPending || sessions.isPending}
            >
              <Plus className="h-4 w-4" />
              {t('chats.new')}
            </Button>
          ) : null
        }
      />
      <SessionUserFilter filter={persistedFilter} className="mb-4" />
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
      {agents.isPending || sessions.isPending ? (
        <EmptyState title={t('chats.loading')} />
      ) : (
        <div className="grid min-w-0 gap-4 md:grid-cols-[260px_minmax(0,1fr)]">
          <nav
            aria-label={t('chats.agents')}
            className="min-w-0 border-b border-border md:border-b-0 md:border-r md:pr-4"
          >
            <ul className="space-y-1 pb-4">
              {groups.map(({ agent, sessions: agentSessions }) => (
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
                    <span
                      className="text-xs tabular-nums text-text-muted"
                      aria-label={t('chats.count')}
                    >
                      {agentSessions.length}
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
                <ul className="divide-y divide-border">
                  {visibleSessions.map((session) => (
                    <li key={session.id}>
                      <Link
                        to={`/chats/${session.id}?returnTo=${encodeURIComponent(returnTo)}`}
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
  agent,
  open,
  onOpenChange,
  returnTo,
}: {
  agent: AgentDirectoryItem
  open: boolean
  onOpenChange: (value: boolean) => void
  returnTo: string
}) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const client = useQueryClient()
  const [title, setTitle] = useState('')
  const [key, setKey] = useState(() => crypto.randomUUID())
  const mutation = useMutation({
    mutationFn: () =>
      createSession({
        primary_agent_id: agent.id,
        title: title.trim(),
        leader_agent_id: null,
        idempotency_key: key,
      }),
    onSuccess: async (session) => {
      await client.invalidateQueries({ queryKey: ['sessions'] })
      onOpenChange(false)
      setTitle('')
      setKey(crypto.randomUUID())
      navigate(`/chats/${session.id}?returnTo=${encodeURIComponent(returnTo)}`)
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
            if (title.trim() && !mutation.isPending) mutation.mutate()
          }}
        >
          <Label htmlFor="chat-title">{t('sessions.sessionTitle')}</Label>
          <Input
            id="chat-title"
            autoFocus
            value={title}
            required
            maxLength={200}
            disabled={mutation.isPending}
            onChange={(event) => {
              setTitle(event.target.value)
              setKey(crypto.randomUUID())
              mutation.reset()
            }}
          />
          {mutation.isError ? <ErrorState message={t('sessions.createError')} /> : null}
          <Button type="submit" disabled={mutation.isPending || !title.trim()}>
            <Plus className="h-4 w-4" />
            {mutation.isPending ? t('sessions.creating') : t('sessions.create')}
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  )
}
