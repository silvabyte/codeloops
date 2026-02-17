import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { cn } from '@/lib/utils'
import { Loader2 } from 'lucide-react'
import { useToast } from '@/components/Toast'
import { WorkTypeSelector } from '@/components/prompt-builder/WorkTypeSelector'
import { Conversation } from '@/components/prompt-builder/Conversation'
import { PreviewPanel } from '@/components/prompt-builder/PreviewPanel'
import { ResizableDivider } from '@/components/prompt-builder/ResizableDivider'
import { PromptHistoryPanel } from '@/components/prompt-builder/PromptHistoryPanel'
import { SectionHeader } from '@/components/SectionHeader'
import { usePromptSession } from '@/hooks/usePromptSession'

function LoadingSkeleton() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] gap-4">
      <Loader2 className="w-8 h-8 text-amber animate-spin" />
      <span className="text-sm text-dim animate-pulse">Loading workspace...</span>
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
    setParentIds,
    save,
    clearError,
    newPrompt,
    loadPrompt,
  } = usePromptSession()

  const { addToast } = useToast()
  const [historyOpen, setHistoryOpen] = useState(false)
  const [previewOpen, setPreviewOpen] = useState(true)
  const [splitPercent, setSplitPercent] = useState(50)
  const [isExiting, setIsExiting] = useState(false)
  const chatEnteredRef = useRef(false)

  // Keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Cmd/Ctrl + P: Toggle preview (only in ready state)
      if ((e.metaKey || e.ctrlKey) && e.key === 'p') {
        e.preventDefault()
        setPreviewOpen(prev => !prev)
        return
      }

      // Escape: Close preview or clear error
      if (e.key === 'Escape') {
        if (previewOpen) {
          e.preventDefault()
          setPreviewOpen(false)
        } else if (error) {
          e.preventDefault()
          clearError()
        }
        return
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [previewOpen, error, clearError])

  const handleSave = useCallback(async () => {
    const path = await save()
    if (path) {
      addToast(`Saved to ${path}`, 'success')
    }
  }, [save, addToast])

  const handleCopy = useCallback(() => {
    addToast('Copied to clipboard', 'success')
  }, [addToast])

  const handleDownload = useCallback(() => {
    addToast('Downloaded prompt.md', 'success')
  }, [addToast])

  const handleNewPrompt = useCallback(async () => {
    await newPrompt()
  }, [newPrompt])

  const handleLoadPrompt = useCallback(async (id: string) => {
    await loadPrompt(id)
  }, [loadPrompt])

  // Derive values for the chat view (needed before early returns for hooks rules)
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
        onClick: () => setPreviewOpen(prev => !prev),
        disabled: !isReady,
        active: previewOpen,
        hint: keyboardHint,
      },
    ],
    [handleNewPrompt, isReady, previewOpen, keyboardHint]
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
                onSelect={(type) => {
                  setIsExiting(true)
                  setTimeout(() => {
                    setIsExiting(false)
                    chatEnteredRef.current = false
                    selectWorkType(type)
                  }, 250)
                }}
                isExiting={isExiting}
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
                  isExiting={isExiting}
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

  // Track chat entrance animation (play once)
  const showChatEnter = !chatEnteredRef.current
  if (isInChatState) chatEnteredRef.current = true

  const isStreamingOrAwaiting = state.status === 'streaming' || state.status === 'awaiting_agent' || state.status === 'creating_session'

  return (
    <div className={cn('h-[calc(100vh-65px)] flex', showChatEnter && 'chat-enter')}>
      {/* Conversation area */}
      <div
        className="flex flex-col transition-all duration-300 min-w-0"
        style={{ width: previewOpen ? `${splitPercent}%` : '100%' }}
      >
        <SectionHeader context={headerContext} actions={headerActions} />

        {/* Streaming progress bar */}
        {isStreamingOrAwaiting && (
          <div className="h-0.5 w-full bg-border overflow-hidden">
            <div
              className="h-full w-1/2 bg-amber"
              style={{ animation: 'indeterminate 1.5s ease-in-out infinite' }}
            />
          </div>
        )}

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

      {/* Resizable divider */}
      {previewOpen && (
        <ResizableDivider
          onResize={(percent) => setSplitPercent(Math.max(30, Math.min(80, percent)))}
        />
      )}

      {/* Preview panel */}
      <div
        className="transition-all duration-300 overflow-hidden"
        style={{ width: previewOpen ? `${100 - splitPercent}%` : '0%' }}
      >
        {previewOpen && (
          <PreviewPanel
            content={promptDraft}
            onContentChange={updatePromptDraft}
            onSave={handleSave}
            onCopy={handleCopy}
            onDownload={handleDownload}
            isSaving={isSaving}
            isStreaming={isStreaming}
            promptId={session.id || undefined}
            parentIds={session.parentIds}
            onParentIdsChange={setParentIds}
          />
        )}
      </div>

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
