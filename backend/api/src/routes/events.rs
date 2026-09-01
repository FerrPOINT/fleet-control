use app::AppContext;
use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::stream::Stream;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

#[utoipa::path(get, path = "/api/v1/events", tag = "runtime", responses((status = 200, description = "SSE event stream")))]
pub async fn events(
    State(ctx): State<Arc<AppContext>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(ctx.events.subscribe()).filter_map(|event| match event {
        Ok(event) => serde_json::to_string(&event)
            .ok()
            .map(|json| Ok(Event::default().event("fleet").data(json))),
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20)))
}
