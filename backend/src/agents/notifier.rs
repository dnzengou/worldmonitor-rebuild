//! Notifier Agent
//! 
//! Manages user notifications and alerts:
//! - Alert matching
//! - Notification delivery
//! - User preference handling
//! - Rate limiting

use async_trait::async_trait;
use tracing::{debug, info, warn};

use super::*;

pub struct NotifierAgent {
    id: AgentId,
    context: Arc<AgentContext>,
}

impl NotifierAgent {
    pub fn new(id: AgentId, context: Arc<AgentContext>) -> Self {
        Self { id, context }
    }

    async fn check_alerts(
        &self,
        user_id: &str,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<MatchedAlert>> {
        // Get user's alerts
        let alerts = self.context.db.get_user_alerts(user_id).await?;
        
        if alerts.is_empty() {
            cot.add_step(ReasoningStep::new(
                1,
                AgentType::Notifier,
                "Checking user alerts",
                "SELECT * FROM alerts WHERE user_id = ?",
                "User has no active alerts",
                1.0,
            ));
            return Ok(vec![]);
        }

        let mut matched = Vec::new();

        for alert in alerts {
            // Check for matching events
            let events = self.context.db.get_events_by_country(&alert.country, 1).await?;
            
            for event in events {
                if event.severity >= alert.threshold {
                    matched.push(MatchedAlert {
                        alert_id: alert.id,
                        country: alert.country.clone(),
                        event: event.clone(),
                        severity: event.severity,
                    });
                }
            }
        }

        cot.add_step(ReasoningStep::new(
            1,
            AgentType::Notifier,
            format!("Checking {} user alerts against recent events", alerts.len()),
            "Match events to alert thresholds",
            format!("Found {} matching alerts", matched.len()),
            0.95,
        ));

        Ok(matched)
    }

    async fn send_notification(
        &self,
        user_id: &str,
        alert: &MatchedAlert,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<bool> {
        // Check rate limits
        let recent_notifications = self.context.db.count_recent_notifications(user_id, 1).await?;
        
        if recent_notifications >= 5 {
            cot.add_step(ReasoningStep::new(
                2,
                AgentType::Notifier,
                "Checking rate limits",
                "Count notifications in last hour",
                "Rate limit exceeded, skipping notification",
                1.0,
            ));
            return Ok(false);
        }

        // Record notification
        self.context.db.record_notification(user_id, alert.alert_id).await?;

        cot.add_step(ReasoningStep::new(
            2,
            AgentType::Notifier,
            "Sending notification",
            "INSERT INTO notifications",
            "Notification queued for delivery",
            0.9,
        ));

        Ok(true)
    }
}

#[async_trait]
impl Agent for NotifierAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Notifier
    }

    fn id(&self) -> AgentId {
        self.id
    }

    async fn execute(&self, task: AgentTask) -> anyhow::Result<AgentResult> {
        let start = std::time::Instant::now();
        let mut cot = ChainOfThought::new(&task.id);

        let payload: NotificationPayload = serde_json::from_value(task.payload.clone())?;
        let user_id = payload.user_id;

        let matched = self.check_alerts(&user_id, &mut cot).await?;
        let mut sent = 0;

        for alert in &matched {
            if self.send_notification(&user_id, alert, &mut cot).await? {
                sent += 1;
            }
        }

        cot.complete(
            &format!("Processed {} alerts, sent {} notifications", matched.len(), sent),
            0.9,
        );

        let result = AgentResult {
            task_id: task.id,
            agent_type: self.agent_type(),
            success: true,
            data: serde_json::json!({
                "alerts_matched": matched.len(),
                "notifications_sent": sent,
            }),
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
struct NotificationPayload {
    user_id: String,
}

#[derive(Debug, Clone)]
struct MatchedAlert {
    alert_id: i64,
    country: String,
    event: crate::models::Event,
    severity: i32,
}
