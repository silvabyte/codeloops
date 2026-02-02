import { useRef, useEffect, useState, useCallback, type KeyboardEvent } from 'react'
import { cn } from '@/lib/utils'

export interface Message {
  id: string
  role: 'user' | 'assistant'
  content: string
}

interface ConversationProps {
  messages: Message[]
  onSend: (content: string) => void
  isLoading?: boolean
  disabled?: boolean
}

export function Conversation({ messages, onSend, isLoading, disabled }: ConversationProps) {
  const [input, setInput] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages, scrollToBottom])

  const handleSend = useCallback(() => {
    const trimmed = input.trim()
    if (!trimmed || disabled || isLoading) return
    onSend(trimmed)
    setInput('')
  }, [input, disabled, isLoading, onSend])

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault()
      handleSend()
    }
  }, [handleSend])

  return (
    <div className="flex flex-col h-full">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto py-8 space-y-6">
        {messages.map((message) => (
          <div
            key={message.id}
            className={cn(
              'max-w-[85%]',
              message.role === 'user' ? 'ml-auto' : 'mr-auto'
            )}
          >
            <p
              className={cn(
                'text-sm leading-relaxed whitespace-pre-wrap',
                message.role === 'assistant' ? 'text-muted-foreground' : 'text-foreground'
              )}
            >
              {message.content}
            </p>
          </div>
        ))}
        {isLoading && (
          <div className="mr-auto">
            <span className="text-sm text-muted-foreground/50">...</span>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="border-t border-border pt-4 pb-2">
        <div className="relative">
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type here..."
            disabled={disabled || isLoading}
            rows={3}
            className={cn(
              'w-full bg-transparent text-foreground placeholder:text-muted-foreground/50',
              'resize-none outline-none text-sm leading-relaxed',
              'disabled:opacity-50'
            )}
          />
          <span className="absolute bottom-0 right-0 text-xs text-muted-foreground/40">
            {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+↵
          </span>
        </div>
      </div>
    </div>
  )
}
