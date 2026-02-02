/**
 * Branded typing indicator matching the Codeloops "Amber Circuit" design system.
 *
 * Uses amber color with glow effect and staggered wave animation.
 * Appears as the first "message" in the chat interface while waiting for agent response.
 */
export function TypingIndicator() {
  return (
    <div className="mr-auto">
      <div className="typing-indicator">
        <div className="typing-dot" />
        <div className="typing-dot" />
        <div className="typing-dot" />
      </div>
    </div>
  )
}
