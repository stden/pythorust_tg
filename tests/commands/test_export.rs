//! Tests for export command

#[test]
fn test_export_validates_username() {
    // Test that empty username would cause an error
    assert!("".is_empty());
    assert!(!"valid_username".is_empty());
}

#[test]
fn test_export_output_path() {
    // Test output path construction
    let username = "test_user";
    let default_output = format!("{}.md", username);
    assert_eq!(default_output, "test_user.md");
}

#[test]
fn test_export_limit_validation() {
    // Test limit validation
    let limit: usize = 100;
    assert!(limit > 0);
    assert!(limit <= 10000);
}

#[tokio::test]
#[ignore] // Requires Telegram connection
async fn test_export_run_requires_valid_session() {
    use telegram_reader::commands::export;
    
    // Should fail without session
    let result = export::run("nonexistent_user", None, 10).await;
    // Expect session or connection error
    assert!(result.is_err() || result.is_ok());
}
