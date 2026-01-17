import { describe, it, expect, vi, beforeAll } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { onlineManager, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useSessionHistory } from '../useSessionHistory'

const mockGet = vi.fn()

vi.mock('../lib/api', () => ({
  apiClient: {
    get: mockGet,
  },
}))

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        gcTime: Infinity,
        networkMode: 'always',
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

describe.skip('useSessionHistory - React 19 Compatibility Issue', () => {
  beforeAll(() => {
    onlineManager.setOnline(true)
  })

  it('fetches session history successfully', async () => {
    const mockMessages = [
      { role: 'user', content: 'Hello' },
      { role: 'assistant', content: 'Hi there!' },
    ]
    mockGet.mockResolvedValue(mockMessages)

    const sessionId = 'test-session-123'
    const { result } = renderHook(() => useSessionHistory({ sessionId }), {
      wrapper: createWrapper(),
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
    })

    expect(result.current.data).toEqual(mockMessages)
    expect(mockGet).toHaveBeenCalledWith(`/api/sessions/${sessionId}/messages`)
  })

  it('handles API errors', async () => {
    mockGet.mockRejectedValue(new Error('Session not found'))

    const { result } = renderHook(() => useSessionHistory({ sessionId: 'invalid-session' }), {
      wrapper: createWrapper(),
    })

    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(result.current.error).toBeTruthy()
  })
})
