import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ChatInput } from '../ChatInput'

describe('ChatInput', () => {
  it('renders textarea', () => {
    render(<ChatInput sessionId="test-123" onSendMessage={vi.fn()} />)
    const textarea = screen.getByRole('textbox')
    expect(textarea).toBeInTheDocument()
  })

  it('renders send button', () => {
    render(<ChatInput sessionId="test-123" onSendMessage={vi.fn()} />)
    const sendButton = screen.getByRole('button')
    expect(sendButton).toBeInTheDocument()
  })

  it('calls onSendMessage when send button is clicked', async () => {
    const mockSend = vi.fn()
    render(<ChatInput sessionId="test-123" onSendMessage={mockSend} />)

    const textarea = screen.getByRole('textbox')
    const sendButton = screen.getByRole('button')

    await userEvent.type(textarea, 'Test message')
    await userEvent.click(sendButton)

    expect(mockSend).toHaveBeenCalledWith('Test message')
  })

  it('disables send button when input is empty', () => {
    render(<ChatInput sessionId="test-123" onSendMessage={vi.fn()} />)
    const sendButton = screen.getByRole('button')
    expect(sendButton).toBeDisabled()
  })

  it('disables input when no sessionId', () => {
    render(<ChatInput sessionId={null} onSendMessage={vi.fn()} />)
    const textarea = screen.getByRole('textbox')
    expect(textarea).toBeDisabled()
  })
})
