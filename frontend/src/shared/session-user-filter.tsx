import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { UserPlus, X } from 'lucide-react'
import { listUsers } from '@/api/auth'
import type { UserResponse } from '@/api/types'
import { useAuthStore } from '@/shared/auth/store'
import { cn } from '@/shared/lib/utils'
import { UserAvatar } from '@/shared/ui/user-avatar'

export type SessionUserFilterState = ReturnType<typeof useSessionUserFilter>

export function useSessionUserFilter() {
  const currentUserId = useAuthStore((state) => state.userId)
  const email = useAuthStore((state) => state.email)
  const username = useAuthStore((state) => state.username)
  const displayName = useAuthStore((state) => state.displayName)
  const isSystemAdmin = useAuthStore((state) => state.isSystemAdmin)
  const systemRole = useAuthStore((state) => state.systemRole)
  const canReadAllSessions = useAuthStore((state) =>
    state.permissions.includes('sessions:read_all'),
  )
  const users = useQuery({ queryKey: ['users'], queryFn: listUsers })
  const [selectedUserIds, setSelectedUserIds] = useState<string[]>(() =>
    currentUserId ? [currentUserId] : [],
  )
  const [initialized, setInitialized] = useState(Boolean(currentUserId))

  useEffect(() => {
    if (initialized || !currentUserId) return
    setSelectedUserIds([currentUserId])
    setInitialized(true)
  }, [currentUserId, initialized])

  useEffect(() => {
    if (canReadAllSessions || !currentUserId || selectedUserIds.length) return
    setSelectedUserIds([currentUserId])
  }, [canReadAllSessions, currentUserId, selectedUserIds.length])

  const mergedUsers = useMemo(() => {
    const knownUsers = users.data ?? []
    if (!currentUserId || knownUsers.some((user) => user.id === currentUserId)) return knownUsers
    const currentUser: UserResponse = {
      id: currentUserId,
      email: email ?? '',
      username: username ?? email ?? 'me',
      display_name: displayName ?? email ?? 'Me',
      system_role: systemRole,
      is_system_admin: isSystemAdmin,
      is_active: true,
    }
    return [currentUser, ...knownUsers]
  }, [currentUserId, displayName, email, isSystemAdmin, systemRole, username, users.data])

  const selectedUsers = selectedUserIds
    .map((id) => mergedUsers.find((user) => user.id === id))
    .filter((user): user is UserResponse => Boolean(user))

  function addUser(userId: string) {
    setSelectedUserIds((current) => (current.includes(userId) ? current : [...current, userId]))
  }

  function removeUser(userId: string) {
    if (!canReadAllSessions && userId === currentUserId) return
    setSelectedUserIds((current) => current.filter((id) => id !== userId))
  }

  return {
    users,
    allUsers: mergedUsers,
    selectedUsers,
    selectedUserIds,
    isSystemAdmin: canReadAllSessions,
    setSelectedUserIds,
    addUser,
    removeUser,
  }
}

export function SessionUserFilter({
  filter,
  className,
}: {
  filter: SessionUserFilterState
  className?: string
}) {
  const { t } = useTranslation()
  const availableUsers = filter.allUsers.filter((user) => !filter.selectedUserIds.includes(user.id))

  return (
    <div className={cn('rounded-md border border-border bg-background p-3', className)}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase text-text-muted">
            {t('sessionFilter.title')}
          </p>
          <p className="mt-1 text-sm text-text-secondary">
            {filter.selectedUserIds.length ? t('sessionFilter.selected') : t('sessionFilter.all')}
          </p>
        </div>
        <label className="flex min-w-0 items-center gap-2 text-sm">
          <UserPlus className="h-4 w-4 shrink-0 text-text-muted" />
          <select
            aria-label={t('sessionFilter.add')}
            value=""
            disabled={!availableUsers.length}
            onChange={(event) => {
              if (!event.target.value) return
              filter.addUser(event.target.value)
              event.currentTarget.value = ''
            }}
            className="h-10 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-text-primary disabled:opacity-60"
          >
            <option value="">{t('sessionFilter.add')}</option>
            {availableUsers.map((user) => (
              <option key={user.id} value={user.id}>
                {user.display_name}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="mt-3 flex flex-wrap gap-2">
        {filter.selectedUsers.length ? (
          filter.selectedUsers.map((user) => (
            <span
              key={user.id}
              className="inline-flex min-h-10 max-w-full items-center gap-2 rounded-md border border-border-strong bg-surface-raised pl-2 text-sm text-text-primary"
            >
              <UserAvatar name={user.display_name} userId={user.id} />
              <span className="min-w-0 truncate">{user.display_name}</span>
              <button
                type="button"
                aria-label={t('sessionFilter.remove', { name: user.display_name })}
                onClick={() => filter.removeUser(user.id)}
                disabled={!filter.isSystemAdmin}
                className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-md text-text-muted hover:bg-border hover:text-text-primary disabled:cursor-not-allowed disabled:opacity-40"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </span>
          ))
        ) : (
          <span className="inline-flex min-h-8 items-center gap-2 rounded-md border border-border-strong bg-surface-raised px-2 text-sm text-text-muted">
            <UserAvatar />
            {t('sessionFilter.all')}
          </span>
        )}
      </div>
      {filter.users.isError ? (
        <div role="alert" className="mt-2 flex flex-wrap items-center gap-2 text-sm text-danger">
          <span>{t('sessionFilter.loadError')}</span>
          <button
            type="button"
            onClick={() => void filter.users.refetch()}
            className="rounded-sm underline underline-offset-2 focus-visible:outline-2 focus-visible:outline-focus"
          >
            {t('sessionFilter.retry')}
          </button>
        </div>
      ) : null}
    </div>
  )
}
