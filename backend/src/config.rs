//! Application configuration

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub groq_api_key: String,
    pub stripe_secret_key: Option<String>,
    pub stripe_publishable_key: Option<String>,
    pub jwt_secret: String,
    pub max_alerts_free: i32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./worldmonitor.db".to_string()),
            groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
            stripe_publishable_key: std::env::var("STRIPE_PUBLISHABLE_KEY").ok(),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production".to_string()),
            max_alerts_free: std::env::var("MAX_ALERTS_FREE")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
        })
    }
}
