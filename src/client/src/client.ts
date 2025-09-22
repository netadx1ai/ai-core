/**
 * AI-CORE Client SDK - Main Client Implementation
 *
 * Official TypeScript/JavaScript client for the AI-CORE SaaS platform.
 * Provides comprehensive blog post automation with real-time progress tracking,
 * quality validation, and enterprise-grade error handling.
 *
 * @version 1.0.0
 * @author AI-CORE Engineering Team
 */

import axios, { AxiosError, AxiosInstance, AxiosResponse } from 'axios';
import { EventEmitter } from 'eventemitter3';
import WebSocket from 'ws';
import {
    ApiResponse,
    BlogPostRequest,
    BlogPostResponse,
    CapabilitiesResponse,
    ClientConfig,
    ClientEvents,
    ClientProfileResponse,
    ClientProfileUpdateRequest,
    ClientRegistrationRequest,
    ClientRegistrationResponse,
    DEFAULT_BASE_URL,
    DEFAULT_MAX_RETRIES,
    DEFAULT_RETRY_DELAY,
    DEFAULT_TIMEOUT,
    HealthResponse,
    RequestConfig,
    WorkflowCancelResponse,
    WorkflowStatusResponse,
    WorkflowsListQuery,
    WorkflowsListResponse,
    isErrorResponse,
    isWorkflowEvent
} from './types';

/**
 * Main AI-CORE client for SaaS platform integration
 */
export class AICoreClient extends EventEmitter<ClientEvents> {
  private readonly httpClient: AxiosInstance;
  private readonly config: Required<Omit<ClientConfig, 'headers'>> & { headers?: Record<string, string> };
  private wsConnection?: WebSocket;
  private reconnectAttempts = 0;
  private readonly maxReconnectAttempts = 5;

  constructor(config: ClientConfig) {
    super();

    // Validate required configuration
    if (!config.apiKey) {
      throw new Error('API key is required');
    }

    // Set up client configuration with defaults
    this.config = {
      baseURL: config.baseURL || DEFAULT_BASE_URL,
      apiKey: config.apiKey,
      timeout: config.timeout || DEFAULT_TIMEOUT,
      maxRetries: config.maxRetries || DEFAULT_MAX_RETRIES,
      debug: config.debug || false,
      headers: config.headers
    };

    // Create HTTP client instance
    this.httpClient = axios.create({
      baseURL: this.config.baseURL,
      timeout: this.config.timeout,
      headers: {
        'X-API-Key': this.config.apiKey,
        'Content-Type': 'application/json',
        'User-Agent': `@ai-core/client/1.0.0`,
        ...this.config.headers
      }
    });

    // Set up request interceptor for retry logic
    this.setupRequestInterceptors();

    // Set up response interceptor for error handling
    this.setupResponseInterceptors();
  }

  // ===== AUTHENTICATION METHODS =====

  /**
   * Register a new client with the AI-CORE platform
   */
  static async register(request: ClientRegistrationRequest): Promise<ClientRegistrationResponse> {
    const tempClient = axios.create({
      baseURL: DEFAULT_BASE_URL,
      timeout: DEFAULT_TIMEOUT
    });

    const response = await tempClient.post<ClientRegistrationResponse>('/auth/register', request);
    return response.data;
  }

  /**
   * Get current client profile and usage statistics
   */
  async getProfile(): Promise<ClientProfileResponse> {
    const response = await this.request<ClientProfileResponse>('GET', '/client/profile');
    return response.data;
  }

  /**
   * Update client profile and preferences
   */
  async updateProfile(update: ClientProfileUpdateRequest): Promise<ClientProfileResponse> {
    const response = await this.request<ClientProfileResponse>('PUT', '/client/profile', update);
    return response.data;
  }

  // ===== CONTENT GENERATION METHODS =====

  /**
   * Generate a blog post from natural language input
   *
   * @param request Blog post generation request
   * @param options Request-specific options
   * @returns Blog post response with content and metrics
   *
   * @example
   * ```typescript
   * const result = await client.generateBlogPost({
   *   topic: "artificial intelligence in healthcare",
   *   audience: "healthcare professionals",
   *   wordCount: 1000,
   *   keywords: ["AI", "healthcare", "machine learning"]
   * });
   *
   * console.log(`Generated: ${result.blogPost?.title}`);
   * console.log(`Quality Score: ${result.qualityScores?.overallScore}`);
   * console.log(`Execution Time: ${result.metrics?.totalExecutionTimeMs}ms`);
   * ```
   */
  async generateBlogPost(
    request: BlogPostRequest,
    options?: RequestConfig
  ): Promise<BlogPostResponse> {
    this.validateBlogPostRequest(request);

    const response = await this.request<BlogPostResponse>(
      'POST',
      '/content/blog-post',
      request,
      options
    );

    const result = response.data;

    // Set up real-time tracking if workflow is async
    if (result.status === 'pending' || result.status === 'running') {
      this.trackWorkflowProgress(result.workflowId);
    }

    return result;
  }

  /**
   * Generate multiple blog posts concurrently
   */
  async generateBlogPostsBatch(
    requests: BlogPostRequest[],
    options?: RequestConfig
  ): Promise<BlogPostResponse[]> {
    if (requests.length === 0) {
      throw new Error('At least one blog post request is required');
    }

    if (requests.length > 10) {
      throw new Error('Maximum 10 concurrent blog post requests allowed');
    }

    const promises = requests.map(request => this.generateBlogPost(request, options));
    return Promise.all(promises);
  }

  // ===== WORKFLOW MANAGEMENT METHODS =====

  /**
   * Get status and progress of a specific workflow
   */
  async getWorkflowStatus(
    workflowId: string,
    includeMetrics = false,
    includeQuality = false
  ): Promise<WorkflowStatusResponse> {
    const params = new URLSearchParams();
    if (includeMetrics) params.set('include_metrics', 'true');
    if (includeQuality) params.set('include_quality', 'true');

    const url = `/workflows/${workflowId}/status${params.toString() ? '?' + params.toString() : ''}`;
    const response = await this.request<WorkflowStatusResponse>('GET', url);
    return response.data;
  }

  /**
   * Cancel a running workflow
   */
  async cancelWorkflow(workflowId: string): Promise<WorkflowCancelResponse> {
    const response = await this.request<WorkflowCancelResponse>(
      'POST',
      `/workflows/${workflowId}/cancel`
    );
    return response.data;
  }

  /**
   * List workflows with optional filtering
   */
  async listWorkflows(query?: WorkflowsListQuery): Promise<WorkflowsListResponse> {
    const params = new URLSearchParams();
    if (query?.limit) params.set('limit', query.limit.toString());
    if (query?.offset) params.set('offset', query.offset.toString());
    if (query?.status) params.set('status', query.status);
    if (query?.fromDate) params.set('from_date', query.fromDate);
    if (query?.toDate) params.set('to_date', query.toDate);

    const url = `/workflows${params.toString() ? '?' + params.toString() : ''}`;
    const response = await this.request<WorkflowsListResponse>('GET', url);
    return response.data;
  }

  /**
   * Wait for a workflow to complete with optional timeout
   */
  async waitForWorkflow(
    workflowId: string,
    timeoutMs = 60000,
    pollIntervalMs = 2000
  ): Promise<WorkflowStatusResponse> {
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutMs) {
      const status = await this.getWorkflowStatus(workflowId, true, true);

      if (status.status === 'completed') {
        return status;
      }

      if (status.status === 'failed' || status.status === 'cancelled') {
        throw new Error(`Workflow ${workflowId} ${status.status}: ${status.error?.message || 'Unknown error'}`);
      }

      await this.sleep(pollIntervalMs);
    }

    throw new Error(`Workflow ${workflowId} timeout after ${timeoutMs}ms`);
  }

  // ===== REAL-TIME TRACKING METHODS =====

  /**
   * Connect to WebSocket for real-time workflow updates
   */
  async connectRealTime(workflowId?: string): Promise<void> {
    if (this.wsConnection && this.wsConnection.readyState === WebSocket.OPEN) {
      return; // Already connected
    }

    const wsUrl = this.config.baseURL.replace('http', 'ws') + '/ws';
    const url = workflowId ? `${wsUrl}?workflow_id=${workflowId}` : wsUrl;

    this.wsConnection = new WebSocket(url, {
      headers: {
        'X-API-Key': this.config.apiKey
      }
    });

    return new Promise((resolve, reject) => {
      if (!this.wsConnection) return reject(new Error('WebSocket creation failed'));

      this.wsConnection.onopen = () => {
        this.reconnectAttempts = 0;
        this.emit('connection.open');
        resolve();
      };

      this.wsConnection.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data.toString());
          this.handleWebSocketMessage(message);
        } catch (error) {
          this.emit('error', new Error(`Failed to parse WebSocket message: ${error}`));
        }
      };

      this.wsConnection.onclose = () => {
        this.emit('connection.close');
        this.attemptReconnect();
      };

      this.wsConnection.onerror = (error) => {
        this.emit('connection.error', new Error(`WebSocket error: ${error}`));
        reject(error);
      };
    });
  }

  /**
   * Disconnect from WebSocket
   */
  disconnectRealTime(): void {
    if (this.wsConnection) {
      this.wsConnection.close();
      this.wsConnection = undefined;
    }
  }

  /**
   * Track progress of a specific workflow
   */
  private async trackWorkflowProgress(workflowId: string): Promise<void> {
    try {
      await this.connectRealTime(workflowId);
    } catch (error) {
      // Fallback to polling if WebSocket fails
      this.pollWorkflowProgress(workflowId);
    }
  }

  // ===== SYSTEM METHODS =====

  /**
   * Check system health status
   */
  async getHealth(): Promise<HealthResponse> {
    const response = await this.request<HealthResponse>('GET', '/health');
    return response.data;
  }

  /**
   * Get system capabilities and limits
   */
  async getCapabilities(): Promise<CapabilitiesResponse> {
    const response = await this.request<CapabilitiesResponse>('GET', '/capabilities');
    return response.data;
  }

  // ===== UTILITY METHODS =====

  /**
   * Test API connectivity and authentication
   */
  async testConnection(): Promise<boolean> {
    try {
      await this.getHealth();
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get current rate limit status
   */
  async getRateLimitStatus(): Promise<{ remaining: number; reset: number; limit: number } | null> {
    try {
      const response = await this.httpClient.get('/client/profile');
      const headers = response.headers;

      if (headers['x-ratelimit-remaining']) {
        return {
          remaining: parseInt(headers['x-ratelimit-remaining']),
          reset: parseInt(headers['x-ratelimit-reset']),
          limit: parseInt(headers['x-ratelimit-limit'])
        };
      }

      return null;
    } catch {
      return null;
    }
  }

  /**
   * Destroy client and clean up resources
   */
  destroy(): void {
    this.disconnectRealTime();
    this.removeAllListeners();
  }

  // ===== PRIVATE HELPER METHODS =====

  private async request<T>(
    method: 'GET' | 'POST' | 'PUT' | 'DELETE',
    url: string,
    data?: unknown,
    options?: RequestConfig
  ): Promise<ApiResponse<T>> {
    try {
      const config = {
        method,
        url,
        data,
        timeout: options?.timeout || this.config.timeout,
        signal: options?.signal
      };

      const response: AxiosResponse<T> = await this.httpClient.request(config);

      return {
        data: response.data,
        status: response.status,
        statusText: response.statusText,
        headers: response.headers as Record<string, string>,
        requestId: response.headers['x-request-id']
      };
    } catch (error) {
      throw this.handleError(error);
    }
  }

  private setupRequestInterceptors(): void {
    this.httpClient.interceptors.request.use(
      (config) => {
        if (this.config.debug) {
          console.log(`[AI-CORE SDK] ${config.method?.toUpperCase()} ${config.url}`);
        }
        return config;
      },
      (error) => Promise.reject(error)
    );
  }

  private setupResponseInterceptors(): void {
    this.httpClient.interceptors.response.use(
      (response) => {
        if (this.config.debug) {
          console.log(`[AI-CORE SDK] Response ${response.status} for ${response.config.url}`);
        }
        return response;
      },
      async (error: AxiosError) => {
        if (this.shouldRetry(error)) {
          return this.retryRequest(error);
        }
        return Promise.reject(error);
      }
    );
  }

  private shouldRetry(error: AxiosError): boolean {
    if (!error.response) return true; // Network error

    const status = error.response.status;
    return status >= 500 || status === 429; // Server errors or rate limits
  }

  private async retryRequest(error: AxiosError, attempt = 1): Promise<unknown> {
    if (attempt >= this.config.maxRetries) {
      return Promise.reject(error);
    }

    const delay = Math.min(
      DEFAULT_RETRY_DELAY * Math.pow(2, attempt - 1),
      10000
    );

    await this.sleep(delay);

    try {
      return await this.httpClient.request(error.config!);
    } catch (retryError) {
      if (this.shouldRetry(retryError as AxiosError)) {
        return this.retryRequest(retryError as AxiosError, attempt + 1);
      }
      return Promise.reject(retryError);
    }
  }

  private handleError(error: unknown): Error {
    if (axios.isAxiosError(error)) {
      const response = error.response;

      if (response && isErrorResponse(response.data)) {
        const errorData = response.data;
        const apiError = new Error(errorData.error.message);
        (apiError as any).code = errorData.error.code;
        (apiError as any).status = response.status;
        (apiError as any).details = errorData.error.details;
        (apiError as any).suggestions = errorData.error.suggestions;
        return apiError;
      }

      if (response) {
        return new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      return new Error(`Network error: ${error.message}`);
    }

    return error instanceof Error ? error : new Error(String(error));
  }

  private validateBlogPostRequest(request: BlogPostRequest): void {
    if (!request.topic || request.topic.trim().length < 10) {
      throw new Error('Topic must be at least 10 characters long');
    }

    if (request.wordCount && (request.wordCount < 300 || request.wordCount > 3000)) {
      throw new Error('Word count must be between 300 and 3000');
    }

    if (request.keywords && request.keywords.length > 20) {
      throw new Error('Maximum 20 keywords allowed');
    }
  }

  private handleWebSocketMessage(message: unknown): void {
    if (!isWorkflowEvent(message)) {
      return;
    }

    const { type, workflowId, data } = message;

    switch (type) {
      case 'workflow.progress':
        this.emit('workflow.progress', { ...data, workflowId });
        break;
      case 'workflow.completed':
        this.emit('workflow.completed', { ...data, workflowId });
        break;
      case 'workflow.failed':
        this.emit('workflow.failed', { ...data, workflowId });
        break;
      default:
        // Handle other workflow events
        this.emit(type as keyof ClientEvents, { workflowId, ...data } as any);
    }
  }

  private async attemptReconnect(): Promise<void> {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      this.emit('error', new Error('Maximum reconnection attempts reached'));
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);

    await this.sleep(delay);

    try {
      await this.connectRealTime();
    } catch (error) {
      // Will trigger another reconnect attempt via onclose
    }
  }

  private async pollWorkflowProgress(workflowId: string): Promise<void> {
    const pollInterval = 2000; // 2 seconds
    const maxPollTime = 300000; // 5 minutes
    const startTime = Date.now();

    const poll = async (): Promise<void> => {
      if (Date.now() - startTime > maxPollTime) {
        return; // Stop polling after max time
      }

      try {
        const status = await this.getWorkflowStatus(workflowId, true, true);

        this.emit('workflow.progress', {
          workflowId,
          progress: status.progress,
          currentStep: status.currentStep || 'Processing',
          estimatedTimeRemaining: 0 // Not available in polling
        });

        if (status.status === 'completed') {
          // Emit completion event (would need to fetch full data)
          return;
        }

        if (status.status === 'failed' || status.status === 'cancelled') {
          this.emit('workflow.failed', {
            workflowId,
            error: status.error || { code: 'UNKNOWN', message: 'Workflow failed' },
            partialResults: undefined
          });
          return;
        }

        if (status.status === 'running' || status.status === 'pending') {
          setTimeout(poll, pollInterval);
        }
      } catch (error) {
        this.emit('error', error instanceof Error ? error : new Error(String(error)));
      }
    };

    poll();
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

/**
 * Create a new AI-CORE client instance
 */
export function createClient(config: ClientConfig): AICoreClient {
  return new AICoreClient(config);
}

/**
 * Default export for convenience
 */
export default AICoreClient;
