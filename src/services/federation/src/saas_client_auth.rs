//! SaaS Client Authentication Service
//!
//! This module provides authentication and authorization services specifically
//! for SaaS clients integrating with the AI-CORE platform for blog post automation.
//! It extends the existing federation service with specialized SaaS client management.

use crate::models::{Client, ClientCredentials, ClientStatus, ResourceLimits};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

/// SaaS Client Authentication Service
#[derive(Debug, Clone)]
pub struct SaasClientAuthService {
    /// Client registry for fast lookups
    client_registry: Arc<DashMap<String, SaasClientProfile>>,
    /// API key to client ID mapping
    api_key_registry: Arc<DashMap<String, String>>,
    /// Rate limiters per client
    rate_limiters: Arc<DashMap<String, Arc<RwLock<RateLimiter>>>>,
    /// Usage tracking
    usage_tracker: Arc<DashMap<String, ClientUsageMetrics>>,
    /// Configuration
    config: SaasAuthConfig,
}

/// SaaS-specific client profile extending the base Client model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasClientProfile {
    /// Base client information
    pub client: Client,
    /// SaaS-specific configuration
    pub saas_config: SaasClientConfig,
    /// Blog automation preferences
    pub blog_preferences: BlogAutomationPreferences,
    /// Brand guidelines for content generation
    pub brand_profile: Option<BrandProfile>,
    /// API usage statistics
    pub usage_stats: ClientUsageStats,
    /// Integration status
    pub integration_status: IntegrationStatus,
}

/// SaaS client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasClientConfig {
    /// Allowed content types
    pub allowed_content_types: Vec<ContentType>,
    /// Quality requirements
    pub quality_settings: QualitySettings,
    /// Performance requirements
    pub performance_requirements: PerformanceRequirements,
    /// Webhook configuration
    pub webhook_config: Option<WebhookConfig>,
    /// Custom integrations
    pub custom_integrations: Vec<CustomIntegration>,
}

/// Blog automation preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogAutomationPreferences {
    /// Default word count range
    pub default_word_count: WordCountRange,
    /// Default tone and style
    pub default_tone: String,
    /// Target audience
    pub target_audience: Option<String>,
    /// SEO preferences
    pub seo_preferences: SeoPreferences,
    /// Image generation preferences
    pub image_preferences: ImagePreferences,
    /// Content validation rules
    pub validation_rules: Vec<ValidationRule>,
}

/// Brand profile for content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandProfile {
    /// Brand name
    pub brand_name: String,
    /// Brand voice description
    pub brand_voice: String,
    /// Brand values and messaging
    pub brand_values: Vec<String>,
    /// Color palette (hex codes)
    pub color_palette: Vec<String>,
    /// Logo and visual assets
    pub visual_assets: Vec<AssetReference>,
    /// Brand guidelines document
    pub guidelines_url: Option<String>,
    /// Content templates
    pub content_templates: Vec<ContentTemplate>,
}

/// Content type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    BlogPost,
    MarketingArticle,
    TechnicalArticle,
    NewsPost,
    ProductDescription,
    SocialMediaPost,
    EmailNewsletter,
    PressRelease,
    Custom(String),
}

/// Quality settings for content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySettings {
    /// Minimum quality score (0.0-5.0)
    pub min_quality_score: f32,
    /// Grammar checking enabled
    pub grammar_check: bool,
    /// Plagiarism checking enabled
    pub plagiarism_check: bool,
    /// Fact-checking enabled
    pub fact_check: bool,
    /// SEO optimization level
    pub seo_optimization: SeoLevel,
    /// Content uniqueness threshold
    pub uniqueness_threshold: f32,
}

/// Performance requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRequirements {
    /// Maximum total execution time (seconds)
    pub max_execution_time: u32,
    /// Maximum content generation time (seconds)
    pub max_content_time: u32,
    /// Maximum image generation time (seconds)
    pub max_image_time: u32,
    /// Parallel processing enabled
    pub parallel_processing: bool,
    /// Priority level
    pub priority: ProcessingPriority,
}

/// Word count range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCountRange {
    pub min: u32,
    pub max: u32,
    pub target: u32,
}

/// SEO preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoPreferences {
    /// Target keywords
    pub target_keywords: Vec<String>,
    /// Meta description preferences
    pub meta_description: bool,
    /// Header structure preferences
    pub header_structure: bool,
    /// Internal linking suggestions
    pub internal_links: bool,
    /// Image alt text generation
    pub image_alt_text: bool,
}

/// Image generation preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePreferences {
    /// Image style
    pub style: ImageStyle,
    /// Aspect ratio
    pub aspect_ratio: String,
    /// Resolution
    pub resolution: ImageResolution,
    /// Brand consistency
    pub brand_consistent: bool,
    /// Custom prompts
    pub custom_prompts: Vec<String>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,
    /// Secret for HMAC verification
    pub secret: String,
    /// Events to notify
    pub events: Vec<WebhookEvent>,
    /// Retry configuration
    pub retry_config: RetryConfig,
    /// Timeout in seconds
    pub timeout_seconds: u32,
}

/// Custom integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomIntegration {
    /// Integration name
    pub name: String,
    /// Integration type
    pub integration_type: String,
    /// Configuration parameters
    pub config: HashMap<String, serde_json::Value>,
    /// Enabled status
    pub enabled: bool,
}

/// Asset reference for brand materials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReference {
    /// Asset type
    pub asset_type: String,
    /// URL or path to asset
    pub url: String,
    /// Description
    pub description: Option<String>,
    /// Usage guidelines
    pub usage_guidelines: Option<String>,
}

/// Content template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTemplate {
    /// Template name
    pub name: String,
    /// Template content
    pub template: String,
    /// Variable placeholders
    pub variables: Vec<String>,
    /// Use cases
    pub use_cases: Vec<String>,
}

/// Validation rule for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Rule parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Severity level
    pub severity: Severity,
}

/// Client usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUsageStats {
    /// Total requests made
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time (ms)
    pub avg_response_time: f64,
    /// Total cost incurred
    pub total_cost: f64,
    /// Last request timestamp
    pub last_request_at: Option<DateTime<Utc>>,
    /// Monthly usage breakdown
    pub monthly_usage: HashMap<String, MonthlyUsage>,
}

/// Monthly usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyUsage {
    /// Requests in this month
    pub requests: u64,
    /// Cost in this month
    pub cost: f64,
    /// Content pieces generated
    pub content_pieces: u64,
    /// Average quality score
    pub avg_quality_score: f32,
}

/// Integration status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    /// Integration health
    pub health: IntegrationHealth,
    /// Last health check
    pub last_health_check: DateTime<Utc>,
    /// Active features
    pub active_features: Vec<String>,
    /// Configuration errors
    pub config_errors: Vec<String>,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
}

/// Rate limiter for client requests
#[derive(Debug)]
pub struct RateLimiter {
    /// Requests in current minute
    pub requests_per_minute: u32,
    /// Requests in current hour
    pub requests_per_hour: u32,
    /// Requests in current day
    pub requests_per_day: u32,
    /// Current minute timestamp
    pub current_minute: DateTime<Utc>,
    /// Current hour timestamp
    pub current_hour: DateTime<Utc>,
    /// Current day timestamp
    pub current_day: DateTime<Utc>,
    /// Rate limits
    pub limits: ResourceLimits,
}

/// Client usage metrics for tracking
#[derive(Debug, Clone)]
pub struct ClientUsageMetrics {
    /// Request count in current window
    pub request_count: u64,
    /// Data transferred in bytes
    pub data_transferred: u64,
    /// Processing time total
    pub processing_time_ms: u64,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Window start time
    pub window_start: DateTime<Utc>,
}

/// SaaS authentication configuration
#[derive(Debug, Clone)]
pub struct SaasAuthConfig {
    /// Token expiration time
    pub token_expiry: Duration,
    /// Refresh token expiration
    pub refresh_token_expiry: Duration,
    /// Rate limiting enabled
    pub rate_limiting_enabled: bool,
    /// Default rate limits
    pub default_rate_limits: ResourceLimits,
    /// Authentication timeout
    pub auth_timeout_ms: u64,
}

/// Enumeration types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeoLevel {
    Basic,
    Standard,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageStyle {
    Photographic,
    Illustration,
    Minimalist,
    Abstract,
    Corporate,
    Creative,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageResolution {
    Low,    // 512x512
    Medium, // 1024x1024
    High,   // 2048x2048
    Ultra,  // 4096x4096
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    ContentGenerated,
    ImageGenerated,
    QualityCheckCompleted,
    WorkflowCompleted,
    WorkflowFailed,
    RateLimitExceeded,
    QuotaExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRuleType {
    WordCount,
    KeywordDensity,
    ReadabilityScore,
    SentimentAnalysis,
    BrandCompliance,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average response time (ms)
    pub avg_response_time: f64,
    /// Success rate (0.0-1.0)
    pub success_rate: f32,
    /// Error rate (0.0-1.0)
    pub error_rate: f32,
    /// Throughput (requests per second)
    pub throughput: f64,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
}

/// Authentication result
#[derive(Debug)]
pub struct AuthResult {
    /// Client profile
    pub client: SaasClientProfile,
    /// Authentication token
    pub token: String,
    /// Token expiration
    pub expires_at: DateTime<Utc>,
    /// Rate limit status
    pub rate_limit_status: RateLimitStatus,
}

/// Rate limit status
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitStatus {
    /// Requests remaining in current minute
    pub requests_remaining_minute: u32,
    /// Requests remaining in current hour
    pub requests_remaining_hour: u32,
    /// Requests remaining in current day
    pub requests_remaining_day: u32,
    /// Reset time for current window
    pub reset_time: DateTime<Utc>,
}

/// SaaS authentication errors
#[derive(Error, Debug)]
pub enum SaasAuthError {
    #[error("Invalid API key: {0}")]
    InvalidApiKey(String),

    #[error("Client not found: {0}")]
    ClientNotFound(String),

    #[error("Client suspended: {0}")]
    ClientSuspended(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Authentication timeout")]
    AuthTimeout,

    #[error("Token expired")]
    TokenExpired,

    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl SaasClientAuthService {
    /// Create a new SaaS client authentication service
    pub fn new(config: SaasAuthConfig) -> Self {
        Self {
            client_registry: Arc::new(DashMap::new()),
            api_key_registry: Arc::new(DashMap::new()),
            rate_limiters: Arc::new(DashMap::new()),
            usage_tracker: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Authenticate a client using API key
    pub async fn authenticate_client(&self, api_key: &str) -> Result<AuthResult, SaasAuthError> {
        let _start_time = std::time::Instant::now();

        // Check authentication timeout
        tokio::time::timeout(
            std::time::Duration::from_millis(self.config.auth_timeout_ms),
            self.do_authenticate(api_key),
        )
        .await
        .map_err(|_| SaasAuthError::AuthTimeout)?
    }

    /// Internal authentication logic
    async fn do_authenticate(&self, api_key: &str) -> Result<AuthResult, SaasAuthError> {
        // Hash the API key for secure lookup
        let api_key_hash = self.hash_api_key(api_key);

        // Look up client ID by API key
        let client_id = self
            .api_key_registry
            .get(&api_key_hash)
            .ok_or_else(|| SaasAuthError::InvalidApiKey("API key not found".to_string()))?
            .clone();

        // Get client profile
        let client_profile = self
            .client_registry
            .get(&client_id)
            .ok_or_else(|| SaasAuthError::ClientNotFound(client_id.clone()))?
            .clone();

        // Check client status
        match client_profile.client.status {
            ClientStatus::Active => {}
            ClientStatus::Suspended => {
                return Err(SaasAuthError::ClientSuspended(client_id));
            }
            _ => {
                return Err(SaasAuthError::ClientNotFound(client_id));
            }
        }

        // Check rate limits
        let rate_limit_status = self.check_rate_limits(&client_id).await?;

        // Generate authentication token
        let token = self.generate_auth_token(&client_id);
        let expires_at = Utc::now() + self.config.token_expiry;

        Ok(AuthResult {
            client: client_profile,
            token,
            expires_at,
            rate_limit_status,
        })
    }

    /// Check rate limits for a client
    pub async fn check_rate_limits(
        &self,
        client_id: &str,
    ) -> Result<RateLimitStatus, SaasAuthError> {
        if !self.config.rate_limiting_enabled {
            return Ok(RateLimitStatus {
                requests_remaining_minute: u32::MAX,
                requests_remaining_hour: u32::MAX,
                requests_remaining_day: u32::MAX,
                reset_time: Utc::now(),
            });
        }

        let rate_limiter = self
            .rate_limiters
            .entry(client_id.to_string())
            .or_insert_with(|| {
                Arc::new(RwLock::new(RateLimiter::new(
                    &self.config.default_rate_limits,
                )))
            })
            .clone();

        let mut limiter = rate_limiter.write().await;
        limiter.check_and_update()
    }

    /// Record a successful request
    pub async fn record_request(&self, client_id: &str, processing_time_ms: u64, data_size: u64) {
        // Update usage metrics
        let mut usage = self
            .usage_tracker
            .entry(client_id.to_string())
            .or_insert_with(|| ClientUsageMetrics::new())
            .clone();

        usage.request_count += 1;
        usage.data_transferred += data_size;
        usage.processing_time_ms += processing_time_ms;
        usage.last_activity = Utc::now();

        self.usage_tracker.insert(client_id.to_string(), usage);

        // Update rate limiter
        if let Some(rate_limiter) = self.rate_limiters.get(client_id) {
            let mut limiter = rate_limiter.write().await;
            limiter.record_request();
        }
    }

    /// Register a new SaaS client
    pub async fn register_client(
        &self,
        request: SaasClientRegistrationRequest,
    ) -> Result<SaasClientProfile, SaasAuthError> {
        let client_id = Uuid::new_v4().to_string();
        let api_key = self.generate_api_key();
        let api_key_hash = self.hash_api_key(&api_key);

        let client_profile = SaasClientProfile {
            client: Client {
                id: Uuid::parse_str(&client_id).unwrap(),
                name: request.name,
                description: request.description,
                tier: request.tier,
                config: request.config,
                credentials: ClientCredentials {
                    api_key,
                    jwt_secret: Some(self.generate_jwt_secret()),
                    oauth_config: None,
                    webhook_secret: request.webhook_secret,
                },
                status: ClientStatus::Active,
                limits: request
                    .limits
                    .unwrap_or(self.config.default_rate_limits.clone()),
                metadata: request
                    .metadata
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(k, v)| (k, v.to_string()))
                    .collect(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_activity_at: None,
            },
            saas_config: request.saas_config,
            blog_preferences: request.blog_preferences,
            brand_profile: request.brand_profile,
            usage_stats: ClientUsageStats::new(),
            integration_status: IntegrationStatus::new(),
        };

        // Store in registries
        self.client_registry
            .insert(client_id.clone(), client_profile.clone());
        self.api_key_registry.insert(api_key_hash, client_id);

        Ok(client_profile)
    }

    /// Get client by ID
    pub async fn get_client(&self, client_id: &str) -> Option<SaasClientProfile> {
        self.client_registry
            .get(client_id)
            .map(|entry| entry.clone())
    }

    /// Update client configuration
    pub async fn update_client(
        &self,
        client_id: &str,
        updates: SaasClientUpdate,
    ) -> Result<SaasClientProfile, SaasAuthError> {
        let mut client = self
            .client_registry
            .get_mut(client_id)
            .ok_or_else(|| SaasAuthError::ClientNotFound(client_id.to_string()))?;

        // Apply updates
        if let Some(saas_config) = updates.saas_config {
            client.saas_config = saas_config;
        }
        if let Some(blog_preferences) = updates.blog_preferences {
            client.blog_preferences = blog_preferences;
        }
        if let Some(brand_profile) = updates.brand_profile {
            client.brand_profile = Some(brand_profile);
        }

        client.client.updated_at = Utc::now();

        Ok(client.clone())
    }

    /// Helper methods
    fn hash_api_key(&self, api_key: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn generate_api_key(&self) -> String {
        format!("sk_live_{}", Uuid::new_v4().to_string().replace("-", ""))
    }

    fn generate_jwt_secret(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
            .collect()
    }

    fn generate_auth_token(&self, _client_id: &str) -> String {
        format!("sat_{}", Uuid::new_v4().to_string().replace("-", ""))
    }
}

impl RateLimiter {
    pub fn new(limits: &ResourceLimits) -> Self {
        let now = Utc::now();
        Self {
            requests_per_minute: 0,
            requests_per_hour: 0,
            requests_per_day: 0,
            current_minute: now,
            current_hour: now,
            current_day: now,
            limits: limits.clone(),
        }
    }

    pub fn check_and_update(&mut self) -> Result<RateLimitStatus, SaasAuthError> {
        let now = Utc::now();

        // Reset counters if time windows have passed
        if now.signed_duration_since(self.current_minute).num_seconds() >= 60 {
            self.requests_per_minute = 0;
            self.current_minute = now;
        }
        if now.signed_duration_since(self.current_hour).num_seconds() >= 3600 {
            self.requests_per_hour = 0;
            self.current_hour = now;
        }
        if now.signed_duration_since(self.current_day).num_days() >= 1 {
            self.requests_per_day = 0;
            self.current_day = now;
        }

        // Check limits
        if self.requests_per_minute >= self.limits.max_requests_per_minute {
            return Err(SaasAuthError::RateLimitExceeded(
                "Minute limit exceeded".to_string(),
            ));
        }
        if self.requests_per_hour >= self.limits.max_requests_per_hour {
            return Err(SaasAuthError::RateLimitExceeded(
                "Hour limit exceeded".to_string(),
            ));
        }
        if self.requests_per_day >= self.limits.max_requests_per_day {
            return Err(SaasAuthError::RateLimitExceeded(
                "Day limit exceeded".to_string(),
            ));
        }

        Ok(RateLimitStatus {
            requests_remaining_minute: self.limits.max_requests_per_minute
                - self.requests_per_minute,
            requests_remaining_hour: self.limits.max_requests_per_hour - self.requests_per_hour,
            requests_remaining_day: self.limits.max_requests_per_day - self.requests_per_day,
            reset_time: self.current_minute + Duration::seconds(60),
        })
    }

    pub fn record_request(&mut self) {
        self.requests_per_minute += 1;
        self.requests_per_hour += 1;
        self.requests_per_day += 1;
    }
}

impl ClientUsageMetrics {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            request_count: 0,
            data_transferred: 0,
            processing_time_ms: 0,
            last_activity: now,
            window_start: now,
        }
    }
}

impl ClientUsageStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time: 0.0,
            total_cost: 0.0,
            last_request_at: None,
            monthly_usage: HashMap::new(),
        }
    }
}

impl IntegrationStatus {
    pub fn new() -> Self {
        Self {
            health: IntegrationHealth::Healthy,
            last_health_check: Utc::now(),
            active_features: vec!["blog_automation".to_string()],
            config_errors: Vec::new(),
            performance_metrics: PerformanceMetrics {
                avg_response_time: 0.0,
                success_rate: 1.0,
                error_rate: 0.0,
                throughput: 0.0,
            },
        }
    }
}

/// SaaS client registration request
#[derive(Debug, Deserialize)]
pub struct SaasClientRegistrationRequest {
    pub name: String,
    pub description: Option<String>,
    pub tier: crate::models::ClientTier,
    pub config: crate::models::ClientConfig,
    pub saas_config: SaasClientConfig,
    pub blog_preferences: BlogAutomationPreferences,
    pub brand_profile: Option<BrandProfile>,
    pub limits: Option<ResourceLimits>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub webhook_secret: Option<String>,
}

/// SaaS client update request
#[derive(Debug, Deserialize)]
pub struct SaasClientUpdate {
    pub saas_config: Option<SaasClientConfig>,
    pub blog_preferences: Option<BlogAutomationPreferences>,
    pub brand_profile: Option<BrandProfile>,
}

impl Default for SaasAuthConfig {
    fn default() -> Self {
        Self {
            token_expiry: Duration::hours(1),
            refresh_token_expiry: Duration::days(7),
            rate_limiting_enabled: true,
            default_rate_limits: ResourceLimits {
                max_requests_per_minute: 60,
                max_requests_per_hour: 1000,
                max_requests_per_day: 10000,
                max_concurrent_connections: 10,
                max_data_transfer_per_day: 1024 * 1024 * 1024, // 1GB
                max_storage_usage: 1024 * 1024 * 1024,         // 1GB
            },
            auth_timeout_ms: 5000,
        }
    }
}

impl Default for WordCountRange {
    fn default() -> Self {
        Self {
            min: 500,
            max: 1500,
            target: 800,
        }
    }
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            min_quality_score: 4.0,
            grammar_check: true,
            plagiarism_check: true,
            fact_check: false,
            seo_optimization: SeoLevel::Standard,
            uniqueness_threshold: 0.8,
        }
    }
}

impl Default for PerformanceRequirements {
    fn default() -> Self {
        Self {
            max_execution_time: 45,
            max_content_time: 25,
            max_image_time: 20,
            parallel_processing: true,
            priority: ProcessingPriority::Normal,
        }
    }
}
