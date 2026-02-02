import { useRef, useEffect, useState, useCallback, type KeyboardEvent } from 'react'
import ReactMarkdown from 'react-markdown'
import { cn } from '@/lib/utils'
import { TypingIndicator } from './TypingIndicator'

export interface Message {
  id: string
  role: 'user' | 'assistant'
  content: string
}

interface ConversationProps {
  messages: Message[]
  onSend: (content: string) => void
  isLoading?: boolean
  showTypingIndicator?: boolean
  disabled?: boolean
}

export function Conversation({
  messages,
  onSend,
  isLoading,
  showTypingIndicator,
  disabled,
}: ConversationProps) {
  const [input, setInput] = useState('')
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages, showTypingIndicator, scrollToBottom])

  const handleSend = useCallback(() => {
    const trimmed = input.trim()
    if (!trimmed || disabled || isLoading) return
    onSend(trimmed)
    setInput('')
  }, [input, disabled, isLoading, onSend])

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
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
            <div
              className={cn(
                'text-sm leading-relaxed max-w-none',
                message.role === 'assistant' ? 'text-muted-foreground' : 'text-foreground',
                // Markdown element styling
                '[&_p]:mb-2 [&_p:last-child]:mb-0',
                '[&_pre]:bg-elevated [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-2',
                '[&_code]:bg-elevated [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs',
                '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
                '[&_a]:text-cyan [&_a]:no-underline hover:[&_a]:underline',
                '[&_ul]:list-disc [&_ul]:pl-4 [&_ul]:my-2',
                '[&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:my-2',
                '[&_li]:mb-1',
                '[&_strong]:text-foreground [&_strong]:font-semibold',
                '[&_h1]:text-lg [&_h1]:font-semibold [&_h1]:text-foreground [&_h1]:mb-2',
                '[&_h2]:text-base [&_h2]:font-semibold [&_h2]:text-foreground [&_h2]:mb-2',
                '[&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:mb-1',
                '[&_blockquote]:border-l-2 [&_blockquote]:border-amber/50 [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:my-2'
              )}
            >
              <ReactMarkdown>{message.content}</ReactMarkdown>
            </div>
          </div>
        ))}
        {showTypingIndicator && <TypingIndicator />}
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
            disabled={disabled || isLoading || showTypingIndicator}
            rows={3}
            className={cn(
              'w-full bg-transparent text-foreground placeholder:text-muted-foreground/50',
              'resize-none outline-none text-sm leading-relaxed',
              'disabled:opacity-50'
            )}
          />
          <span
            className="absolute bottom-0 right-0 text-xs text-muted-foreground/40"
            title="Enter to send"
          >
            ↵
          </span>
        </div>
      </div>
    </div>
  )
}
