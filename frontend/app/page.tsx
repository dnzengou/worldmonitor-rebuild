'use client'

import { useState, useEffect } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { 
  Zap, Globe, Brain, Bell, Shield, 
  TrendingUp, AlertTriangle, CheckCircle,
  ChevronRight, Sparkles, Lock, Menu, X
} from 'lucide-react'
import { cn } from '@/lib/utils'

// Types
interface Event {
  id: string
  country: string
  lat: number
  lon: number
  severity: number
  headline: string
  source: string
  timestamp: number
}

interface AgentStep {
  step_number: number
  agent_type: string
  thought: string
  action: string
  observation: string
  confidence: number
}

interface Brief {
  country: string
  summary: string
  event_count: number
  risk_level: string
  chain_of_thought?: {
    steps: AgentStep[]
  }
}

interface User {
  id: string
  tier: 'free' | 'pro' | 'enterprise'
  streak: number
  interests: string[]
  max_alerts: number
  data_delay_hours: number
  is_new: boolean
}

// API client
const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

const api = {
  getIntelligence: async (): Promise<Event[]> => {
    const res = await fetch(`${API_URL}/api/intelligence`)
    const data = await res.json()
    return data.events || []
  },

  getBrief: async (country: string, includeReasoning = false): Promise<Brief> => {
    const res = await fetch(`${API_URL}/api/brief`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ country, include_reasoning: includeReasoning }),
    })
    return res.json()
  },

  getUser: async (): Promise<User> => {
    const res = await fetch(`${API_URL}/api/user`)
    return res.json()
  },

  getPricing: async () => {
    const res = await fetch(`${API_URL}/api/pricing`)
    return res.json()
  },
}

// Components
function AgentVisualization({ steps }: { steps: AgentStep[] }) {
  const [activeStep, setActiveStep] = useState(0)

  useEffect(() => {
    if (steps.length === 0) return
    const interval = setInterval(() => {
      setActiveStep((prev) => (prev + 1) % steps.length)
    }, 2000)
    return () => clearInterval(interval)
  }, [steps.length])

  const getAgentIcon = (type: string) => {
    switch (type) {
      case 'DataCollector': return <Globe className="w-5 h-5" />
      case 'Analyst': return <Brain className="w-5 h-5" />
      case 'Forecaster': return <TrendingUp className="w-5 h-5" />
      case 'Notifier': return <Bell className="w-5 h-5" />
      case 'Validator': return <Shield className="w-5 h-5" />
      default: return <Zap className="w-5 h-5" />
    }
  }

  const getAgentColor = (type: string) => {
    switch (type) {
      case 'DataCollector': return 'bg-blue-500'
      case 'Analyst': return 'bg-purple-500'
      case 'Forecaster': return 'bg-green-500'
      case 'Notifier': return 'bg-orange-500'
      case 'Validator': return 'bg-red-500'
      default: return 'bg-gray-500'
    }
  }

  return (
    <div className="space-y-4">
      {/* Agent nodes */}
      <div className="flex items-center justify-center gap-4 flex-wrap">
        {['DataCollector', 'Analyst', 'Forecaster', 'Notifier', 'Validator'].map((agent, i) => {
          const isActive = steps[activeStep]?.agent_type === agent
          return (
            <motion.div
              key={agent}
              className={cn(
                'agent-node',
                isActive && 'active scale-110'
              )}
              animate={{
                scale: isActive ? 1.1 : 1,
                opacity: steps.length > 0 ? 1 : 0.5,
              }}
            >
              <div className={cn(
                'w-12 h-12 rounded-full flex items-center justify-center',
                getAgentColor(agent),
                isActive && 'ring-2 ring-white/50'
              )}>
                {getAgentIcon(agent)}
              </div>
              <span className="absolute -bottom-6 text-xs text-gray-400 whitespace-nowrap">
                {agent}
              </span>
            </motion.div>
          )
        })}
      </div>

      {/* Chain of thought steps */}
      <div className="mt-8 space-y-4 max-h-64 overflow-y-auto">
        <AnimatePresence mode="wait">
          {steps.slice(0, activeStep + 1).map((step, index) => (
            <motion.div
              key={index}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 20 }}
              className="cot-step"
            >
              <div className="flex items-center gap-2 mb-1">
                <span className={cn(
                  'text-xs px-2 py-0.5 rounded',
                  getAgentColor(step.agent_type),
                  'text-white'
                )}>
                  {step.agent_type}
                </span>
                <span className="text-xs text-gray-500">
                  Confidence: {(step.confidence * 100).toFixed(0)}%
                </span>
              </div>
              <p className="text-sm text-gray-300">{step.thought}</p>
              <p className="text-xs text-gray-500 mt-1">{step.observation}</p>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  )
}

function MapVisualization({ events, onSelect }: { events: Event[]; onSelect: (e: Event) => void }) {
  const [hovered, setHovered] = useState<string | null>(null)

  // Simple projection (equirectangular)
  const project = (lat: number, lon: number) => ({
    x: ((lon + 180) / 360) * 100,
    y: ((90 - lat) / 180) * 100,
  })

  const getSeverityColor = (severity: number) => {
    if (severity >= 8) return '#ef4444'
    if (severity >= 5) return '#f59e0b'
    return '#3b82f6'
  }

  return (
    <div className="relative w-full aspect-[2/1] bg-[#0f172a] rounded-xl overflow-hidden border border-border">
      {/* Grid */}
      <svg className="absolute inset-0 w-full h-full">
        <defs>
          <pattern id="grid" width="50" height="50" patternUnits="userSpaceOnUse">
            <path d="M 50 0 L 0 0 0 50" fill="none" stroke="#1e293b" strokeWidth="1"/>
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#grid)" />
      </svg>

      {/* Event points */}
      {events.map((event) => {
        const pos = project(event.lat, event.lon)
        const isHovered = hovered === event.id
        return (
          <motion.button
            key={event.id}
            className="absolute transform -translate-x-1/2 -translate-y-1/2"
            style={{
              left: `${pos.x}%`,
              top: `${pos.y}%`,
            }}
            onMouseEnter={() => setHovered(event.id)}
            onMouseLeave={() => setHovered(null)}
            onClick={() => onSelect(event)}
            whileHover={{ scale: 1.5 }}
          >
            <div
              className="rounded-full"
              style={{
                width: Math.max(8, event.severity * 2),
                height: Math.max(8, event.severity * 2),
                backgroundColor: getSeverityColor(event.severity),
                boxShadow: `0 0 ${isHovered ? 20 : 10}px ${getSeverityColor(event.severity)}`,
              }}
            />
            
            {/* Tooltip */}
            <AnimatePresence>
              {isHovered && (
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: 10 }}
                  className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 z-50"
                >
                  <div className="glass rounded-lg p-3 min-w-[200px]">
                    <div className="flex items-center gap-2 mb-1">
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: getSeverityColor(event.severity) }}
                      />
                      <span className="font-medium">{event.country}</span>
                    </div>
                    <p className="text-sm text-gray-300">{event.headline}</p>
                    <p className="text-xs text-gray-500 mt-1">
                      Severity: {event.severity}/10
                    </p>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </motion.button>
        )
      })}

      {/* Legend */}
      <div className="absolute bottom-4 left-4 glass rounded-lg p-3">
        <div className="flex items-center gap-4 text-xs">
          <div className="flex items-center gap-1">
            <div className="w-3 h-3 rounded-full bg-red-500" />
            <span>Critical (8-10)</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-3 h-3 rounded-full bg-yellow-500" />
            <span>Elevated (5-7)</span>
          </div>
          <div className="flex items-center gap-1">
            <div className="w-3 h-3 rounded-full bg-blue-500" />
            <span>Normal (1-4)</span>
          </div>
        </div>
      </div>
    </div>
  )
}

function Onboarding({ onComplete }: { onComplete: () => void }) {
  const [step, setStep] = useState(0)
  const [interests, setInterests] = useState<string[]>([])

  const steps = [
    {
      title: 'Welcome to WorldMonitor Agents',
      description: 'AI-powered global intelligence with chain-of-thought reasoning',
      content: (
        <div className="space-y-4">
          <div className="flex justify-center gap-4">
            {['DataCollector', 'Analyst', 'Forecaster'].map((agent) => (
              <div key={agent} className="text-center">
                <div className="w-16 h-16 rounded-full bg-primary/20 flex items-center justify-center mb-2">
                  {agent === 'DataCollector' && <Globe className="w-8 h-8" />}
                  {agent === 'Analyst' && <Brain className="w-8 h-8" />}
                  {agent === 'Forecaster' && <TrendingUp className="w-8 h-8" />}
                </div>
                <span className="text-xs text-gray-400">{agent}</span>
              </div>
            ))}
          </div>
          <p className="text-gray-400 text-center">
            Our multi-agent system fuses data from 150+ sources and explains its reasoning
          </p>
        </div>
      ),
    },
    {
      title: 'What interests you?',
      description: 'Select topics for personalized intelligence',
      content: (
        <div className="grid grid-cols-2 gap-3">
          {['Security', 'Finance', 'Climate', 'Technology', 'Politics', 'Health'].map((topic) => (
            <button
              key={topic}
              onClick={() => {
                if (interests.includes(topic)) {
                  setInterests(interests.filter((i) => i !== topic))
                } else {
                  setInterests([...interests, topic])
                }
              }}
              className={cn(
                'p-4 rounded-lg border transition-all text-left',
                interests.includes(topic)
                  ? 'border-primary bg-primary/10'
                  : 'border-border hover:border-primary/50'
              )}
            >
              <span className="font-medium">{topic}</span>
            </button>
          ))}
        </div>
      ),
    },
    {
      title: 'You\'re all set!',
      description: 'Start monitoring global intelligence',
      content: (
        <div className="text-center space-y-4">
          <div className="w-20 h-20 rounded-full bg-green-500/20 flex items-center justify-center mx-auto">
            <CheckCircle className="w-10 h-10 text-green-500" />
          </div>
          <p className="text-gray-400">
            Your personalized dashboard is ready. You\'ll receive daily briefings on your selected topics.
          </p>
        </div>
      ),
    },
  ]

  return (
    <div className="fixed inset-0 bg-background/95 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="max-w-md w-full glass rounded-2xl p-8"
      >
        <div className="flex justify-center mb-4">
          <div className="flex gap-2">
            {steps.map((_, i) => (
              <div
                key={i}
                className={cn(
                  'w-2 h-2 rounded-full transition-colors',
                  i === step ? 'bg-primary' : 'bg-gray-600'
                )}
              />
            ))}
          </div>
        </div>

        <h2 className="text-2xl font-bold text-center mb-2">{steps[step].title}</h2>
        <p className="text-gray-400 text-center mb-6">{steps[step].description}</p>

        <div className="mb-8">{steps[step].content}</div>

        <div className="flex gap-3">
          {step > 0 && (
            <button
              onClick={() => setStep(step - 1)}
              className="flex-1 py-3 px-4 rounded-lg border border-border hover:bg-card transition-colors"
            >
              Back
            </button>
          )}
          <button
            onClick={() => {
              if (step < steps.length - 1) {
                setStep(step + 1)
              } else {
                onComplete()
              }
            }}
            disabled={step === 1 && interests.length === 0}
            className="flex-1 py-3 px-4 rounded-lg bg-primary hover:bg-primary-light transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {step === steps.length - 1 ? 'Get Started' : 'Continue'}
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>
      </motion.div>
    </div>
  )
}

function PricingModal({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const [pricing, setPricing] = useState<any>(null)

  useEffect(() => {
    if (isOpen) {
      api.getPricing().then(setPricing)
    }
  }, [isOpen])

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 bg-background/95 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        className="max-w-4xl w-full glass rounded-2xl p-8 max-h-[90vh] overflow-y-auto"
      >
        <div className="flex justify-between items-center mb-8">
          <h2 className="text-2xl font-bold">Choose Your Plan</h2>
          <button onClick={onClose} className="p-2 hover:bg-card rounded-lg">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="grid md:grid-cols-3 gap-6">
          {pricing?.tiers?.map((tier: any) => (
            <div
              key={tier.id}
              className={cn(
                'rounded-xl border p-6',
                tier.id === 'pro'
                  ? 'border-primary bg-primary/5'
                  : 'border-border'
              )}
            >
              <h3 className="text-xl font-bold mb-2">{tier.name}</h3>
              <p className="text-3xl font-bold mb-4">{tier.price_string}</p>
              <ul className="space-y-2 mb-6">
                {tier.features.map((feature: string, i: number) => (
                  <li key={i} className="flex items-center gap-2 text-sm text-gray-400">
                    <CheckCircle className="w-4 h-4 text-green-500" />
                    {feature}
                  </li>
                ))}
              </ul>
              <button
                className={cn(
                  'w-full py-3 rounded-lg font-medium transition-colors',
                  tier.id === 'pro'
                    ? 'bg-primary hover:bg-primary-light'
                    : 'border border-border hover:bg-card'
                )}
              >
                {tier.id === 'free' ? 'Current Plan' : 'Upgrade'}
              </button>
            </div>
          ))}
        </div>
      </motion.div>
    </div>
  )
}

// Main app
export default function Home() {
  const [events, setEvents] = useState<Event[]>([])
  const [user, setUser] = useState<User | null>(null)
  const [selectedCountry, setSelectedCountry] = useState<string | null>(null)
  const [brief, setBrief] = useState<Brief | null>(null)
  const [showOnboarding, setShowOnboarding] = useState(false)
  const [showPricing, setShowPricing] = useState(false)
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'map' | 'brief'>('map')
  const [showReasoning, setShowReasoning] = useState(false)

  useEffect(() => {
    loadData()
  }, [])

  const loadData = async () => {
    try {
      const [eventsData, userData] = await Promise.all([
        api.getIntelligence(),
        api.getUser(),
      ])
      setEvents(eventsData)
      setUser(userData)
      if (userData.is_new) {
        setShowOnboarding(true)
      }
    } catch (error) {
      console.error('Failed to load data:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleCountrySelect = async (event: Event) => {
    setSelectedCountry(event.country)
    setActiveTab('brief')
    setLoading(true)
    try {
      const briefData = await api.getBrief(event.country, showReasoning)
      setBrief(briefData)
    } catch (error) {
      console.error('Failed to get brief:', error)
    } finally {
      setLoading(false)
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="w-12 h-12 border-4 border-primary border-t-transparent rounded-full animate-spin mx-auto mb-4" />
          <p className="text-gray-400">Loading intelligence data...</p>
        </div>
      </div>
    )
  }

  const criticalCount = events.filter((e) => e.severity >= 8).length
  const elevatedCount = events.filter((e) => e.severity >= 5 && e.severity < 8).length

  return (
    <div className="min-h-screen">
      {showOnboarding && <Onboarding onComplete={() => setShowOnboarding(false)} />}
      <PricingModal isOpen={showPricing} onClose={() => setShowPricing(false)} />

      {/* Header */}
      <header className="sticky top-0 z-40 glass border-b border-border">
        <div className="max-w-7xl mx-auto px-4 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-primary to-accent-purple flex items-center justify-center">
              <Zap className="w-6 h-6 text-white" />
            </div>
            <div>
              <h1 className="font-bold text-lg">WorldMonitor Agents</h1>
              <p className="text-xs text-gray-400">AI-Powered Intelligence</p>
            </div>
          </div>

          <div className="flex items-center gap-4">
            {user && user.streak > 0 && (
              <div className="flex items-center gap-1 text-orange-500">
                <Sparkles className="w-4 h-4" />
                <span className="text-sm font-medium">{user.streak} day streak</span>
              </div>
            )}
            <button
              onClick={() => setShowPricing(true)}
              className="px-4 py-2 rounded-lg bg-primary/10 text-primary hover:bg-primary/20 transition-colors text-sm font-medium"
            >
              {user?.tier === 'free' ? 'Upgrade to Pro' : 'Pro Plan'}
            </button>
          </div>
        </div>
      </header>

      {/* Upgrade banner for free users */}
      {user?.tier === 'free' && (
        <div className="bg-gradient-to-r from-orange-500/20 to-yellow-500/20 border-b border-orange-500/30">
          <div className="max-w-7xl mx-auto px-4 py-2 flex items-center justify-between">
            <p className="text-sm text-orange-300">
              <Lock className="w-4 h-4 inline mr-1" />
              Free tier: {user.data_delay_hours}h delayed data, {user.max_alerts} alerts max
            </p>
            <button
              onClick={() => setShowPricing(true)}
              className="text-sm text-orange-400 hover:text-orange-300 font-medium"
            >
              Upgrade to Pro →
            </button>
          </div>
        </div>
      )}

      {/* Main content */}
      <main className="max-w-7xl mx-auto px-4 py-6">
        {/* Stats bar */}
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="glass rounded-xl p-4">
            <p className="text-gray-400 text-sm">Events Tracked</p>
            <p className="text-2xl font-bold">{events.length}</p>
          </div>
          <div className="glass rounded-xl p-4">
            <p className="text-gray-400 text-sm">Critical</p>
            <p className="text-2xl font-bold text-red-500">{criticalCount}</p>
          </div>
          <div className="glass rounded-xl p-4">
            <p className="text-gray-400 text-sm">Elevated</p>
            <p className="text-2xl font-bold text-yellow-500">{elevatedCount}</p>
          </div>
        </div>

        {/* Navigation tabs */}
        <div className="flex gap-2 mb-6">
          <button
            onClick={() => setActiveTab('map')}
            className={cn(
              'px-4 py-2 rounded-lg font-medium transition-colors',
              activeTab === 'map'
                ? 'bg-primary text-white'
                : 'text-gray-400 hover:text-white hover:bg-card'
            )}
          >
            Global Map
          </button>
          <button
            onClick={() => setActiveTab('brief')}
            className={cn(
              'px-4 py-2 rounded-lg font-medium transition-colors',
              activeTab === 'brief'
                ? 'bg-primary text-white'
                : 'text-gray-400 hover:text-white hover:bg-card'
            )}
          >
            {selectedCountry ? `${selectedCountry} Brief` : 'Daily Brief'}
          </button>
        </div>

        {/* Content */}
        <div className="grid lg:grid-cols-3 gap-6">
          {/* Main panel */}
          <div className="lg:col-span-2">
            {activeTab === 'map' ? (
              <MapVisualization events={events} onSelect={handleCountrySelect} />
            ) : (
              <div className="space-y-4">
                {brief ? (
                  <>
                    <div className="glass rounded-xl p-6">
                      <div className="flex items-center justify-between mb-4">
                        <div>
                          <h2 className="text-xl font-bold">{brief.country} Intelligence Brief</h2>
                          <p className="text-sm text-gray-400">
                            Based on {brief.event_count} events • Risk: {brief.risk_level}
                          </p>
                        </div>
                        <button
                          onClick={() => setShowReasoning(!showReasoning)}
                          className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-card hover:bg-card/80 transition-colors text-sm"
                        >
                          <Brain className="w-4 h-4" />
                          {showReasoning ? 'Hide' : 'Show'} Reasoning
                        </button>
                      </div>
                      <p className="text-gray-300 leading-relaxed">{brief.summary}</p>
                    </div>

                    {/* Chain of thought visualization */}
                    <AnimatePresence>
                      {showReasoning && brief.chain_of_thought && (
                        <motion.div
                          initial={{ opacity: 0, height: 0 }}
                          animate={{ opacity: 1, height: 'auto' }}
                          exit={{ opacity: 0, height: 0 }}
                          className="glass rounded-xl p-6 overflow-hidden"
                        >
                          <h3 className="font-bold mb-4 flex items-center gap-2">
                            <Brain className="w-5 h-5 text-primary" />
                            Chain of Thought
                          </h3>
                          <AgentVisualization steps={brief.chain_of_thought.steps} />
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </>
                ) : (
                  <div className="glass rounded-xl p-12 text-center">
                    <Globe className="w-16 h-16 text-gray-600 mx-auto mb-4" />
                    <p className="text-gray-400">Select a country on the map to view its intelligence brief</p>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Sidebar */}
          <div className="space-y-4">
            {/* Recent events */}
            <div className="glass rounded-xl p-4">
              <h3 className="font-bold mb-4 flex items-center gap-2">
                <AlertTriangle className="w-5 h-5 text-yellow-500" />
                Recent Events
              </h3>
              <div className="space-y-3 max-h-96 overflow-y-auto">
                {events.slice(0, 10).map((event) => (
                  <button
                    key={event.id}
                    onClick={() => handleCountrySelect(event)}
                    className="w-full text-left p-3 rounded-lg bg-card/50 hover:bg-card transition-colors"
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <span
                        className={cn(
                          'w-2 h-2 rounded-full',
                          event.severity >= 8 && 'bg-red-500',
                          event.severity >= 5 && event.severity < 8 && 'bg-yellow-500',
                          event.severity < 5 && 'bg-blue-500'
                        )}
                      />
                      <span className="font-medium text-sm">{event.country}</span>
                      <span className="text-xs text-gray-500 ml-auto">
                        {new Date(event.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    <p className="text-sm text-gray-400 line-clamp-2">{event.headline}</p>
                  </button>
                ))}
              </div>
            </div>

            {/* Agent status */}
            <div className="glass rounded-xl p-4">
              <h3 className="font-bold mb-4 flex items-center gap-2">
                <Zap className="w-5 h-5 text-primary" />
                Agent System
              </h3>
              <div className="space-y-2">
                {[
                  { name: 'DataCollector', status: 'active', icon: Globe },
                  { name: 'Analyst', status: 'active', icon: Brain },
                  { name: 'Forecaster', status: 'active', icon: TrendingUp },
                  { name: 'Notifier', status: 'active', icon: Bell },
                  { name: 'Validator', status: 'active', icon: Shield },
                ].map((agent) => (
                  <div key={agent.name} className="flex items-center gap-3 p-2 rounded-lg bg-card/50">
                    <agent.icon className="w-4 h-4 text-gray-400" />
                    <span className="text-sm">{agent.name}</span>
                    <span className="ml-auto flex items-center gap-1">
                      <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                      <span className="text-xs text-green-500">{agent.status}</span>
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
  )
}
