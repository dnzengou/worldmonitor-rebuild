'use client';

import React, { useState, useCallback } from 'react';
import { StatusBar } from '@/components/dashboard/StatusBar';
import { GlobalLiveFeed, LiveEvent } from '@/components/dashboard/GlobalLiveFeed';
import { AgentStatus, Agent } from '@/components/dashboard/AgentStatus';
import { ReasoningTrace, ReasoningStep } from '@/components/dashboard/ReasoningTrace';
import { LayerControl, Layer } from '@/components/map/LayerControl';
import { CommandPalette } from '@/components/ui/CommandPalette';
import { useCommandPalette } from '@/hooks/useCommandPalette';
import { GateBanner } from '@/components/ui/MonetizationGate';
import { cn } from '@/lib/utils';
import type { Command } from '@/types';

// Sample data for demonstration
const sampleEvents: LiveEvent[] = [
  {
    id: '1',
    agentId: 'ag01',
    agentName: 'AG01_SCRAPER',
    timestamp: Date.now() - 120000,
    severity: 'high',
    type: 'cyber',
    title: 'Anomalous network activity detected in Sector 7G',
    description: 'Unusual traffic patterns identified from unknown source. Potential reconnaissance activity.',
    location: 'North America',
    isNew: true,
    isGated: true,
  },
  {
    id: '2',
    agentId: 'ag02',
    agentName: 'AG02_LLM_ANALYZER',
    timestamp: Date.now() - 300000,
    severity: 'medium',
    type: 'cyber',
    title: 'Anomalous network activity detected in Sector 7G',
    description: 'Pattern analysis suggests coordinated probing behavior.',
    location: 'Europe',
    isGated: false,
  },
  {
    id: '3',
    agentId: 'ag02',
    agentName: 'AG02_LLM_ANALYZER',
    timestamp: Date.now() - 600000,
    severity: 'low',
    type: 'cyber',
    title: 'Anomalous network activity detected in...',
    description: 'Routine scan detected and logged.',
    isGated: true,
  },
  {
    id: '4',
    agentId: 'ag01',
    agentName: 'AG01_SCRAPER',
    timestamp: Date.now() - 900000,
    severity: 'critical',
    type: 'cyber',
    title: 'Critical vulnerability scan detected',
    description: 'High-frequency targeted scanning against known CVEs.',
    location: 'Asia Pacific',
    isNew: true,
    isGated: false,
  },
  {
    id: '5',
    agentId: 'ag01',
    agentName: 'AG01_SCRAPER',
    timestamp: Date.now() - 1200000,
    severity: 'info',
    type: 'cyber',
    title: 'Routine health check completed',
    description: 'All monitoring endpoints responding normally.',
    isGated: false,
  },
];

const sampleAgents: Agent[] = [
  {
    id: 'ag01',
    name: 'AGENT_ALPHA',
    status: 'active',
    lastActivity: 'Processing Sector 7G',
    connections: ['ag02', 'ag03'],
    metrics: { tasksProcessed: 1247, latency: 12, confidence: 94 },
  },
  {
    id: 'ag02',
    name: 'AGENT_BETA',
    status: 'thinking',
    lastActivity: 'Analyzing patterns',
    connections: ['ag03'],
    metrics: { tasksProcessed: 892, latency: 45, confidence: 87 },
  },
  {
    id: 'ag03',
    name: 'AGENT_GAMMA',
    status: 'active',
    lastActivity: 'Monitoring feed',
    metrics: { tasksProcessed: 2156, latency: 8, confidence: 98 },
  },
];

const sampleSteps: ReasoningStep[] = [
  {
    id: 'step-1',
    order: 1,
    title: 'Step 1: Data Ingest',
    description: 'Anomalous network activity detected in Sector 7G. Coincidence correlated to the sector 10vg.',
    status: 'completed',
    confidence: 94,
    timestamp: '2 min ago',
  },
  {
    id: 'step-2',
    order: 2,
    title: 'Step 2: Vector Analysis',
    description: 'Analysis vector analysis created a convergent connection, and estimated contact react clusters.',
    status: 'completed',
    confidence: 87,
    timestamp: '1 min ago',
  },
  {
    id: 'step-3',
    order: 3,
    title: 'Step 3: Risk Assessment',
    description: 'Analysis risk assessment methodology. Risk confidence scores the elemental conscience of the OSINT standard system and analyzer.',
    status: 'current',
    confidence: 65,
  },
  {
    id: 'step-4',
    order: 4,
    title: 'Step 4: Decision/Verdict',
    description: 'Anomalous network activity detection authorized.',
    status: 'pending',
  },
];

const sampleLayers: Layer[] = [
  { id: 'cyber', name: 'Cyber Threats', category: 'Threats', enabled: true, count: 127, color: '#FF3E3E' },
  { id: 'network', name: 'Network Traffic', category: 'Threats', enabled: true, count: 89, color: '#00F5FF' },
  { id: 'social', name: 'Social Media', category: 'Intelligence', enabled: true, count: 2341, color: '#8B5CF6' },
  { id: 'darkweb', name: 'Dark Web', category: 'Intelligence', enabled: false, count: 23, color: '#6B7280' },
  { id: 'infrastructure', name: 'Infrastructure', category: 'Assets', enabled: true, count: 156, color: '#00E676' },
];

const commands: Command[] = [
  { id: '1', title: 'Filter: Middle East Region', category: 'Filters', shortcut: '⌘1', icon: '🌍', action: () => console.log('Filter ME') },
  { id: '2', title: 'Filter: Cyber Threats Only', category: 'Filters', shortcut: '⌘2', icon: '⚡', action: () => console.log('Filter Cyber') },
  { id: '3', title: 'View: Risk Analysis Dashboard', category: 'Views', shortcut: '⌘3', icon: '📊', action: () => console.log('View Risk') },
  { id: '4', title: 'View: Agent Activity Monitor', category: 'Views', shortcut: '⌘4', icon: '🤖', action: () => console.log('View Agents') },
  { id: '5', title: 'AI: Generate Briefing', category: 'AI Actions', icon: '🧠', action: () => console.log('Generate Briefing') },
  { id: '6', title: 'AI: Predict Next 24h', category: 'AI Actions', icon: '🔮', action: () => console.log('Predict') },
  { id: '7', title: 'Export: Current View', category: 'Export', shortcut: '⌘E', icon: '📥', action: () => console.log('Export') },
  { id: '8', title: 'Settings: Preferences', category: 'Settings', shortcut: '⌘,', icon: '⚙️', action: () => console.log('Settings') },
];

export default function Dashboard() {
  const [layers, setLayers] = useState<Layer[]>(sampleLayers);
  const [selectedEvent, setSelectedEvent] = useState<LiveEvent | null>(null);

  const {
    isOpen,
    searchQuery,
    filteredCommands,
    selectedIndex,
    close,
    setSearchQuery,
    selectNext,
    selectPrevious,
    executeSelected,
  } = useCommandPalette(commands);

  const handleToggleLayer = useCallback((layerId: string) => {
    setLayers(prev => prev.map(layer =>
      layer.id === layerId ? { ...layer, enabled: !layer.enabled } : layer
    ));
  }, []);

  const handleToggleCategory = useCallback((category: string, enabled: boolean) => {
    if (category === 'all') {
      setLayers(prev => prev.map(layer => ({ ...layer, enabled })));
    } else {
      setLayers(prev => prev.map(layer =>
        layer.category === category ? { ...layer, enabled } : layer
      ));
    }
  }, []);

  return (
    <div className="min-h-screen bg-obsidian flex flex-col">
      {/* Status Bar */}
      <StatusBar
        version="2.6.5"
        isLive={true}
        region="Global"
        lastUpdate={Date.now() - 9000}
        alertCount={3}
      />

      {/* Main Content */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Sidebar - Global Live Feed */}
        <div className="w-80 flex-shrink-0 border-r border-border-default">
          <GlobalLiveFeed
            events={sampleEvents}
            onEventClick={setSelectedEvent}
            onUpgrade={() => console.log('Upgrade clicked')}
            maxHeight="calc(100vh - 48px)"
          />
        </div>

        {/* Center - Interactive Map */}
        <div className="flex-1 relative bg-obsidian">
          {/* Map Controls */}
          <div className="absolute top-4 left-4 z-10 flex items-center gap-2">
            <LayerControl
              layers={layers}
              onToggleLayer={handleToggleLayer}
              onToggleCategory={handleToggleCategory}
            />
            
            {/* Search Bar */}
            <div className="glass-panel rounded-lg flex items-center gap-2 px-3 py-2">
              <svg className="w-4 h-4 text-text-muted" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <span className="text-sm text-text-muted">CMD+K to search</span>
            </div>
          </div>

          {/* Map Zoom Controls */}
          <div className="absolute top-4 right-4 z-10 flex flex-col gap-1">
            <button className="w-8 h-8 glass-panel rounded-lg flex items-center justify-center text-text-secondary hover:text-text-primary transition-colors">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
              </svg>
            </button>
            <button className="w-8 h-8 glass-panel rounded-lg flex items-center justify-center text-text-secondary hover:text-text-primary transition-colors">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M20 12H4" />
              </svg>
            </button>
            <button className="w-8 h-8 glass-panel rounded-lg flex items-center justify-center text-text-secondary hover:text-text-primary transition-colors">
              <svg className="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
              </svg>
            </button>
          </div>

          {/* Map Placeholder with Grid Background */}
          <div className="w-full h-full bg-grid flex items-center justify-center relative">
            {/* World Map Silhouette */}
            <div className="absolute inset-0 flex items-center justify-center opacity-30">
              <svg viewBox="0 0 1000 500" className="w-full h-full max-w-5xl">
                {/* Simplified world map paths */}
                <path
                  fill="none"
                  stroke="rgba(0, 245, 255, 0.2)"
                  strokeWidth="0.5"
                  d="M150,200 Q200,150 300,180 T450,200 T600,180 T750,200 T850,180"
                />
                {/* Data Points */}
                <circle cx="200" cy="180" r="4" fill="#FF3E3E" className="animate-pulse" />
                <circle cx="350" cy="200" r="3" fill="#00F5FF" className="animate-pulse" />
                <circle cx="500" cy="190" r="5" fill="#FF3E3E" className="animate-pulse" />
                <circle cx="650" cy="210" r="3" fill="#00F5FF" className="animate-pulse" />
                <circle cx="800" cy="200" r="4" fill="#00E676" className="animate-pulse" />
                {/* Connection Lines */}
                <path
                  d="M200,180 Q300,150 500,190"
                  fill="none"
                  stroke="rgba(0, 245, 255, 0.3)"
                  strokeWidth="1"
                  strokeDasharray="5,5"
                  className="animate-pulse"
                />
                <path
                  d="M500,190 Q600,180 800,200"
                  fill="none"
                  stroke="rgba(255, 62, 62, 0.3)"
                  strokeWidth="1"
                  strokeDasharray="5,5"
                  className="animate-pulse"
                />
              </svg>
            </div>

            {/* Center Info */}
            <div className="text-center z-10">
              <div className="w-20 h-20 mx-auto mb-4 rounded-full bg-neon/10 border border-neon/30 flex items-center justify-center animate-pulse-neon">
                <svg className="w-10 h-10 text-neon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <h2 className="text-lg font-semibold text-text-primary mb-1">Interactive Map</h2>
              <p className="text-xs text-text-muted">WebGL-accelerated visualization</p>
              
              {/* Stats */}
              <div className="flex items-center justify-center gap-6 mt-4">
                <div className="text-center">
                  <div className="text-lg font-bold text-alert">127</div>
                  <div className="text-2xs text-text-muted uppercase tracking-wider">Alerts</div>
                </div>
                <div className="text-center">
                  <div className="text-lg font-bold text-neon">89</div>
                  <div className="text-2xs text-text-muted uppercase tracking-wider">Active</div>
                </div>
                <div className="text-center">
                  <div className="text-lg font-bold text-success">156</div>
                  <div className="text-2xs text-text-muted uppercase tracking-wider">Monitored</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Right Sidebar - Agent Status & Reasoning Trace */}
        <div className="w-80 flex-shrink-0 border-l border-border-default flex flex-col">
          {/* Agent Status */}
          <div className="flex-1 min-h-0">
            <AgentStatus
              agents={sampleAgents}
              title="Agent Status Overview"
            />
          </div>

          {/* Reasoning Trace */}
          <div className="flex-1 min-h-0 border-t border-border-default">
            <ReasoningTrace
              steps={sampleSteps}
              title="Reasoning Trace"
            />
          </div>

          {/* Monetization Gate Example */}
          <div className="p-3 border-t border-border-default">
            <GateBanner
              title="Access Restricted"
              description="Unlock advanced reasoning traces and historical analysis with Enterprise access."
              requiredTier="enterprise"
              features={[
                'Full CoT transparency',
                'Historical analysis',
                'Custom agent deployment',
                'API access',
              ]}
              onUpgrade={() => console.log('Upgrade to Enterprise')}
            />
          </div>
        </div>
      </div>

      {/* Command Palette */}
      <CommandPalette
        isOpen={isOpen}
        onClose={close}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        commands={filteredCommands}
        selectedIndex={selectedIndex}
        onSelect={(index) => {}}
        onExecute={executeSelected}
      />
    </div>
  );
}
