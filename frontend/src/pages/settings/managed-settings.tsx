import { useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { History, RotateCcw, Save } from 'lucide-react'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from '@sdlc/ui/ui'
import {
  applyManagedSettings,
  getManagedSettings,
  listManagedSettingsVersions,
  previewManagedSettings,
  rollbackManagedSettings,
} from '@/api/fleet'
import type {
  ManagedSettingsPreview,
  ManagedSettingsSnapshot,
  ManagedSettingsVersion,
} from '@/api/types'
import { ErrorState } from '../common'

export type ManagedSettingsTab =
  'runtime' | 'ports' | 'integrations' | 'auth' | 'retention' | 'history'

type PreviewContext =
  | { kind: 'apply'; preview: ManagedSettingsPreview }
  | { kind: 'rollback'; preview: ManagedSettingsPreview; target: ManagedSettingsVersion }

function cloneSnapshot(snapshot: ManagedSettingsSnapshot): ManagedSettingsSnapshot {
  return JSON.parse(JSON.stringify(snapshot)) as ManagedSettingsSnapshot
}

export function ManagedSettingsWorkspace({ tab }: { tab: ManagedSettingsTab }) {
  const { t } = useTranslation()
  const state = useQuery({ queryKey: ['settings', 'managed'], queryFn: getManagedSettings })
  const history = useQuery({
    queryKey: ['settings', 'managed', 'versions'],
    queryFn: () => listManagedSettingsVersions(20),
    enabled: tab === 'history',
  })
  const [draft, setDraft] = useState<ManagedSettingsSnapshot | null>(null)
  const [loadedVersion, setLoadedVersion] = useState<number | null | undefined>(undefined)
  const [previewContext, setPreviewContext] = useState<PreviewContext | null>(null)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)

  const dirty = useMemo(
    () =>
      Boolean(draft && state.data && JSON.stringify(draft) !== JSON.stringify(state.data.snapshot)),
    [draft, state.data],
  )

  useEffect(() => {
    if (!state.data) return
    if (loadedVersion === undefined || (!dirty && loadedVersion !== state.data.active_version)) {
      setDraft(cloneSnapshot(state.data.snapshot))
      setLoadedVersion(state.data.active_version)
    }
  }, [dirty, loadedVersion, state.data])

  const preview = useMutation({
    mutationFn: async (input: {
      snapshot: ManagedSettingsSnapshot
      target?: ManagedSettingsVersion
    }) => ({ result: await previewManagedSettings(input.snapshot), target: input.target }),
    onSuccess: ({ result, target }) => {
      if (result.changes.length === 0) {
        setNotice(t('settings.noChanges'))
        return
      }
      setNotice(null)
      setPreviewContext(
        target ? { kind: 'rollback', preview: result, target } : { kind: 'apply', preview: result },
      )
      setDialogOpen(true)
    },
  })
  const apply = useMutation({
    mutationFn: () => {
      if (!draft || !previewContext) throw new Error(t('settings.previewRequired'))
      return applyManagedSettings(draft, previewContext.preview.active_version)
    },
    onSuccess: (result) => {
      setDialogOpen(false)
      setNotice(result.restart_scheduled ? t('settings.restartScheduled') : t('settings.noChanges'))
    },
  })
  const rollback = useMutation({
    mutationFn: () => {
      if (!previewContext || previewContext.kind !== 'rollback') {
        throw new Error(t('settings.previewRequired'))
      }
      return rollbackManagedSettings(
        previewContext.target.version,
        previewContext.preview.active_version,
      )
    },
    onSuccess: (result) => {
      setDialogOpen(false)
      setNotice(
        result.restart_scheduled ? t('settings.rollbackScheduled') : t('settings.noChanges'),
      )
    },
  })
  const busy = preview.isPending || apply.isPending || rollback.isPending

  function updateSection<K extends keyof ManagedSettingsSnapshot>(
    section: K,
    patch: Partial<ManagedSettingsSnapshot[K]>,
  ) {
    setDraft((current) =>
      current
        ? {
            ...current,
            [section]: { ...current[section], ...patch },
          }
        : current,
    )
    setNotice(null)
    preview.reset()
  }

  if (state.isError && !state.data) {
    return <SettingsLoadError onRetry={() => void state.refetch()} />
  }
  if (!state.data || !draft) {
    return <SettingsLoading />
  }

  return (
    <div className="max-w-5xl space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3 border-y border-border py-3">
        <div>
          <p className="text-sm font-medium text-text-primary">
            {state.data.active_version === null
              ? t('settings.baselineVersion')
              : t('settings.activeVersion', { version: state.data.active_version })}
          </p>
          <p className="text-xs text-text-muted">{t('settings.versionHint')}</p>
        </div>
        {tab !== 'history' && (
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="outline"
              className="h-10"
              disabled={!dirty || busy}
              onClick={() => {
                setDraft(cloneSnapshot(state.data.snapshot))
                setNotice(null)
              }}
            >
              <RotateCcw className="h-4 w-4" aria-hidden />
              {t('settings.resetDraft')}
            </Button>
            <Button
              type="button"
              className="h-10"
              disabled={!dirty || busy}
              onClick={() => preview.mutate({ snapshot: draft })}
            >
              <Save className="h-4 w-4" aria-hidden />
              {preview.isPending ? t('settings.previewing') : t('settings.preview')}
            </Button>
          </div>
        )}
      </div>

      {state.isError && (
        <InlineError message={t('settings.refreshError')} onRetry={() => void state.refetch()} />
      )}
      {preview.isError && (
        <p role="alert" className="text-sm text-danger">
          {preview.error.message}
        </p>
      )}
      {notice && (
        <p role="status" className="border-l-2 border-success pl-3 text-sm text-text-primary">
          {notice}
        </p>
      )}

      {tab === 'history' ? (
        <SettingsHistory
          query={history}
          activeVersion={state.data.active_version}
          busy={busy}
          onRollback={(target) => preview.mutate({ snapshot: target.snapshot, target })}
        />
      ) : (
        <SettingsForm tab={tab} draft={draft} disabled={busy} updateSection={updateSection} />
      )}

      <SettingsConfirmationDialog
        open={dialogOpen}
        context={previewContext}
        pending={apply.isPending || rollback.isPending}
        error={apply.error?.message ?? rollback.error?.message ?? null}
        onOpenChange={(open) => {
          if (!busy) setDialogOpen(open)
        }}
        onConfirm={() => {
          if (previewContext?.kind === 'rollback') rollback.mutate()
          else apply.mutate()
        }}
      />
    </div>
  )
}

function SettingsForm({
  tab,
  draft,
  disabled,
  updateSection,
}: {
  tab: Exclude<ManagedSettingsTab, 'history'>
  draft: ManagedSettingsSnapshot
  disabled: boolean
  updateSection: <K extends keyof ManagedSettingsSnapshot>(
    section: K,
    patch: Partial<ManagedSettingsSnapshot[K]>,
  ) => void
}) {
  const { t } = useTranslation()
  const common = { disabled }

  if (tab === 'runtime') {
    return (
      <SettingsSection
        title={t('settings.runtimeTitle')}
        description={t('settings.runtimeManaged')}
      >
        <TextField
          id="agents-root"
          label={t('settings.agentsRoot')}
          value={draft.runtime.agents_root}
          {...common}
          onChange={(value) => updateSection('runtime', { agents_root: value })}
        />
        <TextField
          id="hermes-source"
          label={t('settings.hermesSource')}
          value={draft.runtime.hermes_source}
          {...common}
          onChange={(value) => updateSection('runtime', { hermes_source: value })}
        />
        <TextField
          id="hermes-command"
          label={t('settings.hermesCommand')}
          value={draft.runtime.hermes_command}
          {...common}
          onChange={(value) => updateSection('runtime', { hermes_command: value })}
        />
        <TextField
          id="java-source"
          label={t('settings.javaAgentSource')}
          value={draft.runtime.java_agent_source}
          {...common}
          onChange={(value) => updateSection('runtime', { java_agent_source: value })}
        />
        <TextField
          id="java-command"
          label={t('settings.javaAgentCommand')}
          value={draft.runtime.java_agent_command}
          {...common}
          onChange={(value) => updateSection('runtime', { java_agent_command: value })}
        />
      </SettingsSection>
    )
  }

  if (tab === 'ports') {
    return (
      <SettingsSection title={t('settings.portsTitle')} description={t('settings.portsManaged')}>
        <NumberField
          id="agent-port-base"
          label={t('settings.agentPortBase')}
          value={draft.ports.agent_port_base}
          min={1024}
          {...common}
          onChange={(value) => updateSection('ports', { agent_port_base: value })}
        />
        <NumberField
          id="agent-port-stride"
          label={t('settings.agentPortStride')}
          value={draft.ports.agent_port_stride}
          min={4}
          {...common}
          onChange={(value) => updateSection('ports', { agent_port_stride: value })}
        />
        <p className="text-sm text-text-muted">{t('settings.infrastructurePorts')}</p>
      </SettingsSection>
    )
  }

  if (tab === 'integrations') {
    return (
      <SettingsSection
        title={t('settings.integrationsTitle')}
        description={t('settings.integrationsManaged')}
      >
        <TextField
          id="forge-api-url"
          label={t('settings.forgeApiUrl')}
          value={draft.integrations.forge_api_url ?? ''}
          {...common}
          onChange={(value) => updateSection('integrations', { forge_api_url: value || null })}
        />
        <TextField
          id="forge-project"
          label={t('settings.forgeProject')}
          value={draft.integrations.forge_project ?? ''}
          {...common}
          onChange={(value) => updateSection('integrations', { forge_project: value || null })}
        />
        <TextField
          id="workflow-url"
          label={t('settings.projectWorkflowUrl')}
          value={draft.integrations.project_workflow_url ?? ''}
          {...common}
          onChange={(value) =>
            updateSection('integrations', { project_workflow_url: value || null })
          }
        />
        <p className="text-sm text-text-muted">{t('settings.integrationSecrets')}</p>
      </SettingsSection>
    )
  }

  if (tab === 'retention') {
    return (
      <SettingsSection
        title={t('settings.retentionTitle')}
        description={t('settings.retentionManaged')}
      >
        <NumberField
          id="stale-days"
          label={t('settings.staleArchivedDays')}
          value={draft.retention.stale_archived_days}
          min={0}
          {...common}
          onChange={(value) => updateSection('retention', { stale_archived_days: value })}
        />
        <NumberField
          id="review-interval"
          label={t('settings.reviewInterval')}
          value={draft.retention.review_interval_secs}
          min={60}
          {...common}
          onChange={(value) => updateSection('retention', { review_interval_secs: value })}
        />
      </SettingsSection>
    )
  }

  return (
    <SettingsSection title={t('settings.authTitle')} description={t('settings.authManaged')}>
      <SelectField
        id="auth-mode"
        label={t('settings.authMode')}
        value={draft.auth.mode}
        disabled={disabled}
        options={[
          ['hmac', 'HMAC'],
          ['oidc', 'OIDC'],
        ]}
        onChange={(value) => updateSection('auth', { mode: value })}
      />
      <TextField
        id="jwt-issuer"
        label={t('settings.jwtIssuer')}
        value={draft.auth.jwt_issuer}
        {...common}
        onChange={(value) => updateSection('auth', { jwt_issuer: value })}
      />
      <TextField
        id="jwt-audience"
        label={t('settings.jwtAudience')}
        value={draft.auth.jwt_audience}
        {...common}
        onChange={(value) => updateSection('auth', { jwt_audience: value })}
      />
      <NumberField
        id="access-ttl"
        label={t('settings.accessTokenTtl')}
        value={draft.auth.access_token_ttl_minutes}
        min={1}
        {...common}
        onChange={(value) => updateSection('auth', { access_token_ttl_minutes: value })}
      />
      <NumberField
        id="refresh-ttl"
        label={t('settings.refreshTokenTtl')}
        value={draft.auth.refresh_token_ttl_days}
        min={1}
        {...common}
        onChange={(value) => updateSection('auth', { refresh_token_ttl_days: value })}
      />
      <TextField
        id="cookie-name"
        label={t('settings.refreshCookieName')}
        value={draft.auth.refresh_cookie_name}
        {...common}
        onChange={(value) => updateSection('auth', { refresh_cookie_name: value })}
      />
      <SelectField
        id="same-site"
        label={t('settings.sameSite')}
        value={draft.auth.refresh_cookie_same_site}
        disabled={disabled}
        options={[
          ['Lax', 'Lax'],
          ['Strict', 'Strict'],
          ['None', 'None'],
        ]}
        onChange={(value) => updateSection('auth', { refresh_cookie_same_site: value })}
      />
      <TextField
        id="cookie-domain"
        label={t('settings.cookieDomain')}
        value={draft.auth.refresh_cookie_domain ?? ''}
        {...common}
        onChange={(value) => updateSection('auth', { refresh_cookie_domain: value || null })}
      />
      <TextField
        id="cookie-path"
        label={t('settings.cookiePath')}
        value={draft.auth.refresh_cookie_path}
        {...common}
        onChange={(value) => updateSection('auth', { refresh_cookie_path: value })}
      />
      <label
        className="flex min-h-10 items-center gap-3 text-sm text-text-primary"
        htmlFor="secure-cookie"
      >
        <input
          id="secure-cookie"
          type="checkbox"
          className="h-5 w-5 accent-accent"
          checked={draft.auth.refresh_cookie_secure}
          disabled={disabled}
          onChange={(event) =>
            updateSection('auth', { refresh_cookie_secure: event.target.checked })
          }
        />
        {t('settings.secureRefreshCookie')}
      </label>
      <p className="text-sm text-text-muted">{t('settings.authSecrets')}</p>
    </SettingsSection>
  )
}

function SettingsSection({
  title,
  description,
  children,
}: {
  title: string
  description: string
  children: React.ReactNode
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <p className="text-sm text-text-secondary">{description}</p>
      </CardHeader>
      <CardContent className="grid gap-4 md:grid-cols-2">{children}</CardContent>
    </Card>
  )
}

function TextField({
  id,
  label,
  value,
  disabled,
  onChange,
}: {
  id: string
  label: string
  value: string
  disabled: boolean
  onChange: (value: string) => void
}) {
  return (
    <div className="min-w-0 space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        value={value}
        disabled={disabled}
        className="min-h-10 font-mono"
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  )
}

function NumberField({
  id,
  label,
  value,
  min,
  disabled,
  onChange,
}: {
  id: string
  label: string
  value: number
  min: number
  disabled: boolean
  onChange: (value: number) => void
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <Input
        id={id}
        type="number"
        min={min}
        value={value}
        disabled={disabled}
        className="min-h-10"
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  )
}

function SelectField({
  id,
  label,
  value,
  options,
  disabled,
  onChange,
}: {
  id: string
  label: string
  value: string
  options: Array<[string, string]>
  disabled: boolean
  onChange: (value: string) => void
}) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={id}>{label}</Label>
      <select
        id={id}
        value={value}
        disabled={disabled}
        className="min-h-10 w-full rounded-md border border-border bg-surface px-3 text-sm text-text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
        onChange={(event) => onChange(event.target.value)}
      >
        {options.map(([optionValue, optionLabel]) => (
          <option key={optionValue} value={optionValue}>
            {optionLabel}
          </option>
        ))}
      </select>
    </div>
  )
}

function SettingsHistory({
  query,
  activeVersion,
  busy,
  onRollback,
}: {
  query: ReturnType<typeof useQuery<ManagedSettingsVersion[], Error>>
  activeVersion: number | null
  busy: boolean
  onRollback: (version: ManagedSettingsVersion) => void
}) {
  const { t, i18n } = useTranslation()
  if (query.isError && !query.data)
    return <SettingsLoadError onRetry={() => void query.refetch()} />
  if (!query.data) return <SettingsLoading />
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.historyTitle')}</CardTitle>
      </CardHeader>
      <CardContent className="divide-y divide-border">
        {query.data.length === 0 && (
          <p className="py-4 text-sm text-text-muted">{t('settings.noVersions')}</p>
        )}
        {query.data.map((version) => (
          <div key={version.id} className="flex min-w-0 flex-wrap items-center gap-3 py-3">
            <History className="h-4 w-4 text-text-muted" aria-hidden />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-text-primary">
                {t('settings.versionNumber', { version: version.version })}
              </p>
              <p className="text-xs text-text-muted">
                {new Intl.DateTimeFormat(i18n.language, {
                  dateStyle: 'medium',
                  timeStyle: 'short',
                }).format(new Date(version.created_at))}
                {version.rollback_of_version
                  ? ` · ${t('settings.rollbackOf', { version: version.rollback_of_version })}`
                  : ''}
              </p>
            </div>
            {version.version === activeVersion ? (
              <span className="rounded border border-success/40 bg-success/10 px-2 py-1 text-xs font-medium text-text-primary">
                {t('settings.active')}
              </span>
            ) : (
              <Button
                type="button"
                variant="outline"
                className="h-10"
                disabled={busy}
                onClick={() => onRollback(version)}
              >
                <RotateCcw className="h-4 w-4" aria-hidden />
                {t('settings.rollback')}
              </Button>
            )}
          </div>
        ))}
        {query.isError && (
          <InlineError message={t('settings.refreshError')} onRetry={() => void query.refetch()} />
        )}
      </CardContent>
    </Card>
  )
}

function SettingsConfirmationDialog({
  open,
  context,
  pending,
  error,
  onOpenChange,
  onConfirm,
}: {
  open: boolean
  context: PreviewContext | null
  pending: boolean
  error: string | null
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
}) {
  const { t } = useTranslation()
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent aria-busy={pending} className="max-h-[min(85vh,44rem)] overflow-y-auto">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {context?.kind === 'rollback'
              ? t('settings.rollbackConfirmTitle', { version: context.target.version })
              : t('settings.applyConfirmTitle')}
          </AlertDialogTitle>
          <AlertDialogDescription>{t('settings.restartWarning')}</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="divide-y divide-border border-y border-border">
          {(context?.preview.changes ?? []).map((change) => (
            <div
              key={change.path}
              className="grid min-w-0 gap-1 py-3 sm:grid-cols-[12rem_minmax(0,1fr)]"
            >
              <p className="break-all font-mono text-xs text-text-muted">{change.path}</p>
              <p className="min-w-0 break-all text-sm text-text-primary">
                {formatValue(change.before)} → {formatValue(change.after)}
              </p>
            </div>
          ))}
        </div>
        {error && (
          <p role="alert" className="text-sm text-danger">
            {error}
          </p>
        )}
        <AlertDialogFooter>
          <AlertDialogCancel className="h-10" disabled={pending}>
            {t('common.cancel')}
          </AlertDialogCancel>
          <AlertDialogAction
            className="h-10"
            disabled={pending}
            onClick={(event) => {
              event.preventDefault()
              onConfirm()
            }}
          >
            {pending
              ? t('settings.applying')
              : context?.kind === 'rollback'
                ? t('settings.confirmRollback')
                : t('settings.confirmApply')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}

function formatValue(value: unknown) {
  if (value === null || value === '') return '—'
  if (typeof value === 'string') return value
  return JSON.stringify(value)
}

function InlineError({ message, onRetry }: { message: string; onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div role="alert" className="flex flex-wrap items-center gap-2 text-sm text-danger">
      <span>{message}</span>
      <Button type="button" variant="outline" className="h-10" onClick={onRetry}>
        {t('settings.retry')}
      </Button>
    </div>
  )
}

function SettingsLoadError({ onRetry }: { onRetry: () => void }) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-wrap items-center gap-3">
      <ErrorState message={t('settings.loadError')} />
      <Button type="button" variant="outline" className="h-10" onClick={onRetry}>
        {t('settings.retry')}
      </Button>
    </div>
  )
}

function SettingsLoading() {
  const { t } = useTranslation()
  return (
    <p role="status" className="py-6 text-sm text-text-muted">
      {t('settings.loading')}
    </p>
  )
}
