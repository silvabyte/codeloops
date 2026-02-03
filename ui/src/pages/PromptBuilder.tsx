import { useState, useEffect, useCallback, useMemo } from 'react'
import { cn } from '@/lib/utils'
import { WorkTypeSelector } from '@/components/prompt-builder/WorkTypeSelector'
import { Conversation } from '@/components/prompt-builder/Conversation'
import { PreviewPanel } from '@/components/prompt-builder/PreviewPanel'
import { PromptHistoryPanel } from '@/components/prompt-builder/PromptHistoryPanel'
import { SectionHeader } from '@/components/SectionHeader'
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
    clearError,
    newPrompt,
    loadPrompt,
  } = usePromptSession()

  const [historyOpen, setHistoryOpen] = useState(false)

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

  const handleNewPrompt = useCallback(async () => {
    await newPrompt()
  }, [newPrompt])

  const handleLoadPrompt = useCallback(async (id: string) => {
    await loadPrompt(id)
  }, [loadPrompt])

  // Derive values for the chat view (needed before early returns for hooks rules)
  const workType = 'workType' in state ? state.workType : session.workType
  const previewOpen = state.status === 'ready' ? (state as { previewOpen: boolean }).previewOpen : false
  const promptDraft = 'promptDraft' in state ? (state as { promptDraft: string }).promptDraft : session.promptDraft
  const isReady = state.status === 'ready'
  const keyboardHint = navigator.platform.includes('Mac') ? '⌘P' : 'Ctrl+P'

  // Build actions for the header (must be before any returns for hooks rules)
  const headerActions = useMemo(
    () => [
      {
        label: 'New',
        onClick: handleNewPrompt,
        disabled: !isReady,
      },
      {
        label: 'History',
        onClick: () => setHistoryOpen(true),
      },
      {
        label: previewOpen ? 'Hide Preview' : 'Preview',
        onClick: togglePreview,
        disabled: !isReady,
        active: previewOpen,
        hint: keyboardHint,
      },
    ],
    [handleNewPrompt, isReady, previewOpen, togglePreview, keyboardHint]
  )

  // Build header context - show prompt title if available, truncated with ellipsis
  const firstUserMessage = session.messages.find((m) => m.role === 'user')
  const promptTitle = firstUserMessage?.content.slice(0, 80) ||
    (promptDraft ? promptDraft.split('\n')[0].replace(/^#\s*/, '').slice(0, 80) : null)
  const headerContext = promptTitle ? (
    <h1 className="text-sm font-medium text-foreground truncate max-w-md" title={promptTitle}>
      {promptTitle}
    </h1>
  ) : 'Prompt Planner'

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
        <div className="flex flex-col h-[calc(100vh-65px)]">
          <SectionHeader
            context="Prompt Planner"
            actions={[
              { label: 'History', onClick: () => setHistoryOpen(true) },
            ]}
          />
          <div className="flex-1 overflow-auto">
            <div className="max-w-3xl mx-auto px-6 py-6">
              <WorkTypeSelector
                projectName={state.projectName}
                onSelect={selectWorkType}
              />
            </div>
          </div>
          {error && (
            <div className="fixed bottom-6 left-1/2 -translate-x-1/2 text-sm text-destructive bg-background/95 px-4 py-2 rounded-md border border-destructive/20">
              {error}
            </div>
          )}
          <PromptHistoryPanel
            isOpen={historyOpen}
            onClose={() => setHistoryOpen(false)}
            onSelect={handleLoadPrompt}
            currentProjectName={state.projectName}
          />
        </div>
      )

    case 'error':
      // For errors, show appropriate UI based on previous state
      if (state.previousState?.status === 'selecting_work_type' || !state.previousState) {
        return (
          <div className="flex flex-col h-[calc(100vh-65px)]">
            <SectionHeader
              context="Prompt Planner"
              actions={[
                { label: 'History', onClick: () => setHistoryOpen(true) },
              ]}
            />
            <div className="flex-1 overflow-auto">
              <div className="max-w-3xl mx-auto px-6 py-6">
                <WorkTypeSelector
                  projectName={state.projectName}
                  onSelect={selectWorkType}
                />
              </div>
            </div>
            <div className="fixed bottom-6 left-1/2 -translate-x-1/2 text-sm text-destructive bg-background/95 px-4 py-2 rounded-md border border-destructive/20">
              {state.error}
              <button
                onClick={clearError}
                className="ml-3 text-xs text-muted-foreground hover:text-foreground"
              >
                Dismiss
              </button>
            </div>
            <PromptHistoryPanel
              isOpen={historyOpen}
              onClose={() => setHistoryOpen(false)}
              onSelect={handleLoadPrompt}
              currentProjectName={state.projectName}
            />
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

  return (
    <div className="h-[calc(100vh-65px)] flex">
      {/* Conversation area */}
      <div
        className={cn(
          'flex flex-col transition-all duration-200',
          previewOpen ? 'w-1/2' : 'w-full'
        )}
      >
        <SectionHeader context={headerContext} actions={headerActions} />

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
        <div className="w-1/2">
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

      {/* History panel */}
      <PromptHistoryPanel
        isOpen={historyOpen}
        onClose={() => setHistoryOpen(false)}
        onSelect={handleLoadPrompt}
        currentProjectName={session.projectName}
      />
    </div>
  )
}
