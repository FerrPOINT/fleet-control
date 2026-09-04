//! Fleet-control telemetry: re-export of the shared `sdlc-telemetry`
//! (services-base). The local copy is gone; call sites are unchanged.

pub use sdlc_telemetry::{REQUEST_ID_HEADER, RequestId, init_tracing, request_id_mw};
