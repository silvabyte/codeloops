//! JSON protocol for agent-TUI communication.
//!
//! The agent communicates with the TUI through structured JSON messages that
//! drive the interview process and update the draft prompt.

use serde::{Deserialize, Serialize};

/// Messages sent from the agent to the TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// A question for the user to answer
    Question {
        /// The question text
        text: String,
        /// Optional context or explanation
        #[serde(default)]
        context: Option<String>,
        /// The type of input expected
        input_type: InputType,
        /// For select/multi-select: the available options
        #[serde(default)]
        options: Vec<SelectOption>,
        /// Which section of the draft this relates to
        #[serde(default)]
        section: Option<String>,
    },

    /// An update to the draft prompt
    DraftUpdate {
        /// The section being updated
        section: String,
        /// The new content for that section
        content: String,
        /// Whether to append or replace
        #[serde(default)]
        append: bool,
    },

    /// Agent is thinking/processing
    Thinking {
        /// What the agent is working on
        message: String,
    },

    /// Agent needs clarification on a previous answer
    Clarification {
        /// The question seeking clarification
        text: String,
        /// The original answer that needs clarification
        original_answer: String,
        /// The type of input expected
        input_type: InputType,
        /// For select/multi-select: the available options
        #[serde(default)]
        options: Vec<SelectOption>,
        /// Whether this is a vague answer warning (user can press Esc to keep original)
        #[serde(default)]
        is_vague_warning: bool,
    },

    /// Agent suggests the interview is complete
    SuggestComplete {
        /// Summary of what was captured
        summary: String,
        /// Confidence level (0.0-1.0)
        #[serde(default)]
        confidence: f32,
        /// Areas that could be improved if user wants to continue
        #[serde(default)]
        could_improve: Vec<String>,
    },

    /// The draft is complete
    DraftComplete {
        /// Final summary of what was captured
        summary: String,
    },

    /// An error occurred
    Error {
        /// Error message
        message: String,
    },
}

/// Types of input the user can provide
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    /// Free-form text input
    #[default]
    Text,
    /// Single selection from options
    Select,
    /// Multiple selections from options
    MultiSelect,
    /// Yes/no confirmation
    Confirm,
    /// Multi-line text editor
    Editor,
}

/// An option for select/multi-select inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    /// The value to store if selected
    pub value: String,
    /// Display label for the option
    pub label: String,
    /// Optional description/help text
    #[serde(default)]
    pub description: Option<String>,
}

/// Response from the user to the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// The user's answer
    pub answer: UserAnswer,
    /// Optional feedback or additional context
    #[serde(default)]
    pub feedback: Option<String>,
}

/// Types of answers the user can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserAnswer {
    /// Free-form text response
    Text(String),
    /// Single selection
    Selection(String),
    /// Multiple selections
    MultiSelection(Vec<String>),
    /// Yes/no confirmation
    Confirm(bool),
}

impl UserAnswer {
    /// Convert to a string representation for the agent
    pub fn to_prompt_string(&self) -> String {
        match self {
            UserAnswer::Text(s) => s.clone(),
            UserAnswer::Selection(s) => s.clone(),
            UserAnswer::MultiSelection(v) => v.join(", "),
            UserAnswer::Confirm(b) => if *b { "yes" } else { "no" }.to_string(),
        }
    }
}

impl UserResponse {
    /// Create a text response
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            answer: UserAnswer::Text(s.into()),
            feedback: None,
        }
    }

    /// Create a selection response
    pub fn selection(s: impl Into<String>) -> Self {
        Self {
            answer: UserAnswer::Selection(s.into()),
            feedback: None,
        }
    }

    /// Create a multi-selection response
    pub fn multi_selection(selections: Vec<String>) -> Self {
        Self {
            answer: UserAnswer::MultiSelection(selections),
            feedback: None,
        }
    }

    /// Create a confirmation response
    pub fn confirm(value: bool) -> Self {
        Self {
            answer: UserAnswer::Confirm(value),
            feedback: None,
        }
    }

    /// Add feedback to the response
    #[allow(dead_code)]
    pub fn with_feedback(mut self, feedback: impl Into<String>) -> Self {
        self.feedback = Some(feedback.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_message_question_serialization() {
        let msg = AgentMessage::Question {
            text: "What is the main goal?".to_string(),
            context: Some("This helps define the scope".to_string()),
            input_type: InputType::Text,
            options: vec![],
            section: Some("goal".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: AgentMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            AgentMessage::Question { text, .. } => {
                assert_eq!(text, "What is the main goal?");
            }
            _ => panic!("Expected Question variant"),
        }
    }

    #[test]
    fn test_agent_message_select_serialization() {
        let msg = AgentMessage::Question {
            text: "What type of project?".to_string(),
            context: None,
            input_type: InputType::Select,
            options: vec![
                SelectOption {
                    value: "web".to_string(),
                    label: "Web Application".to_string(),
                    description: Some("Frontend or full-stack web app".to_string()),
                },
                SelectOption {
                    value: "cli".to_string(),
                    label: "CLI Tool".to_string(),
                    description: None,
                },
            ],
            section: None,
        };

        let json = serde_json::to_string_pretty(&msg).unwrap();
        let parsed: AgentMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            AgentMessage::Question { options, .. } => {
                assert_eq!(options.len(), 2);
                assert_eq!(options[0].value, "web");
            }
            _ => panic!("Expected Question variant"),
        }
    }

    #[test]
    fn test_draft_update_serialization() {
        let msg = AgentMessage::DraftUpdate {
            section: "requirements".to_string(),
            content: "- Must support authentication\n- Must be fast".to_string(),
            append: true,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: AgentMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            AgentMessage::DraftUpdate {
                section, append, ..
            } => {
                assert_eq!(section, "requirements");
                assert!(append);
            }
            _ => panic!("Expected DraftUpdate variant"),
        }
    }

    #[test]
    fn test_user_response_serialization() {
        let response = UserResponse::text("Build a REST API");
        let json = serde_json::to_string(&response).unwrap();
        let parsed: UserResponse = serde_json::from_str(&json).unwrap();

        match parsed.answer {
            UserAnswer::Text(s) => assert_eq!(s, "Build a REST API"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_user_response_multi_selection() {
        let response =
            UserResponse::multi_selection(vec!["auth".to_string(), "logging".to_string()]);
        let json = serde_json::to_string(&response).unwrap();
        let parsed: UserResponse = serde_json::from_str(&json).unwrap();

        match parsed.answer {
            UserAnswer::MultiSelection(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0], "auth");
            }
            _ => panic!("Expected MultiSelection variant"),
        }
    }

    #[test]
    fn test_answer_to_prompt_string() {
        assert_eq!(
            UserAnswer::Text("hello".to_string()).to_prompt_string(),
            "hello"
        );
        assert_eq!(
            UserAnswer::Selection("option1".to_string()).to_prompt_string(),
            "option1"
        );
        assert_eq!(
            UserAnswer::MultiSelection(vec!["a".to_string(), "b".to_string()]).to_prompt_string(),
            "a, b"
        );
        assert_eq!(UserAnswer::Confirm(true).to_prompt_string(), "yes");
        assert_eq!(UserAnswer::Confirm(false).to_prompt_string(), "no");
    }

    // Task 4.3: Tests for new message types

    #[test]
    fn test_suggest_complete_serialization() {
        let msg = AgentMessage::SuggestComplete {
            summary: "Comprehensive API with auth and rate limiting".to_string(),
            confidence: 0.85,
            could_improve: vec![
                "Session timeout details".to_string(),
                "Error message formats".to_string(),
            ],
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: AgentMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            AgentMessage::SuggestComplete {
                summary,
                confidence,
                could_improve,
            } => {
                assert_eq!(summary, "Comprehensive API with auth and rate limiting");
                assert!((confidence - 0.85).abs() < 0.01);
                assert_eq!(could_improve.len(), 2);
            }
            _ => panic!("Expected SuggestComplete variant"),
        }
    }

    #[test]
    fn test_clarification_with_vague_warning() {
        let msg = AgentMessage::Clarification {
            text: "Can you be more specific about 'fast'?".to_string(),
            original_answer: "make it fast".to_string(),
            input_type: InputType::Text,
            options: vec![],
            is_vague_warning: true,
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: AgentMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            AgentMessage::Clarification {
                is_vague_warning, ..
            } => {
                assert!(is_vague_warning);
            }
            _ => panic!("Expected Clarification variant"),
        }
    }

    #[test]
    fn test_clarification_default_vague_warning() {
        // Test that is_vague_warning defaults to false when not specified
        let json = r#"{
            "type": "clarification",
            "text": "Can you clarify?",
            "original_answer": "something",
            "input_type": "text",
            "options": []
        }"#;

        let parsed: AgentMessage = serde_json::from_str(json).unwrap();

        match parsed {
            AgentMessage::Clarification {
                is_vague_warning, ..
            } => {
                assert!(!is_vague_warning); // Should default to false
            }
            _ => panic!("Expected Clarification variant"),
        }
    }

    #[test]
    fn test_suggest_complete_default_values() {
        // Test that optional fields have sensible defaults
        let json = r#"{
            "type": "suggest_complete",
            "summary": "All requirements gathered"
        }"#;

        let parsed: AgentMessage = serde_json::from_str(json).unwrap();

        match parsed {
            AgentMessage::SuggestComplete {
                summary,
                confidence,
                could_improve,
            } => {
                assert_eq!(summary, "All requirements gathered");
                assert_eq!(confidence, 0.0); // Default
                assert!(could_improve.is_empty()); // Default empty vec
            }
            _ => panic!("Expected SuggestComplete variant"),
        }
    }
}
