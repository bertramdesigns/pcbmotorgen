//! Mock transport for offline testing of [`super::KiCadClient`].

use super::KicadTransport;
use crate::errors::KiCadError;

/// Mock transport for offline testing.
///
/// Records every request sent via [`send_and_recv`](KicadTransport::send_and_recv)
/// into [`sent_requests`] and returns `response_to_return` (a clone) on each
/// call. This lets tests:
/// - Inspect the exact bytes the client packed into the `ApiRequest` envelope.
/// - Control the canned `ApiResponse` bytes the client will decode.
pub struct MockTransport {
    /// All requests sent by the client, in order.
    pub sent_requests: Vec<Vec<u8>>,
    /// The raw response bytes returned on each `send_and_recv` call.
    pub response_to_return: Vec<u8>,
}

impl MockTransport {
    /// Creates a mock transport that returns the given response bytes.
    pub fn new(response_to_return: Vec<u8>) -> Self {
        Self {
            sent_requests: Vec::new(),
            response_to_return,
        }
    }
}

impl KicadTransport for MockTransport {
    fn send_and_recv(&mut self, request_bytes: &[u8]) -> Result<Vec<u8>, KiCadError> {
        self.sent_requests.push(request_bytes.to_vec());
        Ok(self.response_to_return.clone())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_transport_records_request_and_returns_response() {
        let mut mock = MockTransport::new(vec![1, 2, 3]);
        let out = mock.send_and_recv(&[9, 8]).expect("send_and_recv");
        assert_eq!(out, vec![1, 2, 3]);
        assert_eq!(mock.sent_requests.len(), 1);
        assert_eq!(mock.sent_requests[0], vec![9, 8]);
    }

    #[test]
    fn test_mock_transport_default_connect_is_noop() {
        let mut mock = MockTransport::new(Vec::new());
        mock.connect().expect("connect must be a no-op");
    }
}