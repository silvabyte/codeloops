import { useCallback, useRef, useState } from 'react'
import { cn } from '@/lib/utils'

interface ResizableDividerProps {
  onResize: (percent: number) => void
}

export function ResizableDivider({ onResize }: ResizableDividerProps) {
  const [isDragging, setIsDragging] = useState(false)
  const dividerRef = useRef<HTMLDivElement>(null)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    setIsDragging(true)

    const handleMouseMove = (e: MouseEvent) => {
      const percent = (e.clientX / window.innerWidth) * 100
      onResize(percent)
    }

    const handleMouseUp = () => {
      setIsDragging(false)
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }

    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }, [onResize])

  return (
    <div
      ref={dividerRef}
      onMouseDown={handleMouseDown}
      className={cn(
        'relative w-0 shrink-0 cursor-col-resize group',
        'before:absolute before:inset-y-0 before:-left-1.5 before:w-3 before:z-10',
        isDragging && 'before:bg-amber/10'
      )}
    >
      {/* Visual line */}
      <div
        className={cn(
          'absolute top-0 bottom-0 left-0 w-px transition-colors',
          isDragging ? 'bg-amber-dim' : 'bg-border group-hover:bg-amber-dim/50'
        )}
      />
      {/* Grip dots on hover */}
      <div
        className={cn(
          'absolute top-1/2 -translate-y-1/2 -translate-x-1/2 flex flex-col gap-1 transition-opacity',
          isDragging ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
        )}
      >
        {[0, 1, 2].map(i => (
          <div key={i} className="w-1 h-1 rounded-full bg-amber-dim/50" />
        ))}
      </div>
    </div>
  )
}
