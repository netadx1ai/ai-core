//! Intent parser service for natural language workflow definition processing

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

use crate::error::{ApiError, Result};

/// Intent parser service for processing natural language workflow definitions
#[derive(Clone)]
pub struct IntentParserService {
    // In a real implementation, this would include ML models, NLP libraries, etc.
    // For now, we'll use rule-based parsing as a foundation
    patterns: HashMap<String, WorkflowPattern>,
}

/// Parsed workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedWorkflow {
    pub intent: WorkflowIntent,
    pub steps: Vec<WorkflowStep>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub estimated_complexity: ComplexityLevel,
    pub required_integrations: Vec<String>,
    pub confidence_score: f64,
}

/// Workflow intent classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowIntent {
    ContentGeneration,
    DataProcessing,
    APIIntegration,
    NotificationSending,
    FileManagement,
    WebScraping,
    EmailAutomation,
    SocialMediaPosting,
    DatabaseOperations,
    ReportGeneration,
    Unknown,
}

/// Individual workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub action: String,
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub estimated_duration_seconds: Option<u32>,
}

/// Complexity level for workflow estimation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityLevel {
    Simple,   // 1-3 steps, basic operations
    Moderate, // 4-8 steps, some integrations
    Complex,  // 9-15 steps, multiple integrations
    Advanced, // 16+ steps, complex logic
}

/// Workflow pattern for rule-based parsing
#[derive(Debug, Clone)]
struct WorkflowPattern {
    keywords: Vec<String>,
    intent: WorkflowIntent,
    template_steps: Vec<String>,
}

/// Validation result for workflow definition
#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

impl IntentParserService {
    /// Create new intent parser service
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Content generation patterns
        patterns.insert(
            "content_blog".to_string(),
            WorkflowPattern {
                keywords: vec![
                    "blog".to_string(),
                    "article".to_string(),
                    "write".to_string(),
                    "content".to_string(),
                    "post".to_string(),
                ],
                intent: WorkflowIntent::ContentGeneration,
                template_steps: vec![
                    "research_topic".to_string(),
                    "generate_outline".to_string(),
                    "write_content".to_string(),
                    "review_content".to_string(),
                    "publish".to_string(),
                ],
            },
        );

        // Email automation patterns
        patterns.insert(
            "email_campaign".to_string(),
            WorkflowPattern {
                keywords: vec![
                    "email".to_string(),
                    "send".to_string(),
                    "campaign".to_string(),
                    "newsletter".to_string(),
                    "notify".to_string(),
                ],
                intent: WorkflowIntent::EmailAutomation,
                template_steps: vec![
                    "select_recipients".to_string(),
                    "personalize_content".to_string(),
                    "send_emails".to_string(),
                    "track_engagement".to_string(),
                ],
            },
        );

        // Data processing patterns
        patterns.insert(
            "data_analysis".to_string(),
            WorkflowPattern {
                keywords: vec![
                    "analyze".to_string(),
                    "data".to_string(),
                    "process".to_string(),
                    "transform".to_string(),
                    "report".to_string(),
                ],
                intent: WorkflowIntent::DataProcessing,
                template_steps: vec![
                    "extract_data".to_string(),
                    "clean_data".to_string(),
                    "analyze_data".to_string(),
                    "generate_insights".to_string(),
                    "create_report".to_string(),
                ],
            },
        );

        // Social media patterns
        patterns.insert(
            "social_media".to_string(),
            WorkflowPattern {
                keywords: vec![
                    "social".to_string(),
                    "twitter".to_string(),
                    "facebook".to_string(),
                    "instagram".to_string(),
                    "linkedin".to_string(),
                    "post".to_string(),
                    "share".to_string(),
                ],
                intent: WorkflowIntent::SocialMediaPosting,
                template_steps: vec![
                    "create_content".to_string(),
                    "schedule_posts".to_string(),
                    "publish_posts".to_string(),
                    "monitor_engagement".to_string(),
                ],
            },
        );

        Self { patterns }
    }

    /// Parse natural language workflow definition
    pub async fn parse_workflow_definition(&self, definition: &str) -> Result<ParsedWorkflow> {
        info!("Parsing workflow definition: {}", definition);

        let definition_lower = definition.to_lowercase();
        let words: Vec<&str> = definition_lower.split_whitespace().collect();

        if words.is_empty() {
            return Err(ApiError::validation(
                "definition",
                "Workflow definition cannot be empty",
            ));
        }

        // Classify intent based on keywords
        let intent = self.classify_intent(&definition_lower);
        debug!("Classified intent as: {:?}", intent);

        // Extract parameters from the definition
        let parameters = self.extract_parameters(&definition_lower);
        debug!("Extracted parameters: {:?}", parameters);

        // Generate workflow steps based on intent and parameters
        let steps = self.generate_steps(&intent, &parameters);
        debug!("Generated {} steps", steps.len());

        // Determine required integrations
        let required_integrations = self.determine_integrations(&definition_lower, &intent);
        debug!("Required integrations: {:?}", required_integrations);

        // Estimate complexity
        let estimated_complexity = self.estimate_complexity(&steps, &required_integrations);
        debug!("Estimated complexity: {:?}", estimated_complexity);

        // Calculate confidence score
        let confidence_score = self.calculate_confidence(&definition_lower, &intent, &steps);
        debug!("Confidence score: {:.2}", confidence_score);

        let parsed_workflow = ParsedWorkflow {
            intent,
            steps,
            parameters,
            estimated_complexity,
            required_integrations,
            confidence_score,
        };

        info!("Successfully parsed workflow definition");
        Ok(parsed_workflow)
    }

    /// Validate a workflow definition
    pub async fn validate_workflow_definition(&self, definition: &str) -> Result<ValidationResult> {
        info!("Validating workflow definition");

        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Basic validation
        if definition.trim().is_empty() {
            errors.push("Workflow definition cannot be empty".to_string());
        }

        if definition.len() < 10 {
            errors
                .push("Workflow definition is too short. Please provide more details".to_string());
        }

        if definition.len() > 5000 {
            warnings.push(
                "Workflow definition is very long. Consider breaking it into smaller workflows"
                    .to_string(),
            );
        }

        // Try to parse and check for issues
        if errors.is_empty() {
            match self.parse_workflow_definition(definition).await {
                Ok(parsed) => {
                    if parsed.confidence_score < 0.5 {
                        warnings.push("Low confidence in workflow interpretation. Consider providing more specific instructions".to_string());
                    }

                    if parsed.steps.len() > 20 {
                        warnings.push(
                            "Workflow has many steps. Consider breaking it into sub-workflows"
                                .to_string(),
                        );
                    }

                    if parsed.intent == WorkflowIntent::Unknown {
                        warnings.push("Unable to clearly identify workflow intent. Consider being more specific about what you want to accomplish".to_string());
                    }

                    // Provide suggestions based on intent
                    match parsed.intent {
                        WorkflowIntent::ContentGeneration => {
                            suggestions.push("Consider specifying the target audience, tone, and publishing platform".to_string());
                        }
                        WorkflowIntent::EmailAutomation => {
                            suggestions.push("Consider adding email templates, scheduling preferences, and success metrics".to_string());
                        }
                        WorkflowIntent::DataProcessing => {
                            suggestions.push("Consider specifying data sources, transformation rules, and output format".to_string());
                        }
                        WorkflowIntent::SocialMediaPosting => {
                            suggestions.push("Consider specifying posting schedule, hashtags, and engagement strategies".to_string());
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to parse workflow: {}", e));
                }
            }
        }

        let is_valid = errors.is_empty();

        info!(
            "Validation complete: valid={}, errors={}, warnings={}",
            is_valid,
            errors.len(),
            warnings.len()
        );

        Ok(ValidationResult {
            is_valid,
            errors,
            warnings,
            suggestions,
        })
    }

    /// Classify the workflow intent based on keywords
    fn classify_intent(&self, definition: &str) -> WorkflowIntent {
        let mut best_match = WorkflowIntent::Unknown;
        let mut best_score = 0;

        for pattern in self.patterns.values() {
            let mut score = 0;
            for keyword in &pattern.keywords {
                if definition.contains(keyword) {
                    score += 1;
                }
            }

            if score > best_score {
                best_score = score;
                best_match = pattern.intent.clone();
            }
        }

        best_match
    }

    /// Extract parameters from the workflow definition
    fn extract_parameters(&self, definition: &str) -> HashMap<String, serde_json::Value> {
        let mut parameters = HashMap::new();

        // Extract common parameters using simple regex-like patterns
        if let Some(topic) =
            self.extract_between(definition, "about", &[" to ", " for ", " on ", ".", "!"])
        {
            parameters.insert("topic".to_string(), serde_json::Value::String(topic));
        }

        if let Some(target) =
            self.extract_between(definition, "for", &[" about ", " on ", ".", "!"])
        {
            parameters.insert(
                "target_audience".to_string(),
                serde_json::Value::String(target),
            );
        }

        if definition.contains("daily") {
            parameters.insert(
                "frequency".to_string(),
                serde_json::Value::String("daily".to_string()),
            );
        } else if definition.contains("weekly") {
            parameters.insert(
                "frequency".to_string(),
                serde_json::Value::String("weekly".to_string()),
            );
        } else if definition.contains("monthly") {
            parameters.insert(
                "frequency".to_string(),
                serde_json::Value::String("monthly".to_string()),
            );
        }

        // Extract email addresses
        let email_regex =
            match regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b") {
                Ok(regex) => regex,
                Err(_) => {
                    // Return parameters if regex compilation fails
                    return parameters;
                }
            };

        // Extract URLs
        if definition.contains("http") {
            let url_parts: Vec<&str> = definition
                .split_whitespace()
                .filter(|word| word.contains("http"))
                .collect();
            if !url_parts.is_empty() {
                parameters.insert(
                    "urls".to_string(),
                    serde_json::Value::Array(
                        url_parts
                            .iter()
                            .map(|url| serde_json::Value::String(url.to_string()))
                            .collect(),
                    ),
                );
            }
        }

        parameters
    }

    /// Extract text between a start word and any of the end patterns
    fn extract_between(&self, text: &str, start: &str, end_patterns: &[&str]) -> Option<String> {
        if let Some(start_pos) = text.find(start) {
            let after_start = &text[start_pos + start.len()..].trim_start();

            for end_pattern in end_patterns {
                if let Some(end_pos) = after_start.find(end_pattern) {
                    return Some(after_start[..end_pos].trim().to_string());
                }
            }

            // If no end pattern found, take the rest of the line or first 50 chars
            let end_pos = after_start.find('\n').unwrap_or_else(|| {
                if after_start.len() > 50 {
                    50
                } else {
                    after_start.len()
                }
            });

            return Some(after_start[..end_pos].trim().to_string());
        }

        None
    }

    /// Generate workflow steps based on intent and parameters
    fn generate_steps(
        &self,
        intent: &WorkflowIntent,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Vec<WorkflowStep> {
        let mut steps = Vec::new();
        let mut step_counter = 1;

        match intent {
            WorkflowIntent::ContentGeneration => {
                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "research_topic".to_string(),
                    description: "Research the specified topic and gather relevant information"
                        .to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec![],
                    estimated_duration_seconds: Some(300), // 5 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "generate_outline".to_string(),
                    description: "Create an outline for the content".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_1".to_string()],
                    estimated_duration_seconds: Some(180), // 3 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "write_content".to_string(),
                    description: "Generate the main content based on the outline".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_2".to_string()],
                    estimated_duration_seconds: Some(600), // 10 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "review_content".to_string(),
                    description: "Review and optimize the generated content".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_3".to_string()],
                    estimated_duration_seconds: Some(240), // 4 minutes
                });
            }

            WorkflowIntent::EmailAutomation => {
                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "prepare_email_list".to_string(),
                    description: "Prepare and validate the email recipient list".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec![],
                    estimated_duration_seconds: Some(120), // 2 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "personalize_emails".to_string(),
                    description: "Personalize email content for each recipient".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_1".to_string()],
                    estimated_duration_seconds: Some(180), // 3 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "send_emails".to_string(),
                    description: "Send the personalized emails to recipients".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_2".to_string()],
                    estimated_duration_seconds: Some(300), // 5 minutes
                });
            }

            WorkflowIntent::DataProcessing => {
                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "extract_data".to_string(),
                    description: "Extract data from specified sources".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec![],
                    estimated_duration_seconds: Some(240), // 4 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "clean_data".to_string(),
                    description: "Clean and validate the extracted data".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_1".to_string()],
                    estimated_duration_seconds: Some(300), // 5 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "analyze_data".to_string(),
                    description: "Perform analysis on the cleaned data".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_2".to_string()],
                    estimated_duration_seconds: Some(480), // 8 minutes
                });
                step_counter += 1;

                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "generate_report".to_string(),
                    description: "Generate a report with analysis results".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec!["step_3".to_string()],
                    estimated_duration_seconds: Some(360), // 6 minutes
                });
            }

            _ => {
                // Generic workflow for unknown intents
                steps.push(WorkflowStep {
                    id: format!("step_{}", step_counter),
                    action: "process_request".to_string(),
                    description: "Process the workflow request".to_string(),
                    parameters: parameters.clone(),
                    dependencies: vec![],
                    estimated_duration_seconds: Some(300), // 5 minutes
                });
            }
        }

        steps
    }

    /// Determine required integrations based on definition and intent
    fn determine_integrations(&self, definition: &str, intent: &WorkflowIntent) -> Vec<String> {
        let mut integrations = Vec::new();

        // Check for specific service mentions
        if definition.contains("gmail") || definition.contains("email") {
            integrations.push("gmail".to_string());
        }

        if definition.contains("slack") {
            integrations.push("slack".to_string());
        }

        if definition.contains("twitter") {
            integrations.push("twitter".to_string());
        }

        if definition.contains("facebook") {
            integrations.push("facebook".to_string());
        }

        if definition.contains("instagram") {
            integrations.push("instagram".to_string());
        }

        if definition.contains("linkedin") {
            integrations.push("linkedin".to_string());
        }

        if definition.contains("google") {
            integrations.push("google_workspace".to_string());
        }

        if definition.contains("salesforce") {
            integrations.push("salesforce".to_string());
        }

        if definition.contains("hubspot") {
            integrations.push("hubspot".to_string());
        }

        // Add default integrations based on intent
        match intent {
            WorkflowIntent::EmailAutomation => {
                if !integrations
                    .iter()
                    .any(|i| i.contains("gmail") || i.contains("email"))
                {
                    integrations.push("email_service".to_string());
                }
            }
            WorkflowIntent::SocialMediaPosting => {
                if integrations.is_empty() {
                    integrations.push("social_media_api".to_string());
                }
            }
            WorkflowIntent::DataProcessing => {
                integrations.push("data_source".to_string());
            }
            _ => {}
        }

        integrations
    }

    /// Estimate workflow complexity
    fn estimate_complexity(
        &self,
        steps: &[WorkflowStep],
        integrations: &[String],
    ) -> ComplexityLevel {
        let step_count = steps.len();
        let integration_count = integrations.len();

        let complexity_score = step_count + (integration_count * 2);

        match complexity_score {
            0..=5 => ComplexityLevel::Simple,
            6..=12 => ComplexityLevel::Moderate,
            13..=20 => ComplexityLevel::Complex,
            _ => ComplexityLevel::Advanced,
        }
    }

    /// Calculate confidence score for the parsing result
    fn calculate_confidence(
        &self,
        definition: &str,
        intent: &WorkflowIntent,
        steps: &[WorkflowStep],
    ) -> f64 {
        let mut score = 0.5f32; // Base score

        // Boost confidence if we found a clear intent
        if *intent != WorkflowIntent::Unknown {
            score += 0.2;
        }

        // Boost confidence based on definition length and detail
        let word_count = definition.split_whitespace().count();
        if word_count > 10 {
            score += 0.1;
        }
        if word_count > 20 {
            score += 0.1;
        }

        // Boost confidence if we generated meaningful steps
        if !steps.is_empty() {
            score += 0.1;
        }

        // Penalize if definition is too vague
        let vague_words = ["something", "anything", "stuff", "things"];
        for vague in &vague_words {
            if definition.contains(vague) {
                score -= 0.1;
            }
        }

        // Ensure score is between 0 and 1
        score.max(0.0).min(1.0).into()
    }
}

impl Default for IntentParserService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_blog_workflow() {
        let parser = IntentParserService::new();
        let definition = "Create a blog post about artificial intelligence for tech enthusiasts";

        let result = parser.parse_workflow_definition(definition).await.unwrap();

        assert_eq!(result.intent, WorkflowIntent::ContentGeneration);
        assert!(!result.steps.is_empty());
        assert!(result.confidence_score > 0.5);
    }

    #[tokio::test]
    async fn test_parse_email_workflow() {
        let parser = IntentParserService::new();
        let definition = "Send a weekly newsletter to our subscribers about product updates";

        let result = parser.parse_workflow_definition(definition).await.unwrap();

        assert_eq!(result.intent, WorkflowIntent::EmailAutomation);
        assert_eq!(
            result
                .parameters
                .get("frequency")
                .unwrap()
                .as_str()
                .unwrap(),
            "weekly"
        );
    }

    #[tokio::test]
    async fn test_validate_empty_definition() {
        let parser = IntentParserService::new();
        let definition = "";

        let result = parser
            .validate_workflow_definition(definition)
            .await
            .unwrap();

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_classify_unknown_intent() {
        let parser = IntentParserService::new();
        let definition = "Do some random task with no clear purpose";

        let result = parser.parse_workflow_definition(definition).await.unwrap();

        assert_eq!(result.intent, WorkflowIntent::Unknown);
        assert!(result.confidence_score < 0.7);
    }

    #[test]
    fn test_extract_parameters() {
        let parser = IntentParserService::new();
        let definition = "create content about machine learning for developers";

        let params = parser.extract_parameters(definition);

        assert_eq!(
            params.get("topic").unwrap().as_str().unwrap(),
            "machine learning"
        );
        assert_eq!(
            params.get("target_audience").unwrap().as_str().unwrap(),
            "developers"
        );
    }

    #[test]
    fn test_complexity_estimation() {
        let parser = IntentParserService::new();

        let simple_steps = vec![WorkflowStep {
            id: "1".to_string(),
            action: "test".to_string(),
            description: "test".to_string(),
            parameters: HashMap::new(),
            dependencies: vec![],
            estimated_duration_seconds: None,
        }];

        let complexity = parser.estimate_complexity(&simple_steps, &[]);
        assert_eq!(complexity, ComplexityLevel::Simple);

        let complex_integrations = vec![
            "gmail".to_string(),
            "slack".to_string(),
            "twitter".to_string(),
        ];
        let complexity = parser.estimate_complexity(&simple_steps, &complex_integrations);
        assert_eq!(complexity, ComplexityLevel::Moderate);
    }
}
