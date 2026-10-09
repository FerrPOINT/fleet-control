import { isCurrentAuth, useAuthStore, type AuthScope } from '@/shared/auth/store'

export const apiBaseUrl = import.meta.env.VITE_API_BASE_URL?.replace('/api/v1', '') ?? ''

export async function refreshAccessToken(
  scope: AuthScope = useAuthStore.getState(),
): Promise<boolean> {
  if (!scope.token || !isCurrentAuth(scope)) return false
  useAuthStore.getState().logout()
  window.location.assign('/login')
  return false
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

// A late result is not a definite rejection of an already dispatched command.
export class AuthContextChangedError extends Error {
  constructor() {
    super('Authentication changed. Reconcile the original command before retrying.')
    this.name = 'AuthContextChangedError'
  }
}

export async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const scope = useAuthStore.getState()
  if (!isCurrentAuth(scope)) throw new AuthContextChangedError()
  const shared = createApiClient({
    baseUrl: apiBaseUrl,
    getAccessToken: () => scope.token,
    refresh: () => refreshAccessToken(scope),
  })
  const headers = new Headers(init.headers)
  // Existing Fleet callers pass serialized JSON without explicit headers.
  if (typeof init.body === 'string' && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }
  try {
    const result = await shared.request<T>(path, { ...init, headers })
    if (!isCurrentAuth(scope)) throw new AuthContextChangedError()
    return result
  } catch (error) {
    if (!isCurrentAuth(scope)) throw new AuthContextChangedError()
    throw error
  }
}

export type { ApiError }
