import { FormEvent, useState } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { useMutation } from '@tanstack/react-query'
import { LogIn } from 'lucide-react'
import { login } from '@/api/auth'
import { permissionsForRole } from '@/api/client'
import { useAuthStore } from '@/shared/auth/store'
import { Button } from '@sdlc/ui/ui'
import { Card, CardContent, CardHeader, CardTitle } from '@sdlc/ui/ui'
import { Input } from '@sdlc/ui/ui'
import { Label } from '@sdlc/ui/ui'
import { ErrorState } from '../common'

export function LoginPage() {
  const navigate = useNavigate()
  const location = useLocation()
  const setAuth = useAuthStore((state) => state.setAuth)
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const mutation = useMutation({
    mutationFn: () => login({ email, password }),
    onSuccess: (response) => {
      setAuth({
        token: response.access_token,
        userId: response.user_id,
        email: response.email,
        username: response.username,
        displayName: response.display_name,
        systemRole: response.system_role,
        isSystemAdmin: response.is_system_admin,
        permissions: permissionsForRole(response.system_role),
      })
      navigate((location.state as { from?: { pathname?: string } } | null)?.from?.pathname ?? '/')
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    mutation.mutate()
  }

  return (
    <AuthFrame title="Sign in">
      <form className="grid gap-3" onSubmit={submit}>
        <div className="grid gap-2">
          <Label htmlFor="email">Email</Label>
          <Input
            id="email"
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
          />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="password">Password</Label>
          <Input
            id="password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
          />
        </div>
        {mutation.isError ? <ErrorState message={mutation.error.message} /> : null}
        <Button type="submit" disabled={mutation.isPending}>
          <LogIn className="h-4 w-4" />
          Sign in
        </Button>
        <Button asChild variant="ghost">
          <Link to="/register">Create first operator</Link>
        </Button>
      </form>
    </AuthFrame>
  )
}

function AuthFrame({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>{title}</CardTitle>
        </CardHeader>
        <CardContent>{children}</CardContent>
      </Card>
    </div>
  )
}
