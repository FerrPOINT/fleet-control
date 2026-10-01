import type { SdlcRole } from '@/api/types'

export const sdlcRoles: { value: SdlcRole; label: string }[] = [
  { value: 'project_manager', label: 'Project Manager' },
  { value: 'analyst', label: 'Analyst' },
  { value: 'architect', label: 'Architect' },
  { value: 'developer', label: 'Developer' },
  { value: 'reviewer', label: 'Reviewer' },
  { value: 'tester', label: 'Tester' },
  { value: 'dev_ops', label: 'DevOps' },
]

export function sdlcRoleLabel(role?: SdlcRole | null) {
  return sdlcRoles.find((item) => item.value === role)?.label ?? '-'
}
