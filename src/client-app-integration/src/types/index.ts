// Types for AI-CORE Client App Integration

// Re-export Axios types for convenience
export type { AxiosInstance, AxiosResponse } from "axios";

export interface WorkflowRequest {
    intent: string;
    workflow_type?: string;
    client_context?: Record<string, unknown>;
}

export interface WorkflowResponse {
    workflow_id: string;
    status: string;
    message: string;
    estimated_duration?: number;
}

export interface WorkflowStatusResponse {
    workflow_id: string;
    status: string;
    progress: number;
    current_step: string | null;
    steps?: WorkflowStep[];
    results?: any;
    error?: string | null;
}

export enum WorkflowStatus {
    CREATED = "created",
    STARTING = "starting",
    RUNNING = "running",
    PARSING_INTENT = "parsing_intent",
    CREATING_WORKFLOW = "creating_workflow",
    EXECUTING_WORKFLOW = "executing_workflow",
    GENERATING_CONTENT = "generating_content",
    CREATING_IMAGE = "creating_image",
    VALIDATING_QUALITY = "validating_quality",
    COMPLETED = "completed",
    FAILED = "failed",
}

export interface WorkflowStep {
    id: string;
    name: string;
    status: StepStatus;
    started_at?: string;
    completed_at?: string;
    duration_ms?: number;
    details?: string;
    progress?: number;
}

export enum StepStatus {
    PENDING = "pending",
    ACTIVE = "active",
    COMPLETED = "completed",
    FAILED = "failed",
}

export interface WorkflowResult {
    content?: BlogPostResult;
    images?: ImageResult[];
    metadata: ResultMetadata;
    quality_score: number;
    execution_metrics: ExecutionMetrics;
}

export interface BlogPostResult {
    title: string;
    content: string;
    summary: string;
    word_count: number;
    reading_time: number;
    seo_keywords: string[];
    featured_image_url?: string;
    meta_description: string;
    tags: string[];
}

export interface ImageResult {
    url: string;
    alt_text: string;
    caption?: string;
    width: number;
    height: number;
    format: string;
    size_bytes: number;
}

export interface ResultMetadata {
    generated_at: string;
    model_used: string;
    tokens_consumed: number;
    cost_usd: number;
    processing_time_ms: number;
}

export interface ExecutionMetrics {
    total_duration_ms: number;
    api_calls_made: number;
    tokens_consumed: number;
    cost_breakdown: CostBreakdown;
    performance_score: number;
}

export interface CostBreakdown {
    text_generation: number;
    image_generation: number;
    api_calls: number;
    total: number;
}

export interface WorkflowMetadata {
    client_id: string;
    session_id: string;
    user_agent: string;
    created_at: string;
    updated_at: string;
    federation_nodes: FederationNode[];
}

export interface FederationNode {
    id: string;
    name: string;
    status: NodeStatus;
    last_active: string;
    response_time_ms?: number;
}

export enum NodeStatus {
    IDLE = "idle",
    ACTIVE = "active",
    COMPLETED = "completed",
    ERROR = "error",
}

export interface ApiLogEntry {
    id: string;
    timestamp: string;
    level: LogLevel;
    message: string;
    details?: LogDetails;
    context?: Record<string, unknown>;
}

export enum LogLevel {
    DEBUG = "debug",
    INFO = "info",
    SUCCESS = "success",
    WARN = "warn",
    ERROR = "error",
}

export interface ServiceStatus {
    name: string;
    url: string;
    status: "healthy" | "unhealthy" | "unknown";
    type: "real" | "mock";
    version?: string;
    uptime?: number;
    lastChecked: Date;
    error?: string;
}

export interface ServicesStatusState {
    federation: ServiceStatus;
    intentParser: ServiceStatus;
    mcpManager: ServiceStatus;
    isChecking: boolean;
}

export interface LogDetails {
    method?: string;
    url?: string;
    status_code?: number;
    request_headers?: Record<string, string>;
    response_headers?: Record<string, string>;
    request_body?: unknown;
    response_body?: unknown;
    duration_ms?: number;
    error_message?: string;
}

export interface DemoScenario {
    id: string;
    title: string;
    icon: string;
    description: string;
    example_prompt: string;
    expected_outcome: string;
    workflow_type: string;
}

export interface ClientMetrics {
    total_requests: number;
    successful_requests: number;
    failed_requests: number;
    average_execution_time_ms: number;
    average_quality_score: number;
    total_cost_usd: number;
    cost_savings_usd: number;
    uptime_percentage: number;
    last_updated: string;
}

export interface WebSocketMessage {
    type: MessageType;
    workflow_id: string;
    data: unknown;
    timestamp: string;
}

export enum MessageType {
    WORKFLOW_STARTED = "workflow_started",
    STEP_UPDATED = "step_updated",
    PROGRESS_UPDATED = "progress_updated",
    LOG_ENTRY = "log_entry",
    WORKFLOW_COMPLETED = "workflow_completed",
    WORKFLOW_FAILED = "workflow_failed",
    FEDERATION_NODE_UPDATED = "federation_node_updated",
}

export interface ApiConfig {
    baseUrl: string;
    apiKey: string;
    timeout: number;
    retries: number;
    websocketUrl: string;
}

export interface AppState {
    isLoading: boolean;
    currentWorkflow: WorkflowStatusResponse | null;
    logs: ApiLogEntry[];
    metrics: ClientMetrics;
    error: string | null;
    success: string | null;
    isConnected: boolean;
    showExamples: boolean;
    selectedTab: string;
    servicesStatus: ServicesStatusState;
}

export interface HttpResponse<T = unknown> {
    data: T;
    status: number;
    statusText: string;
    headers: Record<string, string>;
    config: {
        method: string;
        url: string;
        headers: Record<string, string>;
        data?: unknown;
    };
    duration: number;
}

// Error types
export interface ApiError {
    code: string;
    message: string;
    details?: unknown;
    status: number;
    timestamp: string;
}

export interface ValidationError {
    field: string;
    message: string;
    code: string;
}

// Component props types
export interface HeaderProps {
    title?: string;
    subtitle?: string;
}

export interface DemoInputProps {
    onStartDemo: (prompt: string) => void;
    onLoadExamples: () => void;
    onClearDemo: () => void;
    isLoading: boolean;
    error: string | null;
    success: string | null;
}

export interface ScenarioGridProps {
    scenarios: DemoScenario[];
    onSelectScenario: (scenario: DemoScenario) => void;
    isVisible: boolean;
}

export interface ProgressSectionProps {
    workflow: WorkflowStatusResponse | null;
    isVisible: boolean;
    logs: ApiLogEntry[];
}

export interface ResultsSectionProps {
    workflow: WorkflowStatusResponse | null;
    isVisible: boolean;
    selectedTab: string;
    onTabChange: (tab: string) => void;
}

export interface ExecutionLogsProps {
    logs: ApiLogEntry[];
}

export interface FederationVisualizationProps {
    nodes: FederationNode[];
}

export interface MetricsDisplayProps {
    metrics: ClientMetrics;
}
