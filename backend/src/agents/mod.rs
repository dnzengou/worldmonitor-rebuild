//! Chain-of-Thought Multi-Agent System
//! 
//! Architecture:
//! - AgentCoordinator: Routes tasks to appropriate agents
//! - Specialized Agents: Each handles a specific intelligence domain
//! - Chain-of-Thought: Agents reason through steps before producing output

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub mod coordinator;
pub mod data_collector;
pub mod analyst;
pub mod forecaster;
pub mod notifier;
pub mod validator;

pub use coordinator::AgentCoordinator;
pub use data_collector::DataCollectorAgent;
pub use analyst::AnalystAgent;
pub use forecaster::ForecasterAgent;
pub use notifier::NotifierAgent;
pub use validator::ValidatorAgent;

/// Unique identifier for an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub uuid::Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent types in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    DataCollector,
    Analyst,
    Forecaster,
    Notifier,
    Validator,
}

impl AgentType {
    pub fn name(&self) -> &'static str {
        match self {
            AgentType::DataCollector => "DataCollector",
            AgentType::Analyst => "Analyst",
            AgentType::Forecaster => "Forecaster",
            AgentType::Notifier => "Notifier",
            AgentType::Validator => "Validator",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AgentType::DataCollector => "Collects and fuses intelligence from multiple sources",
            AgentType::Analyst => "Analyzes patterns, identifies anomalies, generates insights",
            AgentType::Forecaster => "Predicts future developments based on current trends",
            AgentType::Notifier => "Manages user alerts and notifications",
            AgentType::Validator => "Verifies data accuracy and confidence scores",
        }
    }
}

/// A single step in chain-of-thought reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: usize,
    pub agent_type: AgentType,
    pub thought: String,
    pub action: String,
    pub observation: String,
    pub confidence: f32,
    pub timestamp: i64,
}

impl ReasoningStep {
    pub fn new(
        step_number: usize,
        agent_type: AgentType,
        thought: impl Into<String>,
        action: impl Into<String>,
        observation: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            step_number,
            agent_type,
            thought: thought.into(),
            action: action.into(),
            observation: observation.into(),
            confidence: confidence.clamp(0.0, 1.0),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Complete chain-of-thought for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfThought {
    pub task_id: String,
    pub steps: Vec<ReasoningStep>,
    pub final_conclusion: String,
    pub overall_confidence: f32,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

impl ChainOfThought {
    pub fn new(task_id: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            steps: Vec::new(),
            final_conclusion: String::new(),
            overall_confidence: 0.0,
            started_at: chrono::Utc::now().timestamp_millis(),
            completed_at: None,
        }
    }

    pub fn add_step(&mut self, step: ReasoningStep) {
        self.steps.push(step);
    }

    pub fn complete(&mut self, conclusion: impl Into<String>, confidence: f32) {
        self.final_conclusion = conclusion.into();
        self.overall_confidence = confidence.clamp(0.0, 1.0);
        self.completed_at = Some(chrono::Utc::now().timestamp_millis());
    }

    pub fn duration_ms(&self) -> Option<i64> {
        self.completed_at.map(|end| end - self.started_at)
    }
}

/// Task for an agent to execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub task_type: TaskType,
    pub payload: serde_json::Value,
    pub priority: TaskPriority,
    pub user_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    CollectIntelligence,
    AnalyzeRegion,
    GenerateBrief,
    ForecastTrends,
    SendNotification,
    ValidateData,
    CorrelateEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
    Low,
}

impl TaskPriority {
    pub fn as_u8(&self) -> u8 {
        match self {
            TaskPriority::Critical => 0,
            TaskPriority::High => 1,
            TaskPriority::Normal => 2,
            TaskPriority::Low => 3,
        }
    }
}

impl AgentTask {
    pub fn new(task_type: TaskType, payload: impl Serialize) -> anyhow::Result<Self> {
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            task_type,
            payload: serde_json::to_value(payload)?,
            priority: TaskPriority::Normal,
            user_id: None,
            created_at: chrono::Utc::now().timestamp_millis(),
        })
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn for_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
}

/// Result from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub task_id: String,
    pub agent_type: AgentType,
    pub success: bool,
    pub data: serde_json::Value,
    pub chain_of_thought: ChainOfThought,
    pub execution_time_ms: i64,
}

/// Core trait for all agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent's type
    fn agent_type(&self) -> AgentType;

    /// Get the agent's unique ID
    fn id(&self) -> AgentId;

    /// Execute a task with chain-of-thought reasoning
    async fn execute(&self, task: AgentTask) -> anyhow::Result<AgentResult>;

    /// Check if agent is healthy
    async fn health_check(&self) -> AgentHealth;
}

/// Agent health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealth {
    pub agent_id: AgentId,
    pub agent_type: AgentType,
    pub status: HealthStatus,
    pub last_heartbeat: i64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub average_execution_time_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

/// Shared state for all agents
pub struct AgentContext {
    pub db: Arc<crate::db::Database>,
    pub cache: Arc<crate::services::cache::CacheService>,
    pub config: Arc<crate::config::Config>,
    pub llm: Arc<crate::services::llm::LlmService>,
    pub metrics: Arc<RwLock<AgentMetrics>>,
}

impl AgentContext {
    pub fn new(
        db: Arc<crate::db::Database>,
        cache: Arc<crate::services::cache::CacheService>,
        config: Arc<crate::config::Config>,
        llm: Arc<crate::services::llm::LlmService>,
    ) -> Self {
        Self {
            db,
            cache,
            config,
            llm,
            metrics: Arc::new(RwLock::new(AgentMetrics::default())),
        }
    }
}

/// Metrics for agent performance
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub total_execution_time_ms: u64,
    pub reasoning_steps_generated: u64,
}

impl AgentMetrics {
    pub fn record_success(&mut self, execution_time_ms: i64) {
        self.tasks_completed += 1;
        self.total_execution_time_ms += execution_time_ms as u64;
    }

    pub fn record_failure(&mut self) {
        self.tasks_failed += 1;
    }

    pub fn record_reasoning_step(&mut self) {
        self.reasoning_steps_generated += 1;
    }

    pub fn average_execution_time_ms(&self) -> f64 {
        if self.tasks_completed == 0 {
            0.0
        } else {
            self.total_execution_time_ms as f64 / self.tasks_completed as f64
        }
    }
}
