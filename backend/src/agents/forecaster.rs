//! Forecaster Agent
//! 
//! Predicts future developments based on current trends:
//! - Trend extrapolation
//! - Scenario modeling
//! - Risk assessment
//! - Timeline predictions

use async_trait::async_trait;
use tracing::{info, warn};

use super::*;

pub struct ForecasterAgent {
    id: AgentId,
    context: Arc<AgentContext>,
}

impl ForecasterAgent {
    pub fn new(id: AgentId, context: Arc<AgentContext>) -> Self {
        Self { id, context }
    }

    async fn analyze_trends(
        &self,
        country: &str,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<TrendAnalysis> {
        let events = self.context.db.get_events_by_country(country, 168).await?; // 7 days

        // Calculate trend
        let recent: Vec<_> = events.iter().take(events.len() / 3).collect();
        let older: Vec<_> = events.iter().skip(events.len() * 2 / 3).collect();

        let recent_avg = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|e| e.severity).sum::<i32>() as f64 / recent.len() as f64
        };

        let older_avg = if older.is_empty() {
            0.0
        } else {
            older.iter().map(|e| e.severity).sum::<i32>() as f64 / older.len() as f64
        };

        let trend_direction = if recent_avg > older_avg + 1.0 {
            "escalating"
        } else if recent_avg < older_avg - 1.0 {
            "de-escalating"
        } else {
            "stable"
        };

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::Forecaster,
            format!("Analyzing 7-day trend for {}", country),
            "Compare recent vs older events",
            format!("Trend: {} (recent avg: {:.1}, older avg: {:.1})", 
                trend_direction, recent_avg, older_avg),
            0.8,
        ));

        Ok(TrendAnalysis {
            direction: trend_direction.to_string(),
            recent_avg,
            older_avg,
            event_count: events.len(),
        })
    }

    async fn generate_forecast(
        &self,
        country: &str,
        trends: &TrendAnalysis,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Forecast> {
        let (risk_level, prediction) = match trends.direction.as_str() {
            "escalating" => ("high", 
                format!("Based on the escalating trend in {}, expect continued tensions. "
                    + "Monitor for potential flashpoints in the next 24-48 hours.", country)),
            "de-escalating" => ("low",
                format!("The situation in {} shows signs of de-escalation. "
                    + "Continue monitoring but risk appears to be decreasing.", country)),
            _ => ("medium",
                format!("The situation in {} remains relatively stable with no clear trend. "
                    + "Maintain standard monitoring protocols.", country)),
        };

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::Forecaster,
            format!("Generating forecast for {}", country),
            "Apply trend analysis to prediction model",
            format!("Risk level: {}, prediction generated", risk_level),
            0.75,
        ));

        Ok(Forecast {
            country: country.to_string(),
            risk_level: risk_level.to_string(),
            prediction,
            confidence: if trends.event_count > 10 { 0.8 } else { 0.6 },
            timeframe: "24-48 hours".to_string(),
        })
    }
}

#[async_trait]
impl Agent for ForecasterAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Forecaster
    }

    fn id(&self) -> AgentId {
        self.id
    }

    async fn execute(&self, task: AgentTask) -> anyhow::Result<AgentResult> {
        let start = std::time::Instant::now();
        let mut cot = ChainOfThought::new(&task.id);

        let payload: ForecastPayload = serde_json::from_value(task.payload.clone())?;

        let trends = self.analyze_trends(&payload.country, &mut cot).await?;
        let forecast = self.generate_forecast(&payload.country, &trends, &mut cot).await?;

        cot.complete(&forecast.prediction, forecast.confidence);

        let result = AgentResult {
            task_id: task.id,
            agent_type: self.agent_type(),
            success: true,
            data: serde_json::to_value(&forecast)?,
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
struct ForecastPayload {
    country: String,
}

#[derive(Debug, Clone)]
struct TrendAnalysis {
    direction: String,
    recent_avg: f64,
    older_avg: f64,
    event_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    pub country: String,
    pub risk_level: String,
    pub prediction: String,
    pub confidence: f32,
    pub timeframe: String,
}
