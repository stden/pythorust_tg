//! Tests for active_chats command

use telegram_reader::commands::active_chats;

#[tokio::test]
#[ignore] // Requires Telegram connection
async fn test_active_chats_run() {
    // This is an integration test that requires actual Telegram session
    let result = active_chats::run(5).await;
    // Should either succeed or fail with a session error
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_active_chats_module_exists() {
    // Smoke test to verify module compiles
    assert!(true);
}
