//! Database layer for WorldMonitor Agents

use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use tracing::{error, info};

use crate::models::*;

#[derive(Clone)]
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> anyhow::Result<()> {
        info!("Running database migrations");

        // Events table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                country TEXT NOT NULL,
                lat REAL NOT NULL,
                lon REAL NOT NULL,
                severity INTEGER NOT NULL CHECK(severity BETWEEN 1 AND 10),
                headline TEXT NOT NULL,
                source TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                confidence REAL DEFAULT 0.5,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)")
            .execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_country ON events(country)")
            .execute(&self.pool).await?;

        // Users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE,
                password_hash TEXT,
                tier TEXT DEFAULT 'free' CHECK(tier IN ('free', 'pro', 'enterprise')),
                interests TEXT DEFAULT '[]',
                countries TEXT DEFAULT '[]',
                streak INTEGER DEFAULT 0,
                last_visit DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                subscription_expires_at DATETIME
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Alerts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                country TEXT NOT NULL,
                threshold INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Notifications table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                alert_id INTEGER NOT NULL,
                event_id TEXT NOT NULL,
                sent_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Subscriptions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS subscriptions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                tier TEXT NOT NULL,
                stripe_subscription_id TEXT,
                status TEXT DEFAULT 'active',
                current_period_start DATETIME,
                current_period_end DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Usage tracking
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                resource_type TEXT NOT NULL,
                amount INTEGER NOT NULL,
                recorded_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        info!("Migrations completed");
        Ok(())
    }

    // ========== Event Operations ==========

    pub async fn batch_insert_events(&self, events: &[Event]) -> anyhow::Result<usize> {
        if events.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut count = 0;

        for event in events {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO events 
                (id, country, lat, lon, severity, headline, source, timestamp, confidence)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&event.id)
            .bind(&event.country)
            .bind(event.lat)
            .bind(event.lon)
            .bind(event.severity)
            .bind(&event.headline)
            .bind(&event.source)
            .bind(event.timestamp)
            .bind(event.confidence)
            .execute(&mut *tx)
            .await?;
            count += 1;
        }

        tx.commit().await?;
        Ok(count)
    }

    pub async fn get_recent_events(&self, limit: i32, hours_back: i32) -> anyhow::Result<Vec<Event>> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events
            WHERE timestamp > strftime('%s', 'now', ?) * 1000
            ORDER BY severity DESC, timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(format!("-{} hours", hours_back))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    pub async fn get_events_by_country(&self, country: &str, hours_back: i32) -> anyhow::Result<Vec<Event>> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events
            WHERE country = ? AND timestamp > strftime('%s', 'now', ?) * 1000
            ORDER BY severity DESC, timestamp DESC
            "#,
        )
        .bind(country)
        .bind(format!("-{} hours", hours_back))
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    pub async fn get_event(&self, id: &str) -> anyhow::Result<Option<Event>> {
        let event = sqlx::query_as::<_, Event>(
            "SELECT * FROM events WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(event)
    }

    pub async fn find_nearby_events(
        &self,
        lat: f64,
        lon: f64,
        radius_degrees: f64,
        hours_back: i32,
    ) -> anyhow::Result<Vec<Event>> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events
            WHERE ABS(lat - ?) < ? AND ABS(lon - ?) < ?
            AND timestamp > strftime('%s', 'now', ?) * 1000
            ORDER BY timestamp DESC
            "#,
        )
        .bind(lat)
        .bind(radius_degrees)
        .bind(lon)
        .bind(radius_degrees)
        .bind(format!("-{} hours", hours_back))
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    pub async fn update_event_confidence(&self, id: &str, confidence: f32) -> anyhow::Result<()> {
        sqlx::query("UPDATE events SET confidence = ? WHERE id = ?")
            .bind(confidence)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========== User Operations ==========

    pub async fn get_or_create_user(&self, id: &str) -> anyhow::Result<User> {
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match user {
            Some(u) => Ok(u),
            None => {
                let new_user = User::new(id);
                sqlx::query(
                    "INSERT INTO users (id, tier, interests, countries, streak, last_visit) VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind(&new_user.id)
                .bind("free")
                .bind(&new_user.interests)
                .bind(&new_user.countries)
                .bind(new_user.streak)
                .bind(new_user.last_visit)
                .execute(&self.pool)
                .await?;
                Ok(new_user)
            }
        }
    }

    pub async fn update_user(&self, user: &User) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE users SET interests = ?, countries = ?, streak = ?, last_visit = ? WHERE id = ?"
        )
        .bind(&user.interests)
        .bind(&user.countries)
        .bind(user.streak)
        .bind(user.last_visit)
        .bind(&user.id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_streak(&self, user_id: &str, streak: i32) -> anyhow::Result<()> {
        sqlx::query("UPDATE users SET streak = ?, last_visit = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(streak)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ========== Alert Operations ==========

    pub async fn create_alert(&self, user_id: &str, country: &str, threshold: i32) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO alerts (user_id, country, threshold) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(country)
            .bind(threshold)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_alerts(&self, user_id: &str) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM alerts WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    pub async fn get_user_alerts(&self, user_id: &str) -> anyhow::Result<Vec<Alert>> {
        let alerts = sqlx::query_as::<_, Alert>("SELECT * FROM alerts WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(alerts)
    }

    // ========== Notification Operations ==========

    pub async fn record_notification(&self, user_id: &str, alert_id: i64) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO notifications (user_id, alert_id, event_id) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(alert_id)
            .bind("pending")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_recent_notifications(&self, user_id: &str, hours: i32) -> anyhow::Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM notifications 
            WHERE user_id = ? AND sent_at > datetime('now', ?)
            "#
        )
        .bind(user_id)
        .bind(format!("-{} hours", hours))
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    // ========== Usage Tracking ==========

    pub async fn record_usage(&self, user_id: &str, resource_type: &str, amount: i32) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO usage_records (user_id, resource_type, amount) VALUES (?, ?, ?)")
            .bind(user_id)
            .bind(resource_type)
            .bind(amount)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_events_since(&self, since: i64) -> anyhow::Result<Vec<Event>> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events
            WHERE timestamp > ?
            ORDER BY timestamp DESC
            LIMIT 100
            "#,
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }
}
