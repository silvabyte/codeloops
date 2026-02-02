import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { WorkTypeSelector, type WorkType } from '../WorkTypeSelector'

describe('WorkTypeSelector', () => {
  const mockOnSelect = vi.fn()
  const mockOnChangeProject = vi.fn()
  const defaultProps = {
    projectName: 'Test Project',
    onSelect: mockOnSelect,
    onChangeProject: mockOnChangeProject,
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('should render all work type options', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    expect(screen.getByText('Feature')).toBeInTheDocument()
    expect(screen.getByText('Defect')).toBeInTheDocument()
    expect(screen.getByText('Risk')).toBeInTheDocument()
    expect(screen.getByText('Debt')).toBeInTheDocument()
    expect(screen.getByText('Custom')).toBeInTheDocument()
  })

  it('should render header text', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    expect(screen.getByText('What are you building?')).toBeInTheDocument()
  })

  it('should display project name', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    expect(screen.getByText('Test Project')).toBeInTheDocument()
  })

  it('should call onSelect with correct work type when clicking Feature', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    fireEvent.click(screen.getByText('Feature'))

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('feature')
  })

  it('should call onSelect with correct work type when clicking Defect', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    fireEvent.click(screen.getByText('Defect'))

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('defect')
  })

  it('should call onSelect with correct work type when clicking Risk', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    fireEvent.click(screen.getByText('Risk'))

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('risk')
  })

  it('should call onSelect with correct work type when clicking Debt', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    fireEvent.click(screen.getByText('Debt'))

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('debt')
  })

  it('should call onSelect with correct work type when clicking Custom', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    fireEvent.click(screen.getByText('Custom'))

    expect(mockOnSelect).toHaveBeenCalledTimes(1)
    expect(mockOnSelect).toHaveBeenCalledWith('custom')
  })

  it('should call onChangeProject when clicking project name', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    fireEvent.click(screen.getByText('Test Project'))

    expect(mockOnChangeProject).toHaveBeenCalledTimes(1)
  })

  it('should render all work type buttons', () => {
    render(<WorkTypeSelector {...defaultProps} />)

    const buttons = screen.getAllByRole('button')
    // 5 work type buttons + 1 project name button
    expect(buttons).toHaveLength(6)
  })

  it('should handle each work type correctly', () => {
    const workTypes: WorkType[] = ['feature', 'defect', 'risk', 'debt', 'custom']
    const labels = ['Feature', 'Defect', 'Risk', 'Debt', 'Custom']

    render(<WorkTypeSelector {...defaultProps} />)

    labels.forEach((label, index) => {
      fireEvent.click(screen.getByText(label))
      expect(mockOnSelect).toHaveBeenLastCalledWith(workTypes[index])
    })

    expect(mockOnSelect).toHaveBeenCalledTimes(5)
  })
})
