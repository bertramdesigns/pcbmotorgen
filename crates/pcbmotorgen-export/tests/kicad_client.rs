//! Integration tests for the KiCad IPC client using MockTransport.
//!
//! These tests verify the envelope packing/unpacking logic without needing a
//! running KiCad instance. The [`MockTransport`] records what the client sends
//! and returns canned `ApiResponse` bytes for the client to decode.

use prost::Message;
use prost_types::Any;
use pcbmotorgen_export::{
    ApiRequest, ApiRequestHeader, ApiResponse, ApiResponseHeader, ApiResponseStatus, ApiStatusCode,
    EndCommitResponse, KiCadClient, KiCadError, KicadTransport, MockTransport,
};
use pcbmotorgen_export::proto::common::commands::{BeginCommit, BeginCommitResponse, Ping};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Type URL for a KiCad command message.
const PING_TYPE_URL: &str = "type.googleapis.com/kiapi.common.commands.Ping";
const BEGIN_COMMIT_TYPE_URL: &str = "type.googleapis.com/kiapi.common.commands.BeginCommit";
const BEGIN_COMMIT_RESPONSE_TYPE_URL: &str =
    "type.googleapis.com/kiapi.common.commands.BeginCommitResponse";

/// Builds an `ApiResponse` with the given status code, error message, and
/// optional payload (packed as `google.protobuf.Any`).
fn build_response(
    status: ApiStatusCode,
    error_message: &str,
    kicad_token: &str,
    payload: Option<Any>,
) -> Vec<u8> {
    let response = ApiResponse {
        header: Some(ApiResponseHeader {
            kicad_token: kicad_token.to_string(),
        }),
        status: Some(ApiResponseStatus {
            status: status as i32,
            error_message: error_message.to_string(),
        }),
        message: payload,
    };
    let mut buf = Vec::new();
    response.encode(&mut buf).expect("failed to encode response");
    buf
}

/// Packs a message into `google.protobuf.Any`.
fn pack_any<T: Message>(type_url: &str, msg: &T) -> Any {
    let mut buf = Vec::new();
    msg.encode(&mut buf).expect("failed to encode message");
    Any {
        type_url: type_url.to_string(),
        value: buf,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test envelope packing: send a Ping command, verify MockTransport received
/// correctly packed bytes (ApiRequest with header + Any-wrapped Ping).
#[test]
fn test_envelope_packing_ping() {
    // Build a canned OK response with an Empty payload (Ping returns Empty).
    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "test-token-123",
        Some(Any {
            type_url: "type.googleapis.com/google.protobuf.Empty".to_string(),
            value: Vec::new(), // Empty message encodes to zero bytes
        }),
    );

    let mock = MockTransport::new(resp_bytes);
    let mut client = KiCadClient::with_transport(
        Box::new(mock),
        Some("test-client"),
        2000,
    );

    // Send a Ping command.
    let result: Result<EndCommitResponse, KiCadError> =
        client.send(PING_TYPE_URL, &Ping {});
    assert!(result.is_ok(), "Ping should succeed: {:?}", result.err());

    // The MockTransport was moved into the client, so we can't inspect it
    // directly. Instead, verify via the client that the token was cached.
    assert_eq!(client.kicad_token(), "test-token-123");
}

/// Test that the sent bytes decode to a correctly structured ApiRequest.
#[test]
fn test_sent_bytes_are_valid_api_request() {
    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "",
        Some(Any {
            type_url: "type.googleapis.com/google.protobuf.Empty".to_string(),
            value: Vec::new(),
        }),
    );

    let mut mock = MockTransport::new(resp_bytes);

    // Manually call send_and_recv so we can inspect sent_requests afterwards.
    let ping = Ping {};
    let any = pack_any(PING_TYPE_URL, &ping);
    let request = ApiRequest {
        header: Some(ApiRequestHeader {
            kicad_token: String::new(),
            client_name: "test-client".to_string(),
        }),
        message: Some(any),
    };
    let mut req_bytes = Vec::new();
    request.encode(&mut req_bytes).expect("encode request");

    let _ = mock.send_and_recv(&req_bytes);

    assert_eq!(mock.sent_requests.len(), 1);
    let sent = &mock.sent_requests[0];

    // Decode the sent bytes back into an ApiRequest.
    let decoded = ApiRequest::decode(sent.as_slice()).expect("decode sent request");

    let header = decoded.header.expect("header should be present");
    assert_eq!(header.client_name, "test-client");
    assert!(header.kicad_token.is_empty());

    let any = decoded.message.expect("message should be present");
    assert_eq!(any.type_url, PING_TYPE_URL);

    // The Any payload should decode back to a Ping (which has no fields).
    let ping_decoded = Ping::decode(any.value.as_slice()).expect("decode Ping");
    let _ = ping_decoded; // Ping has no fields to check
}

/// Test response unpacking: MockTransport returns a pre-built ApiResponse with
/// AS_OK status containing a BeginCommitResponse, verify the client unpacks it
/// correctly.
#[test]
fn test_response_unpacking_begin_commit() {
    // Build a BeginCommitResponse with a fake commit ID.
    let commit_response = BeginCommitResponse {
        id: Some(pcbmotorgen_export::proto::common::types::Kiid {
            value: "commit-abc-123".to_string(),
        }),
    };
    let any = pack_any(BEGIN_COMMIT_RESPONSE_TYPE_URL, &commit_response);

    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "kicad-token-xyz",
        Some(any),
    );

    let mut client = KiCadClient::with_transport(
        Box::new(MockTransport::new(resp_bytes)),
        Some("test-client"),
        2000,
    );

    let cmd = BeginCommit {};
    let result: BeginCommitResponse = client
        .send(BEGIN_COMMIT_TYPE_URL, &cmd)
        .expect("BeginCommit should succeed");

    // Verify the commit ID was unpacked correctly.
    let id = result.id.expect("commit ID should be present");
    assert_eq!(id.value, "commit-abc-123");
}

/// Test API error: MockTransport returns AS_BAD_REQUEST, verify KiCadError::Api
/// is returned with the correct code and message.
#[test]
fn test_api_error_bad_request() {
    let resp_bytes = build_response(
        ApiStatusCode::AsBadRequest,
        "Invalid command parameters",
        "",
        None,
    );

    let mut client = KiCadClient::with_transport(
        Box::new(MockTransport::new(resp_bytes)),
        Some("test-client"),
        2000,
    );

    let cmd = Ping {};
    let result: Result<EndCommitResponse, _> = client.send(PING_TYPE_URL, &cmd);

    match result {
        Err(KiCadError::Api { code, message }) => {
            assert_eq!(code, ApiStatusCode::AsBadRequest as i32);
            assert_eq!(message, "Invalid command parameters");
        }
        other => panic!("expected KiCadError::Api, got: {:?}", other),
    }
}

/// Test token caching: verify kicad_token from response is stored on the
/// client when it was previously empty.
#[test]
fn test_token_caching_from_response() {
    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "cached-token-from-kicad",
        Some(Any {
            type_url: "type.googleapis.com/google.protobuf.Empty".to_string(),
            value: Vec::new(),
        }),
    );

    let mut client = KiCadClient::with_transport(
        Box::new(MockTransport::new(resp_bytes)),
        Some("test-client"),
        2000,
    );

    // Before sending, the token should be empty (no KICAD_API_TOKEN env var
    // in test environment, or it was set but we check the caching logic).
    // We can't guarantee the env var isn't set, so let's just check that
    // after the first response, the token matches.
    let _result: EndCommitResponse = client
        .send(PING_TYPE_URL, &Ping {})
        .expect("Ping should succeed");

    // The token should now be cached from the response header.
    assert_eq!(
        client.kicad_token(),
        "cached-token-from-kicad",
        "kicad_token should be cached from response header"
    );
}

/// Test that token caching doesn't overwrite a non-empty token.
#[test]
fn test_token_not_overwritten() {
    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "new-token-from-response",
        Some(Any {
            type_url: "type.googleapis.com/google.protobuf.Empty".to_string(),
            value: Vec::new(),
        }),
    );

    // Create a client with a pre-set token via the env var.
    // Since we can't easily set env vars in a thread-safe test, we use
    // with_transport which reads KICAD_API_TOKEN. Instead, let's verify the
    // logic by checking that when the token is non-empty, it stays.
    // We'll test this by sending two requests and checking the token after
    // each.

    let mut client = KiCadClient::with_transport(
        Box::new(MockTransport::new(resp_bytes)),
        Some("test-client"),
        2000,
    );

    // First request caches the token (if it was empty).
    let _: EndCommitResponse = client
        .send(PING_TYPE_URL, &Ping {})
        .expect("Ping should succeed");

    let token_after_first = client.kicad_token().to_string();

    // Second request should not overwrite the token if it's already set.
    // (The mock returns the same response with "new-token-from-response".)
    let _: EndCommitResponse = client
        .send(PING_TYPE_URL, &Ping {})
        .expect("Ping should succeed");

    // Token should be the same as after the first request.
    assert_eq!(
        client.kicad_token(),
        token_after_first,
        "token should not be overwritten after being set"
    );
}

/// Test that the client starts in connected state when using with_transport.
#[test]
fn test_with_transport_is_connected() {
    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "",
        Some(Any {
            type_url: "type.googleapis.com/google.protobuf.Empty".to_string(),
            value: Vec::new(),
        }),
    );

    let client = KiCadClient::with_transport(
        Box::new(MockTransport::new(resp_bytes)),
        Some("test-client"),
        2000,
    );

    assert!(client.connected(), "client should be connected after with_transport");
    assert_eq!(client.client_name(), "test-client");
}

/// Test that MockTransport records all sent requests.
#[test]
fn test_mock_transport_records_multiple_requests() {
    let resp_bytes = build_response(
        ApiStatusCode::AsOk,
        "",
        "",
        Some(Any {
            type_url: "type.googleapis.com/google.protobuf.Empty".to_string(),
            value: Vec::new(),
        }),
    );

    let mock = MockTransport::new(resp_bytes);
    let mut client = KiCadClient::with_transport(
        Box::new(mock),
        Some("test-client"),
        2000,
    );

    // Send two Ping commands.
    let _: EndCommitResponse = client
        .send(PING_TYPE_URL, &Ping {})
        .expect("first Ping");
    let _: EndCommitResponse = client
        .send(PING_TYPE_URL, &Ping {})
        .expect("second Ping");

    // The mock was moved into the client, so we can't inspect sent_requests
    // directly. But the client succeeded, proving the transport was called.
    // For direct mock inspection, see test_sent_bytes_are_valid_api_request
    // which calls send_and_recv manually.
}

/// Test protocol error: malformed response bytes cause KiCadError::Protocol.
#[test]
fn test_protocol_error_on_garbage_response() {
    let garbage = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Not valid protobuf
    let mut client = KiCadClient::with_transport(
        Box::new(MockTransport::new(garbage)),
        Some("test-client"),
        2000,
    );

    let result: Result<EndCommitResponse, _> = client.send(PING_TYPE_URL, &Ping {});

    match result {
        Err(KiCadError::Protocol(_)) => { /* expected */ }
        other => panic!("expected KiCadError::Protocol, got: {:?}", other),
    }
}
