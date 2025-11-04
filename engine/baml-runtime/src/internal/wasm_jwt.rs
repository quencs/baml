/// Use the browser's Subtle.Crypto API to sign JWT's.
/// This module is meant to be imported conditionally during wasm builds, to
/// replace our JWT creation code for native targets.
///
/// The main motivation is to use browser APIs via web-sys, and to avoid
/// compiling the native-targeting crypto library `ring`, which can be
/// problematic for some toolchains to cross-compile to WASM.
///
/// At the time of writing, the Vertex provider is the only code in the
/// runtime that produces JWT's.
use aws_smithy_types::event_stream::Header;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use js_sys::{Array, Object, Uint8Array};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, CryptoKey, SubtleCrypto};

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Failed to import private key using WebCrypto API. This typically indicates:\n  1. The private key format is invalid (expected PKCS#8, got something else)\n  2. The private key data is corrupted or incomplete\n  3. The key contains embedded newline characters that need escaping in JSON\n\nTroubleshooting:\n  - Verify your GCP service account JSON has a valid 'private_key' field\n  - Ensure the private key starts with '-----BEGIN PRIVATE KEY-----' (not 'RSA PRIVATE KEY')\n  - Check that newlines in the JSON are properly escaped as \\n\n  Original error: {0:?}")]
    JsError(JsValue),
    #[error("Failed to decode private key from base64. The private key in your service account credentials appears to be malformed. Error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("Failed to serialize JWT claims as JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("WebCrypto API is not available (missing window object). This code must run in a browser or WASM environment.")]
    NoWindow,
    #[error("WebCrypto API is not available (missing crypto API). Your browser may not support the required cryptographic operations.")]
    NoCrypto,
    #[error("Private key is too short ({0} bytes). Expected at least 100 bytes for a valid RSA private key. This likely indicates the key is invalid, corrupted, or just test data.")]
    KeyTooShort(usize),
}

impl From<JsValue> for JwtError {
    fn from(err: JsValue) -> Self {
        JwtError::JsError(err)
    }
}

pub async fn encode_jwt(
    claims: &serde_json::Value,
    private_key_pem: &str,
) -> Result<String, JwtError> {
    // Extract the crypto.subtle API
    let window = window().ok_or(JwtError::NoWindow)?;
    let crypto = window.crypto()?;
    let subtle = crypto.subtle();

    // Create the JWT header and claims segments
    let header_json = json!({
        "alg": "RS256",
        "typ": "JWT"
    });
    let header_segment = URL_SAFE_NO_PAD.encode(header_json.to_string());
    let claims_segment = URL_SAFE_NO_PAD.encode(serde_json::to_string(claims)?);

    // Combine header and claims
    let signing_input = format!("{header_segment}.{claims_segment}");

    // Convert PEM to importable key format
    // Remove PEM headers/footers and whitespace (newlines, carriage returns, tabs)
    // Note: Do NOT remove spaces as they may be part of valid base64 content
    let pem = private_key_pem
        .trim()
        .replace("-----BEGIN PRIVATE KEY-----", "")
        .replace("-----END PRIVATE KEY-----", "")
        .replace("-----BEGIN RSA PRIVATE KEY-----", "")
        .replace("-----END RSA PRIVATE KEY-----", "")
        // Handle both literal \n strings (from JSON) and actual newline characters
        .replace("\\n", "")
        .replace(['\n', '\r', '\t'], "");

    let key_data = STANDARD.decode(&pem)?;

    // Validate key length before attempting to import
    if key_data.len() < 100 {
        return Err(JwtError::KeyTooShort(key_data.len()));
    }

    // Import the key
    let import_params = Object::new();
    js_sys::Reflect::set(&import_params, &"name".into(), &"RSASSA-PKCS1-v1_5".into())?;
    js_sys::Reflect::set(&import_params, &"hash".into(), &"SHA-256".into())?;

    let key_usage = Array::new();
    key_usage.push(&"sign".into());

    let key: CryptoKey = JsFuture::from(subtle.import_key_with_object(
        "pkcs8",
        &Uint8Array::from(&key_data[..]),
        &import_params,
        false,
        &key_usage,
    )?)
    .await?
    .into();

    // Sign the input
    let sign_params = Object::new();
    js_sys::Reflect::set(&sign_params, &"name".into(), &"RSASSA-PKCS1-v1_5".into())?;

    let signature = JsFuture::from(subtle.sign_with_object_and_u8_array(
        &sign_params,
        &key,
        signing_input.as_bytes(),
    )?)
    .await?;

    let signature_array = Uint8Array::new(&signature);
    let mut signature_vec = vec![0; signature_array.length() as usize];
    signature_array.copy_to(&mut signature_vec);

    // Create final JWT
    let signature_segment = URL_SAFE_NO_PAD.encode(&signature_vec);
    Ok(format!(
        "{header_segment}.{claims_segment}.{signature_segment}"
    ))
}
