//! Analytics and A/B testing module
//!
//! Provides:
//! - A/B testing for prompt experiments
//! - Bot analytics and funnel metrics
//! - Conversion tracking

pub mod ab_testing;
pub mod bot_analytics;
pub mod evaluate_dialogs;

pub use ab_testing::{ABTestManager, PromptVariant};
pub use bot_analytics::{BotAnalytics, SessionStats};
pub use evaluate_dialogs::DialogEvaluator;
