use app::AppContext;
use axum::{
    Extension, Json,
    extract::Query,
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use domain::AgentEvent;
use futures_util::stream::Stream;
use serde::Deserialize;
use shared::AppError;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub limit: Option<u64>,
}

#[utoipa::path(get, path = "/api/v1/events/recent", tag = "runtime", responses((status = 200, body = Vec<AgentEvent>)))]
pub async fn recent_events(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Vec<AgentEvent>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_events(query.limit.unwrap_or(100).min(500))
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/events", tag = "runtime", responses((status = 200, description = "SSE event stream")))]
pub async fn events(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    crate::middleware::require_operator(&user)?;
    let stream = BroadcastStream::new(ctx.events.subscribe()).filter_map(|event| match event {
        Ok(event) => serde_json::to_string(&event)
            .ok()
            .map(|json| Ok(Event::default().event("fleet").data(json))),
        Err(_) => None,
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20))))
}
