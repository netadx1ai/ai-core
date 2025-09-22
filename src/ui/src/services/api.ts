import type { AxiosInstance, AxiosResponse } from "axios";
import axios from "axios";
import type {
    ApiResponse,
    AuthResponse,
    LoginRequest,
    RegisterRequest,
    User,
    WorkflowDefinition,
    WorkflowExecution,
} from "../types/api";

// Service health status interface
export interface ServiceStatus {
    healthy: boolean;
    type: "REAL" | "MOCK" | "UNKNOWN";
    service?: string;
    version?: string;
    error?: string;
}

// AI-CORE service endpoints
const AI_CORE_SERVICES = {
    federation: "http://localhost:8801",
    intentParser: "http://localhost:8802",
    mcpManager: "http://localhost:8804",
    mcpProxy: "http://localhost:8803",
};

class ApiService {
    private client: AxiosInstance;
    private token: string | null = null;

    constructor() {
        this.client = axios.create({
            baseURL: this.getApiBaseUrl(),
            headers: {
                "Content-Type": "application/json",
            },
            timeout: 10000,
        });

        // Load token from localStorage if available
        if (typeof window !== "undefined") {
            this.token = localStorage.getItem("auth_token");
            if (this.token) {
                this.setAuthToken(this.token);
            }
        }

        // Response interceptor for handling errors
        this.client.interceptors.response.use(
            (response) => response,
            async (error) => {
                if (error.response?.status === 401) {
                    // Token expired or invalid
                    this.clearToken();
                    window.location.href = "/login";
                }
                return Promise.reject(error);
            },
        );
    }

    private cleanBaseUrl(url: string): string {
        // Remove trailing /v1 to prevent double /v1/v1 paths
        // Also normalize trailing slashes
        return url.replace(/\/v1\/?$/, "").replace(/\/$/, "");
    }

    private getApiBaseUrl(): string {
        // Get base URL from environment variable
        const envUrl = import.meta.env.VITE_AI_CORE_API_URL;

        if (envUrl) {
            return this.cleanBaseUrl(envUrl);
        }

        // In development, use AI-CORE federation service
        if (import.meta.env.DEV) {
            return AI_CORE_SERVICES.federation;
        }

        // Fallback to federation service
        return AI_CORE_SERVICES.federation;
    }

    private setAuthToken(token: string) {
        this.token = token;
        this.client.defaults.headers.common["Authorization"] = `Bearer ${token}`;
        if (typeof window !== "undefined") {
            localStorage.setItem("auth_token", token);
        }
    }

    private clearToken() {
        this.token = null;
        delete this.client.defaults.headers.common["Authorization"];
        if (typeof window !== "undefined") {
            localStorage.removeItem("auth_token");
        }
    }

    // Service health check methods
    async checkServiceHealth(_serviceName: string, serviceUrl: string): Promise<ServiceStatus> {
        try {
            const response = await fetch(`${serviceUrl}/health`, {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                },
                mode: "cors",
            });

            if (response.ok) {
                const data = await response.json();
                const serviceType =
                    data.service?.includes("mock") && !data.service?.includes("proxy") ? "MOCK" : "REAL";

                return {
                    healthy: true,
                    type: serviceType,
                    service: data.service,
                    version: data.version,
                };
            } else {
                return {
                    healthy: false,
                    type: "UNKNOWN",
                    error: `HTTP ${response.status}`,
                };
            }
        } catch (error) {
            return {
                healthy: false,
                type: "UNKNOWN",
                error: error instanceof Error ? error.message : "Unknown error",
            };
        }
    }

    async checkAllServicesHealth(): Promise<Record<string, ServiceStatus>> {
        const results: Record<string, ServiceStatus> = {};

        const checks = [
            ["federation", AI_CORE_SERVICES.federation],
            ["intentParser", AI_CORE_SERVICES.intentParser],
            ["mcpManager", AI_CORE_SERVICES.mcpManager],
            ["mcpProxy", AI_CORE_SERVICES.mcpProxy],
        ];

        await Promise.all(
            checks.map(async ([name, url]) => {
                results[name] = await this.checkServiceHealth(name, url);
            }),
        );

        return results;
    }

    // Authentication methods
    async login(credentials: LoginRequest): Promise<AuthResponse> {
        const response: AxiosResponse<ApiResponse<AuthResponse>> = await this.client.post(
            "/v1/auth/login",
            credentials,
        );

        if (response.data.success && response.data.data) {
            const authData = response.data.data;
            this.setAuthToken(authData.token);
            return authData;
        }

        throw new Error(response.data.error || "Login failed");
    }

    async register(userData: RegisterRequest): Promise<AuthResponse> {
        const response: AxiosResponse<ApiResponse<AuthResponse>> = await this.client.post(
            "/v1/auth/register",
            userData,
        );

        if (response.data.success && response.data.data) {
            const authData = response.data.data;
            this.setAuthToken(authData.token);
            return authData;
        }

        throw new Error(response.data.error || "Registration failed");
    }

    async logout(): Promise<void> {
        try {
            await this.client.post("/v1/auth/logout");
        } finally {
            this.clearToken();
        }
    }

    async getCurrentUser(): Promise<User> {
        const response: AxiosResponse<ApiResponse<User>> = await this.client.get("/v1/auth/me");

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        throw new Error(response.data.error || "Failed to fetch user data");
    }

    // Workflow methods
    async getWorkflows(): Promise<WorkflowDefinition[]> {
        const response: AxiosResponse<ApiResponse<WorkflowDefinition[]>> = await this.client.get("/v1/workflows");

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        return [];
    }

    async getWorkflow(id: string): Promise<WorkflowDefinition> {
        const response: AxiosResponse<ApiResponse<WorkflowDefinition>> = await this.client.get(`/v1/workflows/${id}`);

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        throw new Error(response.data.error || "Failed to fetch workflow");
    }

    async createWorkflow(workflow: Partial<WorkflowDefinition>): Promise<WorkflowDefinition> {
        const response: AxiosResponse<ApiResponse<WorkflowDefinition>> = await this.client.post(
            "/v1/workflows",
            workflow,
        );

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        throw new Error(response.data.error || "Failed to create workflow");
    }

    async updateWorkflow(id: string, workflow: Partial<WorkflowDefinition>): Promise<WorkflowDefinition> {
        const response: AxiosResponse<ApiResponse<WorkflowDefinition>> = await this.client.put(
            `/v1/workflows/${id}`,
            workflow,
        );

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        throw new Error(response.data.error || "Failed to update workflow");
    }

    async deleteWorkflow(id: string): Promise<void> {
        const response: AxiosResponse<ApiResponse<void>> = await this.client.delete(`/v1/workflows/${id}`);

        if (!response.data.success) {
            throw new Error(response.data.error || "Failed to delete workflow");
        }
    }

    async executeWorkflow(id: string): Promise<WorkflowExecution> {
        const response: AxiosResponse<ApiResponse<WorkflowExecution>> = await this.client.post(
            `/v1/workflows/${id}/execute`,
        );

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        throw new Error(response.data.error || "Failed to execute workflow");
    }

    async getWorkflowExecution(id: string): Promise<WorkflowExecution> {
        const response: AxiosResponse<ApiResponse<WorkflowExecution>> = await this.client.get(
            `/v1/workflow-executions/${id}`,
        );

        if (response.data.success && response.data.data) {
            return response.data.data;
        }

        throw new Error(response.data.error || "Failed to fetch workflow execution");
    }

    // Create workflow with intent (AI-CORE specific)
    async createWorkflowFromIntent(
        intent: string,
        workflowType = "blog-post-social",
    ): Promise<{ workflow_id?: string; status?: string; error?: string }> {
        try {
            const response = await fetch(`${AI_CORE_SERVICES.federation}/v1/workflows`, {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    intent,
                    workflow_type: workflowType,
                    client_context: {
                        user_id: this.token ? "authenticated-user" : "demo-user",
                        timestamp: new Date().toISOString(),
                    },
                }),
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            return await response.json();
        } catch (error) {
            console.error("Create workflow from intent error:", error);
            throw error;
        }
    }

    // Get workflow status (AI-CORE specific)
    async getWorkflowStatus(
        workflowId: string,
    ): Promise<{ status?: string; progress?: number; results?: unknown; error?: string }> {
        try {
            const response = await fetch(`${AI_CORE_SERVICES.federation}/v1/workflows/${workflowId}`, {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                },
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            return await response.json();
        } catch (error) {
            console.error("Get workflow status error:", error);
            throw error;
        }
    }

    // Health check
    async healthCheck(): Promise<boolean> {
        try {
            const response = await fetch(`${this.getApiBaseUrl()}/health`);
            return response.ok;
        } catch {
            return false;
        }
    }

    // Get auth status
    isAuthenticated(): boolean {
        return !!this.token;
    }

    getToken(): string | null {
        return this.token;
    }
}

export const apiService = new ApiService();
