//! Zapier Webhook Security
//!
//! This module provides security functionalities for handling Zapier webhooks,
//! primarily focusing on signature verification to ensure the authenticity of
//! incoming requests.

use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;

/// Error types for Zapier security operations.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ZapierSecurityError {
    #[error("Missing Zapier signature header (X-Zapier-Signature)")]
    MissingSignature,

    #[error("Invalid signature format")]
    InvalidSignatureFormat,

    #[error("Signature mismatch")]
    SignatureMismatch,
}

/// Verifies the signature of an incoming Zapier webhook.
///
/// Zapier can send a `X-Zapier-Signature` header which is an HMAC-SHA256 hash of
/// the raw request body, using a secret key. This function computes the same
/// hash and compares it to the provided signature.
///
/// # Arguments
///
/// * `headers` - The `HeaderMap` from the incoming Axum request.
/// * `raw_body` - The raw, unmodified request body as bytes.
/// * `secret` - The Zapier webhook secret key from the application's configuration.
///
/// # Returns
///
/// A `Result<(), ZapierSecurityError>` which is `Ok(())` if the signature is
/// valid, or an `Err` with a `ZapierSecurityError` if it is not.
pub fn verify_zapier_signature(
    headers: &HeaderMap,
    raw_body: &[u8],
    secret: &str,
) -> Result<(), ZapierSecurityError> {
    let signature = headers
        .get("X-Zapier-Signature")
        .and_then(|h| h.to_str().ok())
        .ok_or(ZapierSecurityError::MissingSignature)?;

    // The signature from Zapier is the hex-encoded HMAC hash.
    let provided_signature =
        hex::decode(signature).map_err(|_| ZapierSecurityError::InvalidSignatureFormat)?;

    // Create a new HMAC-SHA256 instance with the secret key.
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take a key of any size");

    // Update the HMAC with the raw request body.
    mac.update(raw_body);

    // Finalize the HMAC and get the result.
    let computed_signature = mac.finalize().into_bytes();

    // Compare the computed signature with the provided signature in a constant-time manner.
    if hmac::verify(&computed_signature, &provided_signature).is_ok() {
        Ok(())
    } else {
        Err(ZapierSecurityError::SignatureMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::HeaderValue;

    fn create_headers(signature: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Zapier-Signature",
            HeaderValue::from_str(signature).unwrap(),
        );
        headers
    }

    #[test]
    fn test_verify_zapier_signature_valid() {
        let secret = "supersecretkey";
        let body = r#"{"hello": "world"}"#;
        // Pre-computed HMAC-SHA256 for the body and secret:
        let signature = "2d2c70081e35c24943f2a5cb823b567b433c2d4f3b72c91620a23c215c0e4c5b";
        let headers = create_headers(signature);

        let result = verify_zapier_signature(&headers, body.as_bytes(), secret);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_verify_zapier_signature_mismatch() {
        let secret = "supersecretkey";
        let body = r#"{"hello": "world"}"#;
        let incorrect_signature =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let headers = create_headers(incorrect_signature);

        let result = verify_zapier_signature(&headers, body.as_bytes(), secret);
        assert_eq!(result, Err(ZapierSecurityError::SignatureMismatch));
    }

    #[test]
    fn test_verify_zapier_signature_invalid_format() {
        let secret = "supersecretkey";
        let body = r#"{"hello": "world"}"#;
        let signature = "this-is-not-a-valid-hex-string";
        let headers = create_headers(signature);

        let result = verify_zapier_signature(&headers, body.as_bytes(), secret);
        assert_eq!(result, Err(ZapierSecurityError::InvalidSignatureFormat));
    }

    #[test]
    fn test_verify_zapier_signature_missing_header() {
        let secret = "supersecretkey";
        let body = r#"{"hello": "world"}"#;
        let headers = HeaderMap::new(); // No signature header

        let result = verify_zapier_signature(&headers, body.as_bytes(), secret);
        assert_eq!(result, Err(ZapierSecurityError::MissingSignature));
    }

    #[test]
    fn test_verify_zapier_signature_empty_body() {
        let secret = "supersecretkey";
        let body = "";
        // Pre-computed HMAC-SHA256 for an empty body and secret:
        let signature = "b6401217e34d30653835de3b817c17b01f3576f3f72e36c12560b8b5493c9e69";
        let headers = create_headers(signature);

        let result = verify_zapier_signature(&headers, body.as_bytes(), secret);
        assert_eq!(result, Ok(()));
    }
}
