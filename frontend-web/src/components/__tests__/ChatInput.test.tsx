import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ChatInput } from '../ChatInput'

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  })

  return function TestWrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        {children}
      </QueryClientProvider>
    )
  }
}

describe('ChatInput', () => {
  it('renders textarea', () => {
    render(<ChatInput sessionId="test-123" onSendMessage={vi.fn()} />, { wrapper: createWrapper() })
    const textarea = screen.getByRole('textbox')
    expect(textarea).toBeInTheDocument()
  })

  it('renders send button', () => {
    render(<ChatInput sessionId="test-123" onSendMessage={vi.fn()} />, { wrapper: createWrapper() })
    const sendButton = screen.getByTestId('send-button')
    expect(sendButton).toBeInTheDocument()
  })

  it('calls onSendMessage when send button is clicked', async () => {
    const mockSend = vi.fn()
    render(<ChatInput sessionId="test-123" onSendMessage={mockSend} />, { wrapper: createWrapper() })

    const textarea = screen.getByRole('textbox')
    const sendButton = screen.getByTestId('send-button')

    await userEvent.type(textarea, 'Test message')
    await userEvent.click(sendButton)

    expect(mockSend).toHaveBeenCalledWith('Test message', [])
  })

  it('disables send button when input is empty', () => {
    render(<ChatInput sessionId="test-123" onSendMessage={vi.fn()} />, { wrapper: createWrapper() })
    const sendButton = screen.getByTestId('send-button')
    expect(sendButton).toBeDisabled()
  })

  it('disables input when no sessionId', () => {
    render(<ChatInput sessionId={null} onSendMessage={vi.fn()} />, { wrapper: createWrapper() })
    const textarea = screen.getByRole('textbox')
    expect(textarea).toBeDisabled()
  })
})
