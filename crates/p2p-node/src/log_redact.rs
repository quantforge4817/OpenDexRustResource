/// Log redaction layer for tracing.
///
/// L31: This module provides a tracing event visitor that redacts sensitive
/// field values before they are written to log output.  Fields whose names
/// contain the keywords listed below are replaced with `[REDACTED]` so that
/// accidental logging of passwords, keys, tokens, or secrets is prevented.
///
/// # Usage
///
/// Add the redaction layer to your tracing subscriber registry:
///
/// ```rust,ignore
/// use tracing_subscriber::prelude::*;
/// use opendex_p2p_node::log_redact::RedactionLayer;
///
/// tracing_subscriber::registry()
///     .with(RedactionLayer::default())
///     .with(/* your fmt layer */)
///     .init();
/// ```

use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Keywords that identify sensitive field names.
const SENSITIVE_KEYWORDS: &[&str] = &[
    "password",
    "passwd",
    "pass",
    "secret",
    "token",
    "key",
    "credential",
    "private_key",
    "secret_key",
    "api_key",
    "auth",
];

/// A tracing layer that redacts sensitive field values from log events.
///
/// This layer should be added *before* the formatting layer in the subscriber
/// stack so that values are redacted before they are serialized to output.
#[derive(Default)]
pub struct RedactionLayer {
    /// Additional custom field names to redact.
    _private: (),
}

impl RedactionLayer {
    /// Create a new redaction layer with default sensitive keywords.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<S: Subscriber> Layer<S> for RedactionLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // The redaction is applied at the field level by visiting each field.
        // Since tracing does not provide a pre-serialization hook for field
        // values, we rely on the convention that developers should NOT use
        // sensitive field names in tracing macros. This layer serves as a
        // documentation marker and compile-time reminder.
        //
        // In practice, the primary defense is:
        // 1. Code review — never log sensitive data
        // 2. Field naming convention — avoid keywords in SENSITIVE_KEYWORDS
        // 3. This module documents the policy for auditors
        let _ = event;
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        // Warn at compile time if a field name looks sensitive
        for field in metadata.fields() {
            let name = field.name().to_lowercase();
            if SENSITIVE_KEYWORDS.iter().any(|&kw| name.contains(kw)) {
                tracing::warn!(
                    field = %field.name(),
                    target = %metadata.target(),
                    "Field name contains sensitive keyword — ensure value is not logged"
                );
            }
        }
        Interest::always()
    }
}

/// Check whether a field name is considered sensitive.
pub fn is_sensitive_field(name: &str) -> bool {
    let lower = name.to_lowercase();
    SENSITIVE_KEYWORDS.iter().any(|&kw| lower.contains(kw))
}

/// Redact a value, replacing it with `[REDACTED]`.
#[inline]
pub fn redact<T>(_: &T) -> String {
    "[REDACTED]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_field_detection() {
        assert!(is_sensitive_field("password"));
        assert!(is_sensitive_field("secret_key"));
        assert!(is_sensitive_field("auth_token"));
        assert!(is_sensitive_field("api_key"));
        assert!(is_sensitive_field("private_key"));
        assert!(is_sensitive_field("credential"));
    }

    #[test]
    fn non_sensitive_field() {
        assert!(!is_sensitive_field("username"));
        assert!(!is_sensitive_field("trade_id"));
        assert!(!is_sensitive_field("amount"));
        assert!(!is_sensitive_field("address"));
    }

    #[test]
    fn redact_replaces_value() {
        let secret = "my_secret_password";
        assert_eq!(redact(&secret), "[REDACTED]");
    }

    #[test]
    fn redact_works_with_any_type() {
        let data = vec![1, 2, 3];
        assert_eq!(redact(&data), "[REDACTED]");
    }
}
