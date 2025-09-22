use crate::error::{ErrorContext, ParseIntentError, Result};
use crate::llm::LLMClient;
use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct IntentParser {
    llm_client: Arc<LLMClient>,
    function_registry: Arc<FunctionRegistry>,
    user_context_cache: Arc<tokio::sync::RwLock<HashMap<Uuid, UserContext>>>,
    validation_cache: Arc<tokio::sync::RwLock<HashMap<String, ValidationResult>>>,
}

#[derive(Debug, Clone)]
pub struct FunctionRegistry {
    functions: HashMap<String, FunctionInfo>,
    domains: Vec<String>,
    integrations: Vec<String>,
    max_complexity_score: f32,
    max_functions_per_workflow: usize,
}

impl IntentParser {
    pub fn new(llm_client: Arc<LLMClient>) -> Self {
        let function_registry = Arc::new(FunctionRegistry::new());
        let user_context_cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let validation_cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));

        Self {
            llm_client,
            function_registry,
            user_context_cache,
            validation_cache,
        }
    }

    pub async fn parse_request(
        &self,
        request: &ParseIntentRequest,
        user_context: Option<UserContext>,
    ) -> Result<ParsedIntent> {
        info!("Parsing intent request for user: {}", request.user_id);

        // Get or use provided user context
        let context = match user_context {
            Some(ctx) => ctx,
            None => self
                .get_user_context(request.user_id)
                .await?
                .unwrap_or_default(),
        };

        // Validate request
        self.validate_request(request)?;

        // Pre-process the request text
        let _processed_text = self.preprocess_text(&request.text)?;

        // Extract intent using LLM
        let mut parsed_intent = self
            .llm_client
            .parse_intent_with_context(request, Some(context.clone()))
            .await
            .with_context("Failed to parse intent with LLM")?;

        // Post-process and enhance the parsed intent
        self.enhance_parsed_intent(&mut parsed_intent, request, &context)
            .await?;

        // Apply optimizations based on user preferences and constraints
        self.optimize_intent(&mut parsed_intent, request, &context)
            .await?;

        // Update user context with learning
        self.update_user_learning(&context, request, &parsed_intent)
            .await?;

        info!(
            "Successfully parsed intent: {} functions, {:.2} confidence",
            parsed_intent.functions.len(),
            parsed_intent.confidence_score
        );

        Ok(parsed_intent)
    }

    pub async fn validate_intent(&self, intent: &ParsedIntent) -> Result<ValidationResult> {
        debug!("Validating parsed intent: {}", intent.workflow_id);

        // Check cache first
        let cache_key = self.generate_validation_cache_key(intent);
        {
            let cache = self.validation_cache.read().await;
            if let Some(cached_result) = cache.get(&cache_key) {
                debug!("Returning cached validation result");
                return Ok(cached_result.clone());
            }
        }

        // Perform comprehensive validation
        let mut validation_result = ValidationResult {
            is_valid: true,
            confidence_score: intent.confidence_score,
            warnings: Vec::new(),
            suggestions: Vec::new(),
            estimated_execution_time: intent.estimated_duration,
            estimated_cost: intent.estimated_cost,
            missing_permissions: Vec::new(),
            invalid_parameters: Vec::new(),
        };

        // Validate function calls
        for function in &intent.functions {
            self.validate_function_call(function, &mut validation_result)?;
        }

        // Validate dependencies
        self.validate_dependencies(
            &intent.dependencies,
            &intent.functions,
            &mut validation_result,
        )?;

        // Validate workflow complexity
        self.validate_complexity(intent, &mut validation_result)?;

        // Validate resource requirements
        self.validate_resources(intent, &mut validation_result)?;

        // Use LLM for additional validation if needed
        if validation_result.confidence_score < 0.8 {
            let llm_validation = self
                .llm_client
                .validate_parsed_intent(intent)
                .await
                .with_context("LLM validation failed")?;

            validation_result.warnings.extend(llm_validation.warnings);
            validation_result
                .suggestions
                .extend(llm_validation.suggestions);
        }

        // Cache the result
        {
            let mut cache = self.validation_cache.write().await;
            cache.insert(cache_key, validation_result.clone());
        }

        info!(
            "Validation complete: valid={}, confidence={:.2}",
            validation_result.is_valid, validation_result.confidence_score
        );

        Ok(validation_result)
    }

    pub async fn get_available_capabilities(&self) -> Result<AvailableCapabilities> {
        Ok(AvailableCapabilities {
            functions: self.function_registry.functions.values().cloned().collect(),
            domains: self.function_registry.domains.clone(),
            integrations: self.function_registry.integrations.clone(),
            max_complexity_score: self.function_registry.max_complexity_score,
            max_functions_per_workflow: self.function_registry.max_functions_per_workflow,
        })
    }

    pub async fn list_available_functions(
        &self,
        domain_filter: Option<&String>,
    ) -> Result<Vec<FunctionInfo>> {
        let functions: Vec<FunctionInfo> = self
            .function_registry
            .functions
            .values()
            .filter(|f| {
                domain_filter
                    .map(|domain| f.domain == *domain)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();

        debug!(
            "Listed {} functions{}",
            functions.len(),
            domain_filter
                .map(|d| format!(" for domain '{}'", d))
                .unwrap_or_default()
        );

        Ok(functions)
    }

    pub async fn get_function_details(&self, function_id: &str) -> Result<Option<FunctionDetails>> {
        if let Some(function_info) = self.function_registry.functions.get(function_id) {
            Ok(Some(FunctionDetails {
                info: function_info.clone(),
                examples: self.get_function_examples(function_id),
                integration_requirements: self.get_integration_requirements(function_id),
                rate_limits: self.get_rate_limits(function_id),
                documentation_url: Some(format!(
                    "https://docs.ai-core.com/functions/{}",
                    function_id
                )),
                changelog: self.get_function_changelog(function_id),
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_user_context(&self, user_id: Uuid) -> Result<Option<UserContext>> {
        let cache = self.user_context_cache.read().await;
        Ok(cache.get(&user_id).cloned())
    }

    pub async fn update_user_context(
        &self,
        user_id: Uuid,
        mut context: UserContext,
    ) -> Result<UserContext> {
        context.user_id = user_id;
        context.last_updated = chrono::Utc::now();

        let mut cache = self.user_context_cache.write().await;
        cache.insert(user_id, context.clone());

        info!("Updated user context for user: {}", user_id);
        Ok(context)
    }

    // Private helper methods

    fn validate_request(&self, request: &ParseIntentRequest) -> Result<()> {
        if request.text.trim().is_empty() {
            return Err(ParseIntentError::InvalidFormat(
                "Request text cannot be empty".to_string(),
            )
            .into());
        }

        if request.text.len() > 10000 {
            return Err(ParseIntentError::InvalidFormat(
                "Request text too long (max 10000 characters)".to_string(),
            )
            .into());
        }

        // Validate budget constraints
        if let Some(budget) = request.budget_limit {
            if budget <= 0.0 {
                return Err(ParseIntentError::InvalidParameter(
                    "Budget limit must be positive".to_string(),
                )
                .into());
            }
        }

        // Validate time constraints
        if let Some(time_limit) = request.time_limit {
            if time_limit <= chrono::Duration::zero() {
                return Err(ParseIntentError::InvalidParameter(
                    "Time limit must be positive".to_string(),
                )
                .into());
            }
        }

        // Validate quality threshold
        if let Some(quality) = request.quality_threshold {
            if quality < 0.0 || quality > 1.0 {
                return Err(ParseIntentError::InvalidParameter(
                    "Quality threshold must be between 0.0 and 1.0".to_string(),
                )
                .into());
            }
        }

        Ok(())
    }

    fn preprocess_text(&self, text: &str) -> Result<String> {
        let mut processed = text.trim().to_string();

        // Normalize whitespace
        processed = processed.split_whitespace().collect::<Vec<_>>().join(" ");

        // Remove potential sensitive information patterns
        processed = self.sanitize_sensitive_data(&processed);

        // Extract and normalize URLs
        processed = self.normalize_urls(&processed);

        // Extract mentions and hashtags for context
        processed = self.extract_social_context(&processed);

        debug!("Preprocessed text: {} -> {}", text.len(), processed.len());
        Ok(processed)
    }

    fn sanitize_sensitive_data(&self, text: &str) -> String {
        let mut sanitized = text.to_string();

        // Basic patterns for common sensitive data
        let sensitive_patterns = vec![
            (
                regex::Regex::new(r"\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b").unwrap(),
                "[CARD_NUMBER]",
            ),
            (
                regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
                "[SSN]",
            ),
            (
                regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
                "[EMAIL]",
            ),
            (
                regex::Regex::new(
                    r"\b(?:\+?1[-.\s]?)?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b",
                )
                .unwrap(),
                "[PHONE]",
            ),
        ];

        for (pattern, replacement) in sensitive_patterns {
            sanitized = pattern.replace_all(&sanitized, replacement).to_string();
        }

        sanitized
    }

    fn normalize_urls(&self, text: &str) -> String {
        let url_pattern = regex::Regex::new(r"https?://[^\s]+").unwrap();
        url_pattern.replace_all(text, "[URL]").to_string()
    }

    fn extract_social_context(&self, text: &str) -> String {
        let mut context_enhanced = text.to_string();

        // Extract hashtags
        let hashtag_pattern = regex::Regex::new(r"#\w+").unwrap();
        let hashtags: Vec<&str> = hashtag_pattern
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        if !hashtags.is_empty() {
            context_enhanced.push_str(&format!(" [HASHTAGS: {}]", hashtags.join(", ")));
        }

        // Extract mentions
        let mention_pattern = regex::Regex::new(r"@\w+").unwrap();
        let mentions: Vec<&str> = mention_pattern
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        if !mentions.is_empty() {
            context_enhanced.push_str(&format!(" [MENTIONS: {}]", mentions.join(", ")));
        }

        context_enhanced
    }

    async fn enhance_parsed_intent(
        &self,
        intent: &mut ParsedIntent,
        request: &ParseIntentRequest,
        context: &UserContext,
    ) -> Result<()> {
        debug!("Enhancing parsed intent with context and optimizations");

        // Enhance function calls with user preferences
        for function in &mut intent.functions {
            self.enhance_function_call(function, context)?;
        }

        // Add missing dependencies based on function requirements
        self.discover_dependencies(intent)?;

        // Enhance scheduling requirements based on user preferences
        self.enhance_scheduling_requirements(intent, context)?;

        // Add provider preferences based on user history
        self.add_provider_preferences(intent, context)?;

        // Update metadata with enhanced information
        self.update_intent_metadata(intent, request, context)?;

        Ok(())
    }

    fn enhance_function_call(
        &self,
        function: &mut FunctionCall,
        context: &UserContext,
    ) -> Result<()> {
        // Apply user preferences to function parameters
        if let Some(preferred_provider) =
            context.preferences.preferred_providers.get(&function.name)
        {
            function.provider = preferred_provider.clone();
        }

        // Adjust cost estimates based on user's subscription tier
        function.estimated_cost =
            self.adjust_cost_for_tier(function.estimated_cost, &context.subscription_tier);

        // Add user-specific parameters
        if let Ok(mut params) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(
            function.parameters.clone(),
        ) {
            // Add timezone if not specified
            if !params.contains_key("timezone") && !context.preferences.default_timezone.is_empty()
            {
                params.insert(
                    "timezone".to_string(),
                    serde_json::Value::String(context.preferences.default_timezone.clone()),
                );
            }

            // Add language preference
            if !params.contains_key("language") && !context.preferences.language.is_empty() {
                params.insert(
                    "language".to_string(),
                    serde_json::Value::String(context.preferences.language.clone()),
                );
            }

            function.parameters = serde_json::Value::Object(params);
        }

        Ok(())
    }

    fn discover_dependencies(&self, intent: &mut ParsedIntent) -> Result<()> {
        let mut new_dependencies = Vec::new();

        // Analyze function calls to discover implicit dependencies
        for i in 0..intent.functions.len() {
            for j in (i + 1)..intent.functions.len() {
                let func_a = &intent.functions[i];
                let func_b = &intent.functions[j];

                // Check if func_b requires output from func_a
                if self.requires_dependency(&func_a.name, &func_b.name) {
                    new_dependencies.push(Dependency {
                        prerequisite: func_a.id.to_string(),
                        dependent: func_b.id.to_string(),
                        data_transfer: self.get_data_transfer_key(&func_a.name, &func_b.name),
                        dependency_type: DependencyType::DataDependency,
                    });
                }
            }
        }

        intent.dependencies.extend(new_dependencies);
        Ok(())
    }

    fn requires_dependency(&self, func_a: &str, func_b: &str) -> bool {
        // Define dependency rules between functions
        match (func_a, func_b) {
            ("create_content_workflow", "schedule_content_campaign") => true,
            ("create_landing_page", "setup_marketing_campaign") => true,
            ("setup_marketing_campaign", "schedule_content_campaign") => true,
            _ => false,
        }
    }

    fn get_data_transfer_key(&self, func_a: &str, func_b: &str) -> Option<String> {
        match (func_a, func_b) {
            ("create_content_workflow", "schedule_content_campaign") => {
                Some("content_items".to_string())
            }
            ("create_landing_page", "setup_marketing_campaign") => {
                Some("landing_page_url".to_string())
            }
            _ => None,
        }
    }

    fn enhance_scheduling_requirements(
        &self,
        intent: &mut ParsedIntent,
        context: &UserContext,
    ) -> Result<()> {
        if let Some(ref mut scheduling) = intent.scheduling_requirements {
            // Set default timezone from user preferences
            if scheduling.timezone.is_none() && !context.preferences.default_timezone.is_empty() {
                scheduling.timezone = Some(context.preferences.default_timezone.clone());
            }

            // Adjust time sensitivity based on user's speed preference
            if scheduling.time_sensitivity.is_none() {
                scheduling.time_sensitivity = Some(if context.preferences.speed_preference > 0.7 {
                    TimeSensitivity::High
                } else if context.preferences.speed_preference > 0.3 {
                    TimeSensitivity::Medium
                } else {
                    TimeSensitivity::Low
                });
            }
        }

        Ok(())
    }

    fn add_provider_preferences(
        &self,
        intent: &mut ParsedIntent,
        context: &UserContext,
    ) -> Result<()> {
        // Create provider preferences based on user's usage statistics
        for domain in &context.usage_statistics.most_used_domains {
            if let Some(preferred_provider) = context.preferences.preferred_providers.get(domain) {
                intent.provider_preferences.push(ProviderPreference {
                    domain: domain.clone(),
                    preferred_providers: vec![preferred_provider.clone()],
                    fallback_providers: vec!["default".to_string()],
                    cost_weight: context.preferences.cost_sensitivity,
                    quality_weight: 1.0 - context.preferences.cost_sensitivity,
                    speed_weight: context.preferences.speed_preference,
                });
            }
        }

        Ok(())
    }

    fn update_intent_metadata(
        &self,
        intent: &mut ParsedIntent,
        request: &ParseIntentRequest,
        context: &UserContext,
    ) -> Result<()> {
        // Update complexity score based on function count and types
        intent.metadata.complexity_score = self.calculate_complexity_score(&intent.functions);

        // Add domain scores
        intent.metadata.domain_scores = self.calculate_domain_scores(&intent.functions);

        // Add user preferences
        intent.metadata.user_preferences = Some(context.preferences.clone());

        // Add context variables
        if let Some(context_data) = &request.context {
            intent
                .metadata
                .context_variables
                .insert("request_context".to_string(), context_data.clone());
        }

        Ok(())
    }

    fn calculate_complexity_score(&self, functions: &[FunctionCall]) -> f32 {
        let base_complexity = functions.len() as f32 * 0.1;
        let function_complexity: f32 = functions
            .iter()
            .map(|f| {
                self.function_registry
                    .functions
                    .get(&f.name)
                    .map(|info| info.complexity_score)
                    .unwrap_or(0.5)
            })
            .sum();

        (base_complexity + function_complexity).min(1.0)
    }

    fn calculate_domain_scores(&self, functions: &[FunctionCall]) -> HashMap<String, f32> {
        let mut domain_scores = HashMap::new();
        let total_functions = functions.len() as f32;

        if total_functions == 0.0 {
            return domain_scores;
        }

        for function in functions {
            if let Some(function_info) = self.function_registry.functions.get(&function.name) {
                let entry = domain_scores
                    .entry(function_info.domain.clone())
                    .or_insert(0.0);
                *entry += 1.0 / total_functions;
            }
        }

        domain_scores
    }

    async fn optimize_intent(
        &self,
        intent: &mut ParsedIntent,
        request: &ParseIntentRequest,
        context: &UserContext,
    ) -> Result<()> {
        debug!("Optimizing parsed intent based on constraints and preferences");

        // Apply budget constraints
        if let Some(budget_limit) = request.budget_limit {
            self.optimize_for_budget(intent, budget_limit)?;
        }

        // Apply time constraints
        if let Some(time_limit) = request.time_limit {
            self.optimize_for_time(intent, time_limit)?;
        }

        // Apply quality constraints
        if let Some(quality_threshold) = request.quality_threshold {
            self.optimize_for_quality(intent, quality_threshold)?;
        }

        // Optimize based on user preferences
        self.optimize_for_user_preferences(intent, context)?;

        Ok(())
    }

    fn optimize_for_budget(&self, intent: &mut ParsedIntent, budget_limit: f64) -> Result<()> {
        if intent.estimated_cost <= budget_limit {
            return Ok(());
        }

        warn!(
            "Intent cost ({:.2}) exceeds budget limit ({:.2}), optimizing...",
            intent.estimated_cost, budget_limit
        );

        // Sort functions by cost-effectiveness (value/cost ratio)
        intent.functions.sort_by(|a, b| {
            let ratio_a = a.confidence_score / (a.estimated_cost.max(0.01) as f32);
            let ratio_b = b.confidence_score / (b.estimated_cost.max(0.01) as f32);
            ratio_b
                .partial_cmp(&ratio_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Remove functions until we're under budget
        let mut total_cost = 0.0;
        intent.functions.retain(|f| {
            if total_cost + f.estimated_cost <= budget_limit {
                total_cost += f.estimated_cost;
                true
            } else {
                false
            }
        });

        intent.estimated_cost = total_cost;

        info!(
            "Budget optimization complete: {:.2} cost, {} functions",
            intent.estimated_cost,
            intent.functions.len()
        );

        Ok(())
    }

    fn optimize_for_time(
        &self,
        intent: &mut ParsedIntent,
        time_limit: chrono::Duration,
    ) -> Result<()> {
        if intent.estimated_duration <= time_limit {
            return Ok(());
        }

        warn!(
            "Intent duration ({} min) exceeds time limit ({} min), optimizing...",
            intent.estimated_duration.num_minutes(),
            time_limit.num_minutes()
        );

        // Sort functions by time efficiency
        intent.functions.sort_by(|a, b| {
            let efficiency_a =
                a.confidence_score / a.estimated_duration.num_seconds().max(1) as f32;
            let efficiency_b =
                b.confidence_score / b.estimated_duration.num_seconds().max(1) as f32;
            efficiency_b
                .partial_cmp(&efficiency_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Select functions that fit within time limit
        let mut total_duration = chrono::Duration::zero();
        intent.functions.retain(|f| {
            if total_duration + f.estimated_duration <= time_limit {
                total_duration = total_duration + f.estimated_duration;
                true
            } else {
                false
            }
        });

        intent.estimated_duration = total_duration;

        info!(
            "Time optimization complete: {} min, {} functions",
            intent.estimated_duration.num_minutes(),
            intent.functions.len()
        );

        Ok(())
    }

    fn optimize_for_quality(
        &self,
        intent: &mut ParsedIntent,
        quality_threshold: f32,
    ) -> Result<()> {
        let current_avg_quality = intent
            .functions
            .iter()
            .map(|f| f.confidence_score)
            .sum::<f32>()
            / intent.functions.len().max(1) as f32;

        if current_avg_quality >= quality_threshold {
            return Ok(());
        }

        warn!(
            "Intent quality ({:.2}) below threshold ({:.2}), filtering low-quality functions",
            current_avg_quality, quality_threshold
        );

        // Remove functions below quality threshold
        intent
            .functions
            .retain(|f| f.confidence_score >= quality_threshold);

        // Update overall confidence score
        if !intent.functions.is_empty() {
            intent.confidence_score = intent
                .functions
                .iter()
                .map(|f| f.confidence_score)
                .sum::<f32>()
                / intent.functions.len() as f32;
        }

        info!(
            "Quality optimization complete: {:.2} avg confidence, {} functions",
            intent.confidence_score,
            intent.functions.len()
        );

        Ok(())
    }

    fn optimize_for_user_preferences(
        &self,
        intent: &mut ParsedIntent,
        context: &UserContext,
    ) -> Result<()> {
        // Adjust provider selection based on user preferences
        for function in &mut intent.functions {
            if let Some(preferred) = context.preferences.preferred_providers.get(&function.name) {
                function.provider = preferred.clone();
            }
        }

        // Optimize based on cost sensitivity
        if context.preferences.cost_sensitivity > 0.7 {
            // High cost sensitivity - prefer cheaper options
            intent.functions.sort_by(|a, b| {
                a.estimated_cost
                    .partial_cmp(&b.estimated_cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Optimize based on speed preference
        if context.preferences.speed_preference > 0.7 {
            // High speed preference - prefer faster functions
            intent
                .functions
                .sort_by(|a, b| a.estimated_duration.cmp(&b.estimated_duration));
        }

        Ok(())
    }

    async fn update_user_learning(
        &self,
        context: &UserContext,
        request: &ParseIntentRequest,
        intent: &ParsedIntent,
    ) -> Result<()> {
        let mut updated_context = context.clone();

        // Update usage statistics
        updated_context.usage_statistics.total_intents_parsed += 1;

        // Update favorite functions
        for function in &intent.functions {
            if !updated_context
                .usage_statistics
                .favorite_functions
                .contains(&function.name)
            {
                updated_context
                    .usage_statistics
                    .favorite_functions
                    .push(function.name.clone());
            }
        }

        // Keep only top 10 favorite functions
        updated_context
            .usage_statistics
            .favorite_functions
            .truncate(10);

        // Update domain usage
        let domain_scores = self.calculate_domain_scores(&intent.functions);
        for domain in domain_scores.keys() {
            if !updated_context
                .usage_statistics
                .most_used_domains
                .contains(domain)
            {
                updated_context
                    .usage_statistics
                    .most_used_domains
                    .push(domain.clone());
            }
        }
        updated_context
            .usage_statistics
            .most_used_domains
            .truncate(10);

        // Update average confidence score
        let total_parses = updated_context.usage_statistics.total_intents_parsed as f32;
        let current_avg = updated_context.usage_statistics.average_confidence_score;
        updated_context.usage_statistics.average_confidence_score =
            (current_avg * (total_parses - 1.0) + intent.confidence_score) / total_parses;

        // Store updated context
        self.update_user_context(request.user_id, updated_context)
            .await?;

        Ok(())
    }

    fn validate_function_call(
        &self,
        function: &FunctionCall,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Check if function exists in registry
        if let Some(function_info) = self.function_registry.functions.get(&function.name) {
            // Validate parameters against function schema
            if let Err(validation_errors) =
                self.validate_function_parameters(function, function_info)
            {
                result.invalid_parameters.extend(validation_errors);
                result.is_valid = false;
            }

            // Check required permissions
            for permission in &function.required_permissions {
                if !self.check_permission_available(permission) {
                    result.missing_permissions.push(permission.clone());
                    result.is_valid = false;
                }
            }

            // Validate cost estimate
            if function.estimated_cost < function_info.cost_range.min_cost {
                result.warnings.push(format!(
                    "Function '{}' cost estimate ({:.2}) below minimum ({:.2})",
                    function.name, function.estimated_cost, function_info.cost_range.min_cost
                ));
            }
        } else {
            result.is_valid = false;
            result
                .warnings
                .push(format!("Unknown function: {}", function.name));
        }

        Ok(())
    }

    fn validate_function_parameters(
        &self,
        function: &FunctionCall,
        function_info: &FunctionInfo,
    ) -> std::result::Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Basic parameter validation - in a real implementation,
        // you'd use JSON Schema validation here
        if let Ok(params) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(
            function.parameters.clone(),
        ) {
            for param_info in &function_info.supported_parameters {
                if param_info.required && !params.contains_key(&param_info.name) {
                    errors.push(format!("Missing required parameter: {}", param_info.name));
                }
            }
        } else {
            errors.push("Invalid parameter format".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn check_permission_available(&self, _permission: &str) -> bool {
        // In a real implementation, this would check against user's actual permissions
        true
    }

    fn validate_dependencies(
        &self,
        dependencies: &[Dependency],
        functions: &[FunctionCall],
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Check for circular dependencies
        if self.has_circular_dependencies(dependencies) {
            result.is_valid = false;
            result
                .warnings
                .push("Circular dependencies detected".to_string());
        }

        // Check that all dependency references are valid
        let function_ids: std::collections::HashSet<String> =
            functions.iter().map(|f| f.id.to_string()).collect();

        for dep in dependencies {
            if !function_ids.contains(&dep.prerequisite) || !function_ids.contains(&dep.dependent) {
                result.is_valid = false;
                result.warnings.push(format!(
                    "Invalid dependency reference: {} -> {}",
                    dep.prerequisite, dep.dependent
                ));
            }
        }

        Ok(())
    }

    fn has_circular_dependencies(&self, dependencies: &[Dependency]) -> bool {
        // Simple cycle detection using DFS
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for dep in dependencies {
            graph
                .entry(dep.prerequisite.clone())
                .or_default()
                .push(dep.dependent.clone());
        }

        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                if self.dfs_has_cycle(node, &graph, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }

    fn dfs_has_cycle(
        &self,
        node: &str,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.dfs_has_cycle(neighbor, graph, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    fn validate_complexity(
        &self,
        intent: &ParsedIntent,
        result: &mut ValidationResult,
    ) -> Result<()> {
        if intent.functions.len() > self.function_registry.max_functions_per_workflow {
            result.warnings.push(format!(
                "Workflow has {} functions, maximum recommended is {}",
                intent.functions.len(),
                self.function_registry.max_functions_per_workflow
            ));
        }

        if intent.metadata.complexity_score > self.function_registry.max_complexity_score {
            result.warnings.push(format!(
                "Workflow complexity score ({:.2}) exceeds recommended maximum ({:.2})",
                intent.metadata.complexity_score, self.function_registry.max_complexity_score
            ));
        }

        Ok(())
    }

    fn validate_resources(
        &self,
        intent: &ParsedIntent,
        result: &mut ValidationResult,
    ) -> Result<()> {
        // Validate total estimated cost
        if intent.estimated_cost > 1000.0 {
            result.warnings.push(format!(
                "High estimated cost: ${:.2}",
                intent.estimated_cost
            ));
        }

        // Validate total estimated duration
        if intent.estimated_duration > chrono::Duration::hours(24) {
            result.warnings.push(format!(
                "Long estimated duration: {} hours",
                intent.estimated_duration.num_hours()
            ));
        }

        Ok(())
    }

    fn adjust_cost_for_tier(&self, base_cost: f64, tier: &SubscriptionTier) -> f64 {
        match tier {
            SubscriptionTier::Free => base_cost * 1.5, // Higher cost for free tier
            SubscriptionTier::Basic => base_cost * 1.2,
            SubscriptionTier::Professional => base_cost * 1.0,
            SubscriptionTier::Enterprise => base_cost * 0.8, // Discount for enterprise
            SubscriptionTier::Custom(_) => base_cost * 0.9,
        }
    }

    fn generate_validation_cache_key(&self, intent: &ParsedIntent) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        intent.workflow_id.hash(&mut hasher);
        intent.functions.len().hash(&mut hasher);
        intent.estimated_cost.to_bits().hash(&mut hasher);

        format!("validation_{:x}", hasher.finish())
    }

    // Helper methods for function details

    fn get_function_examples(&self, function_id: &str) -> Vec<FunctionExample> {
        // In a real implementation, these would come from a database
        match function_id {
            "create_content_workflow" => vec![FunctionExample {
                name: "Blog Post Creation".to_string(),
                description: "Generate a technical blog post with SEO optimization".to_string(),
                input: serde_json::json!({
                    "content_types": ["blog"],
                    "quantity": 1,
                    "topic": "AI in Modern Web Development",
                    "seo_keywords": ["artificial intelligence", "web development", "automation"],
                    "target_audience": "developers"
                }),
                expected_output: serde_json::json!({
                    "content_items": [{
                        "type": "blog",
                        "title": "How AI is Revolutionizing Modern Web Development",
                        "content": "[Generated blog content...]",
                        "meta_description": "Discover how AI is transforming web development...",
                        "seo_score": 85
                    }]
                }),
                notes: Some(
                    "Includes automatic SEO optimization and readability scoring".to_string(),
                ),
            }],
            _ => vec![],
        }
    }

    fn get_integration_requirements(&self, function_id: &str) -> Vec<String> {
        match function_id {
            "create_content_workflow" => vec!["openai".to_string(), "unsplash".to_string()],
            "setup_marketing_campaign" => vec!["facebook".to_string(), "google_ads".to_string()],
            "schedule_content_campaign" => vec!["buffer".to_string(), "hootsuite".to_string()],
            _ => vec![],
        }
    }

    fn get_rate_limits(&self, function_id: &str) -> Option<RateLimit> {
        match function_id {
            "create_content_workflow" => Some(RateLimit {
                requests_per_minute: 10,
                requests_per_hour: 100,
                requests_per_day: 1000,
                burst_limit: 20,
            }),
            _ => None,
        }
    }

    fn get_function_changelog(&self, function_id: &str) -> Vec<ChangelogEntry> {
        match function_id {
            "create_content_workflow" => vec![
                ChangelogEntry {
                    version: "1.2.0".to_string(),
                    date: chrono::Utc::now() - chrono::Duration::days(30),
                    changes: vec![
                        "Added support for video content generation".to_string(),
                        "Improved SEO keyword optimization".to_string(),
                    ],
                    breaking_changes: vec![],
                },
                ChangelogEntry {
                    version: "1.1.0".to_string(),
                    date: chrono::Utc::now() - chrono::Duration::days(60),
                    changes: vec![
                        "Added multi-language support".to_string(),
                        "Enhanced content quality scoring".to_string(),
                    ],
                    breaking_changes: vec!["Changed response format for content_items".to_string()],
                },
            ],
            _ => vec![],
        }
    }
}

impl FunctionRegistry {
    fn new() -> Self {
        let mut functions = HashMap::new();

        // Add built-in functions - in a real system these would come from a database
        functions.insert("create_content_workflow".to_string(), FunctionInfo {
            id: "create_content_workflow".to_string(),
            name: "Create Content Workflow".to_string(),
            description: "Generate various types of content including blogs, videos, images, and social media posts".to_string(),
            domain: "content_creation".to_string(),
            cost_range: CostRange {
                min_cost: 0.5,
                max_cost: 10.0,
                average_cost: 2.5,
                currency: "USD".to_string(),
            },
            estimated_duration: chrono::Duration::minutes(45),
            complexity_score: 0.7,
            popularity_score: 0.9,
            success_rate: 0.95,
            required_permissions: vec!["content.create".to_string(), "content.publish".to_string()],
            supported_parameters: vec![
                ParameterInfo {
                    name: "content_types".to_string(),
                    parameter_type: "array".to_string(),
                    required: true,
                    description: "Types of content to generate".to_string(),
                    default_value: None,
                    validation_rules: vec!["Must contain at least one type".to_string()],
                },
                ParameterInfo {
                    name: "quantity".to_string(),
                    parameter_type: "integer".to_string(),
                    required: true,
                    description: "Number of content items to generate".to_string(),
                    default_value: Some(serde_json::Value::Number(1.into())),
                    validation_rules: vec!["Must be between 1 and 100".to_string()],
                }
            ],
        });

        functions.insert("setup_marketing_campaign".to_string(), FunctionInfo {
            id: "setup_marketing_campaign".to_string(),
            name: "Setup Marketing Campaign".to_string(),
            description: "Create comprehensive marketing campaigns across multiple channels".to_string(),
            domain: "marketing".to_string(),
            cost_range: CostRange {
                min_cost: 2.0,
                max_cost: 50.0,
                average_cost: 15.0,
                currency: "USD".to_string(),
            },
            estimated_duration: chrono::Duration::hours(2),
            complexity_score: 0.8,
            popularity_score: 0.8,
            success_rate: 0.92,
            required_permissions: vec!["marketing.create".to_string(), "ads.manage".to_string()],
            supported_parameters: vec![
                ParameterInfo {
                    name: "campaign_type".to_string(),
                    parameter_type: "enum".to_string(),
                    required: true,
                    description: "Type of marketing campaign".to_string(),
                    default_value: None,
                    validation_rules: vec!["Must be one of: product_launch, brand_awareness, lead_generation, retargeting".to_string()],
                }
            ],
        });

        Self {
            functions,
            domains: vec![
                "content_creation".to_string(),
                "marketing".to_string(),
                "scheduling".to_string(),
                "analytics".to_string(),
                "ecommerce".to_string(),
                "communication".to_string(),
            ],
            integrations: vec![
                "openai".to_string(),
                "facebook".to_string(),
                "twitter".to_string(),
                "linkedin".to_string(),
                "instagram".to_string(),
                "youtube".to_string(),
                "shopify".to_string(),
                "mailchimp".to_string(),
            ],
            max_complexity_score: 1.0,
            max_functions_per_workflow: 20,
        }
    }
}
