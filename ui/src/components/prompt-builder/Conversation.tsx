import { useRef, useEffect, useState, useCallback, type KeyboardEvent } from 'react'
import ReactMarkdown from 'react-markdown'
import { cn } from '@/lib/utils'
import { markdownChat } from '@/lib/markdown-styles'
import { Bot, User, ArrowUp, MessageSquare } from 'lucide-react'
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
  const initialMessageCountRef = useRef(messages.length)

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [])

  useEffect(() => {
    scrollToBottom()
  }, [messages, showTypingIndicator, scrollToBottom])

  // Auto-resize textarea
  useEffect(() => {
    const el = textareaRef.current
    if (!el) return
    el.style.height = 'auto'
    el.style.height = Math.min(el.scrollHeight, 128) + 'px'
  }, [input])

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

  const isEmpty = messages.length === 0 && !showTypingIndicator

  return (
    <div className="flex flex-col h-full">
      {/* Messages */}
      <div className="flex-1 overflow-y-auto py-8 space-y-4">
        {isEmpty && (
          <div className="flex flex-col items-center justify-center h-full gap-4 text-center">
            <div className="w-12 h-12 rounded-full bg-amber/10 flex items-center justify-center">
              <MessageSquare className="w-6 h-6 text-amber" />
            </div>
            <div>
              <p className="text-sm text-muted-foreground">Start a conversation to build your prompt</p>
              <p className="text-xs text-dim mt-1">Describe what you're working on and the agent will help structure it</p>
            </div>
          </div>
        )}

        {messages.map((message, index) => {
          const isNew = index >= initialMessageCountRef.current
          const isUser = message.role === 'user'

          return (
            <div
              key={message.id}
              className={cn(
                'flex gap-3 max-w-[85%]',
                isUser ? 'ml-auto flex-row-reverse' : 'mr-auto',
                isNew && 'message-enter'
              )}
            >
              {/* Avatar */}
              <div
                className={cn(
                  'w-7 h-7 rounded-full flex items-center justify-center shrink-0 mt-0.5',
                  isUser ? 'bg-amber/10' : 'bg-cyan/10'
                )}
              >
                {isUser ? (
                  <User className="w-3.5 h-3.5 text-amber" />
                ) : (
                  <Bot className="w-3.5 h-3.5 text-cyan" />
                )}
              </div>

              {/* Bubble */}
              <div
                className={cn(
                  'rounded-lg px-3.5 py-2.5 text-sm leading-relaxed max-w-none',
                  isUser
                    ? 'bg-amber/5 border border-amber/10 text-foreground'
                    : 'bg-surface border border-border-subtle text-muted-foreground',
                  markdownChat
                )}
              >
                <ReactMarkdown>{message.content}</ReactMarkdown>
              </div>
            </div>
          )
        })}
        {showTypingIndicator && <TypingIndicator />}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="pb-3 pt-2">
        <div
          className={cn(
            'flex items-end gap-2 rounded-lg border bg-surface px-3 py-2 transition-colors',
            'border-border focus-within:border-amber-dim focus-within:ring-1 focus-within:ring-amber/20'
          )}
        >
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Describe what you're building..."
            disabled={disabled || isLoading || showTypingIndicator}
            rows={1}
            className={cn(
              'flex-1 bg-transparent text-foreground placeholder:text-muted-foreground/50',
              'resize-none outline-none text-sm leading-relaxed max-h-32',
              'disabled:opacity-50'
            )}
          />
          <button
            onClick={handleSend}
            disabled={!input.trim() || disabled || isLoading || showTypingIndicator}
            className={cn(
              'p-1.5 rounded-md transition-colors shrink-0',
              input.trim() && !disabled
                ? 'bg-amber text-background hover:bg-amber-bright'
                : 'text-dim'
            )}
            aria-label="Send message"
          >
            <ArrowUp className="w-4 h-4" />
          </button>
        </div>
        <p className="text-[10px] text-dim/50 mt-1 ml-1">Shift+Enter for new line</p>
      </div>
    </div>
  )
}
