//! PriceWatch: tracks product prices on online stores and alerts when a price
//! drops to the user's target.

pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod notify;
pub mod repo;
pub mod scheduler;
pub mod scraper;
pub mod state;

pub use config::Config;
pub use state::AppState;
