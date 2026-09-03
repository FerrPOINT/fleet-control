import { create } from 'zustand'
import { persist } from 'zustand/middleware'

function readStoredAuth(): {
  token: string | null
  userId: string | null
  email: string | null
  username: string | null
  displayName: string | null
  systemRole: 'admin' | 'operator' | 'user'
  isSystemAdmin: boolean
  permissions: string[]
} {
  try {
    localStorage.removeItem('tt-refresh-token')
    localStorage.removeItem('task-tracker-auth')
    const raw = localStorage.getItem('fleet-control-auth')
    if (!raw)
      return {
        token: null,
        userId: null,
        email: null,
        username: null,
        displayName: null,
        systemRole: 'user',
        isSystemAdmin: false,
        permissions: [],
      }
    const parsed = JSON.parse(raw)
    const state = parsed.state ?? parsed
    return {
      token: null,
      userId: state.userId ?? state.user_id ?? null,
      email: state.email ?? null,
      username: state.username ?? null,
      displayName: state.displayName ?? state.display_name ?? null,
      systemRole: state.systemRole ?? state.system_role ?? 'user',
      isSystemAdmin: Boolean(state.isSystemAdmin ?? state.is_system_admin),
      permissions: state.permissions ?? [],
    }
  } catch {
    return {
      token: null,
      userId: null,
      email: null,
      username: null,
      displayName: null,
      systemRole: 'user',
      isSystemAdmin: false,
      permissions: [],
    }
  }
}

// The refresh token lives ONLY in the HttpOnly cookie set by the backend.
// It is never copied into localStorage: an XSS payload must not be able to
// read it and silently extend the session (audit r4, P1).
interface AuthState {
  token: string | null
  userId: string | null
  email: string | null
  username: string | null
  displayName: string | null
  systemRole: 'admin' | 'operator' | 'user'
  isSystemAdmin: boolean
  permissions: string[]
  setAuth: (payload: {
    token: string
    userId: string
    email: string
    username?: string
    displayName?: string
    systemRole?: 'admin' | 'operator' | 'user'
    isSystemAdmin?: boolean
    permissions?: string[]
  }) => void
  setUser: (payload: {
    userId?: string
    email?: string
    username?: string
    displayName?: string
    systemRole?: 'admin' | 'operator' | 'user'
    isSystemAdmin?: boolean
    permissions?: string[]
  }) => void
  logout: () => void
}

const initial = readStoredAuth()

export const useAuthStore = create<AuthState>()(
  persist(
    (set) => ({
      token: initial.token,
      userId: initial.userId,
      email: initial.email,
      username: initial.username,
      displayName: initial.displayName,
      systemRole: initial.systemRole,
      isSystemAdmin: initial.isSystemAdmin,
      permissions: initial.permissions,
      setAuth: (payload) =>
        set({
          token: payload.token,
          userId: payload.userId,
          email: payload.email,
          username: payload.username ?? null,
          displayName: payload.displayName ?? null,
          systemRole: payload.systemRole ?? (payload.isSystemAdmin ? 'admin' : 'user'),
          isSystemAdmin: Boolean(payload.isSystemAdmin ?? payload.systemRole === 'admin'),
          permissions: payload.permissions ?? [],
        }),
      setUser: (payload) =>
        set((state) => ({
          userId: payload.userId ?? state.userId,
          email: payload.email ?? state.email,
          username: payload.username ?? state.username,
          displayName: payload.displayName ?? state.displayName,
          systemRole: payload.systemRole ?? state.systemRole,
          isSystemAdmin: payload.isSystemAdmin ?? state.isSystemAdmin,
          permissions: payload.permissions ?? state.permissions,
        })),
      logout: () => {
        set({
          token: null,
          userId: null,
          email: null,
          username: null,
          displayName: null,
          systemRole: 'user',
          isSystemAdmin: false,
          permissions: [],
        })
      },
    }),
    {
      name: 'fleet-control-auth',
      merge: (persistedState, currentState) => {
        const persisted = (persistedState ?? {}) as Partial<AuthState>
        return {
          ...currentState,
          token: null,
          userId: persisted.userId ?? currentState.userId,
          email: persisted.email ?? currentState.email,
          username: persisted.username ?? currentState.username,
          displayName: persisted.displayName ?? currentState.displayName,
          systemRole: persisted.systemRole ?? currentState.systemRole,
          isSystemAdmin: persisted.isSystemAdmin ?? currentState.isSystemAdmin,
          permissions: persisted.permissions ?? currentState.permissions,
        }
      },
      partialize: (state) => ({
        userId: state.userId,
        email: state.email,
        username: state.username,
        displayName: state.displayName,
        systemRole: state.systemRole,
        isSystemAdmin: state.isSystemAdmin,
        permissions: state.permissions,
      }),
    },
  ),
)
