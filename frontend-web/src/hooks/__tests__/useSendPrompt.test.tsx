import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useSendPrompt } from '../useSendPrompt'

const mockPost = vi.fn()

vi.mock('../lib/api', () => ({
  apiClient: {
    post: mockPost,
  },
}))

vi.mock('../lib/toast', () => ({
  showError: vi.fn(),
}))

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      mutations: {
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

describe.skip('useSendPrompt - React 19 Compatibility Issue', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('sends prompt successfully', async () => {
    const mockResponse = {
      message: 'Prompt sent successfully',
    }
    mockPost.mockResolvedValue(mockResponse)

    const { result } = renderHook(
      () => useSendPrompt(),
      { wrapper: createWrapper() }
    )

    result.current.mutate({
      sessionId: 'session-123',
      prompt: 'Hello, AI!',
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
    })

    expect(mockPost).toHaveBeenCalledWith('/api/sessions/session-123/prompt', {
      prompt: 'Hello, AI!',
    })
  })

  it('handles send errors', async () => {
    mockPost.mockRejectedValue(new Error('Failed to send'))

    const { result } = renderHook(
      () => useSendPrompt(),
      { wrapper: createWrapper() }
    )

    result.current.mutate({
      sessionId: 'session-123',
      prompt: 'Test message',
    })

    await waitFor(() => {
      expect(result.current.isError).toBe(true)
    })

    expect(result.current.error).toBeTruthy()
  })
})
