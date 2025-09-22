import axios, { AxiosError, AxiosInstance } from "axios";
import type {
    ApiConfig,
    ApiError,
    ApiLogEntry,
    AxiosResponse,
    ClientMetrics,
    HttpResponse,
    ServiceStatus,
    WorkflowRequest,
    WorkflowResponse,
    WorkflowStatusResponse,
} from "../types";
import { LogLevel } from "../types";

export class AiCoreClient {
    protected client: AxiosInstance;
    protected config: ApiConfig;
    private logs: ApiLogEntry[] = [];
    private logCallbacks: ((log: ApiLogEntry) => void)[] = [];

    constructor(config: ApiConfig) {
        // Ensure baseURL doesn't end with /v1 to avoid double /v1/v1 URLs
        const cleanedConfig = {
            ...config,
            baseUrl: cleanBaseUrl(config.baseUrl),
        };

        this.config = cleanedConfig;

        // Debug logging to track URL construction
        console.log("🔧 AiCoreClient Debug:");
        console.log("  Original baseUrl:", config.baseUrl);
        console.log("  Cleaned baseUrl:", cleanedConfig.baseUrl);
        console.log("  Environment VITE_AI_CORE_API_URL:", import.meta.env.VITE_AI_CORE_API_URL);

        this.client = axios.create({
            baseURL: cleanedConfig.baseUrl,
            timeout: cleanedConfig.timeout,
            headers: {
                "Content-Type": "application/json",
                "X-API-Key": cleanedConfig.apiKey,
                "User-Agent": "AI-CORE-Client-Integration/1.0.0",
            },
        });

        console.log("  Axios client baseURL:", this.client.defaults.baseURL);
        this.setupInterceptors();
    }

    private setupInterceptors(): void {
        // Request interceptor
        this.client.interceptors.request.use(
            (config) => {
                const startTime = Date.now();
                (config as unknown as Record<string, unknown>).metadata = { startTime };

                // Debug URL construction
                console.log("🌐 Request interceptor:");
                console.log("  config.url:", config.url);
                console.log("  config.baseURL:", config.baseURL);
                const baseURL = config.baseURL || this.client.defaults.baseURL;
                const url = config.url || "";
                console.log("  Full URL will be:", baseURL + url);

                // Log the request
                this.addLog({
                    level: LogLevel.INFO,
                    message: `Starting API request: ${config.method?.toUpperCase()} ${config.url}`,
                    details: {
                        method: config.method?.toUpperCase(),
                        url: config.url,
                        request_headers: config.headers as Record<string, string>,
                        request_body: config.data,
                    },
                });

                return config;
            },
            (error) => {
                this.addLog({
                    level: LogLevel.ERROR,
                    message: `Request setup failed: ${error.message}`,
                    details: {
                        error_message: error.message,
                    },
                });
                return Promise.reject(error);
            },
        );

        // Response interceptor
        this.client.interceptors.response.use(
            (response) => {
                const endTime = Date.now();
                const startTime =
                    ((response.config as unknown as Record<string, unknown>).metadata as { startTime?: number })
                        ?.startTime || endTime;
                const duration = endTime - startTime;

                // Add duration to response
                (response as unknown as Record<string, unknown>).duration = duration;

                // Log successful response
                this.addLog({
                    level: LogLevel.SUCCESS,
                    message: `API request completed: ${response.status} ${response.statusText}`,
                    details: {
                        method: response.config.method?.toUpperCase(),
                        url: response.config.url,
                        status_code: response.status,
                        response_headers: response.headers as Record<string, string>,
                        response_body: response.data,
                        duration_ms: duration,
                    },
                });

                return response;
            },
            (error: AxiosError) => {
                const endTime = Date.now();
                const startTime =
                    ((error.config as unknown as Record<string, unknown>)?.metadata as { startTime?: number })
                        ?.startTime || endTime;
                const duration = endTime - startTime;

                // Log error response
                this.addLog({
                    level: LogLevel.ERROR,
                    message: `API request failed: ${error.response?.status || "Network Error"} - ${error.message}`,
                    details: {
                        method: error.config?.method?.toUpperCase(),
                        url: error.config?.url,
                        status_code: error.response?.status,
                        response_headers: error.response?.headers as Record<string, string>,
                        response_body: error.response?.data,
                        duration_ms: duration,
                        error_message: error.message,
                    },
                });

                return Promise.reject(this.createApiError(error));
            },
        );
    }

    private createApiError(error: AxiosError): ApiError {
        return {
            code: error.code || "UNKNOWN_ERROR",
            message: error.message,
            details: error.response?.data,
            status: error.response?.status || 0,
            timestamp: new Date().toISOString(),
        };
    }

    protected addLog(logData: Omit<ApiLogEntry, "id" | "timestamp">): void {
        const log: ApiLogEntry = {
            id: this.generateLogId(),
            timestamp: new Date().toISOString(),
            ...logData,
        };

        this.logs.push(log);

        // Limit logs to last 100 entries
        if (this.logs.length > 100) {
            this.logs = this.logs.slice(-100);
        }

        // Notify callbacks
        this.logCallbacks.forEach((callback) => callback(log));
    }

    private generateLogId(): string {
        return `log_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    }

    // Public API methods
    public async createWorkflow(request: WorkflowRequest): Promise<HttpResponse<WorkflowResponse>> {
        this.addLog({
            level: LogLevel.INFO,
            message: `Creating workflow: ${request.intent}`,
            context: { workflow_type: request.workflow_type },
        });

        const response: AxiosResponse<WorkflowResponse> = await this.client.post("/v1/workflows", request);

        return {
            data: response.data,
            status: response.status,
            statusText: response.statusText,
            headers: response.headers as Record<string, string>,
            config: {
                method: "POST",
                url: "/v1/workflows",
                headers: response.config.headers as Record<string, string>,
                data: request,
            },
            duration: ((response as unknown as Record<string, unknown>).duration as number) || 0,
        };
    }

    public async getWorkflowStatus(workflowId: string): Promise<HttpResponse<WorkflowStatusResponse>> {
        const response: AxiosResponse<any> = await this.client.get(`/v1/workflows/${workflowId}`);

        // Transform the Federation API response to client format
        const transformedData = this.transformBlogPostResponse(response.data);

        return {
            data: transformedData,
            status: response.status,
            statusText: response.statusText,
            headers: response.headers as Record<string, string>,
            config: {
                method: "GET",
                url: `/v1/workflows/${workflowId}`,
                headers: response.config.headers as Record<string, string>,
            },
            duration: ((response as unknown as Record<string, unknown>).duration as number) || 0,
        };
    }

    /**
     * Transform Federation API response to client-expected format
     */
    private transformBlogPostResponse(apiResponse: any): WorkflowStatusResponse {
        const blogPost = apiResponse.results?.blog_post;
        const qualityScores = apiResponse.results?.quality_scores;
        const metrics = apiResponse.results?.metrics;
        const image = apiResponse.results?.image;

        return {
            workflow_id: apiResponse.workflow_id || "",
            status: apiResponse.status || "unknown",
            progress: apiResponse.progress || 0,
            current_step: apiResponse.current_step || null,
            steps: apiResponse.steps || [],
            results: blogPost
                ? {
                      content: {
                          title: blogPost.title,
                          content: blogPost.content,
                          summary: blogPost.meta_description || "",
                          word_count: blogPost.word_count || 0,
                          reading_time: blogPost.reading_time || 0,
                          seo_keywords: blogPost.seo_keywords || [],
                          featured_image_url: image?.url,
                          meta_description: blogPost.meta_description || "",
                          tags: blogPost.seo_keywords || [],
                      },
                      images: image
                          ? [
                                {
                                    url: image.url,
                                    alt_text: image.alt_text,
                                    width: image.width || 0,
                                    height: image.height || 0,
                                    format: image.format,
                                    size_bytes: image.file_size || 0,
                                },
                            ]
                          : [],
                      metadata: {
                          generated_at: new Date().toISOString(),
                          model_used: "gemini-flash-1.5",
                          tokens_consumed: 0,
                          cost_usd: 0,
                          processing_time_ms: metrics?.execution_time_ms || 0,
                      },
                      quality_score: qualityScores?.overall_score || 0,
                      execution_metrics: {
                          total_duration_ms: metrics?.execution_time_ms || 0,
                          api_calls_made: metrics?.processing_steps || 3,
                          tokens_consumed: 0,
                          cost_breakdown: {
                              text_generation: 0,
                              image_generation: 0,
                              api_calls: 0,
                              total: 0,
                          },
                          performance_score: qualityScores?.overall_score || 0,
                      },
                  }
                : undefined,
            error: apiResponse.error || null,
        };
    }

    public async healthCheck(): Promise<HttpResponse<{ status: string; version: string; timestamp: string }>> {
        // For federation service, health endpoint is at /health not /v1/health
        const healthUrl = "http://localhost:8801/health";
        const response = await axios.get(healthUrl, { timeout: 5000 });

        return {
            data: response.data,
            status: response.status,
            statusText: response.statusText,
            headers: response.headers as Record<string, string>,
            config: {
                method: "GET",
                url: healthUrl,
                headers: response.config.headers as Record<string, string>,
            },
            duration: ((response as unknown as Record<string, unknown>).duration as number) || 0,
        };
    }

    public async getClientMetrics(): Promise<HttpResponse<ClientMetrics>> {
        const response: AxiosResponse<ClientMetrics> = await this.client.get("/metrics");

        return {
            data: response.data,
            status: response.status,
            statusText: response.statusText,
            headers: response.headers as Record<string, string>,
            config: {
                method: "GET",
                url: "/metrics",
                headers: response.config.headers as Record<string, string>,
            },
            duration: ((response as unknown as Record<string, unknown>).duration as number) || 0,
        };
    }

    // WebSocket connection for real-time updates
    public createWebSocketConnection(workflowId: string): WebSocket {
        const wsUrl = `${this.config.websocketUrl}/v1/workflows/${workflowId}`;

        this.addLog({
            level: LogLevel.INFO,
            message: `Establishing WebSocket connection for workflow ${workflowId}`,
            context: { websocket_url: wsUrl },
        });

        const ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            this.addLog({
                level: LogLevel.SUCCESS,
                message: `WebSocket connection established for workflow ${workflowId}`,
                context: { workflow_id: workflowId },
            });
        };

        ws.onclose = (event) => {
            this.addLog({
                level: LogLevel.WARN,
                message: `WebSocket connection closed for workflow ${workflowId}`,
                context: {
                    workflow_id: workflowId,
                    code: event.code,
                    reason: event.reason,
                },
            });
        };

        ws.onerror = (error) => {
            this.addLog({
                level: LogLevel.ERROR,
                message: `WebSocket error for workflow ${workflowId}`,
                context: {
                    workflow_id: workflowId,
                    error: error,
                },
            });
        };

        return ws;
    }

    // Log management
    public getLogs(): ApiLogEntry[] {
        return [...this.logs];
    }

    public clearLogs(): void {
        this.logs = [];
        this.addLog({
            level: LogLevel.INFO,
            message: "Execution logs cleared",
        });
    }

    public onLog(callback: (log: ApiLogEntry) => void): () => void {
        this.logCallbacks.push(callback);

        // Return unsubscribe function
        return () => {
            const index = this.logCallbacks.indexOf(callback);
            if (index > -1) {
                this.logCallbacks.splice(index, 1);
            }
        };
    }

    // Utility methods
    public updateConfig(newConfig: Partial<ApiConfig>): void {
        this.config = { ...this.config, ...newConfig };

        // Update axios instance
        this.client.defaults.baseURL = this.config.baseUrl;
        this.client.defaults.timeout = this.config.timeout;
        this.client.defaults.headers["X-API-Key"] = this.config.apiKey;

        this.addLog({
            level: LogLevel.INFO,
            message: "API client configuration updated",
            context: {
                baseUrl: this.config.baseUrl,
                timeout: this.config.timeout,
            },
        });
    }

    public getConfig(): ApiConfig {
        return { ...this.config };
    }

    // Retry mechanism for failed requests
    public async retryRequest<T>(
        requestFn: () => Promise<HttpResponse<T>>,
        maxRetries: number = this.config.retries,
        delayMs: number = 1000,
    ): Promise<HttpResponse<T>> {
        let lastError: ApiError | null = null;

        for (let attempt = 1; attempt <= maxRetries; attempt++) {
            try {
                this.addLog({
                    level: LogLevel.INFO,
                    message: `Attempting request (${attempt}/${maxRetries})`,
                    context: { attempt, maxRetries },
                });

                return await requestFn();
            } catch (error) {
                lastError = error as ApiError;

                this.addLog({
                    level: LogLevel.WARN,
                    message: `Request attempt ${attempt} failed: ${lastError.message}`,
                    context: {
                        attempt,
                        maxRetries,
                        error: lastError.message,
                        status: lastError.status,
                    },
                });

                if (attempt < maxRetries) {
                    this.addLog({
                        level: LogLevel.INFO,
                        message: `Waiting ${delayMs}ms before retry...`,
                        context: { delayMs },
                    });

                    await this.delay(delayMs);
                    delayMs *= 2; // Exponential backoff
                }
            }
        }

        this.addLog({
            level: LogLevel.ERROR,
            message: `All ${maxRetries} retry attempts failed`,
            context: { maxRetries, finalError: lastError?.message },
        });

        throw lastError;
    }

    private delay(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    // Connection testing
    public async testConnection(): Promise<boolean> {
        try {
            this.addLog({
                level: LogLevel.INFO,
                message: "Testing AI-CORE API connection...",
            });

            await this.healthCheck();

            this.addLog({
                level: LogLevel.SUCCESS,
                message: "AI-CORE API connection test successful",
            });

            return true;
        } catch (error) {
            this.addLog({
                level: LogLevel.ERROR,
                message: `AI-CORE API connection test failed: ${(error as ApiError).message}`,
            });

            return false;
        }
    }
}

// Default configuration
// Helper function to clean base URL and remove trailing /v1
const cleanBaseUrl = (url: string): string => {
    return url.replace(/\/v1\/?$/, "");
};

export const defaultConfig: ApiConfig = {
    baseUrl: cleanBaseUrl(import.meta.env.VITE_AI_CORE_API_URL || "http://localhost:8801"),
    apiKey: import.meta.env.VITE_AI_CORE_API_KEY || "real-api-key",
    timeout: 30000,
    retries: 3,
    websocketUrl: import.meta.env.VITE_AI_CORE_WS_URL || "ws://localhost:8801/ws",
};

// Service endpoints configuration
export const serviceEndpoints = {
    federation: {
        name: "Federation Service",
        health: "/health",
        type: "real" as "real" | "mock",
        url: "http://localhost:8801", // Federation service base URL without /v1
    },
    intentParser: {
        name: "Intent Parser",
        health: "/health",
        type: "real" as "real" | "mock",
        url: "http://localhost:8802",
    },
    mcpManager: {
        name: "Demo Content MCP",
        health: "/health",
        type: "real" as "real" | "mock",
        url: "http://localhost:8804", // Direct connection to real MCP service
    },
};

export class AiCoreClientExtended extends AiCoreClient {
    // Check individual service status
    public async checkServiceStatus(serviceName: keyof typeof serviceEndpoints): Promise<ServiceStatus> {
        const service = serviceEndpoints[serviceName];
        const baseUrl = service.url || this.config.baseUrl;
        const serviceStatus: ServiceStatus = {
            name: service.name,
            url: `${baseUrl}${service.health}`,
            status: "unknown",
            type: service.type,
            lastChecked: new Date(),
        };

        try {
            // Use fetch for all services to avoid CORS issues
            try {
                const response = await fetch(`${baseUrl}${service.health}`, {
                    method: "GET",
                    signal: AbortSignal.timeout(5000),
                });

                if (response.ok) {
                    const data = await response.json();
                    serviceStatus.status = "healthy";

                    // Determine if service is real based on response
                    const responseServiceName = data.service || "";
                    if (responseServiceName.includes("mock") && !responseServiceName.includes("proxy")) {
                        serviceStatus.type = "mock";
                        serviceStatus.error = "Mock service detected";
                    } else {
                        serviceStatus.type = "real";
                    }

                    serviceStatus.version = data.version || "1.0.0";
                    serviceStatus.uptime = data.uptime_seconds;

                    // Log successful connection to real services
                    if (serviceStatus.type === "real") {
                        this.addLog({
                            level: LogLevel.SUCCESS,
                            message: `${service.name} connected successfully`,
                            context: {
                                service: responseServiceName,
                                version: serviceStatus.version,
                                type: "REAL",
                            },
                        });
                    }
                } else {
                    serviceStatus.status = "unhealthy";
                    serviceStatus.error = `HTTP ${response.status}`;
                }
            } catch (fetchError) {
                // Service not reachable
                serviceStatus.status = "unhealthy";
                serviceStatus.error = fetchError instanceof Error ? fetchError.message : "Connection failed";

                this.addLog({
                    level: LogLevel.WARN,
                    message: `${service.name} connection failed`,
                    context: { error: serviceStatus.error, url: serviceStatus.url },
                });
            }
        } catch (error) {
            serviceStatus.status = "unhealthy";
            serviceStatus.error = error instanceof Error ? error.message : "Unknown error";

            this.addLog({
                level: LogLevel.ERROR,
                message: `Service check failed: ${service.name}`,
                context: { error: serviceStatus.error, url: serviceStatus.url },
            });
        }

        return serviceStatus;
    }

    // Check all services status
    public async checkAllServicesStatus(): Promise<{
        federation: ServiceStatus;
        intentParser: ServiceStatus;
        mcpManager: ServiceStatus;
    }> {
        this.addLog({
            level: LogLevel.INFO,
            message: "Checking all AI-CORE services status...",
        });

        const [federation, intentParser, mcpManager] = await Promise.all([
            this.checkServiceStatus("federation"),
            this.checkServiceStatus("intentParser"),
            this.checkServiceStatus("mcpManager"),
        ]);

        // Determine if services are mock based on configuration and actual responses
        if (import.meta.env.VITE_DEMO_MODE === "mock") {
            federation.type = "mock";
            intentParser.type = "mock";
            mcpManager.type = "mock";
        } else {
            // For real mode, determine type based on actual service response
            if (intentParser.error && intentParser.error.includes("Mock service")) {
                intentParser.type = "mock";
            } else if (intentParser.status === "healthy" && !intentParser.error) {
                intentParser.type = "real";
            }

            if (mcpManager.error && mcpManager.error.includes("Mock service")) {
                mcpManager.type = "mock";
            } else if (mcpManager.status === "healthy" && !mcpManager.error) {
                mcpManager.type = "real";
            }
        }

        // Log overall status
        const healthyCount = [federation, intentParser, mcpManager].filter((s) => s.status === "healthy").length;
        this.addLog({
            level: healthyCount === 3 ? LogLevel.SUCCESS : LogLevel.WARN,
            message: `Services status check complete: ${healthyCount}/3 services healthy`,
            context: {
                federation: `${federation.status} (${federation.type})`,
                intentParser: `${intentParser.status} (${intentParser.type})`,
                mcpManager: `${mcpManager.status} (${mcpManager.type})`,
            },
        });

        return { federation, intentParser, mcpManager };
    }
}

// Singleton instance using extended client
export const aiCoreClient = new AiCoreClientExtended(defaultConfig);
