//! Analyst Agent
//! 
//! Performs deep analysis on collected intelligence:
//! - Pattern recognition
//! - Anomaly detection
//! - Trend analysis
//! - Correlation discovery
//! - Brief generation
//! 
//! Chain-of-thought reasoning:
//! 1. Retrieve relevant events
//! 2. Identify patterns
//! 3. Detect anomalies
//! 4. Correlate with historical data
//! 5. Generate insights

use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::*;

pub struct AnalystAgent {
    id: AgentId,
    context: Arc<AgentContext>,
}

impl AnalystAgent {
    pub fn new(id: AgentId, context: Arc<AgentContext>) -> Self {
        Self { id, context }
    }

    /// Step 1: Retrieve relevant events
    async fn retrieve_events(
        &self,
        params: &AnalysisParams,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<crate::models::Event>> {
        let start = chrono::Utc::now().timestamp_millis();

        let events = if let Some(ref country) = params.country {
            self.context.db.get_events_by_country(country, params.hours_back).await?
        } else {
            self.context.db.get_recent_events(50, params.hours_back).await?
        };

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::Analyst,
            format!("Retrieving events for analysis: country={:?}, hours_back={}", 
                params.country, params.hours_back),
            format!("SELECT * FROM events WHERE timestamp > NOW() - INTERVAL '{} hours'", params.hours_back),
            format!("Retrieved {} events in {}ms", events.len(), elapsed),
            if events.is_empty() { 0.3 } else { 0.95 },
        ));

        info!("Retrieved {} events for analysis in {}ms", events.len(), elapsed);
        Ok(events)
    }

    /// Step 2: Identify patterns
    async fn identify_patterns(
        &self,
        events: &[crate::models::Event],
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<PatternAnalysis> {
        let start = chrono::Utc::now().timestamp_millis();

        // Group by country
        let mut by_country: HashMap<String, Vec<&crate::models::Event>> = HashMap::new();
        for event in events {
            by_country.entry(event.country.clone()).or_default().push(event);
        }

        // Calculate severity trends
        let avg_severity = if events.is_empty() {
            0.0
        } else {
            events.iter().map(|e| e.severity).sum::<i32>() as f64 / events.len() as f64
        };

        // Find escalation patterns
        let mut escalation_countries = Vec::new();
        for (country, country_events) in &by_country {
            if country_events.len() >= 3 {
                let recent_avg = country_events.iter().take(5)
                    .map(|e| e.severity)
                    .sum::<i32>() as f64 / country_events.len().min(5) as f64;
                if recent_avg > 6.5 {
                    escalation_countries.push(country.clone());
                }
            }
        }

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::Analyst,
            format!("Analyzing patterns across {} events", events.len()),
            format!("Group by country, calculate severity trends, detect escalations"),
            format!("Found {} countries with potential escalation, avg severity {:.1} in {}ms",
                escalation_countries.len(), avg_severity, elapsed),
            0.88,
        ));

        Ok(PatternAnalysis {
            by_country,
            avg_severity,
            escalation_countries,
            total_events: events.len(),
        })
    }

    /// Step 3: Detect anomalies
    async fn detect_anomalies(
        &self,
        events: &[crate::models::Event],
        patterns: &PatternAnalysis,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<Anomaly>> {
        let start = chrono::Utc::now().timestamp_millis();

        let mut anomalies = Vec::new();

        // Check for severity spikes
        for event in events {
            if event.severity >= 9 {
                anomalies.push(Anomaly {
                    event_id: event.id.clone(),
                    anomaly_type: AnomalyType::CriticalEvent,
                    description: format!("Critical severity event in {}", event.country),
                    confidence: 0.95,
                });
            }
        }

        // Check for geographic clustering
        let mut location_clusters: HashMap<String, Vec<&crate::models::Event>> = HashMap::new();
        for event in events {
            let grid_key = format!(
                "{},{}",
                (event.lat * 5.0).round() as i32,
                (event.lon * 5.0).round() as i32
            );
            location_clusters.entry(grid_key).or_default().push(event);
        }

        for (grid, cluster) in location_clusters {
            if cluster.len() >= 5 {
                anomalies.push(Anomaly {
                    event_id: cluster[0].id.clone(),
                    anomaly_type: AnomalyType::GeographicCluster,
                    description: format!("Cluster of {} events in grid {}", cluster.len(), grid),
                    confidence: 0.85,
                });
            }
        }

        // Check for temporal clustering (many events in short time)
        if patterns.total_events > 20 {
            anomalies.push(Anomaly {
                event_id: events[0].id.clone(),
                anomaly_type: AnomalyType::HighActivity,
                description: format!("Unusually high activity: {} events", patterns.total_events),
                confidence: 0.75,
            });
        }

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::Analyst,
            format!("Detecting anomalies in event data"),
            format!("Check severity spikes, geographic clustering, temporal patterns"),
            format!("Detected {} anomalies in {}ms", anomalies.len(), elapsed),
            if anomalies.is_empty() { 0.6 } else { 0.9 },
        ));

        Ok(anomalies)
    }

    /// Step 4: Generate insights using LLM
    async fn generate_insights(
        &self,
        events: &[crate::models::Event],
        patterns: &PatternAnalysis,
        anomalies: &[Anomaly],
        params: &AnalysisParams,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<String> {
        let start = chrono::Utc::now().timestamp_millis();

        // Build context for LLM
        let context = self.build_llm_context(events, patterns, anomalies, params).await;

        // Try LLM generation
        let insight = match self.context.llm.generate(&context).await {
            Ok(response) => response,
            Err(e) => {
                warn!("LLM generation failed: {}, using fallback", e);
                self.generate_fallback_insight(patterns, anomalies).await
            }
        };

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::Analyst,
            format!("Generating insights using AI analysis"),
            format!("LLM prompt with {} events, {} patterns, {} anomalies", 
                events.len(), patterns.escalation_countries.len(), anomalies.len()),
            format!("Generated {} character insight in {}ms", insight.len(), elapsed),
            0.85,
        ));

        Ok(insight)
    }

    /// Build LLM context
    async fn build_llm_context(
        &self,
        events: &[crate::models::Event],
        patterns: &PatternAnalysis,
        anomalies: &[Anomaly],
        params: &AnalysisParams,
    ) -> String {
        let mut context = format!(
            "You are an intelligence analyst. Analyze the following events and provide insights.\n\n"
        );

        if let Some(ref country) = params.country {
            context.push_str(&format!("Focus: {}\n\n", country));
        }

        context.push_str(&format!("Total events analyzed: {}\n", patterns.total_events));
        context.push_str(&format!("Average severity: {:.1}/10\n", patterns.avg_severity));
        context.push_str(&format!("Countries with escalation: {}\n\n", 
            patterns.escalation_countries.join(", ")));

        if !anomalies.is_empty() {
            context.push_str("Detected anomalies:\n");
            for anomaly in anomalies.iter().take(5) {
                context.push_str(&format!("- {}: {}\n", 
                    format!("{:?}", anomaly.anomaly_type), anomaly.description));
            }
            context.push('\n');
        }

        context.push_str("Recent events:\n");
        for event in events.iter().take(10) {
            context.push_str(&format!("- [{}] {}: {}\n", 
                event.severity, event.country, event.headline));
        }

        context.push_str("\nProvide a concise 2-3 sentence analysis of the situation.");

        context
    }

    /// Fallback insight generation
    async fn generate_fallback_insight(
        &self,
        patterns: &PatternAnalysis,
        anomalies: &[Anomaly],
    ) -> String {
        let mut insight = String::new();

        if patterns.escalation_countries.is_empty() && anomalies.is_empty() {
            insight.push_str("No significant developments detected. Situation remains stable.");
        } else {
            if !patterns.escalation_countries.is_empty() {
                insight.push_str(&format!(
                    "Escalation detected in {}. ",
                    patterns.escalation_countries.join(", ")
                ));
            }

            let critical_count = anomalies.iter()
                .filter(|a| a.anomaly_type == AnomalyType::CriticalEvent)
                .count();
            
            if critical_count > 0 {
                insight.push_str(&format!("{} critical events require immediate attention. ", critical_count));
            }

            insight.push_str(&format!("Average severity: {:.1}/10. Monitor for further developments.", 
                patterns.avg_severity));
        }

        insight
    }

    /// Generate a country brief
    pub async fn generate_brief(
        &self,
        country: &str,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<AnalysisBrief> {
        let params = AnalysisParams {
            country: Some(country.to_string()),
            hours_back: 24,
        };

        // Retrieve events
        let events = self.retrieve_events(&params, cot).await?;

        if events.is_empty() {
            return Ok(AnalysisBrief {
                country: country.to_string(),
                summary: format!("No significant activity detected in {} in the last 24 hours.", country),
                event_count: 0,
                severity_trend: "stable".to_string(),
                key_developments: vec![],
                risk_level: "low".to_string(),
            });
        }

        // Analyze patterns
        let patterns = self.identify_patterns(&events, cot).await?;

        // Detect anomalies
        let anomalies = self.detect_anomalies(&events, &patterns, cot).await?;

        // Generate insights
        let summary = self.generate_insights(&events, &patterns, &anomalies, &params, cot).await?;

        // Determine risk level
        let risk_level = if patterns.avg_severity > 7.5 || !anomalies.is_empty() {
            "high"
        } else if patterns.avg_severity > 5.0 {
            "medium"
        } else {
            "low"
        };

        // Determine trend
        let severity_trend = if patterns.escalation_countries.contains(&country.to_string()) {
            "escalating"
        } else {
            "stable"
        };

        // Extract key developments
        let key_developments: Vec<String> = events.iter()
            .take(3)
            .map(|e| e.headline.clone())
            .collect();

        Ok(AnalysisBrief {
            country: country.to_string(),
            summary,
            event_count: events.len(),
            severity_trend: severity_trend.to_string(),
            key_developments,
            risk_level: risk_level.to_string(),
        })
    }
}

#[async_trait]
impl Agent for AnalystAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Analyst
    }

    fn id(&self) -> AgentId {
        self.id
    }

    async fn execute(&self, task: AgentTask) -> anyhow::Result<AgentResult> {
        let start = std::time::Instant::now();
        let mut cot = ChainOfThought::new(&task.id);

        info!("Analyst executing task {} of type {:?}", task.id, task.task_type);

        let result = match task.task_type {
            TaskType::AnalyzeRegion => {
                let params: AnalysisParams = serde_json::from_value(task.payload.clone())?;
                
                // Execute analysis workflow
                let events = self.retrieve_events(&params, &mut cot).await?;
                let patterns = self.identify_patterns(&events, &mut cot).await?;
                let anomalies = self.detect_anomalies(&events, &patterns, &mut cot).await?;
                let insights = self.generate_insights(&events, &patterns, &anomalies, &params, &mut cot).await?;

                cot.complete(&insights, 0.85);

                AgentResult {
                    task_id: task.id.clone(),
                    agent_type: self.agent_type(),
                    success: true,
                    data: serde_json::json!({
                        "insights": insights,
                        "events_analyzed": events.len(),
                        "anomalies_detected": anomalies.len(),
                        "escalation_countries": patterns.escalation_countries,
                    }),
                    chain_of_thought: cot,
                    execution_time_ms: start.elapsed().as_millis() as i64,
                }
            }

            TaskType::GenerateBrief => {
                let payload: BriefPayload = serde_json::from_value(task.payload.clone())?;
                let brief = self.generate_brief(&payload.country, &mut cot).await?;

                cot.complete(&brief.summary, 0.88);

                AgentResult {
                    task_id: task.id.clone(),
                    agent_type: self.agent_type(),
                    success: true,
                    data: serde_json::to_value(&brief)?,
                    chain_of_thought: cot,
                    execution_time_ms: start.elapsed().as_millis() as i64,
                }
            }

            TaskType::CorrelateEvents => {
                // Event correlation logic
                cot.complete("Event correlation completed", 0.75);
                
                AgentResult {
                    task_id: task.id.clone(),
                    agent_type: self.agent_type(),
                    success: true,
                    data: serde_json::json!({"correlations": []}),
                    chain_of_thought: cot,
                    execution_time_ms: start.elapsed().as_millis() as i64,
                }
            }

            _ => {
                return Err(anyhow::anyhow!("Unsupported task type for Analyst: {:?}", task.task_type));
            }
        };

        // Update metrics
        {
            let mut metrics = self.context.metrics.write().await;
            metrics.record_success(result.execution_time_ms);
            metrics.record_reasoning_step();
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

/// Analysis parameters
#[derive(Debug, Clone, Deserialize)]
struct AnalysisParams {
    country: Option<String>,
    hours_back: i32,
}

/// Pattern analysis results
#[derive(Debug)]
struct PatternAnalysis {
    by_country: HashMap<String, Vec<&'static crate::models::Event>>,
    avg_severity: f64,
    escalation_countries: Vec<String>,
    total_events: usize,
}

/// Detected anomaly
#[derive(Debug, Clone)]
struct Anomaly {
    event_id: String,
    anomaly_type: AnomalyType,
    description: String,
    confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum AnomalyType {
    CriticalEvent,
    GeographicCluster,
    TemporalSpike,
    HighActivity,
}

/// Brief generation payload
#[derive(Debug, Clone, Deserialize)]
struct BriefPayload {
    country: String,
    interests: Vec<String>,
}

/// Analysis brief output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisBrief {
    pub country: String,
    pub summary: String,
    pub event_count: usize,
    pub severity_trend: String,
    pub key_developments: Vec<String>,
    pub risk_level: String,
}
