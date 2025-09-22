//! Intent Parser Service Library
//!
//! This library provides the core functionality for parsing natural language
//! requests into structured automation workflows.

pub mod blog_intent;
pub mod config;
pub mod error;
pub mod llm;
pub mod parser;
pub mod types;

pub use blog_intent::*;
pub use config::Config;
pub use error::{AppError, Result};
pub use parser::IntentParser;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_config_from_env() {
        // Test config can be created with minimal environment
        std::env::set_var("LLM_API_KEY", "test-key");
        std::env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/test");

        let config = Config::from_env();
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8081);
        assert_eq!(config.environment, "development");
    }

    #[test]
    fn test_error_types() {
        let error = AppError::BadRequest("test error".to_string());
        assert_eq!(error.error_code(), "BAD_REQUEST");
        assert!(error.is_client_error());
        assert!(!error.is_server_error());
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_workflow_types() {
        let workflow_type = WorkflowType::ContentCreation;
        assert!(matches!(workflow_type, WorkflowType::ContentCreation));

        let custom_type = WorkflowType::Custom("test".to_string());
        assert!(matches!(custom_type, WorkflowType::Custom(_)));
    }

    #[test]
    fn test_user_context_defaults() {
        let context = UserContext::default();
        assert_eq!(context.preferences.language, "");
        assert_eq!(context.preferences.cost_sensitivity, 0.0);
        assert_eq!(context.preferences.speed_preference, 0.0);
        assert!(context.integrations.is_empty());
        assert!(matches!(context.subscription_tier, SubscriptionTier::Free));
    }

    #[test]
    fn test_parse_intent_request_validation() {
        let request = ParseIntentRequest {
            user_id: Uuid::new_v4(),
            text: "Create a blog post about AI".to_string(),
            context: None,
            federation_context: None,
            preferred_providers: None,
            budget_limit: None,
            time_limit: None,
            quality_threshold: None,
        };

        assert!(!request.text.is_empty());
        assert!(request.text.len() < 10000);
    }

    #[test]
    fn test_function_call_creation() {
        let function_call = FunctionCall {
            id: Uuid::new_v4(),
            name: "create_content_workflow".to_string(),
            description: "Test function".to_string(),
            parameters: serde_json::json!({"test": "value"}),
            provider: "test".to_string(),
            estimated_cost: 1.0,
            estimated_duration: chrono::Duration::minutes(30),
            confidence_score: 0.8,
            required_permissions: vec!["content.create".to_string()],
            mcp_server: Some("content-mcp-server".to_string()),
        };

        assert_eq!(function_call.name, "create_content_workflow");
        assert_eq!(function_call.estimated_cost, 1.0);
        assert_eq!(function_call.confidence_score, 0.8);
    }

    #[test]
    fn test_validation_result() {
        let validation = ValidationResult {
            is_valid: true,
            confidence_score: 0.9,
            warnings: vec!["test warning".to_string()],
            suggestions: vec!["test suggestion".to_string()],
            estimated_execution_time: chrono::Duration::hours(1),
            estimated_cost: 10.0,
            missing_permissions: vec![],
            invalid_parameters: vec![],
        };

        assert!(validation.is_valid);
        assert_eq!(validation.confidence_score, 0.9);
        assert_eq!(validation.warnings.len(), 1);
        assert_eq!(validation.suggestions.len(), 1);
    }

    #[test]
    fn test_scheduling_requirements() {
        let scheduling = SchedulingRequirements {
            needs_scheduling: true,
            schedule_type: Some(ScheduleType::OptimalTiming),
            time_sensitivity: Some(TimeSensitivity::High),
            platforms: vec!["facebook".to_string(), "twitter".to_string()],
            content_calendar_integration: true,
            recurring_pattern: Some(RecurringPattern {
                frequency: "weekly".to_string(),
                interval: 1,
                cron_expression: None,
                end_date: None,
                max_occurrences: Some(10),
            }),
            timezone: Some("UTC".to_string()),
        };

        assert!(scheduling.needs_scheduling);
        assert!(matches!(
            scheduling.schedule_type,
            Some(ScheduleType::OptimalTiming)
        ));
        assert!(matches!(
            scheduling.time_sensitivity,
            Some(TimeSensitivity::High)
        ));
        assert_eq!(scheduling.platforms.len(), 2);
    }

    #[test]
    fn test_subscription_tier_default() {
        let tier = SubscriptionTier::default();
        assert!(matches!(tier, SubscriptionTier::Free));
    }

    #[test]
    fn test_dependency_types() {
        let dependency = Dependency {
            prerequisite: "step1".to_string(),
            dependent: "step2".to_string(),
            data_transfer: Some("output_data".to_string()),
            dependency_type: DependencyType::DataDependency,
        };

        assert_eq!(dependency.prerequisite, "step1");
        assert_eq!(dependency.dependent, "step2");
        assert!(matches!(
            dependency.dependency_type,
            DependencyType::DataDependency
        ));
    }

    #[test]
    fn test_intent_metadata() {
        let metadata = IntentMetadata {
            created_at: chrono::Utc::now(),
            complexity_score: 0.5,
            language: "en".to_string(),
            domain_scores: std::collections::HashMap::new(),
            user_preferences: None,
            context_variables: std::collections::HashMap::new(),
        };

        assert_eq!(metadata.complexity_score, 0.5);
        assert_eq!(metadata.language, "en");
        assert!(metadata.domain_scores.is_empty());
        assert!(metadata.context_variables.is_empty());
    }

    #[test]
    fn test_cost_range() {
        let cost_range = CostRange {
            min_cost: 0.5,
            max_cost: 10.0,
            average_cost: 2.5,
            currency: "USD".to_string(),
        };

        assert_eq!(cost_range.min_cost, 0.5);
        assert_eq!(cost_range.max_cost, 10.0);
        assert_eq!(cost_range.average_cost, 2.5);
        assert_eq!(cost_range.currency, "USD");
    }

    #[test]
    fn test_function_info() {
        let function_info = FunctionInfo {
            id: "test_function".to_string(),
            name: "Test Function".to_string(),
            description: "A test function".to_string(),
            domain: "testing".to_string(),
            cost_range: CostRange {
                min_cost: 1.0,
                max_cost: 5.0,
                average_cost: 2.5,
                currency: "USD".to_string(),
            },
            estimated_duration: chrono::Duration::minutes(15),
            complexity_score: 0.3,
            popularity_score: 0.8,
            success_rate: 0.95,
            required_permissions: vec!["test.execute".to_string()],
            supported_parameters: vec![ParameterInfo {
                name: "input".to_string(),
                parameter_type: "string".to_string(),
                required: true,
                description: "Input parameter".to_string(),
                default_value: None,
                validation_rules: vec!["non-empty".to_string()],
            }],
        };

        assert_eq!(function_info.id, "test_function");
        assert_eq!(function_info.name, "Test Function");
        assert_eq!(function_info.domain, "testing");
        assert_eq!(function_info.complexity_score, 0.3);
        assert_eq!(function_info.success_rate, 0.95);
        assert_eq!(function_info.supported_parameters.len(), 1);
    }
}
