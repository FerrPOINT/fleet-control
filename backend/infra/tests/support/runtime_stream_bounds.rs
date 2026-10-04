use super::*;
use axum::{
    body::{Body, Bytes},
    http::{HeaderValue, Response, header},
};
use serde_json::{Value, json};
use std::{collections::VecDeque, convert::Infallible};

#[derive(Clone)]
pub(super) struct EventResponse {
    chunks: Vec<Vec<u8>>,
    mime: Option<&'static str>,
    encoding: Option<&'static str>,
    delay_after_first: Duration,
}

impl EventResponse {
    pub fn body(chunks: Vec<Vec<u8>>) -> Self {
        Self {
            chunks,
            mime: Some("text/event-stream; charset=utf-8"),
            encoding: None,
            delay_after_first: Duration::from_millis(3),
        }
    }

    pub fn settlement_timeout(&self) -> Duration {
        if self.delay_after_first > Duration::from_secs(1) {
            self.delay_after_first + Duration::from_secs(10)
        } else {
            Duration::from_secs(10)
        }
    }

    pub fn response(self) -> Response<Body> {
        let delay = self.delay_after_first;
        let stream = futures_util::stream::unfold(
            (VecDeque::from(self.chunks), true),
            move |(mut chunks, first)| async move {
                let chunk = chunks.pop_front()?;
                sleep(if first {
                    Duration::from_millis(3)
                } else {
                    delay
                })
                .await;
                Some((Ok::<_, Infallible>(Bytes::from(chunk)), (chunks, false)))
            },
        );
        let mut response = Response::new(Body::from_stream(stream));
        if let Some(mime) = self.mime {
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
        }
        if let Some(encoding) = self.encoding {
            response
                .headers_mut()
                .insert(header::CONTENT_ENCODING, HeaderValue::from_static(encoding));
        }
        response
    }
}

fn running() -> Value {
    json!({"object":"hermes.run","run_id":"fixture-run","status":"running"})
}

async fn rejected(response: EventResponse) {
    let Some((repo, session)) =
        runtime_http_events_scenario(running(), response, SessionRunState::Waiting, false).await
    else {
        return;
    };
    assert!(
        repo.list_session_approvals(session)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.list_session_events(session, 0)
            .await
            .unwrap()
            .iter()
            .all(|event| event.event_type != "session_run_delta")
    );
    assert!(
        repo.list_session_messages(session)
            .await
            .unwrap()
            .iter()
            .all(|message| !matches!(
                message.message_kind,
                MessageKind::ToolEvent | MessageKind::AssistantMessage
            ))
    );
}

#[tokio::test]
async fn runtime_stream_bounds_split_utf8_crlf_multiline_and_empty_event_name_reset() {
    let events = concat!(
        "\u{feff}: heartbeat\r\nevent: tool.event\r\n\r\n",
        "data: {\r\ndata: \"run_id\":\"fixture-run\",\"event\":\"message.delta\",\"delta\":\"\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}\"}\r\n\r\n",
        "data: {\"event\":\"run.completed\",\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"verified response\"}\r\n\r\n"
    );
    let response = EventResponse::body(
        events
            .as_bytes()
            .chunks(1)
            .map(|chunk| chunk.to_vec())
            .collect(),
    );
    let Some((repo, session)) =
        runtime_http_events_scenario(running(), response, SessionRunState::Completed, true).await
    else {
        return;
    };
    let deltas: Vec<_> = repo
        .list_session_events(session, 0)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "session_run_delta")
        .collect();
    assert_eq!(deltas.len(), 1);
    assert_eq!(
        deltas[0].payload["text"],
        "\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}"
    );
    assert!(
        repo.list_session_messages(session)
            .await
            .unwrap()
            .iter()
            .all(|message| message.message_kind != MessageKind::ToolEvent)
    );
}

#[tokio::test]
async fn runtime_stream_bounds_truncated_terminal_is_not_dispatched_at_eof() {
    for ending in ["", "\n", "\r\n"] {
        let frame = format!(
            "event: run.completed\ndata: {{\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"unframed reply\"}}{ending}"
        );
        rejected(EventResponse::body(vec![frame.into_bytes()])).await;
    }
}

#[tokio::test]
async fn runtime_stream_bounds_invalid_json_utf8_and_foreign_control_payloads_never_mirror() {
    let mut cases = vec![
        b"event: approval.request\ndata: not-json\n\n".to_vec(),
        b"event: approval.request\ndata: []\n\n".to_vec(),
        b"event: tool.event\ndata: \xff\n\n".to_vec(),
        b"event: message.delta\ndata: {\"delta\":\"unbound delta\"}\n\n".to_vec(),
        b"event: tool.event\ndata: {\"command\":\"unbound tool\"}\n\n".to_vec(),
    ];
    for payload in [
        json!({"run_id":"foreign-run","request_id":"foreign","command":"foreign action"}),
        json!({"run_id":null,"request_id":"unbound","command":"unbound approval"}),
        json!({"run_id":42,"request_id":"unbound","command":"unbound approval"}),
        json!({"request_id":"unbound","command":"unbound approval"}),
        json!({"run_id":"fixture-run","session_id":"foreign-session","request_id":"foreign","command":"foreign action"}),
        json!({"run_id":"fixture-run","event":"tool.completed","request_id":"foreign","command":"foreign action"}),
        json!({"run_id":"fixture-run","event":42,"request_id":"foreign","command":"foreign action"}),
    ] {
        cases.push(format!("event: approval.request\ndata: {payload}\n\n").into_bytes());
    }
    for frame in cases {
        rejected(EventResponse::body(vec![frame])).await;
    }
}

#[tokio::test]
async fn runtime_stream_bounds_oversized_unterminated_and_multiline_frames_hold_capacity() {
    for line in ["data: ", ": heartbeat "] {
        let frame = format!("{line}{}", "x".repeat(1024 * 1024));
        rejected(EventResponse::body(vec![frame.into_bytes()])).await;
    }
    rejected(EventResponse::body(vec![
        "data: x\n".repeat(140_000).into_bytes(),
    ]))
    .await;
}

#[tokio::test]
async fn runtime_stream_bounds_wrong_mime_and_encoded_bodies_do_not_bypass_framing() {
    let terminal = b"data: {\"event\":\"run.completed\",\"run_id\":\"fixture-run\",\"completed\":true,\"partial\":false,\"interrupted\":false,\"output\":\"unverified reply\"}\n\n";
    for (mime, encoding) in [
        (None, None),
        (Some("text/plain"), None),
        (Some("text/event-stream"), Some("gzip")),
        (Some("text/event-stream"), Some("br")),
    ] {
        let mut response = EventResponse::body(vec![terminal.to_vec()]);
        response.mime = mime;
        response.encoding = encoding;
        rejected(response).await;
    }
}

#[tokio::test]
async fn runtime_stream_bounds_transcript_limit_stops_before_second_delta_is_persisted() {
    let delta = format!(
        "data: {{\"run_id\":\"fixture-run\",\"event\":\"message.delta\",\"delta\":\"{}\"}}\n\n",
        "x".repeat(600_000)
    );
    let response = EventResponse::body(vec![delta.as_bytes().to_vec(), delta.into_bytes()]);
    let Some((repo, session)) =
        runtime_http_events_scenario(running(), response, SessionRunState::Waiting, false).await
    else {
        return;
    };
    let deltas: Vec<_> = repo
        .list_session_events(session, 0)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "session_run_delta")
        .collect();
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].payload["text"].as_str().unwrap().len(), 600_000);
}

#[tokio::test]
async fn runtime_stream_bounds_actual_idle_and_partial_frame_deadlines_retain_run_identity() {
    for (initial, pause) in [
        (b"data: ".as_slice(), 40),
        (b": heartbeat\n\n".as_slice(), 70),
    ] {
        let started = tokio::time::Instant::now();
        let mut response = EventResponse::body(vec![initial.to_vec(), b"late-data".to_vec()]);
        response.delay_after_first = Duration::from_secs(pause);
        let Some((repo, session)) =
            runtime_http_events_scenario(running(), response, SessionRunState::Waiting, false)
                .await
        else {
            return;
        };
        let run = repo
            .list_session_agent_runs(session)
            .await
            .unwrap()
            .pop()
            .unwrap();
        assert!(
            run.last_error
                .as_deref()
                .unwrap()
                .contains("stream deadline elapsed")
        );
        assert_eq!(run.runtime_run_id.as_deref(), Some("fixture-run"));
        assert_eq!(
            run.runtime_session_id.as_deref(),
            Some("native-fixture-session")
        );
        assert!(started.elapsed() >= Duration::from_secs(if pause == 40 { 30 } else { 60 }));
        assert!(started.elapsed() < Duration::from_secs(pause));
    }
}
