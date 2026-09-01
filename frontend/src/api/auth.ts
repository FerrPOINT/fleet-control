import { apiRequest } from './client'
import type {
  AuthResponse,
  LoginRequest,
  RegisterRequest,
  UserListResponse,
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

export async function listUsers(): Promise<UserResponse[]> {
  const response = await apiRequest<UserListResponse>('/api/v1/users')
  return response.users
}

export async function logout(): Promise<void> {
  await apiRequest<void>('/api/v1/auth/logout', { method: 'POST' })
}
