import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function formatDuration(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`
  const mins = Math.floor(secs / 60)
  const remaining = Math.round(secs % 60)
  return `${mins}m ${remaining}s`
}

export function formatDate(date: string): string {
  return new Date(date).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}
