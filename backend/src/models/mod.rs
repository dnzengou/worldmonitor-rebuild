//! Data models for WorldMonitor Agents

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Intelligence event
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: String,
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    pub severity: i32,
    pub headline: String,
    pub source: String,
    pub timestamp: i64,
    pub confidence: f32,
    pub created_at: Option<DateTime<Utc>>,
}

impl Event {
    pub fn new(
        country: impl Into<String>,
        lat: f64,
        lon: f64,
        severity: i32,
        headline: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            country: country.into(),
            lat,
            lon,
            severity: severity.clamp(1, 10),
            headline: headline.into(),
            source: source.into(),
            timestamp: Utc::now().timestamp_millis(),
            confidence: 0.5,
            created_at: Some(Utc::now()),
        }
    }
}

/// User account
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub tier: UserTier,
    pub interests: String, // JSON array
    pub countries: String, // JSON array
    pub streak: i32,
    pub last_visit: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub subscription_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum UserTier {
    Free,
    Pro,
    Enterprise,
}

impl User {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            email: None,
            password_hash: None,
            tier: UserTier::Free,
            interests: "[]".to_string(),
            countries: "[]".to_string(),
            streak: 0,
            last_visit: Some(Utc::now()),
            created_at: Some(Utc::now()),
            subscription_expires_at: None,
        }
    }

    pub fn get_interests(&self) -> Vec<String> {
        serde_json::from_str(&self.interests).unwrap_or_default()
    }

    pub fn get_countries(&self) -> Vec<String> {
        serde_json::from_str(&self.countries).unwrap_or_default()
    }

    pub fn is_pro(&self) -> bool {
        matches!(self.tier, UserTier::Pro | UserTier::Enterprise)
    }

    pub fn max_alerts(&self) -> i32 {
        match self.tier {
            UserTier::Free => 3,
            UserTier::Pro => 50,
            UserTier::Enterprise => 1000,
        }
    }

    pub fn data_delay_hours(&self) -> i32 {
        match self.tier {
            UserTier::Free => 24,
            UserTier::Pro => 0,
            UserTier::Enterprise => 0,
        }
    }
}

/// User alert subscription
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Alert {
    pub id: i64,
    pub user_id: String,
    pub country: String,
    pub threshold: i32,
    pub created_at: Option<DateTime<Utc>>,
}

/// Notification record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: i64,
    pub user_id: String,
    pub alert_id: i64,
    pub event_id: String,
    pub sent_at: Option<DateTime<Utc>>,
}

/// Subscription record for monetization
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub tier: UserTier,
    pub stripe_subscription_id: Option<String>,
    pub status: SubscriptionStatus,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    PastDue,
    Unpaid,
}

/// Usage tracking for metering
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageRecord {
    pub id: i64,
    pub user_id: String,
    pub resource_type: String,
    pub amount: i32,
    pub recorded_at: Option<DateTime<Utc>>,
}

/// API response types
pub mod responses {
    use super::*;

    #[derive(Debug, Serialize)]
    pub struct IntelligenceResponse {
        pub events: Vec<Event>,
        pub total: usize,
        pub updated_at: i64,
    }

    #[derive(Debug, Serialize)]
    pub struct BriefResponse {
        pub country: String,
        pub summary: String,
        pub event_count: usize,
        pub risk_level: String,
        pub generated_at: i64,
        pub chain_of_thought: Option<crate::agents::ChainOfThought>,
    }

    #[derive(Debug, Serialize)]
    pub struct UserResponse {
        pub id: String,
        pub tier: UserTier,
        pub streak: i32,
        pub interests: Vec<String>,
        pub countries: Vec<String>,
        pub max_alerts: i32,
        pub data_delay_hours: i32,
        pub is_new: bool,
    }

    #[derive(Debug, Serialize)]
    pub struct AgentStatusResponse {
        pub agents: Vec<crate::agents::AgentHealth>,
        pub metrics: crate::agents::SystemMetrics,
    }

    #[derive(Debug, Serialize)]
    pub struct ErrorResponse {
        pub error: String,
        pub code: Option<String>,
    }

    #[derive(Debug, Serialize)]
    pub struct SuccessResponse {
        pub success: bool,
        pub message: Option<String>,
    }
}

/// Country coordinates lookup
pub fn get_country_coords(country: &str) -> Option<(f64, f64)> {
    let coords: std::collections::HashMap<&str, (f64, f64)> = [
        ("Ukraine", (48.3794, 31.1656)),
        ("Russia", (61.5240, 105.3188)),
        ("Israel", (31.0461, 34.8516)),
        ("Gaza", (31.5017, 34.4668)),
        ("China", (35.8617, 104.1954)),
        ("Taiwan", (23.6978, 120.9605)),
        ("Iran", (32.4279, 53.6880)),
        ("Syria", (34.8021, 38.9968)),
        ("United States", (37.0902, -95.7129)),
        ("United Kingdom", (55.3781, -3.4360)),
        ("Germany", (51.1657, 10.4515)),
        ("France", (46.2276, 2.2137)),
        ("Turkey", (38.9637, 35.2433)),
        ("India", (20.5937, 78.9629)),
        ("Pakistan", (30.3753, 69.3451)),
        ("North Korea", (40.3399, 127.5101)),
        ("South Korea", (35.9078, 127.7669)),
        ("Japan", (36.2048, 138.2529)),
        ("Brazil", (-14.2350, -51.9253)),
        ("Egypt", (26.8206, 30.8025)),
        ("South Africa", (-30.5595, 22.9375)),
        ("Nigeria", (9.0820, 8.6753)),
        ("Australia", (-25.2744, 133.7751)),
        ("Canada", (56.1304, -106.3468)),
        ("Mexico", (23.6345, -102.5528)),
        ("Saudi Arabia", (23.8859, 45.0792)),
        ("United Arab Emirates", (23.4241, 53.8478)),
        ("Iraq", (33.2232, 43.6793)),
        ("Afghanistan", (33.9391, 67.7100)),
        ("Lebanon", (33.8547, 35.8623)),
        ("Yemen", (15.5527, 48.5164)),
    ].iter().cloned().collect();

    coords.get(country).copied()
}

/// Extract country names from text
pub fn extract_countries(text: &str) -> Vec<String> {
    let countries = vec![
        "Ukraine", "Russia", "Israel", "Gaza", "China", "Taiwan", "Iran", "Syria",
        "United States", "United Kingdom", "Germany", "France", "Turkey", "Saudi Arabia",
        "India", "Pakistan", "North Korea", "South Korea", "Japan", "Australia",
        "Brazil", "Mexico", "Canada", "Egypt", "South Africa", "Nigeria", "Ethiopia",
        "United Arab Emirates", "Iraq", "Afghanistan", "Lebanon", "Jordan", "Yemen",
        "Oman", "Qatar", "Kuwait", "Bahrain", "Morocco", "Algeria", "Tunisia",
        "Libya", "Sudan", "Somalia", "Kenya", "Tanzania", "Uganda", "Ghana",
    ];

    let text_lower = text.to_lowercase();
    countries
        .into_iter()
        .filter(|c| text_lower.contains(&c.to_lowercase()))
        .map(|c| c.to_string())
        .collect()
}
