import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { Plus, Search } from 'lucide-react'
import { listExecutors, listSessions } from '@/api/fleet'
import type { Agent, AgentSession } from '@/api/types'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button, Input } from '@sdlc/ui/ui'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { AgentIdentity, EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

const PAGE_SIZE = 25

export function ExecutorsPage() {
  const { t } = useTranslation()
  const executors = useQuery({ queryKey: ['executors'], queryFn: listExecutors })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'executors', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
  })
  const [search, setSearch] = useState('')
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE)
  const sessionsByExecutor = useMemo(
    () => groupSessionsByAgent(sessions.data ?? []),
    [sessions.data],
  )
  const matchingExecutors = useMemo(() => {
    const query = search.trim().toLocaleLowerCase()
    return (executors.data ?? [])
      .filter((executor) =>
        `${executor.display_name} ${executor.name} ${executor.role}`
          .toLocaleLowerCase()
          .includes(query),
      )
      .sort((a, b) => a.display_name.localeCompare(b.display_name))
  }, [executors.data, search])

  return (
    <>
      <PageHeader
        title={t('executors.title')}
        description={t('executors.description')}
        actions={
          <Button asChild>
            <Link to="/executors/new">
              <Plus className="h-4 w-4" />
              {t('executors.new')}
            </Link>
          </Button>
        }
      />
      <SessionUserFilter filter={userFilter} className="mb-4" />
      <div className="mb-3 flex flex-wrap items-end justify-between gap-3">
        <label className="relative block w-full max-w-sm">
          <span className="sr-only">{t('executors.search')}</span>
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-muted" />
          <Input
            value={search}
            onChange={(event) => {
              setSearch(event.target.value)
              setVisibleCount(PAGE_SIZE)
            }}
            placeholder={t('executors.search')}
            className="h-10 pl-9"
          />
        </label>
        {!executors.isLoading && !executors.isError ? (
          <p className="text-sm text-text-muted">
            {t('executors.count', {
              shown: Math.min(visibleCount, matchingExecutors.length),
              total: matchingExecutors.length,
            })}
          </p>
        ) : null}
      </div>
      {executors.isError ? (
        <LoadError message={t('executors.loadError')} retry={() => void executors.refetch()} />
      ) : executors.isLoading ? (
        <EmptyState title={t('executors.loading')} />
      ) : !executors.data?.length ? (
        <EmptyState title={t('executors.empty')} />
      ) : !matchingExecutors.length ? (
        <EmptyState title={t('executors.noMatches')} />
      ) : (
        <>
          {sessions.isError ? (
            <div className="mb-3">
              <LoadError
                message={t('executors.sessionsError')}
                retry={() => void sessions.refetch()}
              />
            </div>
          ) : null}
          <ul className="divide-y divide-border border-y border-border">
            {matchingExecutors.slice(0, visibleCount).map((executor) => (
              <ExecutorRow
                key={executor.id}
                executor={executor}
                sessions={sessionsByExecutor.get(executor.id) ?? []}
                sessionsLoading={sessions.isLoading}
                sessionsFailed={sessions.isError}
              />
            ))}
          </ul>
          {visibleCount < matchingExecutors.length ? (
            <Button
              type="button"
              variant="outline"
              className="mt-4"
              onClick={() => setVisibleCount((current) => current + PAGE_SIZE)}
            >
              {t('executors.showMore')}
            </Button>
          ) : null}
        </>
      )}
    </>
  )
}

function ExecutorRow({
  executor,
  sessions,
  sessionsLoading,
  sessionsFailed,
}: {
  executor: Agent
  sessions: AgentSession[]
  sessionsLoading: boolean
  sessionsFailed: boolean
}) {
  const { t } = useTranslation()
  return (
    <li className="min-w-0 py-3">
      <div className="flex min-w-0 flex-wrap items-start justify-between gap-3">
        <AgentIdentity agent={executor} />
        <Button asChild variant="outline" size="sm">
          <Link to={`/executors/${executor.id}`}>{t('executors.open')}</Link>
        </Button>
      </div>
      {sessionsFailed ? null : sessionsLoading ? (
        <p className="mt-2 text-xs text-text-muted">{t('executors.loadingSessions')}</p>
      ) : sessions.length ? (
        <details className="mt-2 group">
          <summary className="w-fit cursor-pointer text-sm text-accent underline-offset-2 hover:underline focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus">
            {t('executors.sessionsCount', { count: sessions.length })}
          </summary>
          <div className="mt-2 grid gap-1 sm:grid-cols-2 xl:grid-cols-3">
            {sessions.map((session) => (
              <SessionPreview key={session.id} session={session} />
            ))}
          </div>
        </details>
      ) : (
        <p className="mt-2 text-xs text-text-muted">{t('executors.noSessions')}</p>
      )}
    </li>
  )
}

function LoadError({ message, retry }: { message: string; retry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-wrap items-center gap-2">
      <ErrorState message={message} />
      <Button type="button" variant="outline" size="sm" onClick={retry}>
        {t('executors.retry')}
      </Button>
    </div>
  )
}

function groupSessionsByAgent(sessions: AgentSession[]) {
  const grouped = new Map<string, AgentSession[]>()
  for (const session of sessions) {
    grouped.set(session.primary_agent_id, [
      ...(grouped.get(session.primary_agent_id) ?? []),
      session,
    ])
  }
  return grouped
}

function SessionPreview({ session }: { session: AgentSession }) {
  const { t } = useTranslation()
  return (
    <Link
      to={`/sessions/${session.id}`}
      className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 hover:bg-surface-raised focus-visible:outline focus-visible:outline-2 focus-visible:outline-focus"
    >
      <UserAvatar name={session.user_display_name} userId={session.user_id} />
      <span className="min-w-0 flex-1">
        <span className="flex flex-wrap items-center gap-2">
          <span className="truncate text-sm font-medium text-text-primary">{session.title}</span>
          <StatusBadge value={session.visibility} />
        </span>
        <span className="block truncate text-xs text-text-muted">
          {session.user_display_name} - {session.leader_agent_name ?? t('executors.private')}
        </span>
      </span>
    </Link>
  )
}
