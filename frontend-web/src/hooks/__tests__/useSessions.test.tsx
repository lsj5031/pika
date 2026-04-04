import { describe, it, expect, vi, beforeAll, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { onlineManager, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useSessions } from '../useSessions'

const mockGet = vi.fn()

vi.mock('../../lib/api', () => ({
  apiClient: {
    get: (...args: unknown[]) => mockGet(...args),
  },
}))

// Mock useResolvedSessions to just pass through sessions
vi.mock('../useResolvedSessions', () => ({
  useResolvedSessions: (sessions: unknown) => sessions,
}))

// Mock appStore
const mockSetActiveSessionIds = vi.fn()
vi.mock('../../store/appStore', () => ({
  useAppStore: {
    getState: () => ({
      setActiveSessionIds: mockSetActiveSessionIds,
    }),
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

describe('useSessions', () => {
  beforeAll(() => {
    onlineManager.setOnline(true)
  })

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('syncs activeSessionIds from API response', async () => {
    const mockSessions = [
      {
        id: 'session-1',
        name: 'Active Session',
        project_id: 'proj-1',
        project_path: '/path/1',
        created_at: '2026-01-18T10:00:00Z',
        is_active: true,
      },
      {
        id: 'session-2',
        name: 'Inactive Session',
        project_id: 'proj-2',
        project_path: '/path/2',
        created_at: '2026-01-18T09:00:00Z',
        is_active: false,
      },
      {
        id: 'session-3',
        name: 'Another Active',
        project_id: 'proj-3',
        project_path: '/path/3',
        created_at: '2026-01-18T08:00:00Z',
        is_active: true,
      },
    ]
    mockGet.mockResolvedValue(mockSessions)

    const { result } = renderHook(() => useSessions(), {
      wrapper: createWrapper(),
    })

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true)
    })

    // Verify activeSessionIds are populated from the response
    expect(mockSetActiveSessionIds).toHaveBeenCalled()
    const calledSet = mockSetActiveSessionIds.mock.calls[0][0] as Set<string>
    expect(calledSet).toBeInstanceOf(Set)
    expect(calledSet.has('session-1')).toBe(true)
    expect(calledSet.has('session-3')).toBe(true)
    expect(calledSet.has('session-2')).toBe(false)
  })
})
