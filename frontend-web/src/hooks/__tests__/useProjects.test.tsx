import { describe, it, expect, vi, beforeAll, beforeEach } from 'vitest'
import { renderHook, waitFor } from '@testing-library/react'
import { onlineManager, QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { useProjects } from '../useProjects'

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

describe.skip('useProjects - React 19 Compatibility Issue', () => {
  beforeAll(() => {
    onlineManager.setOnline(true)
  })

  beforeEach(() => {
    mockGet.mockReset()
  })

  it('fetches projects successfully', async () => {
    const mockProjects = [
      { name: 'project1', path: '/path/to/project1' },
      { name: 'project2', path: '/path/to/project2' },
    ]
    mockGet.mockResolvedValue(mockProjects)

    const { result } = renderHook(() => useProjects(), {
      wrapper: createWrapper(),
    })

    await waitFor(
      () => {
        expect(result.current.isSuccess).toBe(true)
      },
      { timeout: 3000 }
    )

    expect(result.current.data).toEqual(mockProjects)
  })

  it('handles API errors', async () => {
    mockGet.mockRejectedValue(new Error('Failed to fetch'))

    const { result } = renderHook(() => useProjects(), {
      wrapper: createWrapper(),
    })

    await waitFor(() => expect(result.current.isError).toBe(true))
    expect(result.current.error).toBeTruthy()
  })
})
