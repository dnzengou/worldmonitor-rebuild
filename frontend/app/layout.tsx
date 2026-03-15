import type { Metadata } from 'next'
import './globals.css'

export const metadata: Metadata = {
  title: 'WorldMonitor Agents - AI-Powered Intelligence',
  description: 'Chain-of-Thought Multi-Agent System for real-time global intelligence',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-background text-foreground">
        {children}
      </body>
    </html>
  )
}
