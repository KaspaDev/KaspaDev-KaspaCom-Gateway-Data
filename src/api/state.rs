use crate::application::{ContentService, KaspaComService, TickerService};
use crate::infrastructure::RateLimiter;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub content_service: Arc<ContentService>,
    pub ticker_service: Arc<TickerService>,
    pub kaspacom_service: Arc<KaspaComService>,
    pub rate_limiter: Arc<RateLimiter>,
}

