import type { QueryClient } from '@tanstack/react-query'
import type { HistoryPage } from '@/api/task-chats'

export function historyMessages(pages: HistoryPage[]) {
  return [
    ...new Map(
      pages
        .slice()
        .reverse()
        .flatMap((page) => page.items)
        .map((message) => [message.id, message]),
    ).values(),
  ]
}

export async function refreshHistory(client: QueryClient, id: string) {
  const queryKey = ['chat-history', id]
  const alreadyFetching = client.isFetching({ queryKey, exact: true }) > 0
  await client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false })
  if (alreadyFetching)
    await client.invalidateQueries({ queryKey, exact: true }, { cancelRefetch: false })
}
