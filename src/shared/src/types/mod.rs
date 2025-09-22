//! Shared type definitions for the AI-CORE Intelligent Automation Platform
//!
//! This module provides all the core type definitions used across the entire
//! AI-CORE platform, ensuring consistency and type safety between all services.

pub mod api;
pub mod core;
pub mod events;

// Re-export core types
pub use core::{
    AiMetadata,
    ApiError,
    AuthConfig,
    AuthCredentials,

    AuthType,
    BasicAuthConfig,
    // Billing
    BillingCycle,
    // Campaign Management
    CampaignBudget,
    CampaignMilestone,
    CampaignPerformance,
    CampaignStatus,

    CampaignTimeline,
    CampaignType,
    ClientInfo,
    ClientStatus,
    // Content Management
    ContentItem,
    ContentMetrics,
    ContentSentiment,
    ContentStatus,
    ContentType,
    ExternalApiConfig,
    // Federation and MCP
    FederatedClient,
    InvoiceStatus,
    McpError,
    McpRequest,
    McpResponse,
    McpServer,
    McpServerInfo,
    McpServerStatus,

    McpTool,
    PaginationInfo,
    ParsedIntent,
    PerformanceMetrics,
    Permission,
    // Additional types
    ProgressUpdate,
    ProviderAllocation,
    RateLimitConfig,
    RateLimits,
    RetryConfig,

    SeoMetadata,

    // System
    ServiceHealth,
    StepStatus,
    StepType,
    SubscriptionStatus,
    SubscriptionTier,
    SuccessResponse,

    SystemInfo,
    TimeFrame,
    TokenClaims,
    TrendDataPoint,

    UsageAnalytics,
    UsageRecord,

    // User and Authentication
    User,
    UserStatus,
    ValidationError,
    // Workflows
    Workflow,
    // Analytics
    WorkflowAnalytics,
    WorkflowDefinition,
    WorkflowDefinitionStatus,
    WorkflowExecution,
    WorkflowExecutionStatus,
    WorkflowPriority,
    WorkflowProgress,
    WorkflowStatus,
    WorkflowStep,
    WorkflowStepDefinition,
    WorkflowTrigger,
    WorkflowType,
};

// Re-export API types with prefixes to avoid conflicts
pub use api::{
    AggregationType,
    AlertSeverity,
    AlertType,
    AnalyticsDataPoint,
    AnalyticsMetric,
    // Analytics API
    AnalyticsQuery,
    AnalyticsResponse,
    AnalyticsSummary,
    ApiResponse,
    BudgetConstraints,
    BulkNotificationRequest,
    BulkNotificationResponse,
    BulkNotificationResult,
    BulkOperationStatus,
    ChannelStats,
    ComponentHealth,
    // Automation API
    CreateAutomationRequest,
    CreateAutomationResponse,
    CreateNotificationRequest,
    CreateSubscriptionRequest,
    CreateTemplateRequest,
    // Workflow API
    CreateWorkflowRequest as ApiCreateWorkflowRequest,
    CreateWorkflowResponse,
    DeliveryAttempt,
    DeliveryStatus,
    ErrorResponse as ApiErrorResponse,
    ExecuteMCPToolRequest,
    ExecuteMCPToolResponse,

    ExecuteWorkflowRequest,
    ExecuteWorkflowResponse,
    ExecutionMode,

    FederatedAutomationRequest,
    FederationContext,
    // Health and System API
    HealthCheckResponse,
    HealthStatus as ApiHealthStatus,
    ListMCPServersQuery,
    ListMCPServersResponse,
    ListWorkflowsQuery,
    ListWorkflowsResponse,
    // Authentication API
    LoginRequest,
    LoginResponse,
    LogoutRequest,
    MCPToolRequest,
    MetricsResponse,
    NotificationChannel,
    NotificationFrequency,
    NotificationPreferences,
    NotificationPriority,
    NotificationResponse,
    NotificationStats,
    NotificationStatus,
    NotificationSubscription,
    NotificationTemplate,
    NotificationType,
    PaginatedResponse,
    ParseIntentRequest,
    ParseIntentResponse,

    PasswordResetConfirmRequest,

    PasswordResetRequest,
    ProviderSummary,
    QualityPreferences,
    QuietHours,
    RefreshTokenRequest,
    RefreshTokenResponse,
    // Federation API
    RegisterClientRequest,
    RegisterClientResponse,
    // MCP API
    RegisterMCPServerRequest,
    RegisterMCPServerResponse,
    RegisterRequest,
    RegisterResponse,
    // Common API types
    SortOrder,
    SystemMetrics,

    TemplateVariable,
    TrendAnalysis,
    TrendDirection,

    UpdateSubscriptionRequest,
    UpdateTemplateRequest,
    UpdateWorkflowRequest,
    // User Preferences
    UserPreferences,
    ValidationErrorResponse,

    VariableType,
    // WebSocket
    WebSocketMessage,
    WebSocketMessageType,
    WorkflowStatusResponse,

    WorkflowStepRequest,
};

// Re-export event types
pub use events::{
    BackupType,
    CancelledBy,

    DatabaseType,
    DeletionType,

    DeregistrationReason,

    // Core event structure
    DomainEvent,
    EventBatch,
    EventEnvelope,
    EventMetadata,
    EventSubscription,
    // Federation events
    FederationEvent,

    HandlerType,

    // Integration events
    IntegrationEvent,
    // Intent events
    IntentEvent,
    LogoutType,
    MaintenanceType,
    ParseErrorType,

    PauseReason,
    RateLimitType,
    ResourceType,
    // Security events
    SecurityEvent,
    SecuritySeverity,

    ShutdownReason,
    SuspiciousActivityType,
    // System events
    SystemEvent,
    TokenType,
    // User events
    UserEvent,
    // Workflow events
    WorkflowEvent,
};

// Re-export error types from core to avoid conflicts
pub use core::{ErrorDetail, ErrorResponse, ErrorType};
