import { describe, it, expect, vi } from 'vitest'

describe('basic test', () => {
  it('should pass', () => {
    expect(true).toBe(true)
  })

  it('should mock functions', () => {
    const mockFn = vi.fn()
    mockFn('test')
    expect(mockFn).toHaveBeenCalledWith('test')
  })
})
