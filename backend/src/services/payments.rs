//! Payment service for subscription management

use crate::models::{Subscription, SubscriptionStatus, UserTier};

pub struct PaymentService {
    stripe_secret_key: Option<String>,
}

impl PaymentService {
    pub fn new(stripe_secret_key: Option<String>) -> Self {
        Self { stripe_secret_key }
    }

    pub fn is_configured(&self) -> bool {
        self.stripe_secret_key.is_some()
    }

    /// Create a checkout session for subscription
    pub async fn create_checkout_session(
        &self,
        user_id: &str,
        tier: UserTier,
    ) -> anyhow::Result<String> {
        // This would integrate with Stripe
        // For now, return a placeholder
        Ok(format!("checkout_session_{}_{:?}", user_id, tier))
    }

    /// Get pricing for a tier
    pub fn get_tier_price(&self, tier: UserTier) -> (i64, &'static str) {
        match tier {
            UserTier::Free => (0, "Free"),
            UserTier::Pro => (900, "$9/month"), // $9.00 in cents
            UserTier::Enterprise => (29900, "$299/month"), // $299.00 in cents
        }
    }

    /// Get tier features for display
    pub fn get_tier_features(&self, tier: UserTier) -> Vec<&'static str> {
        match tier {
            UserTier::Free => vec![
                "24h delayed data",
                "3 alerts maximum",
                "Basic briefings",
                "Community support",
            ],
            UserTier::Pro => vec![
                "Real-time data",
                "50 alerts",
                "AI-powered briefings",
                "90-day history",
                "Email support",
            ],
            UserTier::Enterprise => vec![
                "Real-time data",
                "Unlimited alerts",
                "Custom integrations",
                "Full history",
                "Priority support",
                "API access",
                "Team workspaces",
            ],
        }
    }
}
