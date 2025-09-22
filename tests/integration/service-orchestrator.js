#!/usr/bin/env node

/**
 * AI-CORE Service Orchestrator
 * Master controller for coordinated service startup, health monitoring, and shutdown
 *
 * Usage:
 *   node service-orchestrator.js start    # Start all services
 *   node service-orchestrator.js stop     # Stop all services
 *   node service-orchestrator.js status   # Check service status
 *   node service-orchestrator.js test     # Run full test suite
 */

const { spawn, exec } = require("child_process");
const fs = require("fs");
const path = require("path");
const http = require("http");

class ServiceOrchestrator {
    constructor() {
        this.services = {
            // Built-in MCPs (Internal Services)
            "demo-content-mcp": {
                name: "Content Generation MCP",
                port: 8804,
                path: "src/demo",
                startCommand: "cargo run --bin demo-content-mcp",
                healthEndpoint: "/health",
                dependencies: [],
                type: "rust",
            },
            "text-processing-mcp": {
                name: "Text Processing MCP",
                port: 8805,
                path: "src/demo",
                startCommand: "cargo run --bin text-processing-mcp",
                healthEndpoint: "/health",
                dependencies: [],
                type: "rust",
            },
            "image-generation-internal-mcp": {
                name: "Image Generation MCP (Internal)",
                port: 8806,
                path: "src/demo",
                startCommand: "cargo run --bin image-generation-mcp",
                healthEndpoint: "/health",
                dependencies: [],
                type: "rust",
            },
            "mcp-orchestrator": {
                name: "MCP Workflow Orchestrator",
                port: 8807,
                path: "src/demo",
                startCommand: "cargo run --bin mcp-orchestrator",
                healthEndpoint: "/health",
                dependencies: ["demo-content-mcp", "text-processing-mcp"],
                type: "rust",
            },

            // External MCPs (Node.js Services)
            "image-generation-external-mcp": {
                name: "Image Generation MCP (External)",
                port: 8091,
                path: "external-mcps/image-generation",
                startCommand: "npm start",
                healthEndpoint: "/health",
                dependencies: [],
                type: "node",
                buildCommand: "npm run build",
            },
            "calendar-management-mcp": {
                name: "Calendar Management MCP",
                port: 8092,
                path: "external-mcps/calendar-management",
                startCommand: "npm start",
                healthEndpoint: "/health",
                dependencies: [],
                type: "node",
            },
            "facebook-posting-mcp": {
                name: "Facebook Posting MCP",
                port: 8093,
                path: "external-mcps/facebook-posting",
                startCommand: "npm start",
                healthEndpoint: "/health",
                dependencies: [],
                type: "node",
            },
        };

        this.processes = new Map();
        this.healthChecks = new Map();
        this.startupOrder = [
            // Phase 1: Core built-in services
            ["demo-content-mcp", "text-processing-mcp", "image-generation-internal-mcp"],
            // Phase 2: Orchestration services
            ["mcp-orchestrator"],
            // Phase 3: External services
            ["image-generation-external-mcp", "calendar-management-mcp", "facebook-posting-mcp"],
        ];

        this.logFile = path.join(__dirname, "orchestrator.log");
        this.pidFile = path.join(__dirname, "orchestrator.pid");

        // Create logs directory
        this.logsDir = path.join(__dirname, "logs");
        if (!fs.existsSync(this.logsDir)) {
            fs.mkdirSync(this.logsDir, { recursive: true });
        }
    }

    log(message, level = "INFO") {
        const timestamp = new Date().toISOString();
        const logMessage = `[${timestamp}] [${level}] ${message}\n`;

        console.log(logMessage.trim());
        fs.appendFileSync(this.logFile, logMessage);
    }

    async checkHealth(serviceId) {
        const service = this.services[serviceId];
        if (!service) return false;

        return new Promise((resolve) => {
            const options = {
                hostname: "localhost",
                port: service.port,
                path: service.healthEndpoint,
                method: "GET",
                timeout: 5000,
            };

            const req = http.request(options, (res) => {
                let data = "";
                res.on("data", (chunk) => (data += chunk));
                res.on("end", () => {
                    try {
                        const response = JSON.parse(data);
                        const isHealthy = response.status === "healthy" || res.statusCode === 200;
                        resolve(isHealthy);
                    } catch (e) {
                        resolve(res.statusCode === 200);
                    }
                });
            });

            req.on("error", () => resolve(false));
            req.on("timeout", () => {
                req.destroy();
                resolve(false);
            });

            req.end();
        });
    }

    async waitForHealth(serviceId, maxAttempts = 30, intervalMs = 2000) {
        this.log(`Waiting for ${serviceId} to become healthy...`);

        for (let attempt = 1; attempt <= maxAttempts; attempt++) {
            const isHealthy = await this.checkHealth(serviceId);

            if (isHealthy) {
                this.log(`✅ ${serviceId} is healthy (attempt ${attempt}/${maxAttempts})`);
                return true;
            }

            if (attempt < maxAttempts) {
                this.log(`⏳ ${serviceId} not ready, waiting... (attempt ${attempt}/${maxAttempts})`);
                await new Promise((resolve) => setTimeout(resolve, intervalMs));
            }
        }

        this.log(`❌ ${serviceId} failed to become healthy after ${maxAttempts} attempts`, "ERROR");
        return false;
    }

    async buildService(serviceId) {
        const service = this.services[serviceId];
        if (!service.buildCommand) return true;

        this.log(`🔨 Building ${serviceId}...`);

        return new Promise((resolve) => {
            const buildProcess = spawn("bash", ["-c", service.buildCommand], {
                cwd: path.join(process.cwd(), service.path),
                stdio: ["inherit", "pipe", "pipe"],
            });

            let output = "";
            buildProcess.stdout.on("data", (data) => {
                output += data.toString();
            });

            buildProcess.stderr.on("data", (data) => {
                output += data.toString();
            });

            buildProcess.on("close", (code) => {
                if (code === 0) {
                    this.log(`✅ ${serviceId} built successfully`);
                    resolve(true);
                } else {
                    this.log(`❌ ${serviceId} build failed with code ${code}`, "ERROR");
                    this.log(`Build output: ${output}`, "ERROR");
                    resolve(false);
                }
            });
        });
    }

    async startService(serviceId) {
        const service = this.services[serviceId];
        if (!service) {
            this.log(`❌ Unknown service: ${serviceId}`, "ERROR");
            return false;
        }

        if (this.processes.has(serviceId)) {
            this.log(`⚠️ ${serviceId} is already running`);
            return true;
        }

        // Build service if needed
        if (!(await this.buildService(serviceId))) {
            return false;
        }

        this.log(`🚀 Starting ${service.name}...`);

        const logPath = path.join(this.logsDir, `${serviceId}.log`);
        const errorLogPath = path.join(this.logsDir, `${serviceId}.error.log`);

        const childProcess = spawn("bash", ["-c", service.startCommand], {
            cwd: path.join(process.cwd(), service.path),
            detached: true,
            stdio: ["ignore", fs.openSync(logPath, "a"), fs.openSync(errorLogPath, "a")],
        });

        childProcess.unref();

        this.processes.set(serviceId, {
            pid: childProcess.pid,
            process: childProcess,
            startTime: Date.now(),
            service: service,
        });

        // Wait a moment for the process to start
        await new Promise((resolve) => setTimeout(resolve, 1000));

        // Check if the process is still running
        try {
            childProcess.kill(0); // This just checks if process exists
            this.log(`✅ ${serviceId} started with PID ${childProcess.pid}`);
            return true;
        } catch (e) {
            this.log(`❌ ${serviceId} failed to start`, "ERROR");
            this.processes.delete(serviceId);
            return false;
        }
    }

    async stopService(serviceId) {
        const processInfo = this.processes.get(serviceId);
        if (!processInfo) {
            this.log(`⚠️ ${serviceId} is not running`);
            return true;
        }

        this.log(`🛑 Stopping ${serviceId}...`);

        try {
            // Try graceful shutdown first
            processInfo.process.kill("SIGTERM");

            // Wait a bit for graceful shutdown
            await new Promise((resolve) => setTimeout(resolve, 3000));

            // Check if still running
            try {
                processInfo.process.kill(0);
                // Still running, force kill
                this.log(`🔪 Force killing ${serviceId}...`);
                processInfo.process.kill("SIGKILL");
            } catch (e) {
                // Process is dead, which is what we want
            }

            this.processes.delete(serviceId);
            this.log(`✅ ${serviceId} stopped`);
            return true;
        } catch (error) {
            this.log(`❌ Error stopping ${serviceId}: ${error.message}`, "ERROR");
            return false;
        }
    }

    async startAll() {
        this.log("🚀 Starting AI-CORE Service Orchestration...");

        const startTime = Date.now();
        let totalServices = 0;
        let successfulServices = 0;

        for (const phase of this.startupOrder) {
            this.log(`📋 Starting Phase: [${phase.join(", ")}]`);

            // Start all services in this phase
            const startPromises = phase.map(async (serviceId) => {
                totalServices++;

                // Check dependencies first
                const service = this.services[serviceId];
                for (const dep of service.dependencies) {
                    if (!this.processes.has(dep)) {
                        this.log(`❌ Dependency ${dep} not running for ${serviceId}`, "ERROR");
                        return false;
                    }
                }

                const started = await this.startService(serviceId);
                if (started) {
                    const healthy = await this.waitForHealth(serviceId);
                    if (healthy) {
                        successfulServices++;
                        return true;
                    }
                }
                return false;
            });

            const results = await Promise.all(startPromises);
            const phaseSuccess = results.every((r) => r);

            if (!phaseSuccess) {
                this.log(`❌ Phase failed, aborting startup`, "ERROR");
                await this.stopAll();
                return false;
            }

            this.log(`✅ Phase completed successfully`);
        }

        const duration = Date.now() - startTime;
        this.log(
            `🎉 Service orchestration completed! ${successfulServices}/${totalServices} services healthy (${duration}ms)`,
        );

        // Generate service status report
        await this.generateStatusReport();

        return successfulServices === totalServices;
    }

    async stopAll() {
        this.log("🛑 Stopping all services...");

        const serviceIds = Array.from(this.processes.keys());
        const stopPromises = serviceIds.map((serviceId) => this.stopService(serviceId));

        await Promise.all(stopPromises);

        this.log("✅ All services stopped");
    }

    async getServiceStatus() {
        const status = {
            timestamp: new Date().toISOString(),
            services: {},
            summary: {
                total: Object.keys(this.services).length,
                running: 0,
                healthy: 0,
                unhealthy: 0,
                stopped: 0,
            },
        };

        for (const [serviceId, service] of Object.entries(this.services)) {
            const isRunning = this.processes.has(serviceId);
            const isHealthy = isRunning ? await this.checkHealth(serviceId) : false;

            status.services[serviceId] = {
                name: service.name,
                port: service.port,
                running: isRunning,
                healthy: isHealthy,
                pid: isRunning ? this.processes.get(serviceId).pid : null,
                uptime: isRunning ? Date.now() - this.processes.get(serviceId).startTime : 0,
            };

            if (isRunning) {
                status.summary.running++;
                if (isHealthy) {
                    status.summary.healthy++;
                } else {
                    status.summary.unhealthy++;
                }
            } else {
                status.summary.stopped++;
            }
        }

        return status;
    }

    async generateStatusReport() {
        const status = await this.getServiceStatus();
        const reportPath = path.join(this.logsDir, "service-status.json");

        fs.writeFileSync(reportPath, JSON.stringify(status, null, 2));

        // Also generate human-readable report
        const readableReport = this.formatStatusReport(status);
        const readableReportPath = path.join(this.logsDir, "service-status.txt");
        fs.writeFileSync(readableReportPath, readableReport);

        this.log(`📊 Status report generated: ${reportPath}`);

        return status;
    }

    formatStatusReport(status) {
        let report = `AI-CORE Service Status Report\n`;
        report += `Generated: ${status.timestamp}\n`;
        report += `=`.repeat(50) + "\n\n";

        report += `Summary:\n`;
        report += `  Total Services: ${status.summary.total}\n`;
        report += `  Running: ${status.summary.running}\n`;
        report += `  Healthy: ${status.summary.healthy}\n`;
        report += `  Unhealthy: ${status.summary.unhealthy}\n`;
        report += `  Stopped: ${status.summary.stopped}\n\n`;

        report += `Service Details:\n`;
        report += `-`.repeat(50) + "\n";

        for (const [serviceId, service] of Object.entries(status.services)) {
            const statusIcon = service.healthy ? "🟢" : service.running ? "🟡" : "🔴";
            const uptimeStr = service.uptime > 0 ? `${Math.floor(service.uptime / 1000)}s` : "N/A";

            report += `${statusIcon} ${service.name}\n`;
            report += `   Port: ${service.port}\n`;
            report += `   Status: ${service.running ? "Running" : "Stopped"}\n`;
            report += `   Health: ${service.healthy ? "Healthy" : "Unhealthy"}\n`;
            report += `   Uptime: ${uptimeStr}\n`;
            if (service.pid) {
                report += `   PID: ${service.pid}\n`;
            }
            report += "\n";
        }

        return report;
    }

    async runTests() {
        this.log("🧪 Running comprehensive test suite...");

        // Ensure all services are running
        const status = await this.getServiceStatus();
        if (status.summary.healthy !== status.summary.total) {
            this.log("❌ Not all services are healthy, cannot run tests", "ERROR");
            return false;
        }

        // Run workflow validation tests
        try {
            const workflowValidator = require("./workflow-validator.js");
            const validator = new workflowValidator();
            const testResults = await validator.runAllTests();

            this.log(`📊 Test Results: ${testResults.passed}/${testResults.total} passed`);

            return testResults.passed === testResults.total;
        } catch (error) {
            this.log(`❌ Test execution failed: ${error.message}`, "ERROR");
            return false;
        }
    }
}

// CLI Interface
if (require.main === module) {
    const orchestrator = new ServiceOrchestrator();
    const command = process.argv[2] || "help";

    (async () => {
        try {
            switch (command) {
                case "start":
                    const started = await orchestrator.startAll();
                    process.exit(started ? 0 : 1);
                    break;

                case "stop":
                    await orchestrator.stopAll();
                    process.exit(0);
                    break;

                case "status":
                    const status = await orchestrator.getServiceStatus();
                    console.log(orchestrator.formatStatusReport(status));
                    process.exit(0);
                    break;

                case "test":
                    const testsPassed = await orchestrator.runTests();
                    process.exit(testsPassed ? 0 : 1);
                    break;

                case "restart":
                    await orchestrator.stopAll();
                    await new Promise((resolve) => setTimeout(resolve, 2000));
                    const restarted = await orchestrator.startAll();
                    process.exit(restarted ? 0 : 1);
                    break;

                case "help":
                default:
                    console.log(`AI-CORE Service Orchestrator

Usage:
  node service-orchestrator.js start     # Start all services in order
  node service-orchestrator.js stop      # Stop all services gracefully
  node service-orchestrator.js status    # Show current service status
  node service-orchestrator.js test      # Run comprehensive test suite
  node service-orchestrator.js restart   # Restart all services
  node service-orchestrator.js help      # Show this help

Services Managed:
  - Built-in MCPs (Ports 8804-8807): Content, Text Processing, Image Gen, Orchestrator
  - External MCPs (Ports 8091-8093): Image Gen, Calendar, Facebook

Logs: tests/logs/
Status: tests/logs/service-status.json`);
                    process.exit(0);
            }
        } catch (error) {
            console.error(`Fatal error: ${error.message}`);
            process.exit(1);
        }
    })();
}

module.exports = ServiceOrchestrator;
