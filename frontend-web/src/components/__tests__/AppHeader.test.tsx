import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { AppHeader } from '../AppHeader'

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

describe('AppHeader', () => {
  it('renders header with title', () => {
    render(
      <AppHeader
        connectionStatus="connected"
        isSessionActive={false}
        onMenuToggle={vi.fn()}
      />,
      { wrapper: createWrapper() }
    )
    expect(screen.getByText('Pika')).toBeInTheDocument()
  })

  it('renders menu button on mobile', () => {
    render(
      <AppHeader
        connectionStatus="connected"
        isSessionActive={false}
        onMenuToggle={vi.fn()}
      />,
      { wrapper: createWrapper() }
    )
    const menuButton = screen.getByTestId('session-list-button')
    expect(menuButton).toBeInTheDocument()
  })

  it('calls onMenuToggle when menu button is clicked', async () => {
    const mockToggle = vi.fn()
    render(
      <AppHeader
        connectionStatus="connected"
        isSessionActive={false}
        onMenuToggle={mockToggle}
      />,
      { wrapper: createWrapper() }
    )

    const menuButton = screen.getByTestId('session-list-button')
    menuButton.click()

    expect(mockToggle).toHaveBeenCalledOnce()
  })
})
