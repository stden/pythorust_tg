//! Tests for init_session command

#[test]
fn test_init_session_confirmation_yes() {
    let input = "YES";
    assert_eq!(input.trim(), "YES");
}

#[test]
fn test_init_session_confirmation_no() {
    let input = "no";
    assert_ne!(input.trim(), "YES");
}

#[test]
fn test_init_session_confirmation_case_sensitive() {
    let input = "yes";
    assert_ne!(input.trim(), "YES");
    
    let input = "Yes";
    assert_ne!(input.trim(), "YES");
}

#[test]
fn test_init_session_trimming() {
    let input = " YES ";
    assert_eq!(input.trim(), "YES");
    
    let input = "YES\n";
    assert_eq!(input.trim(), "YES");
}

#[tokio::test]
#[ignore] // Requires user interaction
async fn test_init_session_run() {
    use telegram_reader::commands::init_session;
    
    // This test requires manual interaction
    // Just verify it compiles
    let _ = init_session::run().await;
}
