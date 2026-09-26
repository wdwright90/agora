//! Parsing client frames and building error responses (SPEC-002-R01, R09).

use agora_protocol::{ClientMessage, ErrorCode, ErrorResponse, RequestId};
use serde::Deserialize;
use serde_json::Value;

/// Parse one text frame into a client message, or the error to send back.
pub fn parse(text: &str) -> Result<ClientMessage, ErrorResponse> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| malformed(None, format!("frame is not valid JSON: {e}")))?;
    let Some(object) = value.as_object() else {
        return Err(malformed(None, "frame is not a JSON object"));
    };
    // Echo the request ID in errors whenever it is readable, even if other fields are not.
    let request_id = object
        .get("request_id")
        .and_then(|id| RequestId::deserialize(id).ok());
    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return Err(malformed(request_id, "missing string field `type`"));
    };
    if !ClientMessage::TYPES.contains(&kind) {
        return Err(with_details(
            error(
                request_id,
                ErrorCode::UnknownMessageType,
                format!("unknown message type {kind:?}"),
            ),
            serde_json::json!({ "type": kind }),
        ));
    }
    ClientMessage::deserialize(&value).map_err(|e| malformed(request_id, e.to_string()))
}

pub fn error(
    request_id: Option<RequestId>,
    code: ErrorCode,
    message: impl Into<String>,
) -> ErrorResponse {
    ErrorResponse {
        request_id,
        code,
        message: message.into(),
        details: None,
    }
}

pub fn malformed(request_id: Option<RequestId>, message: impl Into<String>) -> ErrorResponse {
    error(request_id, ErrorCode::MalformedMessage, message)
}

/// Attach `details`, which must be a JSON object.
pub fn with_details(mut error: ErrorResponse, details: Value) -> ErrorResponse {
    let Value::Object(details) = details else {
        panic!("error details must be a JSON object");
    };
    error.details = Some(details);
    error
}
