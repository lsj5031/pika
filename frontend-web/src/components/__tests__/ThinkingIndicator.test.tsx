import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ThinkingIndicator } from '../ThinkingIndicator'

describe('ThinkingIndicator', () => {
  it('renders thinking indicator without content', () => {
    render(<ThinkingIndicator content="" />)
    expect(screen.getByText('Thinking')).toBeInTheDocument()
  })

  it('renders thinking indicator with content', () => {
    render(<ThinkingIndicator content="Analyzing the problem..." />)
    expect(screen.getByText('Thinking')).toBeInTheDocument()
    expect(screen.getByText('Analyzing the problem...')).toBeInTheDocument()
  })

  it('applies custom className', () => {
    const { container } = render(
      <ThinkingIndicator content="test" className="custom-class" />
    )
    expect(container.firstChild).toHaveClass('custom-class')
  })
})
