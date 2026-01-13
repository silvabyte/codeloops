use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

/// The critic's decision after evaluating actor output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CriticDecision {
    /// Task is complete, stop the loop
    Done {
        /// Summary of what was accomplished
        summary: String,
        /// Confidence level (0.0 - 1.0)
        #[serde(default = "default_confidence")]
        confidence: f64,
    },
    /// Task needs more work, continue with feedback
    Continue {
        /// Feedback for the next actor iteration
        feedback: String,
        /// What aspects still need work
        #[serde(default)]
        remaining_issues: Vec<String>,
    },
    /// Actor encountered an error that needs addressing
    Error {
        /// Description of the error
        error_description: String,
        /// Suggested recovery action
        #[serde(default)]
        recovery_suggestion: String,
    },
}

fn default_confidence() -> f64 {
    1.0
}

#[derive(Error, Debug)]
pub enum DecisionParseError {
    #[error("No decision marker found in critic output")]
    NoDecisionFound,

    #[error("Ambiguous decision: both DONE and CONTINUE markers found")]
    AmbiguousDecision,

    #[error("Failed to parse decision JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Invalid decision format: {0}")]
    InvalidFormat(String),
}

impl CriticDecision {
    /// Parse decision from critic's output text
    ///
    /// Expected format in critic output:
    /// ```text
    /// <decision>
    /// {"type": "done", "summary": "...", "confidence": 0.95}
    /// </decision>
    /// ```
    /// or
    /// ```text
    /// <decision>
    /// {"type": "continue", "feedback": "...", "remaining_issues": [...]}
    /// </decision>
    /// ```
    pub fn parse(critic_output: &str) -> Result<Self, DecisionParseError> {
        debug!(output_len = critic_output.len(), "Parsing critic decision");

        // Look for decision block
        if let Some(decision) = Self::parse_decision_block(critic_output)? {
            return Ok(decision);
        }

        // Fallback: look for simple markers
        Self::parse_simple_markers(critic_output)
    }

    fn parse_decision_block(output: &str) -> Result<Option<Self>, DecisionParseError> {
        let decision_start = output.find("<decision>");
        let decision_end = output.find("</decision>");

        match (decision_start, decision_end) {
            (Some(start), Some(end)) if start < end => {
                let json_str = &output[start + 10..end].trim();
                debug!(json = json_str, "Found decision block");
                let decision: CriticDecision = serde_json::from_str(json_str)?;
                Ok(Some(decision))
            }
            (Some(_), Some(_)) => Err(DecisionParseError::InvalidFormat(
                "Malformed decision block".to_string(),
            )),
            _ => Ok(None),
        }
    }

    fn parse_simple_markers(output: &str) -> Result<Self, DecisionParseError> {
        let upper = output.to_uppercase();

        // Look for various completion markers
        let done_markers = [
            "TASK COMPLETE",
            "TASK IS COMPLETE",
            "[DONE]",
            "SUCCESSFULLY COMPLETED",
            "ALL REQUIREMENTS MET",
        ];

        let continue_markers = [
            "NEEDS MORE WORK",
            "NOT YET COMPLETE",
            "[CONTINUE]",
            "ADDITIONAL WORK REQUIRED",
            "ISSUES REMAIN",
        ];

        let has_done = done_markers.iter().any(|m| upper.contains(m));
        let has_continue = continue_markers.iter().any(|m| upper.contains(m));

        match (has_done, has_continue) {
            (true, false) => {
                debug!("Parsed as DONE via simple markers");
                Ok(CriticDecision::Done {
                    summary: "Task marked as complete by critic".into(),
                    confidence: 0.8,
                })
            }
            (false, true) => {
                debug!("Parsed as CONTINUE via simple markers");
                Ok(CriticDecision::Continue {
                    feedback: Self::extract_feedback(output),
                    remaining_issues: vec![],
                })
            }
            (true, true) => Err(DecisionParseError::AmbiguousDecision),
            (false, false) => Err(DecisionParseError::NoDecisionFound),
        }
    }

    fn extract_feedback(output: &str) -> String {
        // Try to extract meaningful feedback from the output
        // Look for common feedback patterns
        let feedback_markers = ["Feedback:", "Issues:", "Problems:", "Suggestions:"];

        for marker in feedback_markers {
            if let Some(pos) = output.find(marker) {
                let start = pos + marker.len();
                let end = output[start..]
                    .find("\n\n")
                    .map(|p| start + p)
                    .unwrap_or(output.len().min(start + 500));
                return output[start..end].trim().to_string();
            }
        }

        // Fall back to the full output (truncated)
        if output.len() > 500 {
            format!("{}...", &output[..500])
        } else {
            output.to_string()
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, CriticDecision::Done { .. })
    }

    pub fn is_continue(&self) -> bool {
        matches!(self, CriticDecision::Continue { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CriticDecision::Error { .. })
    }

    /// Get a short description of the decision for logging
    pub fn short_description(&self) -> String {
        match self {
            CriticDecision::Done { confidence, .. } => {
                format!("DONE (confidence: {:.0}%)", confidence * 100.0)
            }
            CriticDecision::Continue {
                remaining_issues, ..
            } => {
                if remaining_issues.is_empty() {
                    "CONTINUE".to_string()
                } else {
                    format!("CONTINUE ({} issues)", remaining_issues.len())
                }
            }
            CriticDecision::Error { .. } => "ERROR".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_done_decision() {
        let output = r#"
The task has been completed successfully.

<decision>
{"type": "done", "summary": "Implemented the feature", "confidence": 0.95}
</decision>
"#;

        let decision = CriticDecision::parse(output).unwrap();
        assert!(decision.is_done());
        if let CriticDecision::Done { summary, confidence } = decision {
            assert_eq!(summary, "Implemented the feature");
            assert!((confidence - 0.95).abs() < 0.001);
        }
    }

    #[test]
    fn test_parse_continue_decision() {
        let output = r#"
The implementation is incomplete.

<decision>
{"type": "continue", "feedback": "Need to add error handling", "remaining_issues": ["No error handling", "Missing tests"]}
</decision>
"#;

        let decision = CriticDecision::parse(output).unwrap();
        assert!(decision.is_continue());
        if let CriticDecision::Continue {
            feedback,
            remaining_issues,
        } = decision
        {
            assert_eq!(feedback, "Need to add error handling");
            assert_eq!(remaining_issues.len(), 2);
        }
    }

    #[test]
    fn test_parse_simple_done_marker() {
        let output = "The TASK IS COMPLETE and works correctly.";
        let decision = CriticDecision::parse(output).unwrap();
        assert!(decision.is_done());
    }

    #[test]
    fn test_parse_simple_continue_marker() {
        let output = "The implementation NEEDS MORE WORK on error handling.";
        let decision = CriticDecision::parse(output).unwrap();
        assert!(decision.is_continue());
    }

    #[test]
    fn test_parse_no_decision() {
        let output = "This output has no clear decision markers.";
        let result = CriticDecision::parse(output);
        assert!(matches!(result, Err(DecisionParseError::NoDecisionFound)));
    }
}
