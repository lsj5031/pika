import { describe, it, expect, vi, beforeAll } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { onlineManager, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useSessions } from '../useSessions'

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

describe.skip('useSessions - React 19 Compatibility Issue', () => {
  beforeAll(() => {
    onlineManager.setOnline(true)
  })

  it('fetches sessions successfully', async () => {
    const mockSessions = [
      {
        session_id: 'session-1',
        project_name: 'project1',
        created_at: '2026-01-18 10:00:00',
        status: 'running',
      },
      {
        session_id: 'session-2',
        project_name: 'project2',
        created_at: '2026-01-18 09:00:00',
        status: 'stopped',
      },
    ]
    mockGet.mockResolvedValue(mockSessions)

    const { result } = renderHook(() => useSessions(), {
      wrapper: createWrapper(),
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
    })

    expect(result.current.data).toEqual(mockSessions)
    expect(mockGet).toHaveBeenCalledWith('/api/sessions')
  })

  it('handles API errors', async () => {
    mockGet.mockRejectedValue(new Error('Failed to fetch'))

    const { result } = renderHook(() => useSessions(), {
      wrapper: createWrapper(),
    })

    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(result.current.error).toBeTruthy()
  })
})
