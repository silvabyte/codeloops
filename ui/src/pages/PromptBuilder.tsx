import { useEffect, useCallback } from 'react'
import { cn } from '@/lib/utils'
import { WorkTypeSelector } from '@/components/prompt-builder/WorkTypeSelector'
import { Conversation } from '@/components/prompt-builder/Conversation'
import { PreviewPanel } from '@/components/prompt-builder/PreviewPanel'
import { usePromptSession } from '@/hooks/usePromptSession'

function LoadingSkeleton() {
  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <div className="text-muted-foreground/50">Loading...</div>
    </div>
  )
}

export function PromptBuilder() {
  const {
    state,
    session,
    isSaving,
    error,
    isStreaming,
    selectWorkType,
    sendMessage,
    updatePromptDraft,
    togglePreview,
    closePreview,
    save,
    reset,
    clearError,
  } = usePromptSession()

  // Keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Cmd/Ctrl + P: Toggle preview (only in ready state)
      if ((e.metaKey || e.ctrlKey) && e.key === 'p') {
        e.preventDefault()
        togglePreview()
        return
      }

      // Escape: Close preview or clear error
      if (e.key === 'Escape') {
        if (session.previewOpen) {
          e.preventDefault()
          closePreview()
        } else if (error) {
          e.preventDefault()
          clearError()
        }
        return
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [session.previewOpen, togglePreview, closePreview, error, clearError])

  const handleSave = useCallback(async () => {
    const path = await save()
    if (path) {
      // TODO: Show toast notification
      console.log(`Saved to ${path}`)
    }
  }, [save])

  const handleCopy = useCallback(() => {
    // TODO: Show toast notification
    console.log('Copied to clipboard')
  }, [])

  const handleDownload = useCallback(() => {
    // TODO: Show toast notification
    console.log('Downloaded prompt.md')
  }, [])

  // State machine driven rendering
  switch (state.status) {
    case 'loading_context':
      return (
        <div className="max-w-3xl mx-auto px-6">
          <LoadingSkeleton />
        </div>
      )

    case 'selecting_work_type':
      return (
        <div className="max-w-3xl mx-auto px-6">
          <WorkTypeSelector
            projectName={state.projectName}
            onSelect={selectWorkType}
          />
          {error && (
            <div className="fixed bottom-6 left-1/2 -translate-x-1/2 text-sm text-destructive">
              {error}
            </div>
          )}
        </div>
      )

    case 'error':
      // For errors, show appropriate UI based on previous state
      if (state.previousState?.status === 'selecting_work_type' || !state.previousState) {
        return (
          <div className="max-w-3xl mx-auto px-6">
            <WorkTypeSelector
              projectName={state.projectName}
              onSelect={selectWorkType}
            />
            <div className="fixed bottom-6 left-1/2 -translate-x-1/2 text-sm text-destructive bg-background/95 px-4 py-2 rounded-md border border-destructive/20">
              {state.error}
              <button
                onClick={clearError}
                className="ml-3 text-xs text-muted-foreground hover:text-foreground"
              >
                Dismiss
              </button>
            </div>
          </div>
        )
      }
      // Fall through to chat view for errors during chat
      // (handled below with chat states)
      break

    case 'creating_session':
    case 'awaiting_agent':
    case 'streaming':
    case 'ready':
      // All chat states render the same layout, with different loading indicators
      break
  }

  // Chat view (creating_session, awaiting_agent, streaming, ready, or error during chat)
  const showTypingIndicator = state.status === 'creating_session' || state.status === 'awaiting_agent'
  const isInChatState = state.status === 'creating_session' ||
    state.status === 'awaiting_agent' ||
    state.status === 'streaming' ||
    state.status === 'ready' ||
    (state.status === 'error' && state.previousState?.status !== 'selecting_work_type')

  if (!isInChatState) {
    // Fallback - shouldn't reach here
    return (
      <div className="max-w-3xl mx-auto px-6">
        <LoadingSkeleton />
      </div>
    )
  }

  const workType = 'workType' in state ? state.workType : session.workType
  const previewOpen = state.status === 'ready' ? state.previewOpen : false
  const promptDraft = 'promptDraft' in state ? state.promptDraft : session.promptDraft

  return (
    <div className="h-[calc(100vh-65px)] flex">
      {/* Conversation area */}
      <div
        className={cn(
          'flex flex-col transition-all duration-200',
          previewOpen ? 'w-[60%]' : 'w-full'
        )}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-3 border-b border-border">
          <div className="flex items-center gap-4">
            <button
              onClick={reset}
              className="text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Start over
            </button>
            <span className="text-xs text-muted-foreground/50">
              {workType}
            </span>
          </div>
          <button
            onClick={togglePreview}
            disabled={state.status !== 'ready'}
            className={cn(
              'text-sm transition-colors',
              state.status !== 'ready'
                ? 'text-muted-foreground/30 cursor-not-allowed'
                : previewOpen
                  ? 'text-foreground'
                  : 'text-muted-foreground hover:text-foreground'
            )}
          >
            {previewOpen ? 'Hide preview' : 'View prompt'}
            <span className="ml-2 text-xs text-muted-foreground/50">
              {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}+P
            </span>
          </button>
        </div>

        {/* Conversation */}
        <div className="flex-1 max-w-3xl mx-auto w-full px-6 overflow-hidden">
          <Conversation
            messages={session.messages}
            onSend={sendMessage}
            isLoading={isStreaming}
            showTypingIndicator={showTypingIndicator}
            disabled={state.status !== 'ready'}
          />
        </div>

        {/* Error */}
        {error && (
          <div className="px-6 py-2 text-sm text-destructive flex items-center justify-between">
            <span>{error}</span>
            <button
              onClick={clearError}
              className="text-xs text-muted-foreground hover:text-foreground ml-4"
            >
              Dismiss
            </button>
          </div>
        )}
      </div>

      {/* Preview panel */}
      {previewOpen && (
        <div className="w-[40%]">
          <PreviewPanel
            content={promptDraft}
            onContentChange={updatePromptDraft}
            onSave={handleSave}
            onCopy={handleCopy}
            onDownload={handleDownload}
            isSaving={isSaving}
          />
        </div>
      )}
    </div>
  )
}
