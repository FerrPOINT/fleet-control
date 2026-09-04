import { useAuthStore } from '@/shared/auth/store'

export const apiBaseUrl = import.meta.env.VITE_API_BASE_URL?.replace('/api/v1', '') ?? ''

let refreshPromise: Promise<boolean> | null = null

export async function refreshAccessToken(): Promise<boolean> {
  if (refreshPromise) return refreshPromise
  refreshPromise = (async () => {
    try {
      const res = await fetch(`${apiBaseUrl}/api/v1/auth/refresh`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: '{}',
      })
      if (!res.ok) {
        useAuthStore.getState().logout()
        return false
      }
      const data = (await res.json()) as {
        access_token?: string
        user_id?: string
        email?: string
        username?: string
        display_name?: string
        system_role?: 'admin' | 'operator' | 'user'
        is_system_admin?: boolean
      }
      if (!data.access_token || !data.user_id || !data.email) {
        useAuthStore.getState().logout()
        return false
      }
      useAuthStore.getState().setAuth({
        token: data.access_token,
        userId: data.user_id,
        email: data.email,
        username: data.username,
        displayName: data.display_name,
        systemRole: data.system_role ?? (data.is_system_admin ? 'admin' : 'user'),
        isSystemAdmin: Boolean(data.is_system_admin),
        permissions: permissionsForRole(
          data.system_role ?? (data.is_system_admin ? 'admin' : 'user'),
        ),
      })
      return true
    } finally {
      refreshPromise = null
    }
  })()
  return refreshPromise
}

export function permissionsForRole(role: 'admin' | 'operator' | 'user'): string[] {
  const base = ['sessions:read_own', 'sessions:write_own', 'agents:read_directory']
  if (role === 'user') return base
  const operator = [
    ...base,
    'agents:manage',
    'leaders:manage',
    'executors:manage',
    'runtime:manage',
    'config:manage',
    'skills:manage',
    'deployments:manage',
    'logs:read',
    'audit_log:read',
    'settings:manage',
    'sessions:read_all',
  ]
  if (role === 'operator') return operator
  return [...operator, 'users:manage', 'rbac:manage']
}

// Request plumbing comes from the shared fleet client (services-base):
// bearer header, credentials, 401-refresh-retry, structured error envelope.
import { createApiClient, ApiError } from '@sdlc/ui/lib'

const shared = createApiClient({
  baseUrl: apiBaseUrl,
  getAccessToken: () => useAuthStore.getState().token,
  refresh: async () => refreshAccessToken(),
})

export async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method ?? 'GET').toUpperCase()
  const body = init.body === undefined ? undefined : JSON.parse(String(init.body))
  switch (method) {
    case 'POST':
      return shared.post<T>(path, body)
    case 'PUT':
      return shared.put<T>(path, body)
    case 'PATCH':
      return shared.patch<T>(path, body)
    case 'DELETE':
      return shared.delete<T>(path)
    default:
      return shared.get<T>(path)
  }
}

export function jsonBody<T>(body: T): RequestInit {
  return {
    method: 'POST',
    body: JSON.stringify(body),
  }
}

export type { ApiError }
