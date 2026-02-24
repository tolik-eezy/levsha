//! Context window management.
//!
//! Composes the API request context: system prompt + skill prompts + rolling
//! conversation history. Token counting at 4 chars/token (conservative).
//! Oldest messages truncated first. System prompt and skill prompts never truncated.

use crate::history::StoredMessage;
use crate::types::MessageRole;
use tracing::debug;

fn is_false(v: &bool) -> bool {
    !v
}

/// A message formatted for the Anthropic API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

/// Content block within an API message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "is_false")]
        is_error: bool,
    },
}

/// Builds the context window for each API request.
pub struct ContextBuilder {
    pub max_tokens: usize,
    pub response_reserve: usize,
    pub chars_per_token: usize,
}

impl ContextBuilder {
    pub fn new(max_tokens: usize, response_reserve: usize, chars_per_token: usize) -> Self {
        Self {
            max_tokens,
            response_reserve,
            chars_per_token,
        }
    }

    /// Estimate the token count for a string.
    fn estimate_tokens(&self, text: &str) -> usize {
        (text.len() + self.chars_per_token - 1) / self.chars_per_token
    }

    /// Estimate tokens for a stored message.
    fn estimate_message_tokens(&self, msg: &StoredMessage) -> usize {
        let content_tokens = self.estimate_tokens(&msg.content);
        let tool_tokens = msg
            .tool_name
            .as_ref()
            .map(|n| self.estimate_tokens(n))
            .unwrap_or(0)
            + msg
                .tool_args
                .as_ref()
                .map(|a| self.estimate_tokens(a))
                .unwrap_or(0);
        // Per-message overhead for role + formatting.
        content_tokens + tool_tokens + 4
    }

    /// Build the full context for an API request.
    ///
    /// Returns (combined_system_prompt, messages_for_api).
    /// The system prompt is the base prompt + all skill prompts concatenated.
    /// Messages are from most recent history, truncated from oldest if needed.
    pub fn build(
        &self,
        system_prompt: &str,
        skill_prompts: &[String],
        messages: &[StoredMessage],
    ) -> (String, Vec<ApiMessage>) {
        // Combine system prompt with skill prompts.
        let mut combined_system = system_prompt.to_string();
        for prompt in skill_prompts {
            combined_system.push_str("\n\n");
            combined_system.push_str(prompt);
        }

        let system_tokens = self.estimate_tokens(&combined_system);
        let budget = self.max_tokens - self.response_reserve - system_tokens;

        debug!(
            "Context budget: max={} reserve={} system={} available={}",
            self.max_tokens, self.response_reserve, system_tokens, budget
        );

        // Build messages from most recent, adding until budget is exhausted.
        let mut selected: Vec<&StoredMessage> = Vec::new();
        let mut used_tokens = 0;

        for msg in messages.iter().rev() {
            let msg_tokens = self.estimate_message_tokens(msg);
            if used_tokens + msg_tokens > budget && !selected.is_empty() {
                break;
            }
            used_tokens += msg_tokens;
            selected.push(msg);
        }

        // Reverse to chronological order.
        selected.reverse();

        let truncated = selected.len() < messages.len();

        debug!(
            "Selected {} of {} messages ({} tokens), truncated={}",
            selected.len(),
            messages.len(),
            used_tokens,
            truncated
        );

        // Sanitize: remove orphaned tool_calls (tool_call without a following tool_result).
        let selected = remove_orphaned_tool_calls(selected);

        // Convert to API messages.
        let mut api_messages: Vec<ApiMessage> = Vec::new();

        // If truncated, add a notification.
        if truncated {
            api_messages.push(ApiMessage {
                role: "user".to_string(),
                content: vec![ContentBlock::Text {
                    text: "[Note: older messages have been removed to fit the context window]"
                        .to_string(),
                }],
            });
            api_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: vec![ContentBlock::Text {
                    text: "Understood.".to_string(),
                }],
            });
        }

        for msg in selected {
            let api_msg = stored_to_api_message(msg);
            // Merge consecutive messages with the same role, or add as new.
            if let Some(last) = api_messages.last_mut() {
                if last.role == api_msg.role {
                    last.content.extend(api_msg.content);
                    continue;
                }
            }
            api_messages.push(api_msg);
        }

        // Ensure the first message is a user message (API requirement).
        if api_messages.first().map(|m| m.role.as_str()) == Some("assistant") {
            api_messages.insert(
                0,
                ApiMessage {
                    role: "user".to_string(),
                    content: vec![ContentBlock::Text {
                        text: "[conversation start]".to_string(),
                    }],
                },
            );
        }

        (combined_system, api_messages)
    }
}

/// Remove orphaned tool_calls — tool_call messages that have no matching
/// tool_result following them. The Claude API requires every tool_use to have
/// a corresponding tool_result.
fn remove_orphaned_tool_calls(messages: Vec<&StoredMessage>) -> Vec<&StoredMessage> {
    use std::collections::HashSet;

    // Collect all tool_use_ids that have a tool_result.
    let result_ids: HashSet<&str> = messages
        .iter()
        .filter(|m| m.role == MessageRole::ToolResult)
        .filter_map(|m| m.tool_name.as_deref()) // tool_use_id stored in tool_name for results
        .collect();

    // Keep messages, dropping tool_calls without a matching result.
    // Also drop any assistant text that only preceded an orphaned tool_call.
    messages
        .into_iter()
        .filter(|m| {
            if m.role == MessageRole::ToolCall {
                // content holds the tool_use_id for tool_call messages
                result_ids.contains(m.content.as_str())
            } else {
                true
            }
        })
        .collect()
}

/// Convert a StoredMessage to an ApiMessage.
fn stored_to_api_message(msg: &StoredMessage) -> ApiMessage {
    match msg.role {
        MessageRole::User => ApiMessage {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: msg.content.clone(),
            }],
        },
        MessageRole::Assistant => ApiMessage {
            role: "assistant".to_string(),
            content: vec![ContentBlock::Text {
                text: msg.content.clone(),
            }],
        },
        MessageRole::ToolCall => {
            let input: serde_json::Value = msg
                .tool_args
                .as_ref()
                .and_then(|a| serde_json::from_str(a).ok())
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            ApiMessage {
                role: "assistant".to_string(),
                content: vec![ContentBlock::ToolUse {
                    id: msg.content.clone(), // We store tool_use_id in content for tool_call role
                    name: msg.tool_name.clone().unwrap_or_default(),
                    input,
                }],
            }
        }
        MessageRole::ToolResult => ApiMessage {
            role: "user".to_string(),
            content: vec![ContentBlock::ToolResult {
                tool_use_id: msg.tool_name.clone().unwrap_or_default(), // tool_use_id stored in tool_name for tool_result
                content: msg.content.clone(),
                is_error: msg.exit_code.map(|c| c != 0).unwrap_or(false),
            }],
        },
        MessageRole::System => ApiMessage {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: format!("[system] {}", msg.content),
            }],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(id: i64, role: MessageRole, content: &str) -> StoredMessage {
        StoredMessage {
            id,
            role,
            content: content.to_string(),
            tool_name: None,
            tool_args: None,
            exit_code: None,
            timestamp: 1000 + id,
            completion_status: None,
        }
    }

    fn make_tool_call(id: i64, tool_use_id: &str, name: &str, args: &str) -> StoredMessage {
        StoredMessage {
            id,
            role: MessageRole::ToolCall,
            content: tool_use_id.to_string(),
            tool_name: Some(name.to_string()),
            tool_args: Some(args.to_string()),
            exit_code: None,
            timestamp: 1000 + id,
            completion_status: None,
        }
    }

    fn make_tool_result(id: i64, tool_use_id: &str, output: &str) -> StoredMessage {
        StoredMessage {
            id,
            role: MessageRole::ToolResult,
            content: output.to_string(),
            tool_name: Some(tool_use_id.to_string()),
            tool_args: None,
            exit_code: Some(0),
            timestamp: 1000 + id,
            completion_status: None,
        }
    }

    fn default_builder() -> ContextBuilder {
        // 200k tokens budget, 4096 reserve, 4 chars/token
        ContextBuilder::new(200_000, 4096, 4)
    }

    #[test]
    fn estimate_tokens_basic() {
        let cb = ContextBuilder::new(100_000, 4096, 4);
        // "hello world" = 11 chars, ceil(11/4) = 3
        assert_eq!(cb.estimate_tokens("hello world"), 3);
    }

    #[test]
    fn estimate_tokens_empty() {
        let cb = ContextBuilder::new(100_000, 4096, 4);
        assert_eq!(cb.estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_exact_multiple() {
        let cb = ContextBuilder::new(100_000, 4096, 4);
        // 8 chars, 4 chars/token = exactly 2
        assert_eq!(cb.estimate_tokens("abcdefgh"), 2);
    }

    #[test]
    fn build_with_empty_messages() {
        let cb = default_builder();
        let (system, messages) = cb.build("You are helpful.", &[], &[]);
        assert_eq!(system, "You are helpful.");
        assert!(messages.is_empty());
    }

    #[test]
    fn build_includes_skill_prompts_in_system() {
        let cb = default_builder();
        let skills = vec!["Skill A instructions.".to_string()];
        let (system, _) = cb.build("Base prompt.", &skills, &[]);
        assert!(system.contains("Base prompt."));
        assert!(system.contains("Skill A instructions."));
    }

    #[test]
    fn build_with_single_user_message() {
        let cb = default_builder();
        let msgs = vec![make_msg(1, MessageRole::User, "hello")];
        let (_, api_msgs) = cb.build("system", &[], &msgs);

        assert!(!api_msgs.is_empty());
        // First message should be user role
        assert_eq!(api_msgs[0].role, "user");
    }

    #[test]
    fn build_ensures_first_message_is_user() {
        let cb = default_builder();
        let msgs = vec![
            make_msg(1, MessageRole::Assistant, "I started talking"),
            make_msg(2, MessageRole::User, "hello"),
        ];
        let (_, api_msgs) = cb.build("system", &[], &msgs);

        // Should prepend a user message
        assert_eq!(api_msgs[0].role, "user");
    }

    #[test]
    fn build_system_prompt_always_present() {
        let cb = default_builder();
        let msgs = vec![make_msg(1, MessageRole::User, "test")];
        let (system, _) = cb.build("Always here.", &[], &msgs);
        assert_eq!(system, "Always here.");
    }

    #[test]
    fn build_truncates_oldest_when_budget_exceeded() {
        // Very small budget to force truncation
        let cb = ContextBuilder::new(100, 20, 4);
        let msgs: Vec<StoredMessage> = (0..50)
            .map(|i| make_msg(i, MessageRole::User, &format!("Message number {}", i)))
            .collect();

        let (_, api_msgs) = cb.build("sys", &[], &msgs);

        // Should have fewer messages than the original 50
        // Count actual user content blocks (excluding truncation notice)
        let total_content: usize = api_msgs.iter().map(|m| m.content.len()).sum();
        assert!(total_content < 50);
    }

    #[test]
    fn orphaned_tool_calls_are_removed() {
        let cb = default_builder();
        let msgs = vec![
            make_msg(1, MessageRole::User, "do something"),
            make_tool_call(2, "tc_1", "run_command", r#"{"cmd":"ls"}"#),
            // No tool_result for tc_1 -- this is orphaned
            make_msg(3, MessageRole::User, "try again"),
        ];

        let (_, api_msgs) = cb.build("system", &[], &msgs);

        // The orphaned tool_call should be removed, so there shouldn't be
        // any ToolUse blocks in the output
        let has_tool_use = api_msgs.iter().any(|m| {
            m.content.iter().any(|c| matches!(c, ContentBlock::ToolUse { .. }))
        });
        assert!(!has_tool_use);
    }

    #[test]
    fn paired_tool_call_and_result_preserved() {
        let cb = default_builder();
        let msgs = vec![
            make_msg(1, MessageRole::User, "run ls"),
            make_tool_call(2, "tc_1", "run_command", r#"{"cmd":"ls"}"#),
            make_tool_result(3, "tc_1", "file1\nfile2"),
            make_msg(4, MessageRole::Assistant, "Here are your files."),
        ];

        let (_, api_msgs) = cb.build("system", &[], &msgs);

        // Should have a ToolUse block somewhere
        let has_tool_use = api_msgs.iter().any(|m| {
            m.content.iter().any(|c| matches!(c, ContentBlock::ToolUse { .. }))
        });
        assert!(has_tool_use);

        // Should have a ToolResult block
        let has_tool_result = api_msgs.iter().any(|m| {
            m.content.iter().any(|c| matches!(c, ContentBlock::ToolResult { .. }))
        });
        assert!(has_tool_result);
    }

    #[test]
    fn consecutive_same_role_messages_merged() {
        let cb = default_builder();
        let msgs = vec![
            make_msg(1, MessageRole::User, "part 1"),
            make_msg(2, MessageRole::User, "part 2"),
            make_msg(3, MessageRole::Assistant, "reply"),
        ];

        let (_, api_msgs) = cb.build("system", &[], &msgs);

        // The two user messages should be merged into one ApiMessage
        let user_msgs: Vec<_> = api_msgs.iter().filter(|m| m.role == "user").collect();
        assert_eq!(user_msgs.len(), 1);
        assert_eq!(user_msgs[0].content.len(), 2);
    }
}
