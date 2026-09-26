//! Per-session request-ID tracking and result retention (SPEC-002-R07).

use std::collections::VecDeque;

use agora_protocol::{ClientMessage, ErrorCode, ErrorResponse, RequestId, ServerMessage};
use serde_json::json;

use crate::wire;

/// How many of a session's most recent admitted requests keep their results for retries.
pub const RETAINED_RESULTS: usize = 5;

/// A session's admitted request numbers and its most recent results.
pub struct RequestLog {
    highest_admitted: RequestId,
    /// The most recent admitted requests, oldest first. IDs increase along the queue.
    retained: VecDeque<Entry>,
}

struct Entry {
    id: RequestId,
    request: ClientMessage,
    response: ServerMessage,
}

/// How a session should handle an incoming request.
pub enum Admission<'a> {
    /// A new request number: execute the request, then [`RequestLog::record`] its result.
    New,
    /// A retry of a retained request: send its original response again.
    Replay(&'a ServerMessage),
    /// The request number cannot be used. The request is not executed or recorded.
    Rejected(ErrorResponse),
}

impl RequestLog {
    /// Start a log with the request that established the session.
    pub fn new(id: RequestId, request: ClientMessage, response: ServerMessage) -> Self {
        let mut log = Self {
            highest_admitted: id,
            retained: VecDeque::with_capacity(RETAINED_RESULTS),
        };
        log.record(id, request, response);
        log
    }

    pub fn admit(&self, id: RequestId, request: &ClientMessage) -> Admission<'_> {
        if id > self.highest_admitted {
            return Admission::New;
        }
        if let Some(entry) = self.retained.iter().find(|entry| entry.id == id) {
            return if entry.request == *request {
                Admission::Replay(&entry.response)
            } else {
                Admission::Rejected(wire::error(
                    Some(id),
                    ErrorCode::RequestIdConflict,
                    "request ID was already used for a different request",
                ))
            };
        }
        // Numbers older than every retained result had their results discarded, or were
        // skipped; the log does not remember which. Unretained numbers within the retained
        // range were skipped.
        let oldest_retained = self.retained.front().map(|entry| entry.id);
        if oldest_retained.is_some_and(|oldest| id < oldest) {
            Admission::Rejected(wire::error(
                Some(id),
                ErrorCode::ResultExpired,
                "the result of this request is no longer retained",
            ))
        } else {
            Admission::Rejected(wire::with_details(
                wire::error(
                    Some(id),
                    ErrorCode::StaleRequestId,
                    "request ID is not above the highest admitted request ID",
                ),
                json!({ "highest_admitted": self.highest_admitted }),
            ))
        }
    }

    /// Record the result of a newly admitted request.
    pub fn record(&mut self, id: RequestId, request: ClientMessage, response: ServerMessage) {
        self.highest_admitted = self.highest_admitted.max(id);
        if self.retained.len() == RETAINED_RESULTS {
            self.retained.pop_front();
        }
        self.retained.push_back(Entry {
            id,
            request,
            response,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> RequestId {
        RequestId::new(n).unwrap()
    }

    fn start(n: u64) -> ClientMessage {
        ClientMessage::Start { request_id: id(n) }
    }

    fn started(n: u64) -> ServerMessage {
        ServerMessage::Started { request_id: id(n) }
    }

    fn code(admission: Admission<'_>) -> Option<ErrorCode> {
        match admission {
            Admission::Rejected(error) => Some(error.code),
            _ => None,
        }
    }

    #[test]
    fn only_the_most_recent_results_are_retained() {
        let mut log = RequestLog::new(id(1), start(1), started(1));
        for n in 2..=6 {
            log.record(id(n), start(n), started(n));
        }
        assert_eq!(
            code(log.admit(id(1), &start(1))),
            Some(ErrorCode::ResultExpired)
        );
        for n in 2..=6 {
            assert!(
                matches!(log.admit(id(n), &start(n)), Admission::Replay(_)),
                "{n}"
            );
        }
        assert!(matches!(log.admit(id(7), &start(7)), Admission::New));
    }

    #[test]
    fn skipped_numbers_inside_the_retained_range_are_stale() {
        let mut log = RequestLog::new(id(1), start(1), started(1));
        log.record(id(3), start(3), started(3));
        assert_eq!(
            code(log.admit(id(2), &start(2))),
            Some(ErrorCode::StaleRequestId)
        );
    }
}
