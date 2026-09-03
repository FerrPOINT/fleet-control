import { apiRequest } from './client'
import type {
  AuthResponse,
  LoginRequest,
  RegisterRequest,
  UpdateUserRoleRequest,
  UserListResponse,
  UserPermissionsResponse,
  UserResponse,
} from './types'

export async function login(req: LoginRequest): Promise<AuthResponse> {
  return apiRequest<AuthResponse>('/api/v1/auth/login', {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export async function register(req: RegisterRequest): Promise<AuthResponse> {
  return apiRequest<AuthResponse>('/api/v1/auth/register', {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export async function getCurrentUser(): Promise<UserResponse> {
  return apiRequest<UserResponse>('/api/v1/users/me')
}

export async function getCurrentUserPermissions(): Promise<UserPermissionsResponse> {
  return apiRequest<UserPermissionsResponse>('/api/v1/users/me/permissions')
}

export async function listUsers(): Promise<UserResponse[]> {
  const response = await apiRequest<UserListResponse>('/api/v1/users')
  return response.users
}

export async function updateUserRole(
  id: string,
  req: UpdateUserRoleRequest,
): Promise<UserResponse> {
  return apiRequest<UserResponse>(`/api/v1/users/${id}/role`, {
    method: 'PATCH',
    body: JSON.stringify(req),
  })
}

export async function logout(): Promise<void> {
  await apiRequest<void>('/api/v1/auth/logout', { method: 'POST' })
}
