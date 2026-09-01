import { FormEvent, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { useMutation } from '@tanstack/react-query'
import { UserPlus } from 'lucide-react'
import { register } from '@/api/auth'
import { useAuthStore } from '@/shared/auth/store'
import { Button } from '@/shared/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/shared/ui/card'
import { Input } from '@/shared/ui/input'
import { Label } from '@/shared/ui/label'
import { ErrorState } from '../common'

export function RegisterPage() {
  const navigate = useNavigate()
  const setAuth = useAuthStore((state) => state.setAuth)
  const [email, setEmail] = useState('')
  const [username, setUsername] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [password, setPassword] = useState('')
  const mutation = useMutation({
    mutationFn: () =>
      register({
        email,
        username,
        display_name: displayName,
        password,
      }),
    onSuccess: (response) => {
      setAuth({
        token: response.access_token,
        userId: response.user_id,
        email: response.email,
        username: response.username,
        displayName: response.display_name,
      })
      navigate('/')
    },
  })

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    mutation.mutate()
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Create operator</CardTitle>
        </CardHeader>
        <CardContent>
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
              <Label htmlFor="username">Username</Label>
              <Input
                id="username"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="display-name">Display name</Label>
              <Input
                id="display-name"
                value={displayName}
                onChange={(event) => setDisplayName(event.target.value)}
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
              <UserPlus className="h-4 w-4" />
              Create account
            </Button>
            <Button asChild variant="ghost">
              <Link to="/login">Back to sign in</Link>
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
