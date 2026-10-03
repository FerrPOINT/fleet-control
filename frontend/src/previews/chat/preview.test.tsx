import { render, screen, cleanup } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import { MemoryRouter } from 'react-router'
import { ThemeProvider } from '@sdlc/ui/lib'
import { Preview } from './preview'

afterEach(cleanup)
function show(query = '?view=clarify') {
  render(
    <ThemeProvider>
      <MemoryRouter initialEntries={['/' + query]}>
        <Preview />
      </MemoryRouter>
    </ThemeProvider>,
  )
  return userEvent.setup()
}
describe('isolated clarification preview', () => {
  it('supports keyboard navigation between the task tabs', async () => {
    const user = show('?view=chat')
    await user.click(screen.getByRole('tab', { name: 'Диалог' }))
    await user.keyboard('{ArrowRight}')
    expect(
      await screen.findByRole('heading', { name: 'Кто должен видеть задачи проекта?' }),
    ).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: /Уточнения/ })).toHaveAttribute('aria-selected', 'true')
  })
  it('requires explicit selection and separate exact-revision confirmation', async () => {
    const user = show()
    expect(screen.getByRole('button', { name: 'Подтвердить ответ' })).toBeDisabled()
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).not.toBeChecked()
    await user.click(screen.getByRole('radio', { name: /Участники проекта/ }))
    await user.click(screen.getByRole('button', { name: 'Подтвердить ответ' }))
    expect(screen.getByText('Ответ сохранён')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Проверить требования' }))
    const confirm = screen.getByRole('button', { name: 'Подтвердить редакцию 3' })
    expect(confirm).toBeDisabled()
    await user.click(screen.getByRole('checkbox', { name: /Подтверждаю цель/ }))
    await user.click(confirm)
    expect(screen.getByText('Опубликовано')).toBeInTheDocument()
    expect(screen.getByText(/Ожидает назначения Analyst/)).toBeInTheDocument()
  })
  it('requires text for a custom answer', async () => {
    const user = show()
    await user.click(screen.getByRole('radio', { name: /Свой вариант/ }))
    expect(screen.getByRole('button', { name: 'Подтвердить ответ' })).toBeDisabled()
    await user.type(
      screen.getByRole('textbox', { name: /Ваше решение/ }),
      'Access by project membership',
    )
    expect(screen.getByRole('button', { name: 'Подтвердить ответ' })).toBeEnabled()
  })
  it('keeps the local draft when a stale answer is rejected', async () => {
    const user = show('?view=clarify&state=conflict')
    await user.click(screen.getByRole('radio', { name: /Участники проекта/ }))
    await user.type(screen.getByRole('textbox', { name: /Комментарий/ }), 'Keep this comment')
    await user.click(screen.getByRole('button', { name: 'Подтвердить ответ' }))
    expect(screen.getByRole('alert')).toHaveTextContent('Вопрос изменился')
    expect(screen.getByRole('textbox', { name: /Комментарий/ })).toHaveValue('Keep this comment')
    expect(screen.queryByText('Ответ сохранён')).not.toBeInTheDocument()
  })
  it('does not permit clarification edits in read-only mode', () => {
    show('?view=clarify&state=read-only')
    expect(screen.getByRole('radio', { name: /Участники проекта/ })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: /Комментарий/ })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Подтвердить ответ' })).toBeDisabled()
  })
})
