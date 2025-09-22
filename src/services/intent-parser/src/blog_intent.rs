//! Blog Post Intent Models
//!
//! This module defines the specialized intent structures for blog post automation,
//! extending the base intent parser with blog-specific parsing capabilities.

use crate::types::{FunctionCall, ParsedIntent, ProviderPreference, WorkflowStep, WorkflowType};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Blog post intent specialized for blog content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostIntent {
    /// Blog post topic (extracted from user input)
    pub topic: String,
    /// Target audience (optional, can be inferred)
    pub audience: Option<String>,
    /// Content tone and style
    pub tone: Option<BlogTone>,
    /// Target word count
    pub word_count: Option<WordCountTarget>,
    /// Brand guidelines to follow
    pub brand_guidelines: Option<BrandGuidelines>,
    /// SEO requirements
    pub seo_requirements: Option<SeoRequirements>,
    /// Content structure preferences
    pub structure_preferences: Option<ContentStructure>,
    /// Image generation requirements
    pub image_requirements: Option<ImageRequirements>,
    /// Publishing preferences
    pub publishing_preferences: Option<PublishingPreferences>,
    /// Quality validation requirements
    pub quality_requirements: Option<QualityRequirements>,
    /// Performance constraints
    pub performance_constraints: Option<PerformanceConstraints>,
}

/// Blog tone and style options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlogTone {
    Professional,
    Casual,
    Friendly,
    Authoritative,
    Conversational,
    Technical,
    Educational,
    Persuasive,
    Entertaining,
    Inspirational,
    Custom(String),
}

/// Word count target specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordCountTarget {
    /// Minimum word count
    pub min: u32,
    /// Target word count
    pub target: u32,
    /// Maximum word count
    pub max: u32,
    /// Flexibility in word count (percentage)
    pub flexibility: f32,
}

/// Brand guidelines for content generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandGuidelines {
    /// Brand name
    pub brand_name: String,
    /// Brand voice description
    pub brand_voice: String,
    /// Key brand messages
    pub key_messages: Vec<String>,
    /// Brand values
    pub brand_values: Vec<String>,
    /// Tone guidelines
    pub tone_guidelines: Vec<String>,
    /// Do's and don'ts
    pub content_dos: Vec<String>,
    pub content_donts: Vec<String>,
    /// Brand personality traits
    pub personality_traits: Vec<String>,
    /// Industry context
    pub industry_context: Option<String>,
    /// Competitive positioning
    pub competitive_positioning: Option<String>,
}

/// SEO requirements for blog posts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoRequirements {
    /// Primary keywords to target
    pub primary_keywords: Vec<String>,
    /// Secondary keywords
    pub secondary_keywords: Vec<String>,
    /// Long-tail keywords
    pub long_tail_keywords: Vec<String>,
    /// Target keyword density (percentage)
    pub keyword_density: Option<f32>,
    /// Meta description requirements
    pub meta_description: Option<MetaDescriptionRequirements>,
    /// Header structure requirements
    pub header_structure: Option<HeaderStructureRequirements>,
    /// Internal linking preferences
    pub internal_linking: Option<InternalLinkingRequirements>,
    /// Schema markup preferences
    pub schema_markup: Option<bool>,
    /// Featured snippet optimization
    pub featured_snippet_optimization: Option<bool>,
}

/// Meta description requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaDescriptionRequirements {
    /// Target length (characters)
    pub target_length: u32,
    /// Include primary keyword
    pub include_primary_keyword: bool,
    /// Call-to-action style
    pub cta_style: Option<String>,
}

/// Header structure requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderStructureRequirements {
    /// Use H1 for title
    pub use_h1_title: bool,
    /// Minimum number of H2 headers
    pub min_h2_count: u32,
    /// Maximum header nesting level
    pub max_nesting_level: u32,
    /// Include keywords in headers
    pub include_keywords_in_headers: bool,
}

/// Internal linking requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalLinkingRequirements {
    /// Minimum internal links
    pub min_internal_links: u32,
    /// Maximum internal links
    pub max_internal_links: u32,
    /// Suggested pages to link to
    pub suggested_links: Vec<String>,
    /// Link anchor text preferences
    pub anchor_text_preferences: Vec<String>,
}

/// Content structure preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentStructure {
    /// Include introduction
    pub include_introduction: bool,
    /// Include conclusion
    pub include_conclusion: bool,
    /// Include table of contents
    pub include_toc: bool,
    /// Include key takeaways
    pub include_key_takeaways: bool,
    /// Include FAQ section
    pub include_faq: bool,
    /// Paragraph length preferences
    pub paragraph_length: ParagraphLengthPreference,
    /// Section organization
    pub section_organization: SectionOrganization,
    /// Include statistics and data
    pub include_statistics: bool,
    /// Include quotes and testimonials
    pub include_quotes: bool,
    /// Include actionable tips
    pub include_actionable_tips: bool,
}

/// Paragraph length preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphLengthPreference {
    Short,  // 1-3 sentences
    Medium, // 3-5 sentences
    Long,   // 5+ sentences
    Varied, // Mix of lengths
}

/// Section organization preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionOrganization {
    Chronological,
    Importance,
    Complexity,
    ProblemSolution,
    ComparisonContrast,
    CauseEffect,
    Custom(String),
}

/// Image requirements for blog posts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequirements {
    /// Featured image required
    pub featured_image_required: bool,
    /// Featured image specifications
    pub featured_image_specs: Option<ImageSpecifications>,
    /// In-content images
    pub in_content_images: Option<InContentImageRequirements>,
    /// Alt text requirements
    pub alt_text_requirements: Option<AltTextRequirements>,
    /// Image style preferences
    pub style_preferences: Option<ImageStylePreferences>,
}

/// Image specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSpecifications {
    /// Image dimensions
    pub dimensions: ImageDimensions,
    /// Image format
    pub format: ImageFormat,
    /// Image quality
    pub quality: ImageQuality,
    /// File size constraints
    pub max_file_size_kb: Option<u32>,
}

/// Image dimensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: String,
}

/// Image format options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Svg,
}

/// Image quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageQuality {
    Low,
    Medium,
    High,
    Ultra,
}

/// In-content image requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InContentImageRequirements {
    /// Number of images to include
    pub image_count: u32,
    /// Image placement strategy
    pub placement_strategy: ImagePlacementStrategy,
    /// Image types
    pub image_types: Vec<ImageType>,
}

/// Image placement strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImagePlacementStrategy {
    EvenlyDistributed,
    AfterMajorSections,
    DataVisualization,
    ConceptIllustration,
    Custom(String),
}

/// Image type options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageType {
    Illustration,
    Photograph,
    Infographic,
    Chart,
    Diagram,
    Screenshot,
    Icon,
    Custom(String),
}

/// Alt text requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltTextRequirements {
    /// Maximum alt text length
    pub max_length: u32,
    /// Include keywords in alt text
    pub include_keywords: bool,
    /// Descriptive style
    pub descriptive_style: AltTextStyle,
}

/// Alt text style options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AltTextStyle {
    Descriptive,
    Functional,
    Decorative,
    Complex,
}

/// Image style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageStylePreferences {
    /// Visual style
    pub visual_style: VisualStyle,
    /// Color scheme
    pub color_scheme: Option<ColorScheme>,
    /// Mood and atmosphere
    pub mood: Option<String>,
    /// Composition preferences
    pub composition: Option<CompositionStyle>,
}

/// Visual style options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualStyle {
    Realistic,
    Illustration,
    Minimalist,
    Abstract,
    Corporate,
    Creative,
    Technical,
    Custom(String),
}

/// Color scheme options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    /// Primary colors
    pub primary_colors: Vec<String>,
    /// Secondary colors
    pub secondary_colors: Vec<String>,
    /// Color temperature
    pub temperature: ColorTemperature,
}

/// Color temperature options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorTemperature {
    Warm,
    Cool,
    Neutral,
    Vibrant,
    Muted,
}

/// Composition style options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionStyle {
    RuleOfThirds,
    CenterComposition,
    SymmetricalBalance,
    AsymmetricalBalance,
    Leading,
    Framing,
    Custom(String),
}

/// Publishing preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishingPreferences {
    /// Target publishing platform
    pub target_platform: Option<String>,
    /// Scheduling preferences
    pub scheduling: Option<SchedulingPreferences>,
    /// Distribution channels
    pub distribution_channels: Vec<String>,
    /// Social media promotion
    pub social_media_promotion: Option<SocialMediaPromotion>,
    /// Email newsletter inclusion
    pub email_newsletter: Option<bool>,
}

/// Scheduling preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingPreferences {
    /// Immediate publishing
    pub immediate: bool,
    /// Scheduled time
    pub scheduled_time: Option<DateTime<Utc>>,
    /// Optimal timing based on audience
    pub optimal_timing: Option<bool>,
    /// Recurring schedule
    pub recurring_schedule: Option<RecurringSchedule>,
}

/// Recurring schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringSchedule {
    /// Frequency
    pub frequency: ScheduleFrequency,
    /// Interval
    pub interval: u32,
    /// End date
    pub end_date: Option<DateTime<Utc>>,
    /// Maximum occurrences
    pub max_occurrences: Option<u32>,
}

/// Schedule frequency options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Custom(String),
}

/// Social media promotion settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMediaPromotion {
    /// Platforms to promote on
    pub platforms: Vec<String>,
    /// Custom messages per platform
    pub custom_messages: HashMap<String, String>,
    /// Hashtag strategy
    pub hashtag_strategy: Option<HashtagStrategy>,
    /// Posting schedule
    pub posting_schedule: Option<SocialPostingSchedule>,
}

/// Hashtag strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashtagStrategy {
    /// Maximum number of hashtags
    pub max_hashtags: u32,
    /// Hashtag categories
    pub hashtag_categories: Vec<String>,
    /// Include trending hashtags
    pub include_trending: bool,
    /// Brand-specific hashtags
    pub brand_hashtags: Vec<String>,
}

/// Social posting schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPostingSchedule {
    /// Immediate posting
    pub immediate: bool,
    /// Staggered posting times
    pub staggered_times: Vec<Duration>,
    /// Platform-specific timing
    pub platform_timing: HashMap<String, DateTime<Utc>>,
}

/// Quality requirements for blog posts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirements {
    /// Minimum quality score (0.0-5.0)
    pub min_quality_score: f32,
    /// Grammar and spelling check
    pub grammar_check: bool,
    /// Plagiarism detection
    pub plagiarism_check: bool,
    /// Fact-checking requirements
    pub fact_checking: Option<FactCheckingRequirements>,
    /// Readability requirements
    pub readability: Option<ReadabilityRequirements>,
    /// Brand compliance check
    pub brand_compliance: bool,
    /// Content originality threshold
    pub originality_threshold: f32,
    /// Source citation requirements
    pub citation_requirements: Option<CitationRequirements>,
}

/// Fact-checking requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactCheckingRequirements {
    /// Enable fact checking
    pub enabled: bool,
    /// Confidence threshold for facts
    pub confidence_threshold: f32,
    /// Require source verification
    pub require_source_verification: bool,
    /// Fact-checking scope
    pub scope: FactCheckingScope,
}

/// Fact-checking scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactCheckingScope {
    Statistics,
    Claims,
    Quotes,
    Dates,
    All,
    Custom(Vec<String>),
}

/// Readability requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadabilityRequirements {
    /// Target reading level
    pub target_reading_level: ReadingLevel,
    /// Sentence length preferences
    pub sentence_length: SentenceLengthPreference,
    /// Vocabulary complexity
    pub vocabulary_complexity: VocabularyComplexity,
    /// Readability score targets
    pub score_targets: ReadabilityScoreTargets,
}

/// Reading level options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingLevel {
    Elementary,
    MiddleSchool,
    HighSchool,
    College,
    Graduate,
    Professional,
}

/// Sentence length preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SentenceLengthPreference {
    Short,  // 5-15 words
    Medium, // 15-25 words
    Long,   // 25+ words
    Varied, // Mix of lengths
}

/// Vocabulary complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VocabularyComplexity {
    Simple,
    Moderate,
    Advanced,
    Technical,
    Academic,
}

/// Readability score targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadabilityScoreTargets {
    /// Flesch Reading Ease score (0-100)
    pub flesch_reading_ease: Option<f32>,
    /// Flesch-Kincaid Grade Level
    pub flesch_kincaid_grade: Option<f32>,
    /// Gunning Fog Index
    pub gunning_fog_index: Option<f32>,
    /// SMOG Index
    pub smog_index: Option<f32>,
}

/// Citation requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationRequirements {
    /// Citation style
    pub citation_style: CitationStyle,
    /// Minimum number of sources
    pub min_sources: u32,
    /// Source quality requirements
    pub source_quality: SourceQualityRequirements,
    /// Include bibliography
    pub include_bibliography: bool,
}

/// Citation style options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationStyle {
    Apa,
    Mla,
    Chicago,
    Harvard,
    Ieee,
    Custom(String),
}

/// Source quality requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceQualityRequirements {
    /// Accepted source types
    pub accepted_source_types: Vec<SourceType>,
    /// Minimum publication recency (days)
    pub min_recency_days: Option<u32>,
    /// Require peer review
    pub require_peer_review: Option<bool>,
    /// Domain authority threshold
    pub min_domain_authority: Option<u32>,
}

/// Source type options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    AcademicJournal,
    News,
    Government,
    Industry,
    Blog,
    Book,
    Website,
    Research,
    Custom(String),
}

/// Performance constraints for blog generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConstraints {
    /// Maximum total execution time (seconds)
    pub max_execution_time: u32,
    /// Maximum content generation time (seconds)
    pub max_content_generation_time: u32,
    /// Maximum image generation time (seconds)
    pub max_image_generation_time: u32,
    /// Maximum quality check time (seconds)
    pub max_quality_check_time: u32,
    /// Enable parallel processing
    pub enable_parallel_processing: bool,
    /// Processing priority
    pub processing_priority: ProcessingPriority,
    /// Resource allocation limits
    pub resource_limits: Option<ResourceAllocationLimits>,
}

/// Processing priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingPriority {
    Low,
    Normal,
    High,
    Critical,
    RealTime,
}

/// Resource allocation limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocationLimits {
    /// Maximum CPU usage (percentage)
    pub max_cpu_usage: f32,
    /// Maximum memory usage (MB)
    pub max_memory_mb: u32,
    /// Maximum network bandwidth (MB/s)
    pub max_bandwidth_mbps: f32,
    /// Maximum concurrent operations
    pub max_concurrent_operations: u32,
}

/// Blog post workflow specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogPostWorkflow {
    /// Workflow identifier
    pub workflow_id: Uuid,
    /// Blog intent details
    pub blog_intent: BlogPostIntent,
    /// Workflow steps
    pub steps: Vec<BlogWorkflowStep>,
    /// Dependencies between steps
    pub dependencies: Vec<WorkflowDependency>,
    /// Estimated execution time
    pub estimated_duration: Duration,
    /// Estimated cost
    pub estimated_cost: f64,
    /// Provider preferences
    pub provider_preferences: Vec<ProviderPreference>,
    /// Quality validation checkpoints
    pub quality_checkpoints: Vec<QualityCheckpoint>,
}

/// Blog-specific workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlogWorkflowStep {
    /// Step identifier
    pub step_id: Uuid,
    /// Step type
    pub step_type: BlogWorkflowStepType,
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Function calls for this step
    pub function_calls: Vec<FunctionCall>,
    /// Parallel execution allowed
    pub parallel_execution: bool,
    /// Step timeout
    pub timeout_seconds: Option<u64>,
    /// Quality requirements for this step
    pub quality_requirements: Option<StepQualityRequirements>,
    /// Retry configuration
    pub retry_config: Option<StepRetryConfig>,
}

/// Blog workflow step types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlogWorkflowStepType {
    ContentOutlineGeneration,
    ContentWriting,
    ContentEnhancement,
    SeoOptimization,
    FactChecking,
    GrammarCheck,
    PlagiarismCheck,
    ReadabilityAnalysis,
    FeaturedImageGeneration,
    InContentImageGeneration,
    ImageOptimization,
    MetaDataGeneration,
    QualityValidation,
    BrandComplianceCheck,
    FinalAssembly,
    Publishing,
    SocialMediaGeneration,
    AnalyticsSetup,
}

/// Workflow dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDependency {
    /// Prerequisite step ID
    pub prerequisite_step: Uuid,
    /// Dependent step ID
    pub dependent_step: Uuid,
    /// Dependency type
    pub dependency_type: DependencyType,
    /// Data transfer specification
    pub data_transfer: Option<DataTransferSpec>,
}

/// Dependency type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Sequential,
    DataDependency,
    ResourceDependency,
    QualityGate,
    ConditionalDependency,
}

/// Data transfer specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTransferSpec {
    /// Data format
    pub format: String,
    /// Data size estimate (bytes)
    pub estimated_size: u64,
    /// Compression enabled
    pub compression_enabled: bool,
    /// Encryption required
    pub encryption_required: bool,
}

/// Quality checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheckpoint {
    /// Checkpoint identifier
    pub checkpoint_id: Uuid,
    /// Checkpoint name
    pub name: String,
    /// Quality criteria
    pub criteria: Vec<QualityCriterion>,
    /// Failure action
    pub failure_action: QualityFailureAction,
    /// Checkpoint timing
    pub timing: CheckpointTiming,
}

/// Quality criterion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCriterion {
    /// Criterion name
    pub name: String,
    /// Criterion type
    pub criterion_type: QualityCriterionType,
    /// Target value
    pub target_value: f32,
    /// Threshold value
    pub threshold_value: f32,
    /// Weight in overall score
    pub weight: f32,
}

/// Quality criterion types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityCriterionType {
    ContentQuality,
    GrammarAccuracy,
    ReadabilityScore,
    SeoScore,
    BrandCompliance,
    OriginalityScore,
    FactualAccuracy,
    ImageQuality,
    Overall,
}

/// Quality failure action
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityFailureAction {
    Retry,
    Enhance,
    Regenerate,
    ManualReview,
    Abort,
    Continue,
}

/// Checkpoint timing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointTiming {
    BeforeStep(Uuid),
    AfterStep(Uuid),
    Intermediate,
    Final,
}

/// Step quality requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepQualityRequirements {
    /// Minimum quality score for this step
    pub min_quality_score: f32,
    /// Step-specific validation rules
    pub validation_rules: Vec<String>,
    /// Performance requirements
    pub performance_requirements: Option<StepPerformanceRequirements>,
}

/// Step performance requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepPerformanceRequirements {
    /// Maximum execution time for this step
    pub max_execution_time: u32,
    /// Memory limit for this step
    pub memory_limit_mb: Option<u32>,
    /// CPU priority
    pub cpu_priority: Option<ProcessingPriority>,
}

/// Step retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries (milliseconds)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Retry conditions
    pub retry_conditions: Vec<RetryCondition>,
}

/// Retry condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryCondition {
    /// Error type to retry on
    pub error_type: String,
    /// Error message pattern
    pub error_pattern: Option<String>,
    /// Quality threshold for retry
    pub quality_threshold: Option<f32>,
}

/// Default implementations
impl Default for WordCountTarget {
    fn default() -> Self {
        Self {
            min: 700,
            target: 800,
            max: 1200,
            flexibility: 0.1,
        }
    }
}

impl Default for PerformanceConstraints {
    fn default() -> Self {
        Self {
            max_execution_time: 45,
            max_content_generation_time: 25,
            max_image_generation_time: 20,
            max_quality_check_time: 10,
            enable_parallel_processing: true,
            processing_priority: ProcessingPriority::Normal,
            resource_limits: None,
        }
    }
}

impl Default for QualityRequirements {
    fn default() -> Self {
        Self {
            min_quality_score: 4.0,
            grammar_check: true,
            plagiarism_check: true,
            fact_checking: None,
            readability: None,
            brand_compliance: true,
            originality_threshold: 0.8,
            citation_requirements: None,
        }
    }
}

impl Default for ImageRequirements {
    fn default() -> Self {
        Self {
            featured_image_required: true,
            featured_image_specs: Some(ImageSpecifications {
                dimensions: ImageDimensions {
                    width: 1200,
                    height: 630,
                    aspect_ratio: "1.91:1".to_string(),
                },
                format: ImageFormat::Jpeg,
                quality: ImageQuality::High,
                max_file_size_kb: Some(500),
            }),
            in_content_images: None,
            alt_text_requirements: Some(AltTextRequirements {
                max_length: 125,
                include_keywords: true,
                descriptive_style: AltTextStyle::Descriptive,
            }),
            style_preferences: None,
        }
    }
}
