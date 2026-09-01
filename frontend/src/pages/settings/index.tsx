import { useQuery } from '@tanstack/react-query'
import { listUsers } from '@/api/auth'
import { listRuntimeTemplates } from '@/api/fleet'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { JsonBlock, PageHeader } from '../common'

export function SettingsPage() {
  const templates = useQuery({ queryKey: ['runtime-templates'], queryFn: listRuntimeTemplates })
  const users = useQuery({ queryKey: ['users'], queryFn: listUsers })

  return (
    <>
      <PageHeader title="Settings" description="Control-plane defaults and administrative state." />
      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Runtime source policy</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3 text-sm text-text-secondary">
            <p>Hermes source is configured with `FLEET_CONTROL_FLEET__HERMES_SOURCE`.</p>
            <p>Java Agent source is configured with `FLEET_CONTROL_FLEET__JAVA_AGENT_SOURCE`.</p>
            <p>Agent folders are allocated under `FLEET_CONTROL_FLEET__AGENTS_ROOT`.</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Runtime templates</CardTitle>
          </CardHeader>
          <CardContent>
            <JsonBlock value={templates.data ?? []} />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Users</CardTitle>
          </CardHeader>
          <CardContent>
            <JsonBlock value={users.data ?? []} />
          </CardContent>
        </Card>
      </div>
    </>
  )
}
