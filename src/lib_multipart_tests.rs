use super::*;

#[test]
fn credential_echoes_are_removed_from_nested_error_payloads() {
    let bearer = "echoed bearer";
    let api_key = "echoed api credential";
    let mut value = serde_json::json!({
        "error": format!("rejected {bearer}"),
        bearer: [api_key, {"detail": bearer}, 3, false, null],
    });
    redact_credential_echoes(&mut value, &[Some(bearer), Some(api_key)]);
    let output = value.to_string();
    assert!(!output.contains(bearer));
    assert!(!output.contains(api_key));
    assert!(output.contains("[REDACTED]"));
    assert_eq!(value["[REDACTED]"][2], 3);
}

#[test]
fn absent_or_blank_credentials_leave_diagnostics_unchanged() {
    let mut value = serde_json::json!({"error": ["message", 4, true, null]});
    let original = value.clone();
    redact_credential_echoes(&mut value, &[None, Some("  ")]);
    assert_eq!(value, original);
}
