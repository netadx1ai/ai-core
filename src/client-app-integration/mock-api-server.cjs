#!/usr/bin/env node

/**
 * Mock AI-CORE API Server for Client Integration Testing
 *
 * This server simulates the AI-CORE platform API endpoints for testing
 * the client-app integration without requiring the full backend infrastructure.
 *
 * Usage: node mock-api-server.js
 * Server will run on http://localhost:8080
 */

const express = require("express");
const cors = require("cors");
const { WebSocketServer } = require("ws");
const http = require("http");
const { v4: uuidv4 } = require("uuid");

// Configuration
const PORT = 8090;
const WS_PORT = 8091;

// Create Express app
const app = express();
const server = http.createServer(app);

// Middleware
app.use(
    cors({
        origin: ["http://localhost:5173", "http://localhost:4173", "http://localhost:3000"],
        credentials: true,
    }),
);
app.use(express.json());

// Request logging middleware
app.use((req, res, next) => {
    console.log(`${new Date().toISOString()} ${req.method} ${req.path}`, req.body ? JSON.stringify(req.body) : "");
    next();
});

// In-memory storage for mock data
const workflows = new Map();
const clients = new Map();

// WebSocket server for real-time updates
const wss = new WebSocketServer({ port: WS_PORT });
const wsClients = new Map();

console.log(`🔌 WebSocket server running on ws://localhost:${WS_PORT}`);

wss.on("connection", (ws, req) => {
    const clientId = uuidv4();
    wsClients.set(clientId, ws);

    console.log(`🔗 WebSocket client connected: ${clientId}`);

    ws.on("close", () => {
        wsClients.delete(clientId);
        console.log(`🔌 WebSocket client disconnected: ${clientId}`);
    });

    // Send welcome message
    ws.send(
        JSON.stringify({
            type: "connection_established",
            data: { client_id: clientId },
            timestamp: new Date().toISOString(),
        }),
    );
});

// Broadcast to all WebSocket clients
function broadcast(message) {
    const payload = JSON.stringify(message);
    wsClients.forEach((ws) => {
        if (ws.readyState === ws.OPEN) {
            ws.send(payload);
        }
    });
}

// Simulate workflow execution with real-time updates
function simulateWorkflowExecution(workflowId) {
    const workflow = workflows.get(workflowId);
    if (!workflow) return;

    const steps = [
        { id: "intent-parsing", name: "Intent Parsing", duration: 2000 },
        { id: "workflow-creation", name: "Workflow Creation", duration: 3000 },
        { id: "content-generation", name: "Content Generation", duration: 8000 },
        { id: "image-creation", name: "Image Creation", duration: 5000 },
        { id: "quality-validation", name: "Quality Validation", duration: 2000 },
        { id: "final-processing", name: "Final Processing", duration: 1000 },
    ];

    let currentStepIndex = 0;

    // Start workflow
    broadcast({
        type: "workflow_started",
        workflow_id: workflowId,
        data: { message: "Workflow execution started" },
        timestamp: new Date().toISOString(),
    });

    function processNextStep() {
        // Add bounds checking at the start
        if (currentStepIndex >= steps.length) {
            // Workflow completed
            workflow.status = "completed";
            workflow.progress = 100;
            workflow.completed_at = new Date().toISOString();

            // Generate mock results
            workflow.results = {
                content: {
                    title: "The Future of AI Automation: Transforming Business Operations",
                    content: `<h1>The Future of AI Automation: Transforming Business Operations</h1>

<p>Artificial Intelligence automation is revolutionizing how businesses operate, streamlining processes and enhancing productivity across industries. As we move into an era where digital transformation is not just advantageous but essential, AI automation stands at the forefront of this evolution.</p>

<h2>Key Benefits of AI Automation</h2>

<ul>
<li><strong>Enhanced Efficiency:</strong> Automated processes can handle repetitive tasks 24/7 without human intervention</li>
<li><strong>Reduced Errors:</strong> AI systems minimize human error in data processing and decision-making</li>
<li><strong>Cost Optimization:</strong> Significant reduction in operational costs through process optimization</li>
<li><strong>Scalability:</strong> Easy scaling of operations without proportional increase in resources</li>
</ul>

<h2>Industry Applications</h2>

<p>From manufacturing to healthcare, AI automation is making significant impacts:</p>

<ul>
<li><strong>Manufacturing:</strong> Predictive maintenance and quality control</li>
<li><strong>Finance:</strong> Automated trading and fraud detection</li>
<li><strong>Healthcare:</strong> Medical imaging analysis and patient monitoring</li>
<li><strong>Retail:</strong> Inventory management and personalized customer experiences</li>
</ul>

<h2>The Road Ahead</h2>

<p>As AI technology continues to evolve, we can expect even more sophisticated automation solutions. The integration of machine learning, natural language processing, and robotics will create unprecedented opportunities for business transformation.</p>

<p>Companies that embrace AI automation today will be better positioned to compete in tomorrow's digital economy. The question is not whether to adopt AI automation, but how quickly and effectively it can be implemented.</p>`,
                    summary:
                        "AI automation is transforming business operations by enhancing efficiency, reducing errors, optimizing costs, and providing scalability across various industries including manufacturing, finance, healthcare, and retail.",
                    word_count: 324,
                    reading_time: 2,
                    seo_keywords: [
                        "AI automation",
                        "business transformation",
                        "digital transformation",
                        "artificial intelligence",
                        "process optimization",
                        "machine learning",
                    ],
                    featured_image_url:
                        "https://images.unsplash.com/photo-1555255707-c07966088b7b?ixlib=rb-4.0.3&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D&auto=format&fit=crop&w=1932&q=80",
                    meta_description:
                        "Discover how AI automation is revolutionizing business operations, driving efficiency, and creating competitive advantages across industries.",
                    tags: ["AI", "Automation", "Business", "Technology", "Innovation"],
                },
                quality_score: 4.7,
                execution_metrics: {
                    total_duration_ms: 21000,
                    api_calls_made: 8,
                    tokens_consumed: 1247,
                    cost_breakdown: {
                        text_generation: 0.42,
                        image_generation: 0.08,
                        api_calls: 0.05,
                        total: 0.55,
                    },
                    performance_score: 94,
                },
            };

            broadcast({
                type: "workflow_completed",
                workflow_id: workflowId,
                data: {
                    message: "Workflow completed successfully!",
                    results: workflow.results,
                },
                timestamp: new Date().toISOString(),
            });

            return;
        }

        const step = steps[currentStepIndex];
        if (!step) {
            console.error(`Step not found at index ${currentStepIndex}, total steps: ${steps.length}`);
            return;
        }
        workflow.current_step = step.name;
        workflow.progress = Math.round(((currentStepIndex + 1) / steps.length) * 100);

        // Initialize steps array if not exists
        if (!workflow.steps) {
            workflow.steps = steps.map((s, idx) => ({
                id: s.id,
                name: s.name,
                status: "pending",
                started_at: null,
                completed_at: null,
                duration_ms: null,
            }));
        }

        // Update current step status with bounds checking
        if (workflow.steps && currentStepIndex < workflow.steps.length && workflow.steps[currentStepIndex]) {
            workflow.steps[currentStepIndex].status = "active";
            workflow.steps[currentStepIndex].started_at = new Date().toISOString();
        }

        // Broadcast step update
        broadcast({
            type: "step_updated",
            workflow_id: workflowId,
            data: {
                step_name: step.name,
                step_id: step.id,
                status: "active",
                progress: workflow.progress,
            },
            timestamp: new Date().toISOString(),
        });

        // Broadcast progress update
        broadcast({
            type: "progress_updated",
            workflow_id: workflowId,
            data: {
                progress: workflow.progress,
                current_step: step.name,
                estimated_remaining: (steps.length - currentStepIndex - 1) * 3000,
            },
            timestamp: new Date().toISOString(),
        });

        setTimeout(() => {
            // Mark current step as completed with bounds checking
            if (workflow.steps && currentStepIndex < workflow.steps.length && workflow.steps[currentStepIndex]) {
                workflow.steps[currentStepIndex].status = "completed";
                workflow.steps[currentStepIndex].completed_at = new Date().toISOString();
                workflow.steps[currentStepIndex].duration_ms = step.duration;
            }

            broadcast({
                type: "log_entry",
                workflow_id: workflowId,
                data: {
                    message: `✅ ${step.name} completed successfully`,
                    level: "success",
                    details: {
                        step_id: step.id,
                        duration_ms: step.duration,
                        status: "completed",
                    },
                },
                timestamp: new Date().toISOString(),
            });

            currentStepIndex++;
            processNextStep();
        }, step.duration);
    }

    // Start processing steps
    setTimeout(processNextStep, 1000);
}

// API Routes

// Health check
app.get("/health", (req, res) => {
    res.json({
        status: "healthy",
        version: "1.0.0",
        timestamp: new Date().toISOString(),
        services: {
            api: "operational",
            websocket: "operational",
            database: "mocked",
        },
    });
});

app.get("/v1/health", (req, res) => {
    res.json({
        status: "ok",
        version: "1.0.0",
        timestamp: new Date().toISOString(),
    });
});

// Create workflow
app.post("/v1/workflows", (req, res) => {
    const { title, definition, workflow_type, config } = req.body;

    if (!definition) {
        return res.status(400).json({
            error: "validation_error",
            message: "Definition is required",
            details: { field: "definition", code: "required" },
        });
    }

    const workflowId = `wf_${Date.now()}_${Math.random().toString(36).substr(2, 8)}`;

    const workflow = {
        workflow_id: workflowId,
        title: title || "Client Integration Demo",
        definition,
        workflow_type: workflow_type || "blog-post-social",
        status: "starting",
        progress: 0,
        current_step: "Initializing",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        config: config || {},
        steps: [],
        metadata: {
            client_id: "mock-client",
            session_id: uuidv4(),
            user_agent: req.headers["user-agent"] || "unknown",
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString(),
            federation_nodes: [
                { id: "intent-parser", name: "Intent Parser", status: "idle", last_active: new Date().toISOString() },
                {
                    id: "workflow-engine",
                    name: "Workflow Engine",
                    status: "idle",
                    last_active: new Date().toISOString(),
                },
                { id: "content-mcp", name: "Content MCP", status: "idle", last_active: new Date().toISOString() },
                { id: "image-mcp", name: "Image MCP", status: "idle", last_active: new Date().toISOString() },
                { id: "publishing-mcp", name: "Publishing MCP", status: "idle", last_active: new Date().toISOString() },
            ],
        },
    };

    workflows.set(workflowId, workflow);

    // Start workflow execution simulation
    setTimeout(() => simulateWorkflowExecution(workflowId), 500);

    res.status(201).json({
        workflow_id: workflowId,
        status: "starting",
        message: "Workflow created successfully",
        estimated_duration: 25000,
        created_at: workflow.created_at,
    });
});

// Get workflow status
app.get("/v1/workflows/:id", (req, res) => {
    const workflowId = req.params.id;
    const workflow = workflows.get(workflowId);

    if (!workflow) {
        return res.status(404).json({
            error: "not_found",
            message: "Workflow not found",
            workflow_id: workflowId,
        });
    }

    workflow.updated_at = new Date().toISOString();

    res.json(workflow);
});

// Get client metrics
app.get("/v1/metrics", (req, res) => {
    res.json({
        total_requests: workflows.size,
        successful_requests: Array.from(workflows.values()).filter((w) => w.status === "completed").length,
        failed_requests: Array.from(workflows.values()).filter((w) => w.status === "failed").length,
        average_execution_time_ms: 23500,
        average_quality_score: 4.5,
        total_cost_usd: workflows.size * 0.55,
        cost_savings_usd: workflows.size * 12.45,
        uptime_percentage: 99.8,
        last_updated: new Date().toISOString(),
    });
});

// List workflows
app.get("/v1/workflows", (req, res) => {
    const workflowList = Array.from(workflows.values()).map((w) => ({
        workflow_id: w.workflow_id,
        title: w.title,
        status: w.status,
        progress: w.progress,
        created_at: w.created_at,
        updated_at: w.updated_at,
    }));

    res.json({
        workflows: workflowList,
        total: workflowList.length,
        page: 1,
        per_page: 50,
    });
});

// WebSocket endpoint info
app.get("/v1/websocket/info", (req, res) => {
    res.json({
        websocket_url: `ws://localhost:${WS_PORT}`,
        supported_events: [
            "workflow_started",
            "step_updated",
            "progress_updated",
            "log_entry",
            "workflow_completed",
            "workflow_failed",
        ],
    });
});

// Error handling middleware
app.use((error, req, res, next) => {
    console.error("API Error:", error);
    res.status(500).json({
        error: "internal_server_error",
        message: "An internal server error occurred",
        timestamp: new Date().toISOString(),
    });
});

// 404 handler
app.use((req, res) => {
    res.status(404).json({
        error: "not_found",
        message: "Endpoint not found",
        path: req.path,
        method: req.method,
    });
});

// Start the server
server.listen(PORT, () => {
    console.log(`🚀 Mock AI-CORE API Server running on http://localhost:${PORT}`);
    console.log(`📊 Health check: http://localhost:${PORT}/health`);
    console.log(`📝 API docs: http://localhost:${PORT}/v1/workflows`);
    console.log(`🔌 WebSocket: ws://localhost:${WS_PORT}`);
    console.log("");
    console.log("Available endpoints:");
    console.log("  GET  /health - Health check");
    console.log("  GET  /v1/health - API health");
    console.log("  POST /v1/workflows - Create workflow");
    console.log("  GET  /v1/workflows/:id - Get workflow status");
    console.log("  GET  /v1/workflows - List workflows");
    console.log("  GET  /v1/metrics - Client metrics");
    console.log("  GET  /v1/websocket/info - WebSocket info");
    console.log("");
    console.log("💡 Use with AI-CORE client integration for testing");
    console.log("🛑 Press Ctrl+C to stop the server");
});

// Graceful shutdown
process.on("SIGINT", () => {
    console.log("\n🛑 Shutting down mock server...");
    server.close(() => {
        console.log("✅ Server stopped");
        process.exit(0);
    });
});

process.on("SIGTERM", () => {
    console.log("\n🛑 Received SIGTERM, shutting down...");
    server.close(() => {
        console.log("✅ Server stopped");
        process.exit(0);
    });
});
