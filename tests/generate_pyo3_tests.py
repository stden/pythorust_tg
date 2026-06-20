#!/usr/bin/env python3
"""
Test generator for PyO3 bindings.

Generates Rust test cases from Python test specifications.
Usage: python generate_pyo3_tests.py > tests_output.rs
"""

from dataclasses import dataclass


@dataclass
class TestCase:
    """A single test case specification."""

    name: str
    description: str
    setup: str
    assertion: str
    expected: str


@dataclass
class TestSpec:
    """Specification for a PyO3 function/class to test."""

    rust_name: str
    test_cases: list[TestCase]


def generate_rust_test(spec: TestSpec) -> str:
    """Generate Rust test code from a test specification."""
    tests = []

    for tc in spec.test_cases:
        test_code = f"""
    #[test]
    fn {tc.name}() {{
        // {tc.description}
        {tc.setup}

        {tc.assertion}
    }}"""
        tests.append(test_code)

    return "\n".join(tests)


# Test specifications for Phase 2.2 functions
PHASE_2_2_SPECS = [
    TestSpec(
        rust_name="format_messages_for_llm",
        test_cases=[
            TestCase(
                name="test_format_messages_for_llm_empty",
                description="Empty message list returns empty string",
                setup="let messages: Vec<MessageData> = vec![];",
                assertion='assert_eq!(format_messages_for_llm(&messages, false), "");',
                expected="",
            ),
            TestCase(
                name="test_format_messages_for_llm_preserves_order",
                description="Messages are formatted in order",
                setup="""let messages = vec![
            MessageData {
                sender: "A".to_string(),
                text: "First".to_string(),
                date: "10:00".to_string(),
                reactions: None,
            },
            MessageData {
                sender: "B".to_string(),
                text: "Second".to_string(),
                date: "10:01".to_string(),
                reactions: None,
            },
        ];
        let result = format_messages_for_llm(&messages, false);""",
                assertion='assert!(result.find("First").unwrap() < result.find("Second").unwrap());',
                expected="First appears before Second",
            ),
            TestCase(
                name="test_format_messages_for_llm_reactions_flag_respected",
                description="Reactions only included when flag is true",
                setup="""let messages = vec![
            MessageData {
                sender: "User".to_string(),
                text: "Test".to_string(),
                date: "10:00".to_string(),
                reactions: Some("🔥".to_string()),
            },
        ];""",
                assertion="""let with_reactions = format_messages_for_llm(&messages, true);
        let without_reactions = format_messages_for_llm(&messages, false);
        assert!(with_reactions.contains("🔥"));
        assert!(!without_reactions.contains("🔥"));""",
                expected="Reactions appear only with flag=true",
            ),
        ],
    ),
    TestSpec(
        rust_name="get_chat_metadata",
        test_cases=[
            TestCase(
                name="test_get_chat_metadata_counts_unique_senders",
                description="Same sender appearing multiple times counted once",
                setup="""let messages = vec![
            MessageData { sender: "A".to_string(), text: "1".to_string(), date: "".to_string(), reactions: None },
            MessageData { sender: "A".to_string(), text: "2".to_string(), date: "".to_string(), reactions: None },
            MessageData { sender: "B".to_string(), text: "3".to_string(), date: "".to_string(), reactions: None },
        ];
        let metadata = get_chat_metadata(&messages);""",
                assertion="assert_eq!(metadata.unique_senders, 2);",
                expected="2 unique senders",
            ),
        ],
    ),
]


# Test specifications for Phase 2.1 analytics
PHASE_2_1_SPECS = [
    TestSpec(
        rust_name="SessionMetrics",
        test_cases=[
            TestCase(
                name="test_session_metrics_engaged_threshold",
                description="Engaged requires at least 1 meaningful message",
                setup="""let mut metrics = SessionMetrics::new(1, 1, "bot".to_string(), "".to_string());""",
                assertion="""assert!(!metrics.is_engaged());
        metrics.non_command_in = 1;
        assert!(metrics.is_engaged());""",
                expected="Engaged after 1 message",
            ),
            TestCase(
                name="test_session_metrics_multi_turn_threshold",
                description="Multi-turn requires at least 2 meaningful messages",
                setup="""let mut metrics = SessionMetrics::new(1, 1, "bot".to_string(), "".to_string());
        metrics.non_command_in = 1;""",
                assertion="""assert!(!metrics.is_multi_turn());
        metrics.non_command_in = 2;
        assert!(metrics.is_multi_turn());""",
                expected="Multi-turn after 2 messages",
            ),
        ],
    ),
    TestSpec(
        rust_name="RetentionMetrics",
        test_cases=[
            TestCase(
                name="test_retention_metrics_rate_percentage",
                description="Rates are expressed as percentages (0-100)",
                setup="""let mut metrics = RetentionMetrics::new(100, 100);
        metrics.d1_returned = 50;
        metrics.d7_base = 100;
        metrics.d7_returned = 25;""",
                assertion="""assert!((metrics.d1_rate() - 50.0).abs() < 0.01);
        assert!((metrics.d7_rate() - 25.0).abs() < 0.01);""",
                expected="50% and 25%",
            ),
        ],
    ),
]


def generate_all_tests() -> str:
    """Generate all test code."""
    output = [
        "// Auto-generated tests for PyO3 bindings",
        "// Generated by: tests/generate_pyo3_tests.py",
        "",
        "#[cfg(test)]",
        "mod generated_tests {",
        "    use super::*;",
        "",
    ]

    all_specs = PHASE_2_1_SPECS + PHASE_2_2_SPECS

    for spec in all_specs:
        output.append(f"    // Tests for {spec.rust_name}")
        output.append(generate_rust_test(spec))
        output.append("")

    output.append("}")

    return "\n".join(output)


if __name__ == "__main__":
    print(generate_all_tests())
