import { useQuery } from '@tanstack/react-query'
import { Bot, MessageSquare } from 'lucide-react'
import { Link } from 'react-router'
import { useTranslation } from 'react-i18next'
import { Button } from '@sdlc/ui/ui'
import { listAgentDirectory } from '@/api/fleet'
import { sdlcRoleLabel } from '@/shared/sdlc-roles'
import { EmptyState, ErrorState, PageHeader, StatusBadge } from '../common'

export function AgentDirectoryPage() {
  const { t } = useTranslation()
  const agents = useQuery({ queryKey: ['agent-directory'], queryFn: listAgentDirectory })
  return (
    <>
      <PageHeader title={t('navigation.agents')} />
      {agents.isPending ? <EmptyState title={t('common.loading')} /> : null}
      {agents.isError ? (
        <div className="space-y-3">
          <ErrorState message={t('chats.loadError')} />
          <Button variant="outline" onClick={() => void agents.refetch()}>
            {t('sessions.retry')}
          </Button>
        </div>
      ) : null}
      {!agents.isError && agents.data?.length === 0 ? (
        <EmptyState title={t('sessions.noAgents')} />
      ) : null}
      <ul className="divide-y divide-border">
        {agents.data?.map((agent) => (
          <li key={agent.id} className="flex min-w-0 flex-wrap items-center gap-3 py-4">
            <Bot className="h-5 w-5 shrink-0 text-text-muted" />
            <div className="min-w-0 flex-1">
              <h2 className="break-words text-sm font-medium">{agent.display_name}</h2>
              <p className="text-xs text-text-muted">
                {agent.name} · {sdlcRoleLabel(agent.sdlc_role)} · {agent.kind}
              </p>
            </div>
            <StatusBadge value={agent.status} />
            <Button variant="outline" size="icon" asChild>
              <Link
                to={`/chats?agent=${agent.id}`}
                aria-label={`${t('chats.title')}: ${agent.display_name}`}
                title={t('chats.title')}
              >
                <MessageSquare className="h-4 w-4" />
              </Link>
            </Button>
          </li>
        ))}
      </ul>
    </>
  )
}
