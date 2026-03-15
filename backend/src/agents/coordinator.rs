//! Agent Coordinator - Routes tasks to appropriate agents
//! 
//! The coordinator is the central hub that:
//! 1. Receives tasks from the API
//! 2. Determines which agent(s) should handle them
//! 3. Manages agent lifecycle and health
//! 4. Aggregates results from multiple agents
//! 5. Maintains chain-of-thought context

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use super::*;

/// Task queue for pending agent work
pub struct TaskQueue {
    sender: mpsc::Sender<AgentTask>,
    receiver: Arc<RwLock<mpsc::Receiver<AgentTask>>>,
}

impl TaskQueue {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
        }
    }

    pub async fn submit(&self, task: AgentTask) -> anyhow::Result<()> {
        self.sender.send(task).await?;
        Ok(())
    }

    pub async fn receive(&self) -> Option<AgentTask> {
        self.receiver.write().await.recv().await
    }
}

/// The central agent coordinator
pub struct AgentCoordinator {
    context: Arc<AgentContext>,
    agents: RwLock<HashMap<AgentType, Arc<dyn Agent>>>,
    task_queue: TaskQueue,
    active_tasks: RwLock<HashMap<String, TaskHandle>>,
    results_cache: Arc<RwLock<HashMap<String, AgentResult>>>,
}

#[derive(Debug, Clone)]
struct TaskHandle {
    task: AgentTask,
    started_at: i64,
    agent_type: AgentType,
}

impl AgentCoordinator {
    pub fn new(context: Arc<AgentContext>) -> Self {
        Self {
            context,
            agents: RwLock::new(HashMap::new()),
            task_queue: TaskQueue::new(1000),
            active_tasks: RwLock::new(HashMap::new()),
            results_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize and register all agents
    pub async fn initialize(&self) -> anyhow::Result<()> {
        info!("Initializing Agent Coordinator");

        let mut agents = self.agents.write().await;

        // Register Data Collector Agent
        let data_collector = Arc::new(DataCollectorAgent::new(
            AgentId::new(),
            self.context.clone(),
        ));
        agents.insert(AgentType::DataCollector, data_collector);
        info!("Registered DataCollector agent");

        // Register Analyst Agent
        let analyst = Arc::new(AnalystAgent::new(
            AgentId::new(),
            self.context.clone(),
        ));
        agents.insert(AgentType::Analyst, analyst);
        info!("Registered Analyst agent");

        // Register Forecaster Agent
        let forecaster = Arc::new(ForecasterAgent::new(
            AgentId::new(),
            self.context.clone(),
        ));
        agents.insert(AgentType::Forecaster, forecaster);
        info!("Registered Forecaster agent");

        // Register Notifier Agent
        let notifier = Arc::new(NotifierAgent::new(
            AgentId::new(),
            self.context.clone(),
        ));
        agents.insert(AgentType::Notifier, notifier);
        info!("Registered Notifier agent");

        // Register Validator Agent
        let validator = Arc::new(ValidatorAgent::new(
            AgentId::new(),
            self.context.clone(),
        ));
        agents.insert(AgentType::Validator, validator);
        info!("Registered Validator agent");

        // Start the task processor
        self.spawn_task_processor().await;

        info!("Agent Coordinator initialized with {} agents", agents.len());
        Ok(())
    }

    /// Submit a task for execution
    pub async fn submit_task(&self, task: AgentTask) -> anyhow::Result<String> {
        let task_id = task.id.clone();
        info!("Submitting task {} of type {:?}", task_id, task.task_type);

        // Determine which agent should handle this task
        let agent_type = self.route_task(&task);
        
        // Check if we have a cached result for similar tasks
        if let Some(cached) = self.get_cached_result(&task).await {
            info!("Returning cached result for task {}", task_id);
            return Ok(cached.task_id);
        }

        // Add to active tasks
        {
            let mut active = self.active_tasks.write().await;
            active.insert(task_id.clone(), TaskHandle {
                task: task.clone(),
                started_at: chrono::Utc::now().timestamp_millis(),
                agent_type,
            });
        }

        // Submit to queue
        self.task_queue.submit(task).await?;
        
        Ok(task_id)
    }

    /// Execute a task immediately (blocking)
    pub async fn execute_task(&self, task: AgentTask) -> anyhow::Result<AgentResult> {
        let agent_type = self.route_task(&task);
        let agents = self.agents.read().await;
        
        let agent = agents.get(&agent_type)
            .ok_or_else(|| anyhow::anyhow!("Agent {:?} not found", agent_type))?;

        info!("Executing task {} with agent {:?}", task.id, agent_type);
        let start = std::time::Instant::now();
        
        let result = agent.execute(task).await?;
        
        let elapsed = start.elapsed().as_millis() as i64;
        info!("Task {} completed in {}ms", result.task_id, elapsed);

        // Cache the result
        self.cache_result(result.clone()).await;

        Ok(result)
    }

    /// Route a task to the appropriate agent type
    fn route_task(&self, task: &AgentTask) -> AgentType {
        match task.task_type {
            TaskType::CollectIntelligence => AgentType::DataCollector,
            TaskType::AnalyzeRegion => AgentType::Analyst,
            TaskType::GenerateBrief => AgentType::Analyst,
            TaskType::ForecastTrends => AgentType::Forecaster,
            TaskType::SendNotification => AgentType::Notifier,
            TaskType::ValidateData => AgentType::Validator,
            TaskType::CorrelateEvents => AgentType::Analyst,
        }
    }

    /// Execute a multi-agent workflow with chain-of-thought
    pub async fn execute_workflow(
        &self,
        workflow: AgentWorkflow,
    ) -> anyhow::Result<WorkflowResult> {
        info!("Executing workflow {} with {} steps", workflow.id, workflow.steps.len());
        
        let mut results = Vec::new();
        let mut combined_cot = ChainOfThought::new(&workflow.id);

        for (i, step) in workflow.steps.iter().enumerate() {
            info!("Workflow step {}/{}: {:?}", i + 1, workflow.steps.len(), step.task_type);
            
            let task = AgentTask::new(step.task_type.clone(), &step.payload)?
                .with_priority(step.priority);

            match self.execute_task(task).await {
                Ok(result) => {
                    // Merge chain-of-thought
                    for cot_step in &result.chain_of_thought.steps {
                        combined_cot.add_step(cot_step.clone());
                    }
                    results.push(result);
                }
                Err(e) => {
                    warn!("Workflow step {} failed: {}", i, e);
                    if step.critical {
                        return Err(e);
                    }
                }
            }
        }

        // Generate combined conclusion
        let conclusion = self.synthesize_workflow_results(&results).await?;
        combined_cot.complete(&conclusion, 0.85);

        Ok(WorkflowResult {
            workflow_id: workflow.id,
            results,
            combined_chain_of_thought: combined_cot,
            conclusion,
        })
    }

    /// Get the result of a completed task
    pub async fn get_result(&self, task_id: &str) -> Option<AgentResult> {
        // Check cache first
        let cache = self.results_cache.read().await;
        if let Some(result) = cache.get(task_id) {
            return Some(result.clone());
        }
        drop(cache);

        // Check active tasks
        let active = self.active_tasks.read().await;
        if active.contains_key(task_id) {
            // Task is still running
            return None;
        }

        None
    }

    /// Get health status of all agents
    pub async fn health_check(&self) -> Vec<AgentHealth> {
        let agents = self.agents.read().await;
        let mut healths = Vec::new();

        for (agent_type, agent) in agents.iter() {
            healths.push(agent.health_check().await);
        }

        healths
    }

    /// Get system-wide metrics
    pub async fn get_metrics(&self) -> SystemMetrics {
        let metrics = self.context.metrics.read().await;
        let agents = self.agents.read().await;
        let active = self.active_tasks.read().await;

        SystemMetrics {
            total_agents: agents.len(),
            active_tasks: active.len(),
            tasks_completed: metrics.tasks_completed,
            tasks_failed: metrics.tasks_failed,
            average_execution_time_ms: metrics.average_execution_time_ms(),
            reasoning_steps_generated: metrics.reasoning_steps_generated,
        }
    }

    // Private methods

    async fn spawn_task_processor(&self) {
        let queue = self.task_queue.receiver.clone();
        let agents = self.agents.clone();
        let active_tasks = self.active_tasks.clone();
        let results_cache = self.results_cache.clone();

        tokio::spawn(async move {
            loop {
                let task = {
                    let mut rx = queue.write().await;
                    rx.recv().await
                };

                if let Some(task) = task {
                    let task_id = task.id.clone();
                    let agents_guard = agents.read().await;
                    
                    // Determine agent type from task
                    let agent_type = match task.task_type {
                        TaskType::CollectIntelligence => AgentType::DataCollector,
                        TaskType::AnalyzeRegion => AgentType::Analyst,
                        TaskType::GenerateBrief => AgentType::Analyst,
                        TaskType::ForecastTrends => AgentType::Forecaster,
                        TaskType::SendNotification => AgentType::Notifier,
                        TaskType::ValidateData => AgentType::Validator,
                        TaskType::CorrelateEvents => AgentType::Analyst,
                    };

                    if let Some(agent) = agents_guard.get(&agent_type) {
                        let agent = agent.clone();
                        drop(agents_guard);

                        // Execute in background
                        let active_clone = active_tasks.clone();
                        let cache_clone = results_cache.clone();

                        tokio::spawn(async move {
                            match agent.execute(task).await {
                                Ok(result) => {
                                    // Cache result
                                    let mut cache = cache_clone.write().await;
                                    cache.insert(task_id.clone(), result);
                                    
                                    // Remove from active
                                    let mut active = active_clone.write().await;
                                    active.remove(&task_id);
                                }
                                Err(e) => {
                                    error!("Task {} failed: {}", task_id, e);
                                    let mut active = active_clone.write().await;
                                    active.remove(&task_id);
                                }
                            }
                        });
                    }
                }
            }
        });
    }

    async fn get_cached_result(&self, task: &AgentTask) -> Option<AgentResult> {
        // Simple cache key based on task type and payload hash
        let cache_key = format!("{:?}:{}", task.task_type, 
            serde_json::to_string(&task.payload).unwrap_or_default().len());
        
        let cache = self.results_cache.read().await;
        cache.get(&cache_key).cloned()
    }

    async fn cache_result(&self, result: AgentResult) {
        let mut cache = self.results_cache.write().await;
        cache.insert(result.task_id.clone(), result);
        
        // Limit cache size
        if cache.len() > 1000 {
            let keys_to_remove: Vec<String> = cache.keys()
                .take(cache.len() - 1000)
                .cloned()
                .collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
        }
    }

    async fn synthesize_workflow_results(&self, results: &[AgentResult]) -> anyhow::Result<String> {
        // Combine all conclusions into a coherent summary
        let mut conclusions: Vec<String> = results.iter()
            .filter(|r| r.success)
            .map(|r| r.chain_of_thought.final_conclusion.clone())
            .collect();

        if conclusions.is_empty() {
            return Ok("No conclusions available from workflow".to_string());
        }

        if conclusions.len() == 1 {
            return Ok(conclusions.remove(0));
        }

        // Multi-part conclusion
        let mut synthesized = String::from("## Intelligence Analysis Summary\n\n");
        for (i, conclusion) in conclusions.iter().enumerate() {
            synthesized.push_str(&format!("### Part {}\n{}\n\n", i + 1, conclusion));
        }

        Ok(synthesized)
    }
}

/// A multi-step workflow
#[derive(Debug, Clone)]
pub struct AgentWorkflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub task_type: TaskType,
    pub payload: serde_json::Value,
    pub priority: TaskPriority,
    pub critical: bool,
}

/// Result from a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_id: String,
    pub results: Vec<AgentResult>,
    pub combined_chain_of_thought: ChainOfThought,
    pub conclusion: String,
}

/// System-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_agents: usize,
    pub active_tasks: usize,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub average_execution_time_ms: f64,
    pub reasoning_steps_generated: u64,
}
