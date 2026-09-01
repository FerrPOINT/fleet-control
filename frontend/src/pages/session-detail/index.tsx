import { FormEvent, useState } from 'react'
import { Link, useParams } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowRightLeft } from 'lucide-react'
import { getSession, handoffSession, listAgents } from '@/api/fleet'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { ErrorState, JsonBlock, PageHeader, StatusBadge, formatDate } from '../common'

export function SessionDetailPage() {
  const { sessionId } = useParams()
  const queryClient = useQueryClient()
  const session = useQuery({
    queryKey: ['session', sessionId],
    queryFn: () => getSession(sessionId!),
    enabled: Boolean(sessionId),
  })
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const [targetAgentId, setTargetAgentId] = useState('')
  const mutation = useMutation({
    mutationFn: () => handoffSession(sessionId!, { target_agent_id: targetAgentId }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['session', sessionId] }),
        queryClient.invalidateQueries({ queryKey: ['sessions'] }),
      ])
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (targetAgentId) mutation.mutate()
  }

  if (!sessionId) return <ErrorState message="Session id is missing" />
  if (session.isError) return <ErrorState message={session.error.message} />

  return (
    <>
      <PageHeader
        title={session.data?.title ?? 'Session'}
        description="Task chat metadata and cross-agent handoff."
      />
      <div className="grid gap-4 xl:grid-cols-[1fr_360px]">
        <Card>
          <CardHeader>
            <CardTitle>Session state</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {session.data ? (
              <>
                <div className="flex flex-wrap items-center gap-2">
                  <StatusBadge value={session.data.state} />
                  <span className="text-sm text-text-muted">
                    {session.data.task_key ?? 'No task key'}
                  </span>
                </div>
                <div className="flex min-w-0 items-center gap-3 rounded-md border border-border bg-background p-3">
                  <UserAvatar
                    name={session.data.user_display_name}
                    userId={session.data.user_id}
                    size="md"
                  />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-text-primary">
                      {session.data.user_display_name}
                    </p>
                    <p className="truncate text-xs text-text-muted">{session.data.user_email}</p>
                  </div>
                </div>
                <dl className="grid gap-3 text-sm md:grid-cols-2">
                  <Field label="Agent" value={session.data.agent_name} />
                  <Field label="Namespace" value={session.data.namespace_id ?? 'unbound'} />
                  <Field
                    label="External session"
                    value={session.data.external_session_id ?? 'none'}
                  />
                  <Field label="Updated" value={formatDate(session.data.updated_at)} />
                </dl>
                <JsonBlock value={session.data} />
              </>
            ) : null}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Handoff</CardTitle>
          </CardHeader>
          <CardContent>
            <form className="grid gap-3" onSubmit={submit}>
              <select
                value={targetAgentId}
                onChange={(event) => setTargetAgentId(event.target.value)}
                className="h-9 rounded-md border border-border bg-background px-3 text-sm"
              >
                <option value="">Select target agent</option>
                {agents.data
                  ?.filter((agent) => agent.id !== session.data?.agent_id)
                  .map((agent) => (
                    <option key={agent.id} value={agent.id}>
                      {agent.name} - {agent.display_name}
                    </option>
                  ))}
              </select>
              {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
              <Button type="submit" disabled={!targetAgentId || mutation.isPending}>
                <ArrowRightLeft className="h-4 w-4" />
                Handoff session
              </Button>
              {session.data ? (
                <Button asChild variant="outline">
                  <Link to={`/agents/${session.data.agent_id}/sessions`}>Open agent sessions</Link>
                </Button>
              ) : null}
            </form>
          </CardContent>
        </Card>
      </div>
    </>
  )
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs text-text-muted">{label}</dt>
      <dd className="break-words font-medium text-text-primary">{value}</dd>
    </div>
  )
}
