//! Reaction handling utilities

use grammers_client::types::Message;
use grammers_tl_types as tl;

/// Extract reaction count and emoji list from a message's reactions
pub fn extract_reactions(reactions: Option<&tl::enums::MessageReactions>) -> (i32, String) {
    let Some(reactions) = reactions else {
        return (0, String::new());
    };

    let tl::enums::MessageReactions::Reactions(reactions) = reactions;

    let mut total_count = 0;
    let mut emojis = Vec::new();

    for result in &reactions.results {
        let tl::enums::ReactionCount::Count(count) = result;
        total_count += count.count;

        match &count.reaction {
            tl::enums::Reaction::Emoji(emoji) => emojis.push(emoji.emoticon.clone()),
            tl::enums::Reaction::CustomEmoji(custom) => {
                emojis.push(format!("CustomEmoji({})", custom.document_id));
            }
            tl::enums::Reaction::Paid => emojis.push("💎".to_string()),
            tl::enums::Reaction::Empty => {}
        }
    }

    (total_count, emojis.join(""))
}

/// Count total reactions on a message
pub fn count_reactions(msg: &Message) -> i32 {
    // Access raw message reactions
    let raw = &msg.raw;
    match raw {
        grammers_tl_types::enums::Message::Message(m) => {
            let (count, _) = extract_reactions(m.reactions.as_ref());
            count
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grammers_tl_types as tl;

    fn reactions_without_empty() -> tl::enums::MessageReactions {
        let reactions = tl::types::MessageReactions {
            min: false,
            can_see_list: false,
            reactions_as_tags: false,
            results: vec![
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: None,
                    reaction: tl::enums::Reaction::Emoji(tl::types::ReactionEmoji { emoticon: "🔥".into() }),
                    count: 2,
                }),
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: Some(1),
                    reaction: tl::enums::Reaction::CustomEmoji(tl::types::ReactionCustomEmoji { document_id: 42 }),
                    count: 3,
                }),
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: None,
                    reaction: tl::enums::Reaction::Paid,
                    count: 1,
                }),
            ],
            recent_reactions: None,
            top_reactors: None,
        };

        tl::enums::MessageReactions::Reactions(reactions)
    }

    #[test]
    fn returns_zero_for_absent_reactions() {
        let (count, emojis) = extract_reactions(None);
        assert_eq!(count, 0);
        assert!(emojis.is_empty());
    }

    #[test]
    fn aggregates_counts_and_emojis() {
        let (count, emojis) = extract_reactions(Some(&reactions_without_empty()));

        assert_eq!(count, 6);
        assert_eq!(emojis, "🔥CustomEmoji(42)💎");
    }

    #[test]
    fn counts_reactions_without_emoji_symbols() {
        let reactions = tl::types::MessageReactions {
            min: false,
            can_see_list: false,
            reactions_as_tags: false,
            results: vec![
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: None,
                    reaction: tl::enums::Reaction::Emoji(tl::types::ReactionEmoji { emoticon: "🔥".into() }),
                    count: 2,
                }),
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: Some(1),
                    reaction: tl::enums::Reaction::CustomEmoji(tl::types::ReactionCustomEmoji { document_id: 42 }),
                    count: 1,
                }),
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: None,
                    reaction: tl::enums::Reaction::Paid,
                    count: 5,
                }),
                tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                    chosen_order: None,
                    reaction: tl::enums::Reaction::Empty,
                    count: 3,
                }),
            ],
            recent_reactions: None,
            top_reactors: None,
        };

        let (count, emojis) = extract_reactions(Some(&tl::enums::MessageReactions::Reactions(reactions)));

        assert_eq!(count, 11);
        assert_eq!(emojis, "🔥CustomEmoji(42)💎");
    }

    #[test]
    fn counts_only_empty_reactions() {
        let reactions = tl::types::MessageReactions {
            min: false,
            can_see_list: false,
            reactions_as_tags: false,
            results: vec![tl::enums::ReactionCount::Count(tl::types::ReactionCount {
                chosen_order: None,
                reaction: tl::enums::Reaction::Empty,
                count: 4,
            })],
            recent_reactions: None,
            top_reactors: None,
        };

        let (count, emojis) = extract_reactions(Some(&tl::enums::MessageReactions::Reactions(reactions)));

        assert_eq!(count, 4);
        assert!(emojis.is_empty());
    }
}
