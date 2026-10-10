import { useQuery } from '@tanstack/react-query'
import { useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { ApiError } from '@sdlc/ui/lib'
import { Button } from '@sdlc/ui/ui'
import { getSession } from '@/api/fleet'
import { useAuthStore } from '@/shared/auth/store'
import { ErrorState } from '../common'
import { PrivateChatDetailPage } from './private-detail'
import { TaskChatDetailPage } from './task-detail'

export function ChatDetailPage() {
  const { sessionId = '' } = useParams()
  const actorId = useAuthStore((state) => state.userId)
  const authVersion = useAuthStore((state) => state.authVersion)
  return <ChatBindingWorkspace key={`${actorId}:${authVersion}:${sessionId}`} sessionId={sessionId} />
}

function ChatBindingWorkspace({ sessionId }: { sessionId: string }) {
  const { t } = useTranslation()
  const auth = useAuthStore()
  const authorized = Boolean(auth.token && auth.userId && !auth.signingOut)
  const session = useQuery({
    queryKey: ['chat-core-session', sessionId],
    queryFn: () => getSession(sessionId),
    enabled: authorized && Boolean(sessionId),
    retry: false,
    refetchInterval: 10000,
  })
  if (!authorized || session.isPending) return <p role="status">Загрузка чата…</p>
  if (session.isError)
    return (
      <>
        <ErrorState
          message={
            session.error instanceof ApiError && [401, 403, 404].includes(session.error.status)
              ? t('chatCore.denied')
              : 'Чат недоступен'
          }
        />
        <Button
          variant="outline"
          disabled={session.isFetching}
          onClick={() => void session.refetch()}
        >
          {t('sessions.retry')}
        </Button>
      </>
    )
  if (session.data.task_bound === true) return <TaskChatDetailPage />
  if (session.data.task_bound === false) return <PrivateChatDetailPage />
  return <ErrorState message="Не удалось проверить привязку чата" />
}
