import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import { MemoryRouter } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { PmDraftPreview } from './preview'

afterEach(cleanup)
function show(query = '') {
  render(
    <ThemeProvider>
      <MemoryRouter initialEntries={['/' + query]}>
        <PmDraftPreview />
      </MemoryRouter>
    </ThemeProvider>,
  )
  return userEvent.setup()
}
describe('isolated PM creation proposal', () => {
  it('requires explicit project, concrete PM and title; never implies a runtime launch', async () => {
    const user = show()
    const create = screen.getByRole('button', { name: 'Создать задачу' })
    expect(create).toBeDisabled()
    await user.selectOptions(screen.getByLabelText('Проект'), 'portal')
    await user.selectOptions(screen.getByLabelText('Project Manager'), 'agent3')
    await user.type(screen.getByLabelText('Название задачи'), 'Internal portal')
    await user.type(screen.getByLabelText('Исходный запрос'), 'Keep original input')
    await user.click(create)
    expect(screen.getByText('Ответ не получен')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Создать задачу' })).not.toBeInTheDocument()
    expect(screen.getByLabelText('Исходный запрос')).toHaveValue('Keep original input')
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(screen.getByText('Задача создана, подготовка не завершена')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Продолжить подготовку' }))
    expect(screen.getByText(/Агент ещё не запущен/)).toBeInTheDocument()
    expect(screen.getByLabelText('Исходный запрос')).toHaveValue('Keep original input')
  })
  it('preserves unknown acceptance when the dependency is unavailable', async () => {
    const user = show('?state=uncertain&access=unavailable')
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(screen.getByRole('alert')).toHaveTextContent('Tracker недоступен')
    expect(screen.getByText('Ответ не получен')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Создать задачу' })).not.toBeInTheDocument()
  })
  it('offers retry only after explicit absent readback, never preselects project or agent', async () => {
    const user = show('?state=uncertain&result=absent')
    await user.click(screen.getByRole('button', { name: 'Проверить состояние' }))
    expect(screen.getByText('Операция не найдена')).toBeInTheDocument()
    expect(screen.getByLabelText('Проект')).toHaveValue('portal')
    expect(screen.getByLabelText('Project Manager')).toHaveValue('agent3')
  })
  it('does not permit creation without project access', () => {
    show('?access=denied')
    expect(screen.getByRole('alert')).toHaveTextContent('Нет доступа')
    expect(screen.getByRole('button', { name: 'Создать задачу' })).toBeDisabled()
  })
})
