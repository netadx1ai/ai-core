import { render, screen, fireEvent } from '@testing-library/react'
import { vi } from 'vitest'
import WorkflowCard from '../WorkflowCard'

const mockWorkflow = {
  id: '1',
  name: 'Test Workflow',
  description: 'Test workflow description',
  status: 'draft' as const,
  progress: 0,
  createdAt: '2024-01-15T10:00:00Z',
}

const mockHandlers = {
  onStart: vi.fn(),
  onPause: vi.fn(),
  onStop: vi.fn(),
  onEdit: vi.fn(),
}

describe('WorkflowCard', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders workflow information correctly', () => {
    render(<WorkflowCard workflow={mockWorkflow} {...mockHandlers} />)
    
    expect(screen.getByText('Test Workflow')).toBeInTheDocument()
    expect(screen.getByText('Test workflow description')).toBeInTheDocument()
    expect(screen.getByText('draft')).toBeInTheDocument()
  })

  it('shows start button for draft workflow', () => {
    render(<WorkflowCard workflow={mockWorkflow} {...mockHandlers} />)
    
    const startButton = screen.getByTitle('Start workflow')
    expect(startButton).toBeInTheDocument()
    
    fireEvent.click(startButton)
    expect(mockHandlers.onStart).toHaveBeenCalledWith('1')
  })

  it('shows progress bar for running workflow', () => {
    const runningWorkflow = { ...mockWorkflow, status: 'running' as const, progress: 75 }
    render(<WorkflowCard workflow={runningWorkflow} {...mockHandlers} />)
    
    expect(screen.getByText('Progress')).toBeInTheDocument()
    expect(screen.getByText('75%')).toBeInTheDocument()
  })

  it('calls edit handler when edit button is clicked', () => {
    render(<WorkflowCard workflow={mockWorkflow} {...mockHandlers} />)
    
    const editButton = screen.getByText('Edit')
    fireEvent.click(editButton)
    expect(mockHandlers.onEdit).toHaveBeenCalledWith('1')
  })
})