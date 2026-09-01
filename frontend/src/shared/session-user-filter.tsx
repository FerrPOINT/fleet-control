import { useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
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

  const mergedUsers = useMemo(() => {
    const knownUsers = users.data ?? []
    if (!currentUserId || knownUsers.some((user) => user.id === currentUserId)) return knownUsers
    const currentUser: UserResponse = {
      id: currentUserId,
      email: email ?? '',
      username: username ?? email ?? 'me',
      display_name: displayName ?? email ?? 'Me',
      is_system_admin: false,
      is_active: true,
    }
    return [currentUser, ...knownUsers]
  }, [currentUserId, displayName, email, username, users.data])

  const selectedUsers = selectedUserIds
    .map((id) => mergedUsers.find((user) => user.id === id))
    .filter((user): user is UserResponse => Boolean(user))

  function addUser(userId: string) {
    setSelectedUserIds((current) => (current.includes(userId) ? current : [...current, userId]))
  }

  function removeUser(userId: string) {
    setSelectedUserIds((current) => current.filter((id) => id !== userId))
  }

  return {
    users,
    allUsers: mergedUsers,
    selectedUsers,
    selectedUserIds,
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
  const availableUsers = filter.allUsers.filter((user) => !filter.selectedUserIds.includes(user.id))

  return (
    <div className={cn('rounded-md border border-border bg-background p-3', className)}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase text-text-muted">Session users</p>
          <p className="mt-1 text-sm text-text-secondary">
            {filter.selectedUserIds.length ? 'Filtered by selected users' : 'All users'}
          </p>
        </div>
        <label className="flex min-w-0 items-center gap-2 text-sm">
          <UserPlus className="h-4 w-4 shrink-0 text-text-muted" />
          <select
            aria-label="Add session user"
            value=""
            disabled={!availableUsers.length}
            onChange={(event) => {
              if (!event.target.value) return
              filter.addUser(event.target.value)
              event.currentTarget.value = ''
            }}
            className="h-9 min-w-0 rounded-md border border-border bg-background px-3 text-sm text-text-primary disabled:opacity-60"
          >
            <option value="">Add user</option>
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
              className="inline-flex min-h-8 max-w-full items-center gap-2 rounded-md border border-border-strong bg-surface-raised px-2 text-sm text-text-primary"
            >
              <UserAvatar name={user.display_name} userId={user.id} />
              <span className="min-w-0 truncate">{user.display_name}</span>
              <button
                type="button"
                aria-label={`Remove ${user.display_name} filter`}
                onClick={() => filter.removeUser(user.id)}
                className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-text-muted hover:bg-border hover:text-text-primary"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </span>
          ))
        ) : (
          <span className="inline-flex min-h-8 items-center gap-2 rounded-md border border-border-strong bg-surface-raised px-2 text-sm text-text-muted">
            <UserAvatar />
            All users
          </span>
        )}
      </div>
    </div>
  )
}
