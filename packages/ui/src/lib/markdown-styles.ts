/**
 * Shared markdown styling classes for ReactMarkdown containers.
 * Avoids duplicating long Tailwind class strings across components.
 */

/** Base styles shared by all markdown renderers */
export const markdownBase = [
  '[&_p]:mb-2 [&_p:last-child]:mb-0',
  '[&_pre]:bg-elevated [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-2',
  '[&_code]:bg-elevated [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs',
  '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
  '[&_a]:text-cyan [&_a]:no-underline hover:[&_a]:underline',
  '[&_ul]:list-disc [&_ul]:pl-4 [&_ul]:my-2',
  '[&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:my-2',
  '[&_li]:mb-1',
  '[&_strong]:text-foreground [&_strong]:font-semibold',
  '[&_blockquote]:border-l-2 [&_blockquote]:border-amber/50 [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:my-2',
].join(' ')

/** Chat message styles — compact headings, no extra spacing */
export const markdownChat = [
  markdownBase,
  '[&_h1]:text-lg [&_h1]:font-semibold [&_h1]:text-foreground [&_h1]:mb-2',
  '[&_h2]:text-base [&_h2]:font-semibold [&_h2]:text-foreground [&_h2]:mb-2',
  '[&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:mb-1',
].join(' ')

/** Preview panel styles — more spacious, larger headings */
export const markdownPreview = [
  '[&_p]:mb-3 [&_p:last-child]:mb-0',
  '[&_pre]:bg-elevated [&_pre]:p-3 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-3',
  '[&_code]:bg-elevated [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs [&_code]:font-mono',
  '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
  '[&_a]:text-cyan [&_a]:no-underline hover:[&_a]:underline',
  '[&_ul]:list-disc [&_ul]:pl-5 [&_ul]:my-3',
  '[&_ol]:list-decimal [&_ol]:pl-5 [&_ol]:my-3',
  '[&_li]:mb-1',
  '[&_strong]:text-foreground [&_strong]:font-semibold',
  '[&_h1]:text-xl [&_h1]:font-semibold [&_h1]:text-foreground [&_h1]:mb-3 [&_h1]:mt-4 first:[&_h1]:mt-0',
  '[&_h2]:text-lg [&_h2]:font-semibold [&_h2]:text-foreground [&_h2]:mb-2 [&_h2]:mt-4 first:[&_h2]:mt-0',
  '[&_h3]:text-base [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:mb-2 [&_h3]:mt-3 first:[&_h3]:mt-0',
  '[&_blockquote]:border-l-2 [&_blockquote]:border-amber/50 [&_blockquote]:pl-3 [&_blockquote]:italic [&_blockquote]:my-3',
  '[&_hr]:border-border [&_hr]:my-4',
].join(' ')

/** Inherited content preview — compact, slightly dimmed */
export const markdownInherited = [
  '[&_p]:mb-2 [&_p:last-child]:mb-0',
  '[&_pre]:bg-elevated [&_pre]:p-2 [&_pre]:rounded-lg [&_pre]:overflow-x-auto [&_pre]:my-2',
  '[&_code]:bg-elevated [&_code]:px-1 [&_code]:py-0.5 [&_code]:rounded [&_code]:text-amber [&_code]:text-xs [&_code]:font-mono',
  '[&_pre_code]:bg-transparent [&_pre_code]:p-0 [&_pre_code]:text-foreground',
  '[&_ul]:list-disc [&_ul]:pl-4 [&_ul]:my-2',
  '[&_ol]:list-decimal [&_ol]:pl-4 [&_ol]:my-2',
  '[&_li]:mb-0.5',
  '[&_strong]:text-foreground/90 [&_strong]:font-semibold',
  '[&_h1]:text-base [&_h1]:font-semibold [&_h1]:text-foreground/90 [&_h1]:mb-2 [&_h1]:mt-3 first:[&_h1]:mt-0',
  '[&_h2]:text-sm [&_h2]:font-semibold [&_h2]:text-foreground/90 [&_h2]:mb-1 [&_h2]:mt-3 first:[&_h2]:mt-0',
  '[&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-foreground/90 [&_h3]:mb-1 [&_h3]:mt-2 first:[&_h3]:mt-0',
].join(' ')
