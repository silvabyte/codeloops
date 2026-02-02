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
    session,
    isLoading,
    isSaving,
    error,
    contextLoading,
    selectWorkType,
    sendMessage,
    updatePromptDraft,
    togglePreview,
    closePreview,
    save,
    reset,
  } = usePromptSession()

  // Keyboard shortcuts
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Cmd/Ctrl + P: Toggle preview
      if ((e.metaKey || e.ctrlKey) && e.key === 'p') {
        e.preventDefault()
        togglePreview()
        return
      }

      // Escape: Close preview or clear focus
      if (e.key === 'Escape') {
        if (session.previewOpen) {
          e.preventDefault()
          closePreview()
        }
        return
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [session.previewOpen, togglePreview, closePreview])

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

  if (contextLoading) {
    return (
      <div className="max-w-3xl mx-auto px-6">
        <LoadingSkeleton />
      </div>
    )
  }

  // Selecting state - show work type selector
  if (session.status === 'selecting') {
    return (
      <div className="max-w-3xl mx-auto px-6">
        <WorkTypeSelector
          projectName={session.projectName}
          onSelect={selectWorkType}
        />
        {error && (
          <div className="fixed bottom-6 left-1/2 -translate-x-1/2 text-sm text-destructive">
            {error}
          </div>
        )}
      </div>
    )
  }

  // Chatting state - show conversation with optional preview panel
  return (
    <div className="h-[calc(100vh-65px)] flex">
      {/* Conversation area */}
      <div
        className={cn(
          'flex flex-col transition-all duration-200',
          session.previewOpen ? 'w-[60%]' : 'w-full'
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
              {session.workType}
            </span>
          </div>
          <button
            onClick={togglePreview}
            className={cn(
              'text-sm transition-colors',
              session.previewOpen
                ? 'text-foreground'
                : 'text-muted-foreground hover:text-foreground'
            )}
          >
            {session.previewOpen ? 'Hide preview' : 'View prompt'}
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
            isLoading={isLoading}
          />
        </div>

        {/* Error */}
        {error && (
          <div className="px-6 py-2 text-sm text-destructive">
            {error}
          </div>
        )}
      </div>

      {/* Preview panel */}
      {session.previewOpen && (
        <div className="w-[40%]">
          <PreviewPanel
            content={session.promptDraft}
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
