/**
 * AI-CORE Client SDK - Main Entry Point
 *
 * Official TypeScript/JavaScript client for the AI-CORE SaaS platform.
 * Provides comprehensive blog post automation with real-time progress tracking,
 * quality validation, and enterprise-grade error handling.
 *
 * @version 1.0.0
 * @author AI-CORE Engineering Team
 *
 * @example
 * ```typescript
 * import { AICoreClient } from '@ai-core/client';
 *
 * const client = new AICoreClient({
 *   apiKey: 'your-api-key',
 *   baseURL: 'https://api.ai-core.com/v1'
 * });
 *
 * const blogPost = await client.generateBlogPost({
 *   topic: 'artificial intelligence in healthcare',
 *   audience: 'healthcare professionals',
 *   wordCount: 1000
 * });
 *
 * console.log(`Generated: ${blogPost.blogPost?.title}`);
 * ```
 */

// ===== MAIN CLIENT EXPORTS =====
export { AICoreClient, createClient, default } from './client';

// ===== TYPE EXPORTS =====
export type {
    ApiResponse,
    // Utility types
    Awaitable, BlogPostOutput,
    // Content generation types
    BlogPostRequest,
    BlogPostResponse, BlogPreferences,
    // Brand and preferences
    BrandProfile, CapabilitiesResponse,
    // Core client types
    ClientConfig, ClientEvents, ClientProfile,
    ClientProfileResponse,
    ClientProfileUpdateRequest,
    // Authentication types
    ClientRegistrationRequest,
    ClientRegistrationResponse, ErrorDetails, ErrorResponse, ExecutionMetrics, ExecutionOptions,
    // System types
    HealthResponse, ImageOutput, IntegrationStatus, PaginatedQuery,
    PaginatedResponse, PartialExcept, QualityScores,
    // Usage and limits
    RateLimits, RequestConfig, RequiredKeys, RetryConfig, SeoMetadata, UsageStats, WebhookConfig,
    WebhookEvent,
    // Event types
    WebSocketMessage, WordCountRange, WorkflowCancelResponse, WorkflowCompletedEvent, WorkflowEvent, WorkflowFailedEvent, WorkflowProgressEvent, WorkflowsListQuery,
    WorkflowsListResponse, WorkflowStartedResponse,
    // Workflow types
    WorkflowStatus, WorkflowStatusResponse, WorkflowSummary
} from './types';

// ===== TYPE GUARD EXPORTS =====
export {
    isApiResponse, isErrorResponse,
    isWorkflowEvent
} from './types';

// ===== CONSTANT EXPORTS =====
export {
    CLIENT_TIERS, DEFAULT_BASE_URL, DEFAULT_MAX_RETRIES,
    DEFAULT_RETRY_DELAY, DEFAULT_TIMEOUT, EXECUTION_PRIORITIES,
    IMAGE_FORMATS, MAX_EXECUTION_TIME,
    MIN_QUALITY_SCORE,
    WEBHOOK_EVENTS,
    WORKFLOW_STATUSES
} from './types';

// ===== VERSION EXPORT =====
export const VERSION = '1.0.0';

// ===== QUICK START UTILITIES =====

/**
 * Quick client creation with minimal configuration
 */
export function createQuickClient(apiKey: string, baseURL?: string): AICoreClient {
  return new AICoreClient({
    apiKey,
    baseURL: baseURL || DEFAULT_BASE_URL,
    timeout: DEFAULT_TIMEOUT,
    maxRetries: DEFAULT_MAX_RETRIES,
    debug: false
  });
}

/**
 * Generate a blog post with simplified parameters
 */
export async function quickBlogPost(
  apiKey: string,
  topic: string,
  options?: {
    audience?: string;
    wordCount?: number;
    tone?: string;
    baseURL?: string;
  }
): Promise<BlogPostResponse> {
  const client = createQuickClient(apiKey, options?.baseURL);

  try {
    return await client.generateBlogPost({
      topic,
      audience: options?.audience,
      wordCount: options?.wordCount || 1000,
      tone: options?.tone || 'professional'
    });
  } finally {
    client.destroy();
  }
}

/**
 * Register a new client with simplified parameters
 */
export async function quickRegister(
  name: string,
  email: string,
  company: string,
  options?: {
    tier?: 'starter' | 'professional' | 'enterprise';
    baseURL?: string;
  }
): Promise<ClientRegistrationResponse> {
  return AICoreClient.register({
    name,
    email,
    company,
    tier: options?.tier || 'professional'
  });
}

// ===== ERROR CLASSES =====

/**
 * Base error class for AI-CORE SDK
 */
export class AICoreError extends Error {
  public readonly code: string;
  public readonly status?: number;
  public readonly details?: Record<string, unknown>;
  public readonly suggestions?: string[];

  constructor(
    message: string,
    code: string,
    status?: number,
    details?: Record<string, unknown>,
    suggestions?: string[]
  ) {
    super(message);
    this.name = 'AICoreError';
    this.code = code;
    this.status = status;
    this.details = details;
    this.suggestions = suggestions;
  }
}

/**
 * Authentication error
 */
export class AuthenticationError extends AICoreError {
  constructor(message = 'Authentication failed') {
    super(message, 'AUTHENTICATION_ERROR', 401);
    this.name = 'AuthenticationError';
  }
}

/**
 * Rate limit error
 */
export class RateLimitError extends AICoreError {
  public readonly retryAfter: number;

  constructor(message = 'Rate limit exceeded', retryAfter = 60) {
    super(message, 'RATE_LIMIT_ERROR', 429);
    this.name = 'RateLimitError';
    this.retryAfter = retryAfter;
  }
}

/**
 * Validation error
 */
export class ValidationError extends AICoreError {
  constructor(message: string, details?: Record<string, unknown>) {
    super(message, 'VALIDATION_ERROR', 400, details);
    this.name = 'ValidationError';
  }
}

/**
 * Workflow timeout error
 */
export class WorkflowTimeoutError extends AICoreError {
  public readonly workflowId: string;

  constructor(workflowId: string, timeoutMs: number) {
    super(`Workflow ${workflowId} timed out after ${timeoutMs}ms`, 'WORKFLOW_TIMEOUT');
    this.name = 'WorkflowTimeoutError';
    this.workflowId = workflowId;
  }
}

// ===== UTILITY FUNCTIONS =====

/**
 * Validate an API key format
 */
export function isValidApiKey(apiKey: string): boolean {
  return /^ak_[a-zA-Z0-9]{32,}$/.test(apiKey);
}

/**
 * Estimate reading time for content
 */
export function estimateReadingTime(content: string, wordsPerMinute = 200): number {
  const wordCount = content.trim().split(/\s+/).length;
  return Math.ceil(wordCount / wordsPerMinute);
}

/**
 * Calculate quality score from individual metrics
 */
export function calculateOverallQuality(scores: Partial<QualityScores>): number {
  const weights = {
    contentQuality: 0.3,
    grammarScore: 0.2,
    readabilityScore: 0.2,
    seoScore: 0.15,
    brandComplianceScore: 0.1,
    originalityScore: 0.05
  };

  let weightedSum = 0;
  let totalWeight = 0;

  Object.entries(weights).forEach(([key, weight]) => {
    const score = scores[key as keyof QualityScores];
    if (typeof score === 'number') {
      weightedSum += score * weight;
      totalWeight += weight;
    }
  });

  return totalWeight > 0 ? Number((weightedSum / totalWeight).toFixed(2)) : 0;
}

/**
 * Format execution time for display
 */
export function formatExecutionTime(milliseconds: number): string {
  if (milliseconds < 1000) {
    return `${milliseconds}ms`;
  }

  const seconds = Math.floor(milliseconds / 1000);
  const remainingMs = milliseconds % 1000;

  if (seconds < 60) {
    return remainingMs > 0 ? `${seconds}.${Math.floor(remainingMs / 100)}s` : `${seconds}s`;
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;

  return `${minutes}m ${remainingSeconds}s`;
}

/**
 * Check if SDK version is compatible with API
 */
export function checkCompatibility(apiVersion: string): boolean {
  const sdkMajor = parseInt(VERSION.split('.')[0]!);
  const apiMajor = parseInt(apiVersion.split('.')[0]!);

  return sdkMajor === apiMajor;
}

// ===== MODULE METADATA =====
export const SDK_INFO = {
  name: '@ai-core/client',
  version: VERSION,
  author: 'AI-CORE Engineering Team',
  homepage: 'https://docs.ai-core.com/sdk',
  repository: 'https://github.com/ai-core/client-sdk',
  license: 'MIT'
} as const;
