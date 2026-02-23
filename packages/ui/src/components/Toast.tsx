import { createContext, useContext, useState, useCallback, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { Check, AlertCircle, Info, X } from 'lucide-react'

type ToastType = 'success' | 'error' | 'info'

interface Toast {
  id: string
  message: string
  type: ToastType
}

interface ToastContextValue {
  addToast: (message: string, type?: ToastType) => void
}

const ToastContext = createContext<ToastContextValue | null>(null)

export function useToast() {
  const ctx = useContext(ToastContext)
  if (!ctx) throw new Error('useToast must be used within ToastProvider')
  return ctx
}

const icons: Record<ToastType, typeof Check> = {
  success: Check,
  error: AlertCircle,
  info: Info,
}

const borderColors: Record<ToastType, string> = {
  success: 'border-success',
  error: 'border-destructive',
  info: 'border-border',
}

const iconColors: Record<ToastType, string> = {
  success: 'text-success',
  error: 'text-destructive',
  info: 'text-cyan',
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([])

  const addToast = useCallback((message: string, type: ToastType = 'success') => {
    const id = crypto.randomUUID()
    setToasts(prev => [...prev, { id, message, type }])
    setTimeout(() => {
      setToasts(prev => prev.filter(t => t.id !== id))
    }, 3000)
  }, [])

  const removeToast = useCallback((id: string) => {
    setToasts(prev => prev.filter(t => t.id !== id))
  }, [])

  return (
    <ToastContext.Provider value={{ addToast }}>
      {children}
      {createPortal(
        <div className="fixed bottom-6 right-6 z-[100] flex flex-col gap-2">
          {toasts.map(toast => {
            const Icon = icons[toast.type]
            return (
              <div
                key={toast.id}
                className={`toast-enter flex items-center gap-3 px-4 py-3 rounded-lg border ${borderColors[toast.type]} bg-surface shadow-lg min-w-[280px] max-w-[400px]`}
              >
                <Icon className={`w-4 h-4 shrink-0 ${iconColors[toast.type]}`} />
                <span className="text-sm text-foreground flex-1">{toast.message}</span>
                <button
                  onClick={() => removeToast(toast.id)}
                  className="p-0.5 text-dim hover:text-foreground transition-colors shrink-0"
                  aria-label="Dismiss"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </div>
            )
          })}
        </div>,
        document.body
      )}
    </ToastContext.Provider>
  )
}
