//! Bounded multipart responses preserve transport safety at the SDK boundary.

use tinyhumans_sdk::TinyHumansClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LIMIT: usize = 65_536;

#[tokio::test]
async fn oversized_multipart_response_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"text": "x".repeat(LIMIT)})),
        )
        .mount(&server)
        .await;
    let client = TinyHumansClient::new(server.uri());
    let result = client
        .raw()
        .post_multipart_with_response_limit(
            "/openai/v1/audio/transcriptions",
            reqwest::multipart::Form::new(),
            LIMIT,
        )
        .await;
    assert!(
        result.is_err(),
        "response exceeded the transcription byte budget"
    );
}

#[tokio::test]
async fn rejected_multipart_response_does_not_echo_credentials() {
    let server = MockServer::start().await;
    let secret = "sensitive echoed credential";
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(serde_json::json!({"error": secret, "nested": [secret]})),
        )
        .mount(&server)
        .await;
    let client = TinyHumansClient::new(server.uri()).with_token(Some(secret.into()));
    let result = client
        .raw()
        .post_multipart_with_response_limit(
            "/openai/v1/audio/transcriptions",
            reqwest::multipart::Form::new(),
            LIMIT,
        )
        .await
        .unwrap_err();
    assert!(
        !result.to_string().contains(secret),
        "error leaked the supplied credential"
    );
}

#[tokio::test]
async fn response_at_the_budget_is_accepted_without_redacting_success() {
    let server = MockServer::start().await;
    let secret = "successful transcription";
    let value = serde_json::json!({"text": secret});
    let budget = value.to_string().len();
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(value.clone()))
        .mount(&server)
        .await;
    let client = TinyHumansClient::new(server.uri()).with_token(Some(secret.into()));
    let result = client
        .raw()
        .post_multipart_with_response_limit(
            "/openai/v1/audio/transcriptions",
            reqwest::multipart::Form::new(),
            budget,
        )
        .await
        .unwrap();
    assert_eq!(result, value);
}

#[tokio::test]
async fn unsuccessful_envelope_redacts_api_key_echoes_on_success_status() {
    let server = MockServer::start().await;
    let secret = "echoed api credential";
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            serde_json::json!({"success": false, "error": secret, "details": [secret]}),
        ))
        .mount(&server)
        .await;
    let client = TinyHumansClient::new(server.uri()).with_api_key(Some(secret.into()));
    let result = client
        .raw()
        .post_multipart_with_response_limit(
            "/openai/v1/audio/transcriptions",
            reqwest::multipart::Form::new(),
            LIMIT,
        )
        .await
        .unwrap_err();
    assert!(!result.to_string().contains(secret));
    assert!(result.to_string().contains("[REDACTED]"));
}
