/**
 * AI-CORE Client SDK - Type Definitions
 *
 * Comprehensive TypeScript types for the AI-CORE SaaS platform integration.
 * These types match the OpenAPI 3.0 specification for complete type safety.
 *
 * @version 1.0.0
 * @author AI-CORE Engineering Team
 */

// ===== CORE CLIENT TYPES =====

export interface ClientConfig {
  /** API base URL */
  baseURL?: string;
  /** API key for authentication */
  apiKey: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Maximum retry attempts */
  maxRetries?: number;
  /** Enable debug logging */
  debug?: boolean;
  /** Custom headers to include in requests */
  headers?: Record<string, string>;
}

export interface ApiResponse<T> {
  data: T;
  status: number;
  statusText: string;
  headers: Record<string, string>;
  requestId?: string;
}

export interface ErrorResponse {
  error: ErrorDetails;
  timestamp: string;
  requestId?: string;
}

export interface ErrorDetails {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  suggestions?: string[];
}

// ===== AUTHENTICATION TYPES =====

export interface ClientRegistrationRequest {
  name: string;
  description?: string;
  email: string;
  company: string;
  tier: 'starter' | 'professional' | 'enterprise';
  brandProfile?: BrandProfile;
  blogPreferences?: BlogPreferences;
  webhookConfig?: WebhookConfig;
}

export interface ClientRegistrationResponse {
  clientId: string;
  apiKey: string;
  profile: ClientProfile;
  message: string;
  nextSteps: string[];
}

export interface ClientProfile {
  clientId: string;
  name: string;
  description?: string;
  tier: 'starter' | 'professional' | 'enterprise';
  brandProfile?: BrandProfile;
  blogPreferences?: BlogPreferences;
  rateLimits: RateLimits;
  createdAt: string;
}

export interface ClientProfileResponse {
  profile: ClientProfile;
  usageStats: UsageStats;
  integrationStatus: IntegrationStatus;
}

export interface ClientProfileUpdateRequest {
  brandProfile?: BrandProfile;
  blogPreferences?: BlogPreferences;
  webhookConfig?: WebhookConfig;
}

// ===== BRAND & PREFERENCES TYPES =====

export interface BrandProfile {
  brandName: string;
  brandVoice: string;
  brandValues?: string[];
  colorPalette?: string[];
  industryContext?: string;
}

export interface BlogPreferences {
  defaultWordCount?: WordCountRange;
  defaultTone?: string;
  targetAudience?: string;
  seoEnabled?: boolean;
  imageGenerationEnabled?: boolean;
}

export interface WordCountRange {
  min: number;
  max: number;
  target: number;
}

export interface WebhookConfig {
  url: string;
  events: WebhookEvent[];
  timeoutSeconds?: number;
}

export type WebhookEvent =
  | 'workflow.started'
  | 'workflow.progress'
  | 'workflow.completed'
  | 'workflow.failed'
  | 'workflow.cancelled';

// ===== CONTENT GENERATION TYPES =====

export interface BlogPostRequest {
  topic: string;
  audience?: string;
  tone?: string;
  wordCount?: number;
  keywords?: string[];
  customInstructions?: string;
  brandVoiceOverride?: string;
  executionOptions?: ExecutionOptions;
  callbackUrl?: string;
}

export interface ExecutionOptions {
  parallelProcessing?: boolean;
  priority?: 'low' | 'normal' | 'high' | 'critical' | 'realtime';
  maxExecutionTime?: number;
  qualityThreshold?: number;
  realTimeUpdates?: boolean;
}

export interface BlogPostResponse {
  workflowId: string;
  status: WorkflowStatus;
  blogPost?: BlogPostOutput;
  metrics?: ExecutionMetrics;
  qualityScores?: QualityScores;
  estimatedCompletion?: string;
  progress?: number;
  error?: ErrorDetails;
}

export interface BlogPostOutput {
  title: string;
  content: string;
  contentMarkdown: string;
  featuredImage?: ImageOutput;
  metaDescription: string;
  seoMetadata: SeoMetadata;
  wordCount: number;
  readingTime: number;
  generatedAt: string;
}

export interface ImageOutput {
  url: string;
  altText: string;
  width: number;
  height: number;
  fileSize: number;
  format: 'jpg' | 'png' | 'webp';
}

export interface SeoMetadata {
  primaryKeywords: string[];
  secondaryKeywords: string[];
  keywordDensity: Record<string, number>;
  seoScore: number;
  metaTags: Record<string, string>;
}

export interface ExecutionMetrics {
  totalExecutionTimeMs: number;
  contentGenerationTimeMs: number;
  imageGenerationTimeMs: number;
  qualityValidationTimeMs: number;
  totalCost: number;
  currency: string;
}

export interface QualityScores {
  overallScore: number;
  contentQuality: number;
  grammarScore: number;
  readabilityScore: number;
  seoScore: number;
  brandComplianceScore: number;
  originalityScore: number;
}

// ===== WORKFLOW TYPES =====

export type WorkflowStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface WorkflowStartedResponse {
  workflowId: string;
  status: 'pending' | 'running';
  estimatedCompletion: string;
  progressUrl: string;
  websocketUrl?: string;
}

export interface WorkflowStatusResponse {
  workflowId: string;
  status: WorkflowStatus;
  progress: number;
  startedAt: string;
  completedAt?: string;
  estimatedCompletion?: string;
  currentStep?: string;
  metrics?: ExecutionMetrics;
  qualityScores?: QualityScores;
  error?: ErrorDetails;
}

export interface WorkflowCancelResponse {
  workflowId: string;
  status: 'cancelled';
  cancelledAt: string;
  reason?: string;
}

export interface WorkflowsListQuery {
  limit?: number;
  offset?: number;
  status?: WorkflowStatus;
  fromDate?: string;
  toDate?: string;
}

export interface WorkflowsListResponse {
  workflows: WorkflowSummary[];
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
}

export interface WorkflowSummary {
  workflowId: string;
  topic: string;
  status: WorkflowStatus;
  createdAt: string;
  completedAt?: string;
  executionTimeMs?: number;
  qualityScore?: number;
  wordCount?: number;
}

// ===== USAGE & LIMITS TYPES =====

export interface RateLimits {
  requestsPerMinute: number;
  requestsPerHour: number;
  requestsPerDay: number;
}

export interface UsageStats {
  totalWorkflows: number;
  successfulWorkflows: number;
  failedWorkflows: number;
  averageExecutionTime: number;
  averageQualityScore: number;
  totalContentGenerated: number;
  currentMonthUsage: number;
}

export interface IntegrationStatus {
  status: 'active' | 'inactive' | 'suspended' | 'testing';
  lastActivity: string;
  healthScore: number;
  issues?: string[];
}

// ===== SYSTEM TYPES =====

export interface HealthResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  timestamp: string;
  services: Record<string, 'healthy' | 'degraded' | 'unhealthy'>;
  metrics?: {
    averageResponseTime: number;
    activeWorkflows: number;
    queueLength: number;
  };
}

export interface CapabilitiesResponse {
  features: string[];
  limits: RateLimits;
  supportedFormats: {
    input: string[];
    output: string[];
  };
  performanceTargets: {
    maxExecutionTime: number;
    minQualityScore: number;
    maxResponseTime: number;
  };
}

// ===== EVENT TYPES =====

export interface WebSocketMessage {
  type: string;
  workflowId: string;
  timestamp: string;
  data: unknown;
}

export interface WorkflowProgressEvent extends WebSocketMessage {
  type: 'workflow.progress';
  data: {
    progress: number;
    currentStep: string;
    estimatedTimeRemaining: number;
  };
}

export interface WorkflowCompletedEvent extends WebSocketMessage {
  type: 'workflow.completed';
  data: {
    blogPost: BlogPostOutput;
    metrics: ExecutionMetrics;
    qualityScores: QualityScores;
  };
}

export interface WorkflowFailedEvent extends WebSocketMessage {
  type: 'workflow.failed';
  data: {
    error: ErrorDetails;
    partialResults?: Partial<BlogPostOutput>;
  };
}

export type WorkflowEvent =
  | WorkflowProgressEvent
  | WorkflowCompletedEvent
  | WorkflowFailedEvent;

// ===== CLIENT EVENTS =====

export interface ClientEvents {
  'workflow.started': (event: { workflowId: string }) => void;
  'workflow.progress': (event: WorkflowProgressEvent['data'] & { workflowId: string }) => void;
  'workflow.completed': (event: WorkflowCompletedEvent['data'] & { workflowId: string }) => void;
  'workflow.failed': (event: WorkflowFailedEvent['data'] & { workflowId: string }) => void;
  'workflow.cancelled': (event: { workflowId: string; reason?: string }) => void;
  'connection.open': () => void;
  'connection.close': () => void;
  'connection.error': (error: Error) => void;
  'rate.limit': (event: { retryAfter: number; limit: string }) => void;
  'error': (error: Error) => void;
}

// ===== RETRY & ERROR HANDLING =====

export interface RetryConfig {
  maxRetries: number;
  baseDelay: number;
  maxDelay: number;
  backoffFactor: number;
  retryCondition?: (error: unknown) => boolean;
}

export interface RequestConfig {
  timeout?: number;
  retries?: number;
  retryDelay?: number;
  signal?: AbortSignal;
  onProgress?: (progress: number) => void;
}

// ===== UTILITY TYPES =====

export type Awaitable<T> = T | Promise<T>;

export type RequiredKeys<T, K extends keyof T> = T & Required<Pick<T, K>>;

export type PartialExcept<T, K extends keyof T> = Partial<T> & Pick<T, K>;

export interface PaginatedQuery {
  limit?: number;
  offset?: number;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  offset: number;
  limit: number;
  hasMore: boolean;
}

// ===== TYPE GUARDS =====

export function isErrorResponse(response: unknown): response is ErrorResponse {
  return (
    typeof response === 'object' &&
    response !== null &&
    'error' in response &&
    typeof (response as ErrorResponse).error === 'object'
  );
}

export function isWorkflowEvent(event: unknown): event is WorkflowEvent {
  return (
    typeof event === 'object' &&
    event !== null &&
    'type' in event &&
    'workflowId' in event &&
    'timestamp' in event &&
    'data' in event
  );
}

export function isApiResponse<T>(response: unknown): response is ApiResponse<T> {
  return (
    typeof response === 'object' &&
    response !== null &&
    'data' in response &&
    'status' in response &&
    typeof (response as ApiResponse<T>).status === 'number'
  );
}

// ===== CONSTANTS =====

export const DEFAULT_BASE_URL = 'https://api.ai-core.com/v1';
export const DEFAULT_TIMEOUT = 30000;
export const DEFAULT_MAX_RETRIES = 3;
export const DEFAULT_RETRY_DELAY = 1000;
export const MAX_EXECUTION_TIME = 45000;
export const MIN_QUALITY_SCORE = 4.0;

export const WEBHOOK_EVENTS: WebhookEvent[] = [
  'workflow.started',
  'workflow.progress',
  'workflow.completed',
  'workflow.failed',
  'workflow.cancelled'
];

export const WORKFLOW_STATUSES: WorkflowStatus[] = [
  'pending',
  'running',
  'completed',
  'failed',
  'cancelled'
];

export const CLIENT_TIERS = ['starter', 'professional', 'enterprise'] as const;

export const EXECUTION_PRIORITIES = ['low', 'normal', 'high', 'critical', 'realtime'] as const;

export const IMAGE_FORMATS = ['jpg', 'png', 'webp'] as const;
