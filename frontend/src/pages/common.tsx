import type { ReactNode } from 'react'
import { AlertCircle } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { cn } from '@/shared/lib/utils'
import type { Agent, AgentKind, AgentStatus, SessionState, SkillState } from '@/api/types'

export function PageHeader({
  title,
  description,
  actions,
}: {
  title: string
  description?: string
  actions?: ReactNode
}) {
  return (
    <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
      <div className="min-w-0">
        <h1 className="text-2xl font-semibold text-text-primary">{title}</h1>
        {description ? (
          <p className="mt-1 max-w-3xl text-sm text-text-muted">{description}</p>
        ) : null}
      </div>
      {actions ? <div className="flex flex-wrap gap-2">{actions}</div> : null}
    </div>
  )
}

export function StatCard({
  label,
  value,
  tone,
}: {
  label: string
  value: ReactNode
  tone?: string
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-xs uppercase">{label}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className={cn('text-3xl font-semibold text-text-primary', tone)}>{value}</div>
      </CardContent>
    </Card>
  )
}

export function StatusBadge({
  value,
}: {
  value: AgentStatus | SkillState | SessionState | string | null | undefined
}) {
  const normalized = value ?? 'unknown'
  return (
    <span
      className={cn(
        'inline-flex min-h-6 items-center rounded-md border px-2 text-xs font-medium',
        badgeTone(normalized),
      )}
    >
      {labelize(normalized)}
    </span>
  )
}

export function KindBadge({ kind }: { kind: AgentKind }) {
  return (
    <span className="inline-flex min-h-6 items-center rounded-md border border-border-strong bg-surface-raised px-2 text-xs font-medium text-text-secondary">
      {kind === 'java_agent' ? 'Java Agent' : 'Hermes'}
    </span>
  )
}

export function ProductRoleBadge({ value }: { value: Agent['product_role'] }) {
  return (
    <span className="inline-flex min-h-6 items-center rounded-md border border-border-strong bg-surface-raised px-2 text-xs font-medium text-text-secondary">
      {value === 'leader' ? 'Leader' : 'Executor'}
    </span>
  )
}

export function ErrorState({ message }: { message: string }) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-danger/40 bg-danger/10 px-3 py-2 text-sm text-danger">
      <AlertCircle className="h-4 w-4" />
      {message}
    </div>
  )
}

export function AccessDeniedState() {
  return (
    <div className="rounded-md border border-warning/40 bg-warning/10 p-5">
      <div className="flex items-start gap-3">
        <AlertCircle className="mt-0.5 h-5 w-5 text-warning" />
        <div>
          <h2 className="text-base font-semibold text-text-primary">Access denied</h2>
          <p className="mt-1 text-sm text-text-muted">
            Your role can use private sessions, but this fleet administration view requires an
            operator or admin permission.
          </p>
        </div>
      </div>
    </div>
  )
}

export function NotFoundState() {
  return (
    <div className="rounded-md border border-border bg-surface p-5">
      <h2 className="text-base font-semibold text-text-primary">Page not found</h2>
      <p className="mt-1 text-sm text-text-muted">
        This route is not part of the Fleet Control application map.
      </p>
    </div>
  )
}

export function EmptyState({ title }: { title: string }) {
  return (
    <div className="rounded-md border border-dashed border-border p-6 text-center text-sm text-text-muted">
      {title}
    </div>
  )
}

export function JsonBlock({ value }: { value: unknown }) {
  return (
    <pre className="max-h-80 w-full max-w-full overflow-auto rounded-md border border-border bg-background p-3 text-xs text-text-secondary">
      {JSON.stringify(value, null, 2)}
    </pre>
  )
}

export function AgentIdentity({ agent }: { agent: Agent }) {
  return (
    <div className="min-w-0">
      <div className="flex flex-wrap items-center gap-2">
        <h2 className="truncate text-base font-semibold text-text-primary">{agent.display_name}</h2>
        <KindBadge kind={agent.kind} />
        <ProductRoleBadge value={agent.product_role} />
        <StatusBadge value={agent.status} />
      </div>
      <p className="mt-1 text-xs text-text-muted">
        {agent.name} - profile {labelize(agent.role)} - namespace {agent.namespace_id ?? 'unbound'}
      </p>
    </div>
  )
}

export function formatDate(value: string | null | undefined) {
  if (!value) return 'never'
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value))
}

export function labelize(value: string) {
  return value.replaceAll('_', ' ')
}

function badgeTone(value: string) {
  switch (value) {
    case 'running':
    case 'enabled':
    case 'done':
    case 'completed':
    case 'dispatched':
      return 'border-success/40 bg-success/10 text-success'
    case 'failed':
    case 'missing':
    case 'blocked':
      return 'border-danger/40 bg-danger/10 text-danger'
    case 'starting':
    case 'provisioning':
    case 'degraded':
    case 'stopping':
    case 'handoff_requested':
    case 'dirty':
    case 'waiting':
      return 'border-warning/40 bg-warning/10 text-warning'
    default:
      return 'border-border-strong bg-surface-raised text-text-muted'
  }
}
