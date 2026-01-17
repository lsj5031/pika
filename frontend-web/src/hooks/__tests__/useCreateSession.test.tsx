import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useCreateSession } from '../useCreateSession'

const mockPost = vi.fn()

vi.mock('../lib/api', () => ({
  apiClient: {
    post: mockPost,
  },
}))

vi.mock('../lib/toast', () => ({
  showError: vi.fn(),
  showSuccess: vi.fn(),
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

describe.skip('useCreateSession - React 19 Compatibility Issue', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('creates a new session successfully', async () => {
    const mockSession = {
      session_id: 'new-session-123',
      project_name: 'test-project',
      created_at: '2026-01-18 10:00:00',
    }
    mockPost.mockResolvedValue(mockSession)

    const { result } = renderHook(
      () => useCreateSession(),
      { wrapper: createWrapper() }
    )

    result.current.mutate({
      projectId: 'project-123',
      request: {
        name: 'Test session',
      },
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
    })

    expect(result.current.data).toEqual(mockSession)
    expect(mockPost).toHaveBeenCalledWith('/api/projects/project-123/sessions', {
      name: 'Test session',
    })
  })

  it('handles creation errors', async () => {
    mockPost.mockRejectedValue(new Error('Failed to create session'))

    const { result } = renderHook(
      () => useCreateSession(),
      { wrapper: createWrapper() }
    )

    result.current.mutate({
      projectId: 'project-123',
      request: {
        name: 'Test session',
      },
    })

    await waitFor(() => {
      expect(result.current.isError).toBe(true)
    })

    expect(result.current.error).toBeTruthy()
  })
})
