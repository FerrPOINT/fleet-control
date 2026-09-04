// Route guard from @sdlc/ui/auth: fleet-control supplies its richer local
// role/permission store and refresh function.
import { RequireAuth as Guard } from '@sdlc/ui/auth'
import { refreshAccessToken } from '@/api/client'
import { useAuthStore } from '@/shared/auth/store'

export function RequireAuth() {
  return <Guard store={useAuthStore} refresh={refreshAccessToken} />
}
