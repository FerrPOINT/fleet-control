import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, Play, RefreshCw } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@sdlc/ui/ui'
import {
  activateAgentConfigRevision,
  getAgentSdlcReadiness,
  listAgentConfigRevisions,
  validateAgentConfigRevision,
} from '@/api/fleet'
import { EmptyState, ErrorState, StatusBadge } from '../common'

export function ConfigRevisions({ agentId }: { agentId: string }) {
  const { t } = useTranslation()
  const client = useQueryClient()
  const revisions = useQuery({
    queryKey: ['agent-config-revisions', agentId],
    queryFn: () => listAgentConfigRevisions(agentId),
    refetchInterval: (query) =>
      query.state.data?.some((revision) => revision.state === 'activating') ? 1000 : false,
  })
  const readiness = useQuery({
    queryKey: ['agent-readiness', agentId],
    queryFn: () => getAgentSdlcReadiness(agentId),
    refetchInterval: 10_000,
  })
  const operation = useMutation({
    mutationFn: ({ revision, action }: { revision: number; action: 'validate' | 'activate' }) =>
      action === 'validate'
        ? validateAgentConfigRevision(agentId, revision)
        : activateAgentConfigRevision(agentId, revision),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['agent-config-revisions', agentId] })
      await client.invalidateQueries({ queryKey: ['agent-readiness', agentId] })
    },
  })
  return (
    <section
      className="mt-6 min-w-0 border-y border-border py-4"
      aria-labelledby="config-revisions-title"
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h2 id="config-revisions-title" className="text-base font-semibold text-text-primary">
          {t('configRevisions.title')}
        </h2>
        <Button
          size="icon"
          variant="ghost"
          aria-label={t('sessions.retry')}
          title={t('sessions.retry')}
          onClick={() => {
            void revisions.refetch()
            void readiness.refetch()
          }}
        >
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>
      {readiness.data ? (
        <div className="mb-4 flex flex-wrap items-center gap-2 text-sm">
          <span>{t('configRevisions.runtime')}</span>
          <StatusBadge value={readiness.data.runtime_healthy ? 'running' : 'stopped'} />
          <span className="ml-2">SDLC</span>
          <StatusBadge value={readiness.data.ready_for_sdlc ? 'ready' : 'blocked'} />
          <span className="text-xs text-text-muted">
            {t('configRevisions.effective')}: {readiness.data.effective_revision ?? '-'}
          </span>
        </div>
      ) : null}
      {readiness.data?.blockers.length ? (
        <ul className="mb-4 list-inside list-disc text-xs text-text-muted">
          {readiness.data.blockers.map((blocker) => (
            <li key={blocker}>
              {t(`configRevisions.blockers.${blocker}`, {
                defaultValue: blocker.replaceAll('_', ' '),
              })}
            </li>
          ))}
        </ul>
      ) : null}
      {revisions.isError || readiness.isError ? (
        <ErrorState message={t('configRevisions.loadError')} />
      ) : null}
      {operation.isError ? <ErrorState message={operation.error.message} /> : null}
      {revisions.isPending ? <EmptyState title={t('configRevisions.loading')} /> : null}
      {revisions.data?.length === 0 ? <EmptyState title={t('configRevisions.empty')} /> : null}
      <ul className="divide-y divide-border">
        {revisions.data?.map((revision) => (
          <li key={revision.revision} className="py-3">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm font-medium">#{revision.revision}</span>
              <StatusBadge value={revision.state} />
              {revision.is_effective ? (
                <span className="text-xs text-success">{t('configRevisions.effective')}</span>
              ) : null}
              {revision.is_desired ? (
                <span className="text-xs text-text-muted">{t('configRevisions.desired')}</span>
              ) : null}
              {revision.draining && revision.state === 'activating' ? (
                <span className="text-xs text-warning">{t('configRevisions.draining')}</span>
              ) : null}
              <div className="ml-auto flex gap-2">
                {revision.is_desired &&
                ['draft', 'validated', 'failed'].includes(revision.state) ? (
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={operation.isPending || revision.draining}
                    onClick={() =>
                      operation.mutate({ revision: revision.revision, action: 'validate' })
                    }
                  >
                    <Check className="h-4 w-4" />
                    {t('configRevisions.validate')}
                  </Button>
                ) : null}
                {revision.is_desired && revision.state === 'validated' ? (
                  <Button
                    size="sm"
                    disabled={operation.isPending || revision.draining}
                    onClick={() =>
                      operation.mutate({ revision: revision.revision, action: 'activate' })
                    }
                  >
                    <Play className="h-4 w-4" />
                    {t('configRevisions.activate')}
                  </Button>
                ) : null}
              </div>
            </div>
            {revision.validation_errors.length ? (
              <ul className="mt-2 list-inside list-disc break-words text-xs text-danger">
                {revision.validation_errors.map((error) => (
                  <li key={error}>{error}</li>
                ))}
              </ul>
            ) : null}
            {revision.last_error ? (
              <p className="mt-2 break-words text-xs text-danger">{revision.last_error}</p>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  )
}
