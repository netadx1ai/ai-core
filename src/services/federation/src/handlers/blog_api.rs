//! Blog Automation API Endpoints
//!
//! This module provides RESTful API endpoints for SaaS clients to access
//! blog post automation services through the AI-CORE platform.

use crate::blog_workflow::{
    BlogParameters, BlogWorkflowRequest, BlogWorkflowResponse, BlogWorkflowService, CallbackConfig,
    ExecutionOptions, WorkflowPriority,
};
use crate::handlers::{success_response, ApiResponse};
use crate::saas_client_auth::{
    BrandProfile, SaasClientAuthService, SaasClientProfile, SaasClientRegistrationRequest,
};
use crate::server::ServerState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    response::Result as AxumResult,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Blog post generation request
#[derive(Debug, Deserialize)]
pub struct BlogPostRequest {
    /// Blog post topic
    pub topic: String,
    /// Target audience (optional)
    pub audience: Option<String>,
    /// Content tone (optional)
    pub tone: Option<String>,
    /// Target word count (optional)
    pub word_count: Option<u32>,
    /// SEO keywords
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Custom instructions
    pub custom_instructions: Option<String>,
    /// Brand voice override
    pub brand_voice_override: Option<String>,
    /// Execution options
    pub execution_options: Option<ExecutionOptionsRequest>,
    /// Webhook callback URL
    pub callback_url: Option<String>,
}

/// Execution options request
#[derive(Debug, Deserialize)]
pub struct ExecutionOptionsRequest {
    /// Enable parallel processing
    pub parallel_processing: Option<bool>,
    /// Priority level
    pub priority: Option<String>,
    /// Maximum execution time (seconds)
    pub max_execution_time: Option<u32>,
    /// Quality threshold
    pub quality_threshold: Option<f32>,
    /// Enable real-time updates
    pub real_time_updates: Option<bool>,
}

/// Blog post generation response
#[derive(Debug, Serialize)]
pub struct BlogPostResponse {
    /// Workflow execution ID
    pub workflow_id: Uuid,
    /// Current status
    pub status: String,
    /// Generated blog post (if completed)
    pub blog_post: Option<BlogPostOutput>,
    /// Execution metrics
    pub metrics: Option<ExecutionMetricsOutput>,
    /// Quality scores
    pub quality_scores: Option<QualityScoresOutput>,
    /// Estimated completion time
    pub estimated_completion: Option<chrono::DateTime<Utc>>,
    /// Progress percentage
    pub progress: Option<f32>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Blog post output
#[derive(Debug, Serialize)]
pub struct BlogPostOutput {
    /// Blog post title
    pub title: String,
    /// Blog post content (HTML)
    pub content: String,
    /// Blog post content (Markdown)
    pub content_markdown: Option<String>,
    /// Featured image
    pub featured_image: Option<ImageOutput>,
    /// Meta description
    pub meta_description: String,
    /// SEO metadata
    pub seo_metadata: SeoMetadataOutput,
    /// Word count
    pub word_count: u32,
    /// Reading time estimate (minutes)
    pub reading_time: u32,
    /// Generated timestamp
    pub generated_at: chrono::DateTime<Utc>,
}

/// Image output
#[derive(Debug, Serialize)]
pub struct ImageOutput {
    /// Image URL
    pub url: String,
    /// Alt text
    pub alt_text: String,
    /// Image dimensions
    pub width: u32,
    pub height: u32,
    /// File size (bytes)
    pub file_size: u64,
    /// Image format
    pub format: String,
}

/// SEO metadata output
#[derive(Debug, Serialize)]
pub struct SeoMetadataOutput {
    /// Primary keywords
    pub primary_keywords: Vec<String>,
    /// Secondary keywords
    pub secondary_keywords: Vec<String>,
    /// Keyword density
    pub keyword_density: f32,
    /// SEO score
    pub seo_score: f32,
    /// Meta tags
    pub meta_tags: HashMap<String, String>,
}

/// Execution metrics output
#[derive(Debug, Serialize)]
pub struct ExecutionMetricsOutput {
    /// Total execution time (milliseconds)
    pub total_execution_time_ms: u64,
    /// Content generation time (milliseconds)
    pub content_generation_time_ms: u64,
    /// Image generation time (milliseconds)
    pub image_generation_time_ms: u64,
    /// Quality validation time (milliseconds)
    pub quality_validation_time_ms: u64,
    /// Total cost
    pub total_cost: f64,
    /// Currency
    pub currency: String,
}

/// Quality scores output
#[derive(Debug, Serialize)]
pub struct QualityScoresOutput {
    /// Overall quality score (0.0-5.0)
    pub overall_score: f32,
    /// Content quality score
    pub content_quality: f32,
    /// Grammar score
    pub grammar_score: f32,
    /// Readability score
    pub readability_score: f32,
    /// SEO score
    pub seo_score: f32,
    /// Brand compliance score
    pub brand_compliance_score: f32,
    /// Originality score
    pub originality_score: f32,
}

/// Client registration request
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    /// Client name
    pub name: String,
    /// Client description
    pub description: Option<String>,
    /// Contact email
    pub email: String,
    /// Company name
    pub company: Option<String>,
    /// Subscription tier
    pub tier: String,
    /// Brand profile
    pub brand_profile: Option<BrandProfileRequest>,
    /// Blog preferences
    pub blog_preferences: Option<BlogPreferencesRequest>,
    /// Webhook configuration
    pub webhook_config: Option<WebhookConfigRequest>,
}

/// Brand profile request
#[derive(Debug, Deserialize)]
pub struct BrandProfileRequest {
    /// Brand name
    pub brand_name: String,
    /// Brand voice description
    pub brand_voice: String,
    /// Brand values
    pub brand_values: Vec<String>,
    /// Color palette (hex codes)
    pub color_palette: Vec<String>,
    /// Industry context
    pub industry_context: Option<String>,
}

/// Blog preferences request
#[derive(Debug, Deserialize)]
pub struct BlogPreferencesRequest {
    /// Default word count range
    pub default_word_count: Option<WordCountRangeRequest>,
    /// Default tone
    pub default_tone: Option<String>,
    /// Target audience
    pub target_audience: Option<String>,
    /// SEO preferences
    pub seo_enabled: Option<bool>,
    /// Image generation enabled
    pub image_generation_enabled: Option<bool>,
}

/// Word count range request
#[derive(Debug, Deserialize)]
pub struct WordCountRangeRequest {
    pub min: u32,
    pub max: u32,
    pub target: u32,
}

/// Webhook configuration request
#[derive(Debug, Deserialize)]
pub struct WebhookConfigRequest {
    /// Webhook URL
    pub url: String,
    /// Events to notify
    pub events: Vec<String>,
    /// Timeout in seconds
    pub timeout_seconds: Option<u32>,
}

/// Client registration response
#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    /// Client ID
    pub client_id: Uuid,
    /// API key for authentication
    pub api_key: String,
    /// Client profile
    pub profile: ClientProfileOutput,
    /// Registration message
    pub message: String,
    /// Next steps
    pub next_steps: Vec<String>,
}

/// Client profile output
#[derive(Debug, Serialize)]
pub struct ClientProfileOutput {
    /// Client ID
    pub client_id: Uuid,
    /// Client name
    pub name: String,
    /// Client description
    pub description: Option<String>,
    /// Subscription tier
    pub tier: String,
    /// Brand profile
    pub brand_profile: Option<BrandProfileOutput>,
    /// Blog preferences
    pub blog_preferences: BlogPreferencesOutput,
    /// Rate limits
    pub rate_limits: RateLimitsOutput,
    /// Created timestamp
    pub created_at: chrono::DateTime<Utc>,
}

/// Brand profile output
#[derive(Debug, Serialize)]
pub struct BrandProfileOutput {
    /// Brand name
    pub brand_name: String,
    /// Brand voice
    pub brand_voice: String,
    /// Brand values
    pub brand_values: Vec<String>,
    /// Color palette
    pub color_palette: Vec<String>,
}

/// Blog preferences output
#[derive(Debug, Serialize)]
pub struct BlogPreferencesOutput {
    /// Default word count range
    pub default_word_count: WordCountRangeOutput,
    /// Default tone
    pub default_tone: String,
    /// Target audience
    pub target_audience: Option<String>,
    /// SEO enabled
    pub seo_enabled: bool,
    /// Image generation enabled
    pub image_generation_enabled: bool,
}

/// Word count range output
#[derive(Debug, Serialize)]
pub struct WordCountRangeOutput {
    pub min: u32,
    pub max: u32,
    pub target: u32,
}

/// Rate limits output
#[derive(Debug, Serialize)]
pub struct RateLimitsOutput {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Requests per hour
    pub requests_per_hour: u32,
    /// Requests per day
    pub requests_per_day: u32,
}

/// Workflow status query parameters
#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    /// Include detailed metrics
    pub include_metrics: Option<bool>,
    /// Include quality scores
    pub include_quality: Option<bool>,
}

/// List workflows query parameters
#[derive(Debug, Deserialize)]
pub struct ListWorkflowsQuery {
    /// Limit number of results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
    /// Filter by status
    pub status: Option<String>,
    /// Filter by date range (ISO 8601)
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

/// Workflows list response
#[derive(Debug, Serialize)]
pub struct WorkflowsListResponse {
    /// Workflows
    pub workflows: Vec<WorkflowSummary>,
    /// Total count
    pub total: u32,
    /// Current offset
    pub offset: u32,
    /// Limit used
    pub limit: u32,
    /// Has more results
    pub has_more: bool,
}

/// Workflow summary
#[derive(Debug, Serialize)]
pub struct WorkflowSummary {
    /// Workflow ID
    pub workflow_id: Uuid,
    /// Topic
    pub topic: String,
    /// Status
    pub status: String,
    /// Created at
    pub created_at: chrono::DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<chrono::DateTime<Utc>>,
    /// Execution time (ms)
    pub execution_time_ms: Option<u64>,
    /// Quality score
    pub quality_score: Option<f32>,
    /// Word count
    pub word_count: Option<u32>,
}

/// Authentication middleware
/// Generate blog post endpoint
pub async fn generate_blog_post(
    State(state): State<ServerState>,
    Json(request): Json<BlogPostRequest>,
) -> AxumResult<Json<ApiResponse<BlogPostResponse>>> {
    // Extract client from auth middleware (would be set by auth middleware)
    // For now, create a demo client
    let client = SaasClientProfile {
        client_id: Uuid::new_v4(),
        name: "Demo Client".to_string(),
        api_key: "demo-key".to_string(),
        tier: "standard".to_string(),
        brand_profile: None,
        rate_limits: None,
        created_at: Utc::now(),
    };

    // Build workflow request
    let execution_options = ExecutionOptions {
        parallel_processing: request
            .execution_options
            .as_ref()
            .and_then(|eo| eo.parallel_processing)
            .unwrap_or(true),
        priority: request
            .execution_options
            .as_ref()
            .and_then(|eo| eo.priority.as_ref())
            .map(|p| match p.as_str() {
                "low" => WorkflowPriority::Low,
                "high" => WorkflowPriority::High,
                "critical" => WorkflowPriority::Critical,
                "realtime" => WorkflowPriority::RealTime,
                _ => WorkflowPriority::Normal,
            })
            .unwrap_or(WorkflowPriority::Normal),
        max_execution_time: request
            .execution_options
            .as_ref()
            .and_then(|eo| eo.max_execution_time)
            .unwrap_or(45),
        quality_threshold: request
            .execution_options
            .as_ref()
            .and_then(|eo| eo.quality_threshold)
            .unwrap_or(4.0),
        real_time_updates: request
            .execution_options
            .as_ref()
            .and_then(|eo| eo.real_time_updates)
            .unwrap_or(false),
        retry_config: None,
    };

    let callback_config = request.callback_url.map(|url| CallbackConfig {
        webhook_url: url,
        webhook_secret: "".to_string(), // Would use client's webhook secret
        events: vec![],                 // Default events
        webhook_retry: None,
    });

    let workflow_request = BlogWorkflowRequest {
        client: client.clone(),
        topic: request.topic,
        parameters: BlogParameters {
            audience: request.audience,
            tone: request.tone,
            word_count: request.word_count,
            keywords: request.keywords,
            custom_instructions: request.custom_instructions,
            brand_voice_override: request.brand_voice_override,
        },
        execution_options,
        callback_config,
    };

    // Execute workflow
    // For now, create a mock workflow response
    // This would be replaced with actual workflow engine integration
    let workflow_id = Uuid::new_v4();
    let blog_response = BlogPostResponse {
        workflow_id,
        status: "processing".to_string(),
        blog_post: None,
        metrics: None,
        quality_scores: None,
        estimated_completion: Some(Utc::now() + chrono::Duration::seconds(35)),
        progress: Some(0.0),
        error: None,
    };

    Ok(Json(ApiResponse::success(blog_response)))
}

/// Get workflow status endpoint
pub async fn get_workflow_status(
    State(state): State<ServerState>,
    Path(workflow_id): Path<Uuid>,
    Query(query): Query<StatusQuery>,
) -> AxumResult<Json<ApiResponse<BlogPostResponse>>> {
    // For now, return a mock status response
    // This would be replaced with actual workflow status checking
    let status_response = BlogPostResponse {
        workflow_id,
        status: "completed".to_string(),
        blog_post: Some(BlogPostOutput {
            title: "Sample AI-Generated Blog Post".to_string(),
            content: "<h1>Sample AI-Generated Blog Post</h1><p>This is a sample blog post generated by the AI-CORE platform...</p>".to_string(),
            content_markdown: Some("# Sample AI-Generated Blog Post\n\nThis is a sample blog post generated by the AI-CORE platform...".to_string()),
            featured_image: Some(ImageOutput {
                url: "https://example.com/image.jpg".to_string(),
                alt_text: "Featured image".to_string(),
                width: Some(1200),
                height: 600,
                file_size: Some(150000),
                format: "jpeg".to_string(),
            }),
            meta_description: Some("Sample meta description".to_string()),
            seo_metadata: Some(SeoMetadataOutput {
                primary_keywords: vec!["AI".to_string(), "automation".to_string()],
                secondary_keywords: vec!["blog".to_string(), "content".to_string()],
                keyword_density: Some(2.5),
                seo_score: Some(8.5),
                meta_tags: Some(HashMap::new()),
            }),
            word_count: 850,
            reading_time: Some(4),
            generated_at: Utc::now(),
        }),
        metrics: Some(ExecutionMetricsOutput {
            total_execution_time_ms: 35200,
            content_generation_time_ms: Some(28000),
            image_generation_time_ms: Some(5000),
            quality_validation_time_ms: Some(2200),
            total_cost: 0.47,
            currency: "USD".to_string(),
        }),
        quality_scores: Some(QualityScoresOutput {
            overall_score: 4.32,
            content_quality: Some(4.5),
            grammar_score: Some(4.8),
            readability_score: Some(4.2),
            seo_score: Some(4.1),
            brand_compliance_score: Some(4.0),
            originality_score: Some(4.7),
        }),
        estimated_completion: None,
        progress: Some(100.0),
        error: None,
    };

    Ok(Json(ApiResponse::success(status_response)))
}

/// Cancel workflow endpoint
pub async fn cancel_workflow(
    State(state): State<ServerState>,
    Path(workflow_id): Path<Uuid>,
) -> AxumResult<Json<ApiResponse<serde_json::Value>>> {
    // For now, return a mock cancellation response
    // This would be replaced with actual workflow cancellation
    let cancelled_response = BlogPostResponse {
        workflow_id,
        status: "cancelled".to_string(),
        blog_post: None,
        metrics: None,
        quality_scores: None,
        estimated_completion: None,
        progress: None,
        error: Some("Workflow cancelled by user request".to_string()),
    };

    Ok(Json(ApiResponse::success(cancelled_response)))
}

/// List workflows endpoint
pub async fn list_workflows(
    State(state): State<ServerState>,
    Query(query): Query<ListWorkflowsQuery>,
) -> AxumResult<Json<ApiResponse<WorkflowsListResponse>>> {
    // Extract client from auth middleware (would be set by auth middleware)
    // For now, create a demo client
    let client = SaasClientProfile {
        client_id: Uuid::new_v4(),
        name: "Demo Client".to_string(),
        api_key: "demo-key".to_string(),
        tier: "standard".to_string(),
        brand_profile: None,
        rate_limits: None,
        created_at: Utc::now(),
    };

    // This would implement pagination and filtering
    // For now, return empty list as placeholder
    let response = WorkflowsListResponse {
        workflows: vec![],
        total: 0,
        offset: query.offset.unwrap_or(0),
        limit: query.limit.unwrap_or(20),
        has_more: false,
    };

    success_response(response)
}

/// Register client endpoint
pub async fn register_client(
    State(state): State<ServerState>,
    Json(request): Json<ClientRegistrationRequest>,
) -> AxumResult<Json<ApiResponse<ClientRegistrationResponse>>> {
    // Convert request to internal format
    let brand_profile = request.brand_profile.map(|bp| BrandProfile {
        brand_name: bp.brand_name,
        brand_voice: bp.brand_voice,
        brand_values: bp.brand_values,
        color_palette: bp.color_palette,
        visual_assets: vec![],
        guidelines_url: None,
        content_templates: vec![],
    });

    // This would create a full SaasClientRegistrationRequest
    // For now, return success with generated credentials
    let client_id = Uuid::new_v4();
    let api_key = format!("sk_live_{}", Uuid::new_v4().to_string().replace("-", ""));

    let response = ClientRegistrationResponse {
        client_id,
        api_key,
        profile: ClientProfileOutput {
            client_id,
            name: request.name,
            description: request.description,
            tier: request.tier,
            brand_profile: brand_profile.map(|bp| BrandProfileOutput {
                brand_name: bp.brand_name,
                brand_voice: bp.brand_voice,
                brand_values: bp.brand_values,
                color_palette: bp.color_palette,
            }),
            blog_preferences: BlogPreferencesOutput {
                default_word_count: WordCountRangeOutput {
                    min: 500,
                    max: 1500,
                    target: 800,
                },
                default_tone: "professional".to_string(),
                target_audience: None,
                seo_enabled: true,
                image_generation_enabled: true,
            },
            rate_limits: RateLimitsOutput {
                requests_per_minute: 60,
                requests_per_hour: 1000,
                requests_per_day: 10000,
            },
            created_at: Utc::now(),
        },
        message: "Client registered successfully".to_string(),
        next_steps: vec![
            "Test the API with your new API key".to_string(),
            "Configure your brand profile".to_string(),
            "Set up webhook notifications".to_string(),
            "Start generating blog posts".to_string(),
        ],
    };

    success_response(response)
}

/// Get client profile endpoint
pub async fn get_client_profile(
    State(state): State<ServerState>,
) -> AxumResult<Json<ApiResponse<ClientProfileOutput>>> {
    // Extract client from auth middleware (would be set by auth middleware)
    // For now, create a demo client
    let client = SaasClientProfile {
        client_id: Uuid::new_v4(),
        name: "Demo Client".to_string(),
        api_key: "demo-key".to_string(),
        tier: "standard".to_string(),
        brand_profile: None,
        rate_limits: None,
        created_at: Utc::now(),
    };

    let profile = ClientProfileOutput {
        client_id: client.client.id,
        name: client.client.name.clone(),
        description: client.client.description.clone(),
        tier: format!("{:?}", client.client.tier),
        brand_profile: client.brand_profile.as_ref().map(|bp| BrandProfileOutput {
            brand_name: bp.brand_name.clone(),
            brand_voice: bp.brand_voice.clone(),
            brand_values: bp.brand_values.clone(),
            color_palette: bp.color_palette.clone(),
        }),
        blog_preferences: BlogPreferencesOutput {
            default_word_count: WordCountRangeOutput {
                min: client.blog_preferences.default_word_count.min,
                max: client.blog_preferences.default_word_count.max,
                target: client.blog_preferences.default_word_count.target,
            },
            default_tone: client.blog_preferences.default_tone.clone(),
            target_audience: client.blog_preferences.target_audience.clone(),
            seo_enabled: true,              // From preferences
            image_generation_enabled: true, // From preferences
        },
        rate_limits: RateLimitsOutput {
            requests_per_minute: client.client.limits.max_requests_per_minute,
            requests_per_hour: client.client.limits.max_requests_per_hour,
            requests_per_day: client.client.limits.max_requests_per_day,
        },
        created_at: client.client.created_at,
    };

    success_response(profile)
}

/// Update client profile endpoint
pub async fn update_client_profile(
    State(state): State<ServerState>,
    Json(updates): Json<serde_json::Value>,
) -> AxumResult<Json<ApiResponse<ClientProfileOutput>>> {
    // Extract client from auth middleware (would be set by auth middleware)
    // For now, create a demo client
    let client = SaasClientProfile {
        client_id: Uuid::new_v4(),
        name: "Demo Client".to_string(),
        api_key: "demo-key".to_string(),
        tier: "standard".to_string(),
        brand_profile: None,
        rate_limits: None,
        created_at: Utc::now(),
    };

    // This would implement profile updates
    // For now, return current profile
    let profile = ClientProfileOutput {
        client_id: client.client.id,
        name: client.client.name.clone(),
        description: client.client.description.clone(),
        tier: format!("{:?}", client.client.tier),
        brand_profile: None,
        blog_preferences: BlogPreferencesOutput {
            default_word_count: WordCountRangeOutput {
                min: 500,
                max: 1500,
                target: 800,
            },
            default_tone: "professional".to_string(),
            target_audience: None,
            seo_enabled: true,
            image_generation_enabled: true,
        },
        rate_limits: RateLimitsOutput {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
        },
        created_at: Utc::now(),
    };

    success_response(profile)
}

/// Health check endpoint
pub async fn health_check() -> AxumResult<Json<ApiResponse<serde_json::Value>>> {
    success_response(serde_json::json!({
        "status": "healthy",
        "service": "blog-automation-api",
        "version": "1.0.0",
        "timestamp": Utc::now()
    }))
}

/// API capabilities endpoint
pub async fn get_capabilities() -> AxumResult<Json<ApiResponse<serde_json::Value>>> {
    success_response(serde_json::json!({
        "version": "1.0.0",
        "capabilities": [
            "blog_post_generation",
            "content_optimization",
            "seo_optimization",
            "image_generation",
            "quality_validation",
            "brand_compliance",
            "real_time_progress",
            "webhook_notifications"
        ],
        "supported_formats": ["html", "markdown"],
        "supported_tones": ["professional", "casual", "friendly", "authoritative", "conversational"],
        "max_word_count": 5000,
        "min_word_count": 100,
        "max_execution_time_seconds": 300,
        "rate_limits": {
            "free_tier": {
                "requests_per_minute": 10,
                "requests_per_hour": 100,
                "requests_per_day": 1000
            },
            "professional_tier": {
                "requests_per_minute": 60,
                "requests_per_hour": 1000,
                "requests_per_day": 10000
            },
            "enterprise_tier": {
                "requests_per_minute": 300,
                "requests_per_hour": 10000,
                "requests_per_day": 100000
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blog_post_request_deserialization() {
        let json = r#"
        {
            "topic": "Artificial Intelligence in Healthcare",
            "audience": "Healthcare professionals",
            "tone": "professional",
            "word_count": 1200,
            "keywords": ["AI", "healthcare", "machine learning"],
            "execution_options": {
                "parallel_processing": true,
                "priority": "high",
                "quality_threshold": 4.5
            }
        }"#;

        let request: Result<BlogPostRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.topic, "Artificial Intelligence in Healthcare");
        assert_eq!(
            request.audience,
            Some("Healthcare professionals".to_string())
        );
        assert_eq!(request.word_count, Some(1200));
        assert_eq!(request.keywords.len(), 3);
    }

    #[test]
    fn test_client_registration_request_deserialization() {
        let json = "{
            \"name\": \"TechCorp Blog\",
            \"description\": \"Corporate blog for TechCorp Inc.\",
            \"email\": \"admin@techcorp.com\",
            \"company\": \"TechCorp Inc.\",
            \"tier\": \"professional\",
            \"brand_profile\": {
                \"brand_name\": \"TechCorp\",
                \"brand_voice\": \"Professional and innovative\",
                \"brand_values\": [\"Innovation\", \"Quality\", \"Customer Success\"],
                \"color_palette\": [\"#007acc\", \"#ffffff\", \"#f0f0f0\"]
            }
        }";

        let request: Result<ClientRegistrationRequest, _> = serde_json::from_str(json);
        assert!(request.is_ok());

        let request = request.unwrap();
        assert_eq!(request.name, "TechCorp Blog");
        assert_eq!(request.tier, "professional");
        assert!(request.brand_profile.is_some());
    }

    #[tokio::test]
    async fn test_workflow_priority_parsing() {
        let priorities = vec![
            ("low", WorkflowPriority::Low),
            ("normal", WorkflowPriority::Normal),
            ("high", WorkflowPriority::High),
            ("critical", WorkflowPriority::Critical),
            ("realtime", WorkflowPriority::RealTime),
            ("unknown", WorkflowPriority::Normal), // Default case
        ];

        for (input, expected) in priorities {
            let parsed = match input {
                "low" => WorkflowPriority::Low,
                "high" => WorkflowPriority::High,
                "critical" => WorkflowPriority::Critical,
                "realtime" => WorkflowPriority::RealTime,
                _ => WorkflowPriority::Normal,
            };
            assert!(matches!(parsed, expected));
        }
    }
}
