//! Data Collector Agent
//! 
//! Collects and fuses intelligence from multiple sources:
//! - GDELT (conflict events)
//! - RSS feeds (news)
//! - User reports
//! - External APIs
//! 
//! Uses chain-of-thought reasoning to:
//! 1. Identify which sources to query
//! 2. Fetch data from each source
//! 3. Normalize and deduplicate
//! 4. Assign confidence scores
//! 5. Fuse into unified events

use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use super::*;

/// Data sources for intelligence collection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Gdelt,
    Rss,
    UserReport,
    ExternalApi,
}

pub struct DataCollectorAgent {
    id: AgentId,
    context: Arc<AgentContext>,
}

impl DataCollectorAgent {
    pub fn new(id: AgentId, context: Arc<AgentContext>) -> Self {
        Self { id, context }
    }

    /// Step 1: Identify which sources to query
    async fn identify_sources(
        &self,
        task: &AgentTask,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<DataSource>> {
        let start = chrono::Utc::now().timestamp_millis();
        
        // Parse task payload to determine sources
        let payload: CollectionPayload = serde_json::from_value(task.payload.clone())?;
        
        let mut sources = vec![DataSource::Gdelt, DataSource::Rss];
        
        if payload.include_user_reports {
            sources.push(DataSource::UserReport);
        }
        
        if payload.external_apis.is_some() {
            sources.push(DataSource::ExternalApi);
        }

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::DataCollector,
            format!("Task requires collecting intelligence for region: {:?}", payload.region),
            format!("Selected {} data sources: {:?}", sources.len(), sources),
            format!("Sources identified based on task parameters and availability"),
            0.95,
        ));

        debug!("Identified {} sources in {}ms", sources.len(), 
            chrono::Utc::now().timestamp_millis() - start);
        
        Ok(sources)
    }

    /// Step 2: Fetch data from GDELT
    async fn fetch_gdelt(
        &self,
        region: &Option<String>,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<RawEvent>> {
        let start = chrono::Utc::now().timestamp_millis();
        info!("Fetching from GDELT for region: {:?}", region);

        let query = region.as_deref().unwrap_or("conflict");
        let url = format!(
            "https://api.gdeltproject.org/api/v2/geo/geo?query={}&format=geojson&timespan=1h",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        let response = client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("GDELT returned status: {}", response.status()));
        }

        let gdelt_data: serde_json::Value = response.json().await?;
        let mut events = Vec::new();

        if let Some(features) = gdelt_data.get("features").and_then(|f| f.as_array()) {
            for feature in features {
                if let Some(event) = self.parse_gdelt_feature(feature).await {
                    events.push(event);
                }
            }
        }

        let elapsed = chrono::Utc::now().timestamp_millis() - start;
        
        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::DataCollector,
            format!("Querying GDELT API for recent conflict events"),
            format!("GET {}", url),
            format!("Retrieved {} events in {}ms", events.len(), elapsed),
            if events.is_empty() { 0.5 } else { 0.9 },
        ));

        info!("Fetched {} events from GDELT in {}ms", events.len(), elapsed);
        Ok(events)
    }

    /// Step 3: Fetch data from RSS feeds
    async fn fetch_rss(
        &self,
        region: &Option<String>,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<RawEvent>> {
        let start = chrono::Utc::now().timestamp_millis();
        info!("Fetching from RSS for region: {:?}", region);

        let feeds = vec![
            "https://feeds.reuters.com/reuters/conflicts",
            "https://feeds.bbci.co.uk/news/world/rss.xml",
            "https://feeds.france24.com/en",
        ];

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let mut all_events = Vec::new();
        let mut successful_feeds = 0;

        for feed_url in feeds {
            let url = format!(
                "https://rss2json.com/api.json?rss_url={}",
                urlencoding::encode(feed_url)
            );

            match client.get(&url).send().await {
                Ok(response) => {
                    if let Ok(rss_data) = response.json::<serde_json::Value>().await {
                        if let Some(items) = rss_data.get("items").and_then(|i| i.as_array()) {
                            for item in items.iter().take(10) {
                                if let Some(event) = self.parse_rss_item(item).await {
                                    // Filter by region if specified
                                    if let Some(ref r) = region {
                                        if event.country.to_lowercase().contains(&r.to_lowercase())
                                            || event.headline.to_lowercase().contains(&r.to_lowercase()) {
                                            all_events.push(event);
                                        }
                                    } else {
                                        all_events.push(event);
                                    }
                                }
                            }
                            successful_feeds += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch RSS feed {}: {}", feed_url, e);
                }
            }
        }

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::DataCollector,
            format!("Querying {} RSS feeds for news items", feeds.len()),
            format!("Parallel fetch from {} feeds", feeds.len()),
            format!("Retrieved {} events from {} feeds in {}ms", 
                all_events.len(), successful_feeds, elapsed),
            if all_events.is_empty() { 0.4 } else { 0.85 },
        ));

        info!("Fetched {} events from RSS in {}ms", all_events.len(), elapsed);
        Ok(all_events)
    }

    /// Step 4: Deduplicate and fuse events
    async fn fuse_events(
        &self,
        all_events: Vec<RawEvent>,
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<Vec<IntelligenceEvent>> {
        let start = chrono::Utc::now().timestamp_millis();
        
        // Grid-based deduplication (0.1 degree precision)
        let mut grid: HashMap<String, RawEvent> = HashMap::new();
        let mut duplicates = 0;

        for event in all_events {
            let key = format!(
                "{}:{}",
                (event.lat * 10.0).round() as i32,
                (event.lon * 10.0).round() as i32
            );

            if let Some(existing) = grid.get(&key) {
                // Keep the one with higher severity or more recent
                if event.severity > existing.severity || 
                   (event.severity == existing.severity && event.timestamp > existing.timestamp) {
                    grid.insert(key, event);
                }
                duplicates += 1;
            } else {
                grid.insert(key, event);
            }
        }

        // Convert to IntelligenceEvent
        let fused: Vec<IntelligenceEvent> = grid.into_values()
            .map(|e| IntelligenceEvent {
                id: uuid::Uuid::new_v4().to_string(),
                country: e.country,
                lat: e.lat,
                lon: e.lon,
                severity: e.severity,
                headline: e.headline,
                source: e.source,
                timestamp: e.timestamp,
                confidence: e.confidence,
            })
            .collect();

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::DataCollector,
            format!("Fusing events using grid-based deduplication (0.1° precision)"),
            format!("Grid dedup with {} total events", all_events.len()),
            format!("Removed {} duplicates, {} unique events remaining in {}ms", 
                duplicates, fused.len(), elapsed),
            0.92,
        ));

        info!("Fused {} events (removed {} duplicates) in {}ms", 
            fused.len(), duplicates, elapsed);
        
        Ok(fused)
    }

    /// Step 5: Store events in database
    async fn store_events(
        &self,
        events: &[IntelligenceEvent],
        cot: &mut ChainOfThought,
    ) -> anyhow::Result<usize> {
        let start = chrono::Utc::now().timestamp_millis();

        let count = self.context.db.batch_insert_events(events).await?;

        let elapsed = chrono::Utc::now().timestamp_millis() - start;

        cot.add_step(ReasoningStep::new(
            cot.steps.len() + 1,
            AgentType::DataCollector,
            format!("Storing fused events in database"),
            format!("Batch INSERT into events table"),
            format!("Stored {} events in {}ms", count, elapsed),
            0.98,
        ));

        info!("Stored {} events in database in {}ms", count, elapsed);
        Ok(count)
    }

    // Helper methods

    async fn parse_gdelt_feature(&self, feature: &serde_json::Value) -> Option<RawEvent> {
        let geometry = feature.get("geometry")?;
        let coords = geometry.get("coordinates")?.as_array()?;
        if coords.len() < 2 {
            return None;
        }

        let lon = coords[0].as_f64()?;
        let lat = coords[1].as_f64()?;

        let properties = feature.get("properties")?;
        let headline = properties
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Conflict Event")
            .to_string();

        let country = properties
            .get("country")
            .and_then(|c| c.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let fatalities = properties
            .get("fatalities")
            .and_then(|f| f.as_i64())
            .unwrap_or(0);

        let severity = if fatalities > 10 {
            10
        } else if fatalities > 0 {
            8
        } else {
            5
        };

        Some(RawEvent {
            country,
            lat,
            lon,
            severity,
            headline,
            source: "gdelt".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            confidence: 0.85,
        })
    }

    async fn parse_rss_item(&self, item: &serde_json::Value) -> Option<RawEvent> {
        let title = item.get("title")?.as_str()?.to_string();
        let description = item
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();

        let text = format!("{} {}", title, description);
        
        // Extract country from text
        let countries = crate::models::extract_countries(&text);
        let country = countries.first()?.clone();
        
        let coords = crate::models::get_country_coords(&country)?;

        // Calculate severity based on keywords
        let severity = calculate_severity_from_text(&text);

        Some(RawEvent {
            country,
            lat: coords.0,
            lon: coords.1,
            severity,
            headline: title,
            source: "rss".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            confidence: 0.75,
        })
    }
}

#[async_trait]
impl Agent for DataCollectorAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::DataCollector
    }

    fn id(&self) -> AgentId {
        self.id
    }

    async fn execute(&self, task: AgentTask) -> anyhow::Result<AgentResult> {
        let start = std::time::Instant::now();
        let mut cot = ChainOfThought::new(&task.id);

        info!("DataCollector executing task {}", task.id);

        // Step 1: Identify sources
        let sources = self.identify_sources(&task, &mut cot).await?;

        // Step 2 & 3: Fetch from all sources in parallel
        let payload: CollectionPayload = serde_json::from_value(task.payload.clone())?;
        
        let mut all_events = Vec::new();

        // Fetch from GDELT
        if sources.contains(&DataSource::Gdelt) {
            match self.fetch_gdelt(&payload.region, &mut cot).await {
                Ok(events) => all_events.extend(events),
                Err(e) => warn!("GDELT fetch failed: {}", e),
            }
        }

        // Fetch from RSS
        if sources.contains(&DataSource::Rss) {
            match self.fetch_rss(&payload.region, &mut cot).await {
                Ok(events) => all_events.extend(events),
                Err(e) => warn!("RSS fetch failed: {}", e),
            }
        }

        // Step 4: Fuse events
        let fused = self.fuse_events(all_events, &mut cot).await?;

        // Step 5: Store in database
        let stored_count = self.store_events(&fused, &mut cot).await?;

        // Complete chain of thought
        cot.complete(
            format!("Successfully collected and fused {} intelligence events from {} sources", 
                stored_count, sources.len()),
            0.88,
        );

        let execution_time = start.elapsed().as_millis() as i64;

        // Update metrics
        {
            let mut metrics = self.context.metrics.write().await;
            metrics.record_success(execution_time);
            metrics.record_reasoning_step();
        }

        Ok(AgentResult {
            task_id: task.id,
            agent_type: self.agent_type(),
            success: true,
            data: serde_json::json!({
                "events_collected": stored_count,
                "sources": sources.len(),
            }),
            chain_of_thought: cot,
            execution_time_ms: execution_time,
        })
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

/// Raw event before fusion
#[derive(Debug, Clone)]
struct RawEvent {
    country: String,
    lat: f64,
    lon: f64,
    severity: i32,
    headline: String,
    source: String,
    timestamp: i64,
    confidence: f32,
}

/// Collection task payload
#[derive(Debug, Clone, Deserialize)]
struct CollectionPayload {
    region: Option<String>,
    include_user_reports: bool,
    external_apis: Option<Vec<String>>,
}

/// Intelligence event after fusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceEvent {
    pub id: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub severity: i32,
    pub headline: String,
    pub source: String,
    pub timestamp: i64,
    pub confidence: f32,
}

fn calculate_severity_from_text(text: &str) -> i32 {
    let text_lower = text.to_lowercase();
    let mut score = 5;

    let high_keywords = [
        "attack", "killed", "death", "bomb", "explosion", "war", "invasion",
        "missile", "strike", "casualties", "fatalities", "massacre", "terror",
    ];

    let medium_keywords = [
        "conflict", "clash", "protest", "riot", "violence", "tension",
        "crisis", "emergency", "threat", "sanctions", "dispute",
    ];

    for keyword in &high_keywords {
        if text_lower.contains(keyword) {
            score = 8;
            break;
        }
    }

    if score == 5 {
        for keyword in &medium_keywords {
            if text_lower.contains(keyword) {
                score = 6;
                break;
            }
        }
    }

    score.clamp(1, 10)
}
