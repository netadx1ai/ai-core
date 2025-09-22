import { render, screen } from '@testing-library/react'
import FormInput from '../FormInput'

describe('FormInput', () => {
  it('renders input field with label', () => {
    render(
      <FormInput
        id="test-input"
        label="Test Label"
        type="text"
        placeholder="Test placeholder"
      />
    )
    
    expect(screen.getByLabelText('Test Label')).toBeInTheDocument()
    expect(screen.getByPlaceholderText('Test placeholder')).toBeInTheDocument()
  })

  it('displays error message when provided', () => {
    render(
      <FormInput
        id="test-input"
        label="Test Label"
        type="text"
        error="Test error message"
      />
    )
    
    expect(screen.getByText('Test error message')).toBeInTheDocument()
  })
})