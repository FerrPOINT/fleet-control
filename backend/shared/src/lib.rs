pub mod config;
pub mod events;
pub mod id;
pub mod telemetry;

pub use config::*;
pub use events::*;
pub use id::*;

// Fleet-shared error lives in sdlc-shared (services-base); re-export keeps
// every `shared::AppError` call site intact.
pub use sdlc_shared::{AppError, AppResult, ErrorBody, ErrorEnvelope};

use chrono::{DateTime, FixedOffset, Utc};

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub type Timestamp = DateTime<FixedOffset>;

pub fn now() -> Timestamp {
    Utc::now().into()
}
