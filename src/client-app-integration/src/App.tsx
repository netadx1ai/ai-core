import React, { useState, useEffect, useCallback } from "react";
import { aiCoreClient } from "./services/aiCoreClient";
import { demoScenarios, federationNodes, sampleMetrics } from "./data/scenarios";
import type { AppState, WorkflowRequest, ApiLogEntry, DemoScenario, WebSocketMessage } from "./types";
import { WorkflowStatus, MessageType, LogLevel } from "./types";
import "./index.css";

const App: React.FC = () => {
    const [state, setState] = useState<AppState>({
        isLoading: false,
        currentWorkflow: null,
        logs: [],
        metrics: sampleMetrics,
        error: null,
        success: null,
        isConnected: false,
        showExamples: false,
        selectedTab: "content",
        servicesStatus: {
            federation: {
                name: "Federation Service",
                url: "",
                status: "unknown",
                type: "real",
                lastChecked: new Date(),
            },
            intentParser: {
                name: "Intent Parser",
                url: "",
                status: "unknown",
                type: "real",
                lastChecked: new Date(),
            },
            mcpManager: {
                name: "MCP Manager",
                url: "",
                status: "unknown",
                type: "real",
                lastChecked: new Date(),
            },
            isChecking: false,
        },
    });

    const [userInput, setUserInput] = useState("");
    const [websocket, setWebsocket] = useState<WebSocket | null>(null);

    // Initialize logs subscription
    useEffect(() => {
        const unsubscribe = aiCoreClient.onLog((log: ApiLogEntry) => {
            setState((prev) => ({
                ...prev,
                logs: [...prev.logs, log],
            }));
        });

        return unsubscribe;
    }, []);

    // Test connection and check services on mount
    useEffect(() => {
        const testConnection = async () => {
            try {
                const isConnected = await aiCoreClient.testConnection();
                setState((prev) => ({ ...prev, isConnected }));

                // Check services status if connected
                if (isConnected) {
                    try {
                        const servicesStatus = await aiCoreClient.checkAllServicesStatus();
                        setState((prev) => ({
                            ...prev,
                            servicesStatus: {
                                ...servicesStatus,
                                isChecking: false,
                            },
                        }));
                    } catch (error) {
                        setState((prev) => ({
                            ...prev,
                            servicesStatus: { ...prev.servicesStatus, isChecking: false },
                            error: `Failed to check services status: ${error instanceof Error ? error.message : "Unknown error"}`,
                        }));
                    }
                }
            } catch {
                setState((prev) => ({
                    ...prev,
                    isConnected: false,
                    error: "Failed to connect to AI-CORE API. Please check your configuration.",
                }));
            }
        };

        testConnection();
    }, []);

    // Check services status
    const checkServicesStatus = useCallback(async () => {
        setState((prev) => ({
            ...prev,
            servicesStatus: { ...prev.servicesStatus, isChecking: true },
        }));

        try {
            const servicesStatus = await aiCoreClient.checkAllServicesStatus();
            setState((prev) => ({
                ...prev,
                servicesStatus: {
                    ...servicesStatus,
                    isChecking: false,
                },
            }));
        } catch (error) {
            setState((prev) => ({
                ...prev,
                servicesStatus: { ...prev.servicesStatus, isChecking: false },
                error: `Failed to check services status: ${error instanceof Error ? error.message : "Unknown error"}`,
            }));
        }
    }, []);

    const addLog = useCallback(
        (message: string, level: LogLevel = LogLevel.INFO, details?: Record<string, unknown>) => {
            const log: ApiLogEntry = {
                id: `log_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
                timestamp: new Date().toISOString(),
                level,
                message,
                details,
                context: details,
            };

            setState((prev) => ({
                ...prev,
                logs: [...prev.logs, log],
            }));
        },
        [],
    );

    const clearMessages = () => {
        setState((prev) => ({ ...prev, error: null, success: null }));
    };

    const showError = (message: string) => {
        setState((prev) => ({ ...prev, error: message, success: null }));
        setTimeout(() => clearMessages(), 5000);
    };

    const showSuccess = (message: string) => {
        setState((prev) => ({ ...prev, success: message, error: null }));
        setTimeout(() => clearMessages(), 5000);
    };

    const startDemo = async () => {
        if (!userInput.trim()) {
            showError("Please enter a request description");
            return;
        }

        if (!state.isConnected) {
            showError("Not connected to AI-CORE API");
            return;
        }

        setState((prev) => ({
            ...prev,
            isLoading: true,
            currentWorkflow: null,
            showExamples: false,
        }));

        clearMessages();

        try {
            addLog("🚀 Starting demo workflow...", LogLevel.INFO);

            const workflowRequest: WorkflowRequest = {
                intent: userInput,
                workflow_type: "blog-post-social",
                client_context: {
                    client_demo: true,
                    real_time_updates: true,
                },
            };

            const response = await aiCoreClient.createWorkflow(workflowRequest);
            const workflowId = response.data.workflow_id;

            addLog(`✅ Workflow created: ${workflowId}`, LogLevel.SUCCESS);
            showSuccess(`Workflow started: ${workflowId}`);

            // Establish WebSocket connection for real-time updates
            const ws = aiCoreClient.createWebSocketConnection(workflowId);
            setWebsocket(ws);

            ws.onmessage = (event) => {
                try {
                    const message: WebSocketMessage = JSON.parse(event.data);
                    handleWebSocketMessage(message);
                } catch (error) {
                    addLog("❌ Failed to parse WebSocket message", LogLevel.ERROR, { error });
                }
            };

            // Start polling for status updates
            pollWorkflowStatus(workflowId);
        } catch (error: unknown) {
            const errorMessage = error instanceof Error ? error.message : "Unknown error";
            addLog("❌ Failed to start workflow", LogLevel.ERROR, { error: errorMessage });
            showError(errorMessage || "Failed to start workflow");
            setState((prev) => ({ ...prev, isLoading: false }));
        }
    };

    const handleWebSocketMessage = (message: WebSocketMessage) => {
        switch (message.type) {
            case MessageType.WORKFLOW_STARTED:
                addLog("🔄 Workflow execution started", LogLevel.INFO);
                break;
            case MessageType.STEP_UPDATED:
                addLog(
                    `📋 Step updated: ${(message.data as any)?.step_name || "Unknown step"}`,
                    LogLevel.INFO,
                    message.data as Record<string, unknown>,
                );
                break;
            case MessageType.PROGRESS_UPDATED:
                addLog(
                    `📊 Progress: ${(message.data as any)?.progress || 0}%`,
                    LogLevel.INFO,
                    message.data as Record<string, unknown>,
                );
                break;
            case MessageType.LOG_ENTRY:
                addLog(
                    (message.data as any)?.message || "Log entry",
                    (message.data as any)?.level || LogLevel.INFO,
                    (message.data as any)?.details,
                );
                break;
            case MessageType.WORKFLOW_COMPLETED:
                addLog("✅ Workflow completed successfully!", LogLevel.SUCCESS);
                showSuccess("Workflow completed successfully!");
                setState((prev) => ({ ...prev, isLoading: false }));
                break;
            case MessageType.WORKFLOW_FAILED:
                addLog("❌ Workflow failed", LogLevel.ERROR, message.data as Record<string, unknown>);
                showError("Workflow execution failed");
                setState((prev) => ({ ...prev, isLoading: false }));
                break;
        }
    };

    const pollWorkflowStatus = async (workflowId: string) => {
        let attempts = 0;
        const maxAttempts = 120; // 2 minutes with 1-second intervals

        const poll = async () => {
            if (attempts >= maxAttempts) {
                addLog("⏰ Workflow status polling timeout", LogLevel.WARN);
                setState((prev) => ({ ...prev, isLoading: false }));
                return;
            }

            try {
                const statusResponse = await aiCoreClient.getWorkflowStatus(workflowId);
                const workflow = statusResponse.data;

                setState((prev) => ({ ...prev, currentWorkflow: workflow }));

                if (workflow.status === WorkflowStatus.COMPLETED) {
                    addLog("🎉 Workflow completed!", LogLevel.SUCCESS);
                    showSuccess("Workflow completed successfully!");
                    setState((prev) => ({ ...prev, isLoading: false }));
                    websocket?.close();
                    return;
                }

                if (workflow.status === WorkflowStatus.FAILED) {
                    addLog("💥 Workflow failed", LogLevel.ERROR, { error: workflow.error });
                    showError(workflow.error || "Workflow execution failed");
                    setState((prev) => ({ ...prev, isLoading: false }));
                    websocket?.close();
                    return;
                }

                attempts++;
                setTimeout(poll, 1000);
            } catch (error: unknown) {
                const errorMessage = error instanceof Error ? error.message : "Unknown error";
                addLog("⚠️ Failed to get workflow status", LogLevel.WARN, { error: errorMessage });
                attempts++;
                setTimeout(poll, 2000); // Longer delay on error
            }
        };

        poll();
    };

    const loadExamples = () => {
        setState((prev) => ({ ...prev, showExamples: !prev.showExamples }));
        if (!state.showExamples) {
            addLog("📋 Loading example scenarios", LogLevel.INFO);
        }
    };

    const selectScenario = (scenario: DemoScenario) => {
        setUserInput(scenario.example_prompt);
        setState((prev) => ({ ...prev, showExamples: false }));
        addLog(`📌 Selected scenario: ${scenario.title}`, LogLevel.INFO);
        showSuccess(`Selected: ${scenario.title}`);
    };

    const clearDemo = () => {
        setUserInput("");
        setState((prev) => ({
            ...prev,
            currentWorkflow: null,
            isLoading: false,
            showExamples: false,
            selectedTab: "content",
        }));
        aiCoreClient.clearLogs();
        websocket?.close();
        setWebsocket(null);
        clearMessages();
        addLog("🗑️ Demo cleared", LogLevel.INFO);
    };

    const showTab = (tabName: string) => {
        setState((prev) => ({ ...prev, selectedTab: tabName }));
    };

    const formatLogTimestamp = (timestamp: string) => {
        return new Date(timestamp).toLocaleTimeString();
    };

    const getStepStatus = (stepId: string) => {
        if (!state.currentWorkflow) return "pending";

        const step = state.currentWorkflow.steps?.find((s) => s.id === stepId);
        return step?.status || "pending";
    };

    return (
        <div className="min-h-screen bg-gray-50">
            <div className="container py-8">
                {/* Header */}
                <div className="header">
                    <h1>🚀 AI-CORE MVP Demo</h1>
                    <p>Complete Intelligent Automation Platform Demonstration</p>
                    <div className="workflow-diagram">
                        <div className="workflow-step">Natural Language</div>
                        <div className="workflow-arrow">→</div>
                        <div className="workflow-step">API</div>
                        <div className="workflow-arrow">→</div>
                        <div className="workflow-step">Intent Parsing</div>
                        <div className="workflow-arrow">→</div>
                        <div className="workflow-step">Workflow Orchestration</div>
                        <div className="workflow-arrow">→</div>
                        <div className="workflow-step">Federation with MCPs</div>
                        <div className="workflow-arrow">→</div>
                        <div className="workflow-step">Results</div>
                    </div>
                </div>

                {/* Services Status Section */}
                <div className="services-status-section mb-8">
                    <div className="bg-white rounded-lg shadow-sm border border-gray-200 p-4">
                        <div className="flex items-center justify-between mb-4">
                            <h3 className="text-lg font-semibold text-gray-800">🔧 AI-CORE Services Status</h3>
                            <button
                                onClick={checkServicesStatus}
                                disabled={state.servicesStatus.isChecking}
                                className="px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
                            >
                                {state.servicesStatus.isChecking ? "Checking..." : "Refresh"}
                            </button>
                        </div>
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                            {/* Federation Service */}
                            <div className="service-status-card">
                                <div className="flex items-center justify-between mb-2">
                                    <h4 className="font-medium text-gray-800">
                                        {state.servicesStatus.federation.name}
                                    </h4>
                                    <span
                                        className={`px-2 py-1 text-xs rounded-full ${
                                            state.servicesStatus.federation.status === "healthy"
                                                ? "bg-green-100 text-green-800"
                                                : state.servicesStatus.federation.status === "unhealthy"
                                                  ? "bg-red-100 text-red-800"
                                                  : "bg-gray-100 text-gray-800"
                                        }`}
                                    >
                                        {state.servicesStatus.federation.status}
                                    </span>
                                </div>
                                <div className="text-sm text-gray-600 mb-1">
                                    <span
                                        className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium ${
                                            state.servicesStatus.federation.type === "real"
                                                ? "bg-blue-100 text-blue-800"
                                                : "bg-yellow-100 text-yellow-800"
                                        }`}
                                    >
                                        {state.servicesStatus.federation.type.toUpperCase()}
                                    </span>
                                </div>
                                {state.servicesStatus.federation.version && (
                                    <div className="text-xs text-gray-500">
                                        v{state.servicesStatus.federation.version}
                                    </div>
                                )}
                                {state.servicesStatus.federation.error && (
                                    <div className="text-xs text-red-500 mt-1">
                                        {state.servicesStatus.federation.error}
                                    </div>
                                )}
                            </div>

                            {/* Intent Parser Service */}
                            <div className="service-status-card">
                                <div className="flex items-center justify-between mb-2">
                                    <h4 className="font-medium text-gray-800">
                                        {state.servicesStatus.intentParser.name}
                                    </h4>
                                    <span
                                        className={`px-2 py-1 text-xs rounded-full ${
                                            state.servicesStatus.intentParser.status === "healthy"
                                                ? "bg-green-100 text-green-800"
                                                : state.servicesStatus.intentParser.status === "unhealthy"
                                                  ? "bg-red-100 text-red-800"
                                                  : "bg-gray-100 text-gray-800"
                                        }`}
                                    >
                                        {state.servicesStatus.intentParser.status}
                                    </span>
                                </div>
                                <div className="text-sm text-gray-600 mb-1">
                                    <span
                                        className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium ${
                                            state.servicesStatus.intentParser.type === "real"
                                                ? "bg-blue-100 text-blue-800"
                                                : "bg-yellow-100 text-yellow-800"
                                        }`}
                                    >
                                        {state.servicesStatus.intentParser.type.toUpperCase()}
                                    </span>
                                </div>
                                {state.servicesStatus.intentParser.version && (
                                    <div className="text-xs text-gray-500">
                                        v{state.servicesStatus.intentParser.version}
                                    </div>
                                )}
                                {state.servicesStatus.intentParser.error && (
                                    <div className="text-xs text-red-500 mt-1">
                                        {state.servicesStatus.intentParser.error}
                                    </div>
                                )}
                            </div>

                            {/* MCP Manager Service */}
                            <div className="service-status-card">
                                <div className="flex items-center justify-between mb-2">
                                    <h4 className="font-medium text-gray-800">
                                        {state.servicesStatus.mcpManager.name}
                                    </h4>
                                    <span
                                        className={`px-2 py-1 text-xs rounded-full ${
                                            state.servicesStatus.mcpManager.status === "healthy"
                                                ? "bg-green-100 text-green-800"
                                                : state.servicesStatus.mcpManager.status === "unhealthy"
                                                  ? "bg-red-100 text-red-800"
                                                  : "bg-gray-100 text-gray-800"
                                        }`}
                                    >
                                        {state.servicesStatus.mcpManager.status}
                                    </span>
                                </div>
                                <div className="text-sm text-gray-600 mb-1">
                                    <span
                                        className={`inline-flex items-center px-2 py-1 rounded text-xs font-medium ${
                                            state.servicesStatus.mcpManager.type === "real"
                                                ? "bg-blue-100 text-blue-800"
                                                : "bg-yellow-100 text-yellow-800"
                                        }`}
                                    >
                                        {state.servicesStatus.mcpManager.type.toUpperCase()}
                                    </span>
                                </div>
                                {state.servicesStatus.mcpManager.version && (
                                    <div className="text-xs text-gray-500">
                                        v{state.servicesStatus.mcpManager.version}
                                    </div>
                                )}
                                {state.servicesStatus.mcpManager.error && (
                                    <div className="text-xs text-red-500 mt-1">
                                        {state.servicesStatus.mcpManager.error}
                                    </div>
                                )}
                            </div>
                        </div>
                    </div>
                </div>

                {/* Demo Input Section */}
                <div className="demo-section">
                    <div className="demo-input-area">
                        <h2>🎯 Try the Demo</h2>
                        <p className="mb-4 text-gray-600">Enter your automation request in natural language:</p>
                        <div className="input-container">
                            <textarea
                                className="demo-input"
                                value={userInput}
                                onChange={(e) => setUserInput(e.target.value)}
                                placeholder="Create a blog post about AI automation trends and schedule it on our WordPress site and LinkedIn"
                                rows={3}
                                disabled={state.isLoading}
                            />
                        </div>
                        <div className="button-group">
                            <button
                                className="btn btn-primary"
                                onClick={startDemo}
                                disabled={state.isLoading || !state.isConnected}
                            >
                                {state.isLoading ? "⏳ Processing..." : "🚀 Start Demo"}
                            </button>
                            <button className="btn btn-secondary" onClick={loadExamples} disabled={state.isLoading}>
                                📋 Load Example Scenarios
                            </button>
                            <button className="btn btn-secondary" onClick={clearDemo}>
                                🗑️ Clear
                            </button>
                        </div>

                        {/* Status Messages */}
                        {state.error && <div className="error-message block">❌ {state.error}</div>}
                        {state.success && <div className="success-message block">✅ {state.success}</div>}

                        {/* Connection Status */}
                        <div className="flex items-center justify-between mt-4 p-3 bg-gray-100 rounded-lg">
                            <div className="flex items-center gap-2">
                                <div
                                    className={`w-3 h-3 rounded-full ${state.isConnected ? "bg-green-500" : "bg-red-500"}`}
                                ></div>
                                <span className="text-sm">
                                    {state.isConnected ? "Connected to AI-CORE API" : "Disconnected"}
                                </span>
                            </div>
                            <div className="text-xs text-gray-500">{state.logs.length} log entries</div>
                        </div>
                    </div>

                    {/* Example Scenarios */}
                    {state.showExamples && (
                        <div className="example-scenarios mt-6">
                            {demoScenarios.map((scenario) => (
                                <div
                                    key={scenario.id}
                                    className="scenario-card"
                                    onClick={() => selectScenario(scenario)}
                                >
                                    <h3>{scenario.title}</h3>
                                    <div className="example">{scenario.example_prompt}</div>
                                    <div className="outcome">{scenario.expected_outcome}</div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>

                {/* Workflow Progress Section */}
                {(state.isLoading || state.currentWorkflow) && (
                    <div className="progress-section active">
                        <div className="progress-header">
                            <h2>🔄 Workflow Progress</h2>
                            <div className="workflow-id">Workflow ID: {state.currentWorkflow?.workflow_id || "--"}</div>
                        </div>

                        <div className="cost-tracker">
                            <span>💰 Cost Tracking:</span>
                            <strong>$0.00</strong>
                        </div>

                        <div className="federation-visualization">
                            {federationNodes.map((node) => (
                                <div
                                    key={node.id}
                                    className={`federation-node ${getStepStatus(node.id) === "active" ? "active" : ""}`}
                                >
                                    {node.name}
                                </div>
                            ))}
                        </div>

                        <div className="workflow-steps">
                            {[
                                {
                                    id: "intent-parsing",
                                    name: "Intent Parsing",
                                    details: "Analyzing user input and extracting intent",
                                },
                                {
                                    id: "workflow-creation",
                                    name: "Workflow Creation",
                                    details: "Creating optimized workflow based on parsed intent",
                                },
                                {
                                    id: "content-generation",
                                    name: "Content Generation",
                                    details: "Generating high-quality content using AI models",
                                },
                                {
                                    id: "image-creation",
                                    name: "Image Creation",
                                    details: "Creating featured images and visual content",
                                },
                                {
                                    id: "quality-validation",
                                    name: "Quality Validation",
                                    details: "Validating content quality and SEO optimization",
                                },
                                {
                                    id: "final-processing",
                                    name: "Final Processing",
                                    details: "Final formatting and delivery preparation",
                                },
                            ].map((step, index) => {
                                const status = getStepStatus(step.id);
                                return (
                                    <div key={step.id} className={`step ${status}`}>
                                        <div className="step-icon">
                                            {status === "completed" ? "✓" : status === "active" ? "⏳" : index + 1}
                                        </div>
                                        <div className="step-content">
                                            <div className="step-title">{step.name}</div>
                                            <div className="step-details">{step.details}</div>
                                        </div>
                                        <div className="step-duration">
                                            {status === "completed" ? "2.3s" : status === "active" ? "..." : "--"}
                                        </div>
                                    </div>
                                );
                            })}
                        </div>

                        <div className="log-container">
                            {state.logs.length === 0 ? (
                                <div className="log-entry">
                                    <span className="log-timestamp">[Waiting for demo start...]</span>
                                </div>
                            ) : (
                                state.logs.slice(-10).map((log) => (
                                    <div key={log.id} className="log-entry">
                                        <span className="log-timestamp">[{formatLogTimestamp(log.timestamp)}]</span>
                                        <span
                                            className={`log-level text-${log.level === LogLevel.ERROR ? "red" : log.level === LogLevel.SUCCESS ? "green" : log.level === LogLevel.WARN ? "yellow" : "blue"}-400`}
                                        >
                                            {log.level.toUpperCase()}
                                        </span>
                                        <span className="ml-2">{log.message}</span>
                                    </div>
                                ))
                            )}
                        </div>
                    </div>
                )}

                {/* Results Section */}
                {state.currentWorkflow?.status === WorkflowStatus.COMPLETED && (
                    <div className="results-section active">
                        <h2 className="text-2xl font-bold mb-6">📊 Results</h2>

                        <div className="results-tabs">
                            <button
                                className={`tab ${state.selectedTab === "content" ? "active" : ""}`}
                                onClick={() => showTab("content")}
                            >
                                📝 Generated Content
                            </button>
                            <button
                                className={`tab ${state.selectedTab === "metadata" ? "active" : ""}`}
                                onClick={() => showTab("metadata")}
                            >
                                📊 Metadata
                            </button>
                            <button
                                className={`tab ${state.selectedTab === "logs" ? "active" : ""}`}
                                onClick={() => showTab("logs")}
                            >
                                🔍 Execution Logs
                            </button>
                            <button
                                className={`tab ${state.selectedTab === "api" ? "active" : ""}`}
                                onClick={() => showTab("api")}
                            >
                                🔌 API Details
                            </button>
                        </div>

                        <div className="tab-content active">
                            {state.selectedTab === "content" && (
                                <div className="generated-content">
                                    <h3 className="font-bold mb-4">Generated Blog Post</h3>
                                    <div className="prose max-w-none">
                                        {state.currentWorkflow.results?.content ? (
                                            <div
                                                dangerouslySetInnerHTML={{
                                                    __html: state.currentWorkflow.results.content.content,
                                                }}
                                            />
                                        ) : (
                                            <p>Blog content would be displayed here...</p>
                                        )}
                                    </div>
                                </div>
                            )}

                            {state.selectedTab === "metadata" && (
                                <div className="metadata-grid">
                                    <div className="metadata-item">
                                        <div className="metadata-label">Word Count</div>
                                        <div className="metadata-value">
                                            {state.currentWorkflow.results?.content?.word_count || "N/A"}
                                        </div>
                                    </div>
                                    <div className="metadata-item">
                                        <div className="metadata-label">Quality Score</div>
                                        <div className="metadata-value">
                                            {state.currentWorkflow.results?.quality_score || "N/A"}/5.0
                                        </div>
                                    </div>
                                    <div className="metadata-item">
                                        <div className="metadata-label">Execution Time</div>
                                        <div className="metadata-value">
                                            {state.currentWorkflow.results?.execution_metrics?.total_duration_ms ||
                                                "N/A"}
                                            ms
                                        </div>
                                    </div>
                                    <div className="metadata-item">
                                        <div className="metadata-label">Cost</div>
                                        <div className="metadata-value">
                                            $
                                            {state.currentWorkflow.results?.execution_metrics?.cost_breakdown?.total ||
                                                "0.00"}
                                        </div>
                                    </div>
                                </div>
                            )}

                            {state.selectedTab === "logs" && (
                                <div className="log-container">
                                    {state.logs.map((log) => (
                                        <div key={log.id} className="log-entry">
                                            <span className="log-timestamp">[{formatLogTimestamp(log.timestamp)}]</span>
                                            <span
                                                className={`log-level text-${log.level === LogLevel.ERROR ? "red" : log.level === LogLevel.SUCCESS ? "green" : log.level === LogLevel.WARN ? "yellow" : "blue"}-400`}
                                            >
                                                {log.level.toUpperCase()}
                                            </span>
                                            <span className="ml-2">{log.message}</span>
                                            {log.details && (
                                                <details className="mt-2 text-xs">
                                                    <summary className="cursor-pointer text-gray-400">Details</summary>
                                                    <pre className="mt-1 text-gray-300 overflow-x-auto">
                                                        {JSON.stringify(log.details, null, 2)}
                                                    </pre>
                                                </details>
                                            )}
                                        </div>
                                    ))}
                                </div>
                            )}

                            {state.selectedTab === "api" && (
                                <div className="space-y-4">
                                    <div className="bg-gray-100 p-4 rounded-lg">
                                        <h4 className="font-bold mb-2">API Request Details</h4>
                                        <pre className="text-sm overflow-x-auto">
                                            {JSON.stringify(
                                                {
                                                    method: "POST",
                                                    url: "/v1/workflows",
                                                    headers: {
                                                        "Content-Type": "application/json",
                                                        "X-API-Key": "[REDACTED]",
                                                    },
                                                    body: {
                                                        title: "Client Integration Demo",
                                                        definition: userInput,
                                                        workflow_type: "blog-post-social",
                                                    },
                                                },
                                                null,
                                                2,
                                            )}
                                        </pre>
                                    </div>
                                    <div className="bg-gray-100 p-4 rounded-lg">
                                        <h4 className="font-bold mb-2">API Response</h4>
                                        <pre className="text-sm overflow-x-auto">
                                            {JSON.stringify(state.currentWorkflow, null, 2)}
                                        </pre>
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>
                )}

                {/* Features Grid */}
                <div className="features-grid mt-8">
                    <div className="feature-card">
                        <div className="feature-icon">⚡</div>
                        <div className="feature-title">Real-Time Processing</div>
                        <div className="feature-description">Live progress tracking with WebSocket updates</div>
                    </div>
                    <div className="feature-card">
                        <div className="feature-icon">🔗</div>
                        <div className="feature-title">MCP Federation</div>
                        <div className="feature-description">Multi-service orchestration and coordination</div>
                    </div>
                    <div className="feature-card">
                        <div className="feature-icon">📊</div>
                        <div className="feature-title">Complete Logging</div>
                        <div className="feature-description">Full API request/response logging and monitoring</div>
                    </div>
                    <div className="feature-card">
                        <div className="feature-icon">🚀</div>
                        <div className="feature-title">Production Ready</div>
                        <div className="feature-description">Enterprise-grade reliability and performance</div>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default App;
