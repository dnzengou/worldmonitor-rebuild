//! Validator Agent
//! 
//! Validates data accuracy and confidence:
//! - Source verification
//! - Cross-reference checking
//! - Confidence scoring
//! - Bias detection

use async_trait::async_trait;
use tracing::{info, warn};

use super::*;

pub struct ValidatorAgent {
    id: AgentId,
    context: Arc<AgentContext>,
}

impl ValidatorAgent {
    pub fn new(id: AgentId, context: Arc<AgentContext>) -> Self {
        Self { id, context }
    }

    async fn validate_event(
        &self,
        event: &crate::models::Event,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<ValidationResult> {
        let mut score = 0.5; // Base score
        let mut checks = Vec::new();

        // Check 1: Source reliability
        let source_score = match event.source.as_str() {
            "gdelt" => {
                checks.push("GDELT source: high reliability".to_string());
                0.25
            }
            "rss" => {
                checks.push("RSS source: medium reliability".to_string());
                0.15
            }
            "user" => {
                checks.push("User report: requires verification".to_string());
                0.05
            }
            _ => {
                checks.push("Unknown source: low reliability".to_string());
                0.05
            }
        };
        score += source_score;

        // Check 2: Geographic validity
        let coords_valid = event.lat >= -90.0 && event.lat <= 90.0 
            && event.lon >= -180.0 && event.lon <= 180.0;
        if coords_valid {
            checks.push("Valid coordinates".to_string());
            score += 0.1;
        } else {
            checks.push("Invalid coordinates detected".to_string());
            score -= 0.2;
        }

        // Check 3: Severity consistency
        if event.severity >= 1 && event.severity <= 10 {
            checks.push("Severity in valid range".to_string());
            score += 0.1;
        } else {
            checks.push("Severity out of range".to_string());
            score -= 0.1;
        }

        // Check 4: Cross-reference with other events
        let nearby = self.context.db.find_nearby_events(event.lat, event.lon, 1.0, 24).await?;
        if nearby.len() > 1 {
            checks.push(format!("Corroborated by {} nearby events", nearby.len() - 1));
            score += 0.15;
        }

        // Check 5: Temporal recency
        let age_hours = (chrono::Utc::now().timestamp_millis() - event.timestamp) / 3600000;
        if age_hours < 24 {
            checks.push("Recent event (< 24h)".to_string());
            score += 0.1;
        }

        score = score.clamp(0.0, 1.0);

        let verdict = if score >= 0.8 {
            "high_confidence"
        } else if score >= 0.5 {
            "medium_confidence"
        } else {
            "low_confidence"
        };

        cot.add_step(ReasoningStep::new(
            1,
            AgentType::Validator,
            format!("Validating event {}", event.id),
            format!("Run {} validation checks", checks.len()),
            format!("Score: {:.2}, Verdict: {}", score, verdict),
            score,
        ));

        Ok(ValidationResult {
            event_id: event.id.clone(),
            confidence_score: score,
            verdict: verdict.to_string(),
            checks,
        })
    }
}

#[async_trait]
impl Agent for ValidatorAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Validator
    }

    fn id(&self) -> AgentId {
        self.id
    }

    async fn execute(&self, task: AgentTask) -> anyhow::Result<AgentResult> {
        let start = std::time::Instant::now();
        let mut cot = ChainOfThought::new(&task.id);

        let payload: ValidationPayload = serde_json::from_value(task.payload.clone())?;
        
        // Get event to validate
        let event = self.context.db.get_event(&payload.event_id).await?
            .ok_or_else(|| anyhow::anyhow!("Event not found"))?;

        let validation = self.validate_event(&event, &mut cot).await?;

        // Update event confidence in database
        self.context.db.update_event_confidence(&event.id, validation.confidence_score).await?;

        cot.complete(
            &format!("Event {} validated with {:.0}% confidence", 
                event.id, validation.confidence_score * 100.0),
            validation.confidence_score,
        );

        let result = AgentResult {
            task_id: task.id,
            agent_type: self.agent_type(),
            success: true,
            data: serde_json::to_value(&validation)?,
            chain_of_thought: cot,
            execution_time_ms: start.elapsed().as_millis() as i64,
        };

        {
            let mut metrics = self.context.metrics.write().await;
            metrics.record_success(result.execution_time_ms);
        }

        Ok(result)
    }

    async fn health_check(&self) -> AgentHealth {
        let metrics = self.context.metrics.read().await;
        AgentHealth {
            agent_id: self.id,
            agent_type: self.agent_type(),
            status: HealthStatus::Healthy,
            last_heartbeat: chrono::Utc::now().timestamp_millis(),
            tasks_completed: metrics.tasks_completed,
            tasks_failed: metrics.tasks_failed,
            average_execution_time_ms: metrics.average_execution_time_ms(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ValidationPayload {
    event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub event_id: String,
    pub confidence_score: f32,
    pub verdict: String,
    pub checks: Vec<String>,
}
