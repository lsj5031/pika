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
        onOpenCommandPalette={vi.fn()}
      />,
      { wrapper: createWrapper() }
    )
    expect(screen.getByText('Pika')).toBeInTheDocument()
  })

  it('renders command palette button on mobile', () => {
    render(
      <AppHeader
        connectionStatus="connected"
        isSessionActive={false}
        onOpenCommandPalette={vi.fn()}
      />,
      { wrapper: createWrapper() }
    )
    const paletteButton = screen.getByTestId('command-palette-button')
    expect(paletteButton).toBeInTheDocument()
  })

  it('calls onOpenCommandPalette when command palette button is clicked', async () => {
    const mockOpen = vi.fn()
    render(
      <AppHeader
        connectionStatus="connected"
        isSessionActive={false}
        onOpenCommandPalette={mockOpen}
      />,
      { wrapper: createWrapper() }
    )

    const paletteButton = screen.getByTestId('command-palette-button')
    paletteButton.click()

    expect(mockOpen).toHaveBeenCalledOnce()
  })
})
