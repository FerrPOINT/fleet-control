import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import { MemoryRouter } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { PmDraftPreview } from './preview'
import { agents, projects, type Command } from './fixtures'

afterEach(cleanup)
function show(query = '', commands: Command[] = []) {
  render(
    <ThemeProvider>
      <MemoryRouter initialEntries={['/' + query]}>
        <PmDraftPreview observe={(command) => commands.push(command)} />
      </MemoryRouter>
    </ThemeProvider>,
  )
  return userEvent.setup()
}
describe('isolated PM creation proposal', () => {
  it('requires explicit project, concrete PM and title; never implies a runtime launch', async () => {
    const user = show('?result=unknown')
    const create = screen.getByRole('button', { name: 'Создать задачу' })
    expect(create).toBeDisabled()
    await user.selectOptions(screen.getByLabelText('Проект'), projects.projects[0]!.id)
    await user.selectOptions(screen.getByLabelText('Project Manager'), agents[0]!.id)
    await user.type(screen.getByLabelText('Название задачи'), 'Internal portal')
    await user.type(screen.getByLabelText('Исходный запрос'), 'Keep original input')
    await user.click(create)
    expect(screen.getByText('Ответ не получен')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Создать задачу' })).not.toBeInTheDocument()
    expect(screen.getByLabelText('Исходный запрос')).toHaveValue('Keep original input')
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(screen.getByText('Задача создана, подготовка не завершена')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Продолжить подготовку' }))
    await user.click(screen.getByRole('button', { name: 'Продолжить подготовку' }))
    expect(screen.getByText(/Агент ещё не запущен/)).toBeInTheDocument()
    expect(screen.getByLabelText('Исходный запрос')).toHaveValue('Keep original input')
  })
  it('preserves unknown acceptance when the dependency is unavailable', async () => {
    const user = show('?state=uncertain&access=unavailable')
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(screen.getByRole('alert')).toHaveTextContent('Исход операции остаётся неизвестным')
    expect(screen.getByText('Ответ не получен')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Создать задачу' })).not.toBeInTheDocument()
  })
  it('offers retry only after explicit absent readback, never preselects project or agent', async () => {
    const user = show('?state=uncertain&result=absent')
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(screen.getByText('Операция не найдена')).toBeInTheDocument()
    expect(screen.getByLabelText('Проект')).toHaveValue(projects.projects[0]!.id)
    expect(screen.getByLabelText('Project Manager')).toHaveValue(agents[0]!.id)
  })
  it('does not permit creation without project access', () => {
    show('?access=denied')
    expect(screen.getByRole('alert')).toHaveTextContent('Нет доступа')
    expect(screen.getByRole('button', { name: 'Создать задачу' })).toBeDisabled()
  })
  it('looks up the original key and replays the exact request only after absence', async () => {
    const commands: Command[] = []
    const user = show('?result=absent', commands)
    await user.selectOptions(screen.getByLabelText('Проект'), projects.projects[0]!.id)
    await user.selectOptions(screen.getByLabelText('Project Manager'), agents[0]!.id)
    await user.type(screen.getByLabelText('Название задачи'), 'Исходная задача')
    await user.type(screen.getByLabelText('Исходный запрос'), 'Точный запрос & детали')
    await user.click(screen.getByRole('button', { name: 'Создать задачу' }))
    expect(screen.getByLabelText('Название задачи')).toHaveAttribute('readonly')
    expect(screen.queryByRole('button', { name: 'Повторить исходное создание' })).toBeNull()
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    const first = commands[0]!
    expect(first.kind).toBe('create')
    if (first.kind !== 'create') throw new Error('Expected original create')
    expect(commands[1]).toEqual({
      kind: 'lookup',
      projectId: first.projectId,
      key: first.request.idempotency_key,
    })
    await user.click(screen.getByRole('button', { name: 'Повторить исходное создание' }))
    expect(commands[2]).toEqual(first)
    expect(screen.getByText('Подготовка задачи не завершена')).toBeInTheDocument()
    expect(screen.queryByText(/UX-102/)).toBeNull()
  })
  it('reconciles unknown continuation by operation ID without another create', async () => {
    const commands: Command[] = []
    const user = show('?state=incomplete&step=chat&continue=unknown', commands)
    await user.click(screen.getByRole('button', { name: 'Продолжить подготовку' }))
    expect(screen.getByText('Ответ не получен')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(commands.map((command) => command.kind)).toEqual(['continue', 'readback'])
    expect(commands[0]).toMatchObject({ operationId: '33333333-3333-4333-8333-333333333333' })
    await user.click(screen.getByRole('button', { name: 'Открыть чат' }))
    expect(screen.getByRole('region', { name: 'Сохранённый PM-чат' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Отправить сообщение' })).toBeDisabled()
    await user.click(screen.getByRole('button', { name: 'Вернуться к состоянию подготовки' }))
    expect(screen.getByLabelText('Исходный запрос')).toHaveValue(
      'Внутренний портал: заявки, исполнители и история изменений. Доступ только участникам проекта. Уточнить критерии приёмки.',
    )
  })
  it.each(['readonly', 'denied', 'disabled'] as const)(
    'holds saved preparation for %s',
    async (access) => {
      const commands: Command[] = []
      const user = show(`?state=incomplete&access=${access}`, commands)
      expect(screen.getByRole('button', { name: 'Продолжить подготовку' })).toBeDisabled()
      await user.click(screen.getByRole('button', { name: 'Продолжить подготовку' }))
      expect(commands).toEqual([])
      expect(screen.getByText('Задача создана, подготовка не завершена')).toBeInTheDocument()
      await user.click(screen.getByRole('button', { name: 'Вернуться в чаты' }))
      await user.click(screen.getByRole('button', { name: 'Вернуться к подготовке задачи' }))
      expect(screen.getByRole('button', { name: 'Продолжить подготовку' })).toBeDisabled()
    },
  )
  it('keeps pagination on an empty filtered page without automatically choosing a project', async () => {
    const user = show('?access=empty-page')
    expect(screen.getByLabelText('Проект')).toHaveValue('')
    await user.click(screen.getByRole('button', { name: 'Ещё проекты' }))
    expect(screen.getByRole('option', { name: 'UX · Портал заявок' })).toBeInTheDocument()
    expect(screen.getByLabelText('Проект')).toHaveValue('')
    expect(screen.getByRole('button', { name: 'Создать задачу' })).toBeDisabled()
  })
  it('retains an unsent form when navigating through the chat list', async () => {
    const user = show()
    await user.type(screen.getByLabelText('Название задачи'), 'Keep this draft')
    await user.click(screen.getByRole('button', { name: 'Вернуться в чаты' }))
    await user.click(screen.getByRole('button', { name: 'Вернуться к черновику' }))
    expect(screen.getByLabelText('Название задачи')).toHaveValue('Keep this draft')
  })
})
