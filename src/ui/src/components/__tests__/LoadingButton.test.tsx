import { render, screen } from '@testing-library/react'
import LoadingButton from '../LoadingButton'

describe('LoadingButton', () => {
  it('renders button with text when not loading', () => {
    render(
      <LoadingButton loading={false}>
        Click me
      </LoadingButton>
    )
    
    expect(screen.getByRole('button')).toBeInTheDocument()
    expect(screen.getByText('Click me')).toBeInTheDocument()
  })

  it('shows loading state when loading prop is true', () => {
    render(
      <LoadingButton loading={true}>
        Click me
      </LoadingButton>
    )
    
    const button = screen.getByRole('button')
    expect(button).toBeDisabled()
  })
})