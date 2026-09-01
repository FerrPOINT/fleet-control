import { FormEvent, useState } from 'react'
import { Link } from 'react-router'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { MessageSquarePlus } from 'lucide-react'
import { createSession, listAgents, listSessions } from '@/api/fleet'
import { SessionUserFilter, useSessionUserFilter } from '@/shared/session-user-filter'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
import { UserAvatar } from '@/shared/ui/user-avatar'
import { EmptyState, ErrorState, PageHeader, StatusBadge, formatDate } from '../common'

export function SessionsPage() {
  const queryClient = useQueryClient()
  const agents = useQuery({ queryKey: ['agents'], queryFn: listAgents })
  const userFilter = useSessionUserFilter()
  const sessions = useQuery({
    queryKey: ['sessions', 'list', userFilter.selectedUserIds],
    queryFn: () => listSessions(undefined, userFilter.selectedUserIds),
  })
  const [agentId, setAgentId] = useState('')
  const [title, setTitle] = useState('New task session')
  const [taskKey, setTaskKey] = useState('')

  const mutation = useMutation({
    mutationFn: () =>
      createSession({
        agent_id: agentId || agents.data?.[0]?.id || '',
        title,
        task_key: taskKey || null,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['sessions'] })
      setTaskKey('')
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    mutation.mutate()
  }

  return (
    <>
      <PageHeader
        title="Sessions"
        description="Every chat is tracked as a task session and can move between agents."
      />
      <div className="grid gap-4 xl:grid-cols-[380px_1fr]">
        <Card>
          <CardHeader>
            <CardTitle>Create session</CardTitle>
          </CardHeader>
          <CardContent>
            <form className="grid gap-3" onSubmit={submit}>
              <div className="grid gap-2">
                <Label htmlFor="session-agent">Agent</Label>
                <select
                  id="session-agent"
                  value={agentId}
                  onChange={(event) => setAgentId(event.target.value)}
                  className="h-9 rounded-md border border-border bg-background px-3 text-sm"
                >
                  {agents.data?.map((agent) => (
                    <option key={agent.id} value={agent.id}>
                      {agent.name} - {agent.display_name}
                    </option>
                  ))}
                </select>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="session-title">Title</Label>
                <Input
                  id="session-title"
                  value={title}
                  onChange={(event) => setTitle(event.target.value)}
                />
              </div>
              <div className="grid gap-2">
                <Label htmlFor="task-key">Task key</Label>
                <Input
                  id="task-key"
                  value={taskKey}
                  onChange={(event) => setTaskKey(event.target.value)}
                  placeholder="CARD-123"
                />
              </div>
              {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
              <Button type="submit" disabled={mutation.isPending || !agents.data?.length}>
                <MessageSquarePlus className="h-4 w-4" />
                Create session
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Task sessions</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <SessionUserFilter filter={userFilter} className="mb-3" />
            {sessions.data?.length ? (
              sessions.data.map((session) => (
                <Link
                  key={session.id}
                  to={`/sessions/${session.id}`}
                  className="block rounded-md border border-border p-3 hover:bg-surface-raised"
                >
                  <div className="flex min-w-0 items-start gap-3">
                    <UserAvatar
                      name={session.user_display_name}
                      userId={session.user_id}
                      size="md"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <p className="font-medium text-text-primary">{session.title}</p>
                        <StatusBadge value={session.state} />
                        {session.task_key ? (
                          <span className="text-xs text-text-muted">{session.task_key}</span>
                        ) : null}
                      </div>
                      <p className="mt-1 text-xs text-text-muted">
                        {session.user_display_name} - {session.agent_name} - namespace{' '}
                        {session.namespace_id ?? 'unbound'} - {formatDate(session.updated_at)}
                      </p>
                    </div>
                  </div>
                </Link>
              ))
            ) : (
              <EmptyState title={sessions.isLoading ? 'Loading sessions...' : 'No sessions yet'} />
            )}
          </CardContent>
        </Card>
      </div>
    </>
  )
}
