//! Bounded framing for the pinned Hermes JSON event-stream profile.
use reqwest::{Response, StatusCode, header};
use shared::AppError;
use std::time::Duration;
use tokio::time::Instant;

pub(super) const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_STREAM_BYTES: usize = 32 * 1024 * 1024;
const MAX_EVENTS: usize = 8192;
const MAX_TRANSCRIPT_BYTES: usize = 1024 * 1024;
const MAX_MIRROR_BYTES: usize = 16 * 1024 * 1024;
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const FRAME_TIMEOUT: Duration = Duration::from_secs(30);
const STREAM_LIFETIME: Duration = Duration::from_secs(30 * 60);

fn invalid(detail: &'static str) -> AppError {
    AppError::Unavailable(detail.into())
}

pub(super) fn verify_headers(response: &Response) -> Result<(), AppError> {
    let valid_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .eq_ignore_ascii_case("text/event-stream")
        });
    if response.status() != StatusCode::OK
        || !valid_type
        || response
            .headers()
            .get(header::CONTENT_ENCODING)
            .is_some_and(|value| value != "identity")
        || response
            .content_length()
            .is_some_and(|bytes| bytes > MAX_STREAM_BYTES as u64)
    {
        return Err(invalid(
            "Hermes event stream headers do not match the bounded profile",
        ));
    }
    Ok(())
}

pub(super) struct Event {
    pub name: Option<String>,
    pub data: String,
}

#[derive(Default)]
pub(super) struct Decoder {
    line: Vec<u8>,
    name: Option<String>,
    data: String,
    data_present: bool,
    frame_bytes: usize,
    skip_lf: bool,
    first_line_seen: bool,
}

impl Decoder {
    pub fn pending(&self) -> bool {
        self.frame_bytes != 0
    }

    // Line decoding waits for the delimiter, so split UTF-8 codepoints are never lossy.
    pub fn push(&mut self, byte: u8) -> Result<Option<Event>, AppError> {
        if std::mem::take(&mut self.skip_lf) && byte == b'\n' {
            // A dispatched CR delimiter already ended the frame; only its optional LF is ignored.
            if self.pending() {
                self.count_byte()?;
            }
            return Ok(None);
        }
        self.count_byte()?;
        if !matches!(byte, b'\r' | b'\n') {
            self.line.push(byte);
            return Ok(None);
        }
        self.skip_lf = byte == b'\r';
        let line = std::str::from_utf8(&self.line)
            .map_err(|_| invalid("Hermes event frame is not valid UTF-8"))?;
        let line = if !self.first_line_seen {
            line.strip_prefix('\u{feff}').unwrap_or(line)
        } else {
            line
        };
        self.first_line_seen = true;
        if line.is_empty() {
            let name = self.name.take();
            let data = std::mem::take(&mut self.data);
            let present = std::mem::take(&mut self.data_present);
            self.frame_bytes = 0;
            self.line.clear();
            return Ok(present.then_some(Event { name, data }));
        }
        if !line.starts_with(':') {
            let (field, value) = line.split_once(':').unwrap_or((line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => self.name = (!value.is_empty()).then(|| value.to_owned()),
                "data" => {
                    if self.data_present {
                        self.data.push('\n');
                    }
                    self.data.push_str(value);
                    self.data_present = true;
                }
                _ => {}
            }
        }
        self.line.clear();
        Ok(None)
    }

    fn count_byte(&mut self) -> Result<(), AppError> {
        if self.frame_bytes >= MAX_FRAME_BYTES {
            return Err(invalid("Hermes event frame exceeds its byte limit"));
        }
        self.frame_bytes += 1;
        Ok(())
    }
    // EOF deliberately has no flush: only a blank-line delimiter dispatches a frame.
}

pub(super) struct Budget {
    idle_deadline: Instant,
    absolute_deadline: Instant,
    frame_started: Option<Instant>,
    bytes: usize,
    events: usize,
}

impl Budget {
    pub fn new(now: Instant) -> Self {
        Self {
            idle_deadline: now + IDLE_TIMEOUT,
            absolute_deadline: now + STREAM_LIFETIME,
            frame_started: None,
            bytes: 0,
            events: 0,
        }
    }

    pub fn deadline(&self) -> Instant {
        self.idle_deadline.min(self.absolute_deadline).min(
            self.frame_started
                .map(|time| time + FRAME_TIMEOUT)
                .unwrap_or(self.absolute_deadline),
        )
    }

    pub fn receive(&mut self, count: usize, now: Instant) -> Result<(), AppError> {
        if now >= self.deadline() {
            return Err(invalid("Hermes event stream deadline elapsed"));
        }
        self.bytes = self
            .bytes
            .checked_add(count)
            .filter(|bytes| *bytes <= MAX_STREAM_BYTES)
            .ok_or_else(|| invalid("Hermes event stream exceeds its total byte limit"))?;
        if count != 0 {
            self.idle_deadline = now + IDLE_TIMEOUT;
        }
        Ok(())
    }

    pub fn frame(&mut self, pending: bool, now: Instant) {
        if pending {
            self.frame_started.get_or_insert(now);
        } else {
            self.frame_started = None;
        }
    }

    pub fn event(&mut self) -> Result<(), AppError> {
        self.events += 1;
        if self.events > MAX_EVENTS {
            return Err(invalid("Hermes event stream exceeds its event limit"));
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Transcript {
    pub text: String,
    mirror_bytes: usize,
}

impl Transcript {
    pub fn append(&mut self, delta: &str) -> Result<(), AppError> {
        if self.text.len().saturating_add(delta.len()) > MAX_TRANSCRIPT_BYTES {
            return Err(invalid("Hermes transcript exceeds its byte limit"));
        }
        self.text.push_str(delta);
        Ok(())
    }

    pub fn mirror(&mut self, text: &str) -> Result<(), AppError> {
        self.mirror_bytes = self
            .mirror_bytes
            .checked_add(text.len())
            .filter(|bytes| *bytes <= MAX_MIRROR_BYTES)
            .ok_or_else(|| invalid("Hermes delta snapshots exceed their byte budget"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(bytes: &[u8]) -> Result<Vec<Event>, AppError> {
        let mut decoder = Decoder::default();
        let mut events = Vec::new();
        for &byte in bytes {
            if let Some(event) = decoder.push(byte)? {
                events.push(event);
            }
        }
        Ok(events)
    }

    #[test]
    fn split_unicode_bom_crlf_comments_and_multiline_data_are_preserved() {
        let source = "\u{feff}: heartbeat\r\nevent: message.delta\r\ndata: {\r\ndata:   \"delta\":\"\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}\"}\r\n\r\n";
        let events = frames(source.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name.as_deref(), Some("message.delta"));
        assert_eq!(
            events[0].data,
            "{\n  \"delta\":\"\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442}\"}"
        );
    }

    #[test]
    fn empty_frame_resets_name_and_eof_never_flushes_unterminated_data() {
        let events =
            frames(b"event: run.completed\n\ndata: {}\n\nevent: run.failed\ndata: {}\n").unwrap();
        assert_eq!(events.len(), 1);
        assert!(events[0].name.is_none());
        assert!(
            frames(b"event: run.completed\ndata: {}")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bare_cr_and_empty_field_values_are_framed_correctly() {
        let events = frames(b"event: old\revent:\rdata\r\rdata:  text\r\r").unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[0].name.is_none());
        assert_eq!(events[0].data, "");
        assert_eq!(events[1].data, " text");
    }

    #[test]
    fn malformed_utf8_is_rejected_without_lossy_replacement() {
        assert!(frames(b"data: \xff\n\n").is_err());
        assert!(frames(b": \xff\n\n").is_err());
    }

    #[test]
    fn crlf_inside_a_frame_consumes_the_same_raw_byte_budget_as_other_bytes() {
        for line in [b":x\n".as_slice(), b":x\r\n".as_slice()] {
            let mut decoder = Decoder::default();
            for &byte in line.iter().cycle().take(MAX_FRAME_BYTES) {
                assert!(decoder.push(byte).unwrap().is_none());
            }
            assert_eq!(decoder.frame_bytes, MAX_FRAME_BYTES);
            assert!(decoder.push(b'x').is_err());
        }
        let mut decoder = Decoder::default();
        for &byte in b":x\r\n\r\n" {
            assert!(decoder.push(byte).unwrap().is_none());
        }
        assert!(!decoder.pending());
        assert_eq!(decoder.frame_bytes, 0);
    }

    #[test]
    fn unterminated_line_and_many_data_or_comment_lines_share_one_frame_budget() {
        for prefix in [b"data: ".as_slice(), b": ".as_slice()] {
            let mut decoder = Decoder::default();
            let mut rejected = false;
            for byte in prefix
                .iter()
                .copied()
                .chain(std::iter::repeat_n(b'x', MAX_FRAME_BYTES))
            {
                if decoder.push(byte).is_err() {
                    rejected = true;
                    break;
                }
            }
            assert!(rejected);
            assert!(decoder.line.len() <= MAX_FRAME_BYTES);
        }
        let mut decoder = Decoder::default();
        let line = b"data: x\n";
        assert!((0..MAX_FRAME_BYTES).any(|_| line.iter().any(|&byte| decoder.push(byte).is_err())));
    }

    #[test]
    fn heartbeat_traffic_cannot_extend_partial_or_absolute_deadlines() {
        let now = Instant::now();
        let mut budget = Budget::new(now);
        budget.receive(1, now).unwrap();
        budget.frame(true, now);
        budget.receive(1, now + Duration::from_secs(20)).unwrap();
        assert_eq!(budget.deadline(), now + FRAME_TIMEOUT);
        assert!(budget.receive(1, now + FRAME_TIMEOUT).is_err());
        let mut budget = Budget::new(now);
        for second in (0..STREAM_LIFETIME.as_secs()).step_by(20) {
            budget
                .receive(1, now + Duration::from_secs(second))
                .unwrap();
            budget.frame(false, now);
        }
        assert!(budget.receive(1, now + STREAM_LIFETIME).is_err());
    }

    #[test]
    fn idle_and_total_byte_event_budgets_are_independent() {
        let now = Instant::now();
        assert!(Budget::new(now).receive(1, now + IDLE_TIMEOUT).is_err());
        let mut empty_chunks = Budget::new(now);
        empty_chunks
            .receive(0, now + Duration::from_secs(20))
            .unwrap();
        empty_chunks
            .receive(0, now + Duration::from_secs(40))
            .unwrap();
        assert_eq!(empty_chunks.deadline(), now + IDLE_TIMEOUT);
        assert!(empty_chunks.receive(0, now + IDLE_TIMEOUT).is_err());
        let mut budget = Budget::new(now);
        budget.receive(MAX_STREAM_BYTES, now).unwrap();
        assert!(budget.receive(1, now).is_err());
        let mut budget = Budget::new(now);
        for _ in 0..MAX_EVENTS {
            budget.event().unwrap();
        }
        assert!(budget.event().is_err());
    }

    #[test]
    fn transcript_limit_rejects_growth_without_changing_retained_text() {
        let mut transcript = Transcript::default();
        transcript
            .append(&"x".repeat(MAX_TRANSCRIPT_BYTES))
            .unwrap();
        assert!(transcript.append("x").is_err());
        assert_eq!(transcript.text.len(), MAX_TRANSCRIPT_BYTES);
    }

    #[test]
    fn snapshot_budget_caps_repeated_full_text_deltas() {
        let mut transcript = Transcript::default();
        let text = "x".repeat(MAX_TRANSCRIPT_BYTES);
        for _ in 0..MAX_MIRROR_BYTES / text.len() {
            transcript.mirror(&text).unwrap();
        }
        assert!(transcript.mirror("x").is_err());
    }
}
