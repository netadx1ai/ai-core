use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::*;
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct LLMClient {
    client: Client,
    config: LLMConfig,
    provider: LLMProvider,
}

#[derive(Clone)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Clone)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Ollama,
    AzureOpenAI,
    Gemini,
}

#[derive(Debug, Serialize)]
pub struct LLMRequest {
    pub messages: Vec<LLMMessage>,
    pub functions: Option<Vec<LLMFunction>>,
    pub function_call: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct LLMFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Deserialize)]
pub struct LLMResponse {
    pub choices: Vec<LLMChoice>,
    pub usage: Option<LLMUsage>,
}

#[derive(Debug, Deserialize)]
pub struct LLMChoice {
    pub message: LLMResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LLMResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub function_call: Option<LLMFunctionCall>,
}

#[derive(Debug, Deserialize)]
pub struct LLMFunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
pub struct LLMUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl LLMClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let provider = match config.llm.provider.as_str() {
            "openai" => LLMProvider::OpenAI,
            "anthropic" => LLMProvider::Anthropic,
            "ollama" => LLMProvider::Ollama,
            "azure" => LLMProvider::AzureOpenAI,
            "gemini" => LLMProvider::Gemini,
            _ => {
                return Err(AppError::ConfigurationError(format!(
                    "Unsupported LLM provider: {}",
                    config.llm.provider
                )))
            }
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(config.llm.timeout_seconds))
            .build()
            .map_err(|e| {
                AppError::InternalServerError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            config: LLMConfig {
                provider: config.llm.provider.clone(),
                api_key: config.llm.api_key.clone(),
                api_url: config.llm.api_url.clone(),
                model: config.llm.model.clone(),
                max_tokens: config.llm.max_tokens,
                temperature: config.llm.temperature,
                timeout_seconds: config.llm.timeout_seconds,
                max_retries: config.llm.max_retries,
            },
            provider,
        })
    }

    pub async fn parse_intent_with_context(
        &self,
        request: &ParseIntentRequest,
        user_context: Option<UserContext>,
    ) -> Result<ParsedIntent> {
        info!("Parsing intent with LLM for user: {}", request.user_id);

        let system_prompt = self.build_system_prompt(&user_context);
        let user_prompt = self.build_user_prompt(request);
        let functions = self.get_available_functions();

        let llm_request = LLMRequest {
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: user_prompt,
                },
            ],
            functions: Some(functions),
            function_call: Some("auto".to_string()),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        let response = self.send_request(llm_request).await?;
        self.parse_llm_response(response, request).await
    }

    pub async fn validate_parsed_intent(&self, intent: &ParsedIntent) -> Result<ValidationResult> {
        info!(
            "Validating parsed intent with {} functions",
            intent.functions.len()
        );

        let validation_prompt = self.build_validation_prompt(intent);

        let llm_request = LLMRequest {
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: "You are an expert workflow validator. Analyze the provided workflow for correctness, efficiency, and potential issues. Provide detailed feedback.".to_string(),
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: validation_prompt,
                },
            ],
            functions: None,
            function_call: None,
            temperature: 0.1,
            max_tokens: 2000,
        };

        let response = self.send_request(llm_request).await?;
        self.parse_validation_response(response).await
    }

    pub async fn health_check(&self) -> Result<()> {
        debug!("Performing LLM health check");

        let simple_request = LLMRequest {
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "Respond with 'OK' if you can process this message.".to_string(),
            }],
            functions: None,
            function_call: None,
            temperature: 0.0,
            max_tokens: 10,
        };

        match self.send_request(simple_request).await {
            Ok(_) => {
                debug!("LLM health check successful");
                Ok(())
            }
            Err(e) => {
                error!("LLM health check failed: {:?}", e);
                Err(e)
            }
        }
    }

    async fn send_request(&self, request: LLMRequest) -> Result<LLMResponse> {
        let mut attempts = 0;
        let max_retries = self.config.max_retries;

        while attempts <= max_retries {
            match self.send_request_once(&request).await {
                Ok(response) => return Ok(response),
                Err(e) if attempts < max_retries => {
                    attempts += 1;
                    let delay = Duration::from_millis(1000 * (2_u64.pow(attempts - 1)));
                    warn!(
                        "LLM request failed (attempt {}/{}), retrying in {:?}: {:?}",
                        attempts,
                        max_retries + 1,
                        delay,
                        e
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => return Err(e),
            }
        }

        Err(AppError::InternalServerError(
            "Maximum retry attempts exceeded".to_string(),
        ))
    }

    async fn send_request_once(&self, request: &LLMRequest) -> Result<LLMResponse> {
        let req_builder = match &self.provider {
            LLMProvider::OpenAI => self.build_openai_request(request)?,
            LLMProvider::Anthropic => self.build_anthropic_request(request)?,
            LLMProvider::Ollama => self.build_ollama_request(request)?,
            LLMProvider::AzureOpenAI => self.build_azure_request(request)?,
            LLMProvider::Gemini => self.build_gemini_request(request)?,
        };

        let response = req_builder.send().await.map_err(|e| {
            AppError::ExternalServiceError(format!("LLM API request failed: {}", e))
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalServiceError(format!(
                "LLM API returned error {}: {}",
                status, error_text
            )));
        }

        let llm_response: LLMResponse = response.json().await.map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to parse LLM response: {}", e))
        })?;

        Ok(llm_response)
    }

    fn build_openai_request(&self, request: &LLMRequest) -> Result<RequestBuilder> {
        let mut body = json!({
            "model": self.config.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });

        if let Some(functions) = &request.functions {
            body["functions"] = json!(functions);
            if let Some(function_call) = &request.function_call {
                body["function_call"] = json!(function_call);
            }
        }

        Ok(self
            .client
            .post(&self.config.api_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body))
    }

    fn build_anthropic_request(&self, request: &LLMRequest) -> Result<RequestBuilder> {
        let mut messages = request.messages.clone();
        let system_message = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        // Remove system message from messages for Anthropic
        messages.retain(|m| m.role != "system");

        let mut body = json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        if let Some(system) = system_message {
            body["system"] = json!(system);
        }

        // Anthropic uses tools instead of functions
        if let Some(functions) = &request.functions {
            let tools: Vec<Value> = functions
                .iter()
                .map(|f| {
                    json!({
                        "name": f.name,
                        "description": f.description,
                        "input_schema": f.parameters
                    })
                })
                .collect();

            body["tools"] = json!(tools);
            if request.function_call.is_some() {
                body["tool_choice"] = json!({"type": "auto"});
            }
        }

        Ok(self
            .client
            .post(&self.config.api_url)
            .header("x-api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&body))
    }

    fn build_ollama_request(&self, request: &LLMRequest) -> Result<RequestBuilder> {
        let body = json!({
            "model": self.config.model,
            "messages": request.messages,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
            },
            "stream": false,
        });

        Ok(self
            .client
            .post(&self.config.api_url)
            .header("Content-Type", "application/json")
            .json(&body))
    }

    fn build_azure_request(&self, request: &LLMRequest) -> Result<RequestBuilder> {
        let body = json!({
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "functions": request.functions,
            "function_call": request.function_call,
        });

        Ok(self
            .client
            .post(&self.config.api_url)
            .header("api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&body))
    }

    fn build_system_prompt(&self, user_context: &Option<UserContext>) -> String {
        let mut prompt = r#"You are an intelligent automation system parser. Convert natural language requests into structured workflows using the available functions.

Available function categories:
1. CONTENT CREATION: blogs, videos, images, social media posts, infographics
2. MARKETING AUTOMATION: campaigns, ads, email sequences, landing pages, SEO
3. SCHEDULING & PUBLISHING: social media, blogs, email campaigns, content calendars
4. ECOMMERCE OPERATIONS: product management, inventory, pricing, orders
5. BUSINESS INTELLIGENCE: analytics, reporting, dashboards, KPI tracking
6. COMMUNICATION: email automation, notifications, scheduling, CRM
7. CLIENT INTEGRATIONS: federated workflows using client's existing systems

Enhanced capabilities:
- Multi-platform content scheduling with optimal timing
- Cross-domain workflow orchestration
- Client system integration and federation
- Cost optimization and provider selection
- Real-time progress monitoring and notifications

Consider:
1. Task dependencies and optimal execution order
2. Parallel vs sequential execution opportunities
3. Scheduling requirements and platform-specific timing
4. Resource requirements and cost constraints
5. Error handling and rollback strategies
6. Client integration capabilities and preferences
7. Content calendar and publishing workflows
8. Multi-platform optimization and coordination

Response format: Use function calling with the registered functions."#.to_string();

        if let Some(context) = user_context {
            prompt.push_str("\n\nUser Context:\n");
            prompt.push_str(&format!(
                "- Subscription Tier: {:?}\n",
                context.subscription_tier
            ));
            prompt.push_str(&format!(
                "- Preferred Language: {}\n",
                context.preferences.language
            ));
            prompt.push_str(&format!(
                "- Cost Sensitivity: {:.1}/1.0\n",
                context.preferences.cost_sensitivity
            ));
            prompt.push_str(&format!(
                "- Speed Preference: {:.1}/1.0\n",
                context.preferences.speed_preference
            ));
            prompt.push_str(&format!(
                "- Default Timezone: {}\n",
                context.preferences.default_timezone
            ));

            if !context.integrations.is_empty() {
                prompt.push_str("- Available Integrations: ");
                let integration_names: Vec<String> = context
                    .integrations
                    .iter()
                    .map(|i| format!("{} ({})", i.platform, i.integration_type))
                    .collect();
                prompt.push_str(&integration_names.join(", "));
                prompt.push('\n');
            }

            if !context.usage_statistics.favorite_functions.is_empty() {
                prompt.push_str("- Frequently Used Functions: ");
                prompt.push_str(&context.usage_statistics.favorite_functions.join(", "));
                prompt.push('\n');
            }
        }

        prompt
    }

    fn build_user_prompt(&self, request: &ParseIntentRequest) -> String {
        let mut prompt = format!("User Request: {}\n", request.text);

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Additional Context: {}\n", context));
        }

        if let Some(providers) = &request.preferred_providers {
            prompt.push_str(&format!("Preferred Providers: {}\n", providers.join(", ")));
        }

        if let Some(budget) = request.budget_limit {
            prompt.push_str(&format!("Budget Limit: ${:.2}\n", budget));
        }

        if let Some(time_limit) = request.time_limit {
            prompt.push_str(&format!("Time Limit: {} hours\n", time_limit.num_hours()));
        }

        if let Some(quality) = request.quality_threshold {
            prompt.push_str(&format!("Minimum Quality Threshold: {:.1}/1.0\n", quality));
        }

        if let Some(federation) = &request.federation_context {
            if let Some(client_id) = &federation.client_id {
                prompt.push_str(&format!("Client ID: {}\n", client_id));
            }
            if !federation.available_providers.is_empty() {
                prompt.push_str("Available Federated Providers:\n");
                for provider in &federation.available_providers {
                    prompt.push_str(&format!(
                        "- {} ({}): cost=${:.3}, quality={:.2}, response_time={}ms\n",
                        provider.provider_name,
                        provider.capabilities.join(", "),
                        provider.cost_per_request.unwrap_or(0.0),
                        provider.quality_score.unwrap_or(0.0),
                        provider.response_time_ms.unwrap_or(0)
                    ));
                }
            }
        }

        prompt.push_str("\nGenerate a structured workflow plan with function calls, dependencies, and execution steps.");

        prompt
    }

    fn build_validation_prompt(&self, intent: &ParsedIntent) -> String {
        format!(
            r#"Validate this parsed workflow intent:

Workflow Type: {:?}
Functions Count: {}
Estimated Duration: {} hours
Estimated Cost: ${:.2}
Confidence Score: {:.2}

Functions:
{}

Dependencies:
{}

Scheduling Requirements: {}

Please analyze:
1. Are all function calls valid and properly configured?
2. Are dependencies correctly identified and realistic?
3. Is the estimated cost and duration reasonable?
4. Are there any missing steps or potential issues?
5. Can the workflow be optimized for better performance?
6. Are there any security or permission concerns?

Provide a structured response with validation results, warnings, and suggestions for improvement."#,
            intent.workflow_type,
            intent.functions.len(),
            intent.estimated_duration.num_hours(),
            intent.estimated_cost,
            intent.confidence_score,
            intent
                .functions
                .iter()
                .map(|f| format!(
                    "- {} ({}): ${:.2}, {}min",
                    f.name,
                    f.provider,
                    f.estimated_cost,
                    f.estimated_duration.num_minutes()
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            intent
                .dependencies
                .iter()
                .map(|d| format!(
                    "- {} -> {} ({:?})",
                    d.prerequisite, d.dependent, d.dependency_type
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            intent
                .scheduling_requirements
                .as_ref()
                .map(|s| format!(
                    "Required ({})",
                    s.schedule_type
                        .as_ref()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|| "None".to_string())
                ))
                .unwrap_or_else(|| "Not Required".to_string())
        )
    }

    fn get_available_functions(&self) -> Vec<LLMFunction> {
        vec![
            LLMFunction {
                name: "create_content_workflow".to_string(),
                description: "Generate blogs, videos, images, social media content with scheduling"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content_types": {
                            "type": "array",
                            "items": {"enum": ["blog", "video", "image", "social_post", "infographic", "carousel", "story", "reel"]}
                        },
                        "quantity": {"type": "integer", "minimum": 1},
                        "topic": {"type": "string"},
                        "brand": {"type": "string"},
                        "target_audience": {"type": "string"},
                        "seo_keywords": {"type": "array", "items": {"type": "string"}},
                        "platforms": {"type": "array", "items": {"type": "string"}},
                        "scheduling": {
                            "type": "object",
                            "properties": {
                                "strategy": {"enum": ["immediate", "optimal_times", "even_distribution", "custom_schedule"]},
                                "time_range": {"type": "object"},
                                "recurring": {"type": "boolean"}
                            }
                        }
                    },
                    "required": ["content_types", "quantity", "topic"]
                }),
            },
            LLMFunction {
                name: "setup_marketing_campaign".to_string(),
                description: "Create comprehensive marketing campaigns with scheduling".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "campaign_type": {"enum": ["product_launch", "brand_awareness", "lead_generation", "retargeting"]},
                        "channels": {
                            "type": "array",
                            "items": {"enum": ["social_media", "email", "ads", "content_marketing", "seo", "landing_pages"]}
                        },
                        "budget": {"type": "number"},
                        "duration": {"type": "string"},
                        "target_demographics": {"type": "object"},
                        "kpis": {"type": "array", "items": {"type": "string"}}
                    },
                    "required": ["campaign_type", "channels"]
                }),
            },
            LLMFunction {
                name: "schedule_content_campaign".to_string(),
                description: "Schedule content across multiple platforms with smart timing"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "campaign_name": {"type": "string"},
                        "content_items": {"type": "array"},
                        "scheduling_strategy": {"enum": ["immediate", "optimal_times", "even_distribution", "custom_schedule", "follower_activity"]},
                        "time_range": {"type": "object"},
                        "publishing_rules": {"type": "object"}
                    },
                    "required": ["campaign_name", "content_items", "scheduling_strategy"]
                }),
            },
            LLMFunction {
                name: "automate_ecommerce_operations".to_string(),
                description: "Manage products, inventory, pricing, orders with scheduling"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "operations": {
                            "type": "array",
                            "items": {"enum": ["product_listing", "inventory_sync", "price_optimization", "order_processing", "marketing_automation"]}
                        },
                        "platforms": {
                            "type": "array",
                            "items": {"enum": ["shopify", "amazon", "woocommerce", "ebay", "etsy"]}
                        }
                    },
                    "required": ["operations", "platforms"]
                }),
            },
            LLMFunction {
                name: "integrate_client_systems".to_string(),
                description: "Integrate and orchestrate workflows with client's existing systems"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "client_systems": {"type": "array", "items": {"type": "string"}},
                        "integration_type": {"enum": ["mcp_server", "rest_api", "webhook"]},
                        "workflow_coordination": {"type": "object"}
                    },
                    "required": ["client_systems", "integration_type"]
                }),
            },
        ]
    }

    fn build_gemini_request(&self, request: &LLMRequest) -> Result<RequestBuilder> {
        let mut body = json!({
            "contents": [{
                "parts": request.messages.iter().map(|msg| {
                    json!({"text": msg.content})
                }).collect::<Vec<_>>()
            }],
            "generationConfig": {
                "temperature": request.temperature,
                "maxOutputTokens": request.max_tokens,
            }
        });

        // Gemini uses tools instead of functions
        if let Some(functions) = &request.functions {
            let function_declarations: Vec<Value> = functions
                .iter()
                .map(|f| {
                    json!({
                        "name": f.name,
                        "description": f.description,
                        "parameters": f.parameters
                    })
                })
                .collect();

            body["tools"] = json!([{
                "functionDeclarations": function_declarations
            }]);

            if request.function_call.is_some() {
                body["toolConfig"] = json!({
                    "functionCallingConfig": {
                        "mode": "ANY"
                    }
                });
            }
        }

        let req_builder = self
            .client
            .post(&self.config.api_url)
            .header("Content-Type", "application/json")
            .header("X-goog-api-key", &self.config.api_key)
            .json(&body);

        Ok(req_builder)
    }

    async fn parse_llm_response(
        &self,
        response: LLMResponse,
        request: &ParseIntentRequest,
    ) -> Result<ParsedIntent> {
        let choice = response.choices.first().ok_or_else(|| {
            AppError::ExternalServiceError("No choices in LLM response".to_string())
        })?;

        if let Some(function_call) = &choice.message.function_call {
            self.parse_function_call_response(function_call, request)
                .await
        } else if let Some(content) = &choice.message.content {
            self.parse_text_response(content, request).await
        } else {
            Err(AppError::ExternalServiceError(
                "Invalid LLM response format".to_string(),
            ))
        }
    }

    async fn parse_function_call_response(
        &self,
        function_call: &LLMFunctionCall,
        _request: &ParseIntentRequest,
    ) -> Result<ParsedIntent> {
        let args: Value = serde_json::from_str(&function_call.arguments).map_err(|e| {
            AppError::ExternalServiceError(format!("Invalid function call arguments: {}", e))
        })?;

        let workflow_id = uuid::Uuid::new_v4();
        let workflow_type = self.determine_workflow_type(&function_call.name);

        let function_call_obj = FunctionCall {
            id: uuid::Uuid::new_v4(),
            name: function_call.name.clone(),
            description: format!("Generated function call for {}", function_call.name),
            parameters: args.clone(),
            provider: "default".to_string(),
            estimated_cost: self.estimate_function_cost(&function_call.name, &args),
            estimated_duration: self.estimate_function_duration(&function_call.name, &args),
            confidence_score: 0.85, // Default confidence
            required_permissions: self.get_required_permissions(&function_call.name),
            mcp_server: self.get_mcp_server(&function_call.name),
        };

        let estimated_duration = function_call_obj.estimated_duration;
        let estimated_cost = function_call_obj.estimated_cost;

        Ok(ParsedIntent {
            workflow_id,
            workflow_type,
            functions: vec![function_call_obj],
            dependencies: vec![],
            estimated_duration,
            estimated_cost,
            confidence_score: 0.85,
            steps: vec![WorkflowStep {
                step_id: uuid::Uuid::new_v4(),
                step_type: StepType::ContentCreation, // Default step type
                name: function_call.name.clone(),
                description: "Auto-generated workflow step".to_string(),
                function_calls: vec![],
                parallel_execution: false,
                retry_policy: None,
                timeout_seconds: Some(3600),
            }],
            required_integrations: self.get_required_integrations(&function_call.name),
            scheduling_requirements: self.extract_scheduling_requirements(&args),
            provider_preferences: vec![],
            metadata: IntentMetadata {
                created_at: chrono::Utc::now(),
                complexity_score: 0.5,
                language: "en".to_string(),
                domain_scores: std::collections::HashMap::new(),
                user_preferences: None,
                context_variables: std::collections::HashMap::new(),
            },
        })
    }

    async fn parse_text_response(
        &self,
        _content: &str,
        _request: &ParseIntentRequest,
    ) -> Result<ParsedIntent> {
        // Fallback text parsing when function calling is not available
        warn!("Using fallback text parsing for LLM response");

        let workflow_id = uuid::Uuid::new_v4();
        let workflow_type = WorkflowType::Custom("parsed_from_text".to_string());

        Ok(ParsedIntent {
            workflow_id,
            workflow_type,
            functions: vec![],
            dependencies: vec![],
            estimated_duration: chrono::Duration::minutes(30),
            estimated_cost: 1.0,
            confidence_score: 0.3, // Low confidence for text parsing
            steps: vec![],
            required_integrations: vec![],
            scheduling_requirements: None,
            provider_preferences: vec![],
            metadata: IntentMetadata {
                created_at: chrono::Utc::now(),
                complexity_score: 0.2,
                language: "en".to_string(),
                domain_scores: std::collections::HashMap::new(),
                user_preferences: None,
                context_variables: std::collections::HashMap::new(),
            },
        })
    }

    async fn parse_validation_response(&self, response: LLMResponse) -> Result<ValidationResult> {
        let choice = response.choices.first().ok_or_else(|| {
            AppError::ExternalServiceError("No choices in validation response".to_string())
        })?;

        let content = choice.message.content.as_ref().ok_or_else(|| {
            AppError::ExternalServiceError("No content in validation response".to_string())
        })?;

        // Parse validation response - this is a simplified implementation
        // In a real system, you'd want more sophisticated parsing
        let is_valid = !content.to_lowercase().contains("invalid")
            && !content.to_lowercase().contains("error");
        let confidence_score = if is_valid { 0.8 } else { 0.3 };

        Ok(ValidationResult {
            is_valid,
            confidence_score,
            warnings: vec![],
            suggestions: vec![],
            estimated_execution_time: chrono::Duration::minutes(60),
            estimated_cost: 5.0,
            missing_permissions: vec![],
            invalid_parameters: vec![],
        })
    }

    fn determine_workflow_type(&self, function_name: &str) -> WorkflowType {
        match function_name {
            "create_content_workflow" => WorkflowType::ContentCreation,
            "setup_marketing_campaign" => WorkflowType::MarketingCampaign,
            "schedule_content_campaign" => WorkflowType::ScheduledPublishing,
            "automate_ecommerce_operations" => WorkflowType::EcommerceOperation,
            "integrate_client_systems" => WorkflowType::ClientIntegration,
            _ => WorkflowType::Custom(function_name.to_string()),
        }
    }

    fn estimate_function_cost(&self, function_name: &str, _args: &Value) -> f64 {
        match function_name {
            "create_content_workflow" => 2.5,
            "setup_marketing_campaign" => 5.0,
            "schedule_content_campaign" => 1.0,
            "automate_ecommerce_operations" => 3.0,
            "integrate_client_systems" => 4.0,
            _ => 1.0,
        }
    }

    fn estimate_function_duration(&self, function_name: &str, _args: &Value) -> chrono::Duration {
        match function_name {
            "create_content_workflow" => chrono::Duration::minutes(45),
            "setup_marketing_campaign" => chrono::Duration::hours(2),
            "schedule_content_campaign" => chrono::Duration::minutes(15),
            "automate_ecommerce_operations" => chrono::Duration::minutes(90),
            "integrate_client_systems" => chrono::Duration::hours(1),
            _ => chrono::Duration::minutes(30),
        }
    }

    fn get_required_permissions(&self, function_name: &str) -> Vec<String> {
        match function_name {
            "create_content_workflow" => {
                vec!["content.create".to_string(), "content.publish".to_string()]
            }
            "setup_marketing_campaign" => {
                vec!["marketing.create".to_string(), "ads.manage".to_string()]
            }
            "schedule_content_campaign" => {
                vec!["schedule.create".to_string(), "social.publish".to_string()]
            }
            "automate_ecommerce_operations" => vec![
                "ecommerce.manage".to_string(),
                "inventory.update".to_string(),
            ],
            "integrate_client_systems" => {
                vec!["integrations.create".to_string(), "api.access".to_string()]
            }
            _ => vec![],
        }
    }

    fn get_mcp_server(&self, function_name: &str) -> Option<String> {
        match function_name {
            "create_content_workflow" => Some("content-mcp-server".to_string()),
            "setup_marketing_campaign" => Some("marketing-mcp-server".to_string()),
            "schedule_content_campaign" => Some("scheduling-mcp-server".to_string()),
            "automate_ecommerce_operations" => Some("ecommerce-mcp-server".to_string()),
            "integrate_client_systems" => Some("integration-mcp-server".to_string()),
            _ => None,
        }
    }

    fn get_required_integrations(&self, function_name: &str) -> Vec<String> {
        match function_name {
            "create_content_workflow" => vec!["openai".to_string(), "unsplash".to_string()],
            "setup_marketing_campaign" => vec![
                "facebook".to_string(),
                "google_ads".to_string(),
                "mailchimp".to_string(),
            ],
            "schedule_content_campaign" => vec!["buffer".to_string(), "hootsuite".to_string()],
            "automate_ecommerce_operations" => vec!["shopify".to_string(), "amazon".to_string()],
            "integrate_client_systems" => vec!["webhook".to_string(), "rest_api".to_string()],
            _ => vec![],
        }
    }

    fn extract_scheduling_requirements(&self, args: &Value) -> Option<SchedulingRequirements> {
        if let Some(scheduling) = args.get("scheduling") {
            Some(SchedulingRequirements {
                needs_scheduling: true,
                schedule_type: scheduling.get("strategy").and_then(|s| s.as_str()).map(
                    |s| match s {
                        "immediate" => ScheduleType::Immediate,
                        "optimal_times" => ScheduleType::OptimalTiming,
                        "even_distribution" => ScheduleType::Delayed,
                        "follower_activity" => ScheduleType::FollowerActivity,
                        _ => ScheduleType::Delayed,
                    },
                ),
                time_sensitivity: Some(TimeSensitivity::Medium),
                platforms: scheduling
                    .get("platforms")
                    .and_then(|p| p.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                content_calendar_integration: true,
                recurring_pattern: scheduling.get("recurring").and_then(|r| {
                    if r.as_bool().unwrap_or(false) {
                        Some(RecurringPattern {
                            frequency: "weekly".to_string(),
                            interval: 1,
                            cron_expression: None,
                            end_date: None,
                            max_occurrences: None,
                        })
                    } else {
                        None
                    }
                }),
                timezone: Some("UTC".to_string()),
            })
        } else {
            None
        }
    }
}
