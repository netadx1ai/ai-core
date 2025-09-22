#!/usr/bin/env node

/**
 * Test script to verify mock API server fix
 * Tests workflow creation and step processing without race conditions
 */

import http from "http";
import WebSocket from "ws";

const API_BASE = "http://localhost:8090";
const WS_URL = "ws://localhost:8091";

let testResults = {
    passed: 0,
    failed: 0,
    tests: [],
};

function log(message) {
    console.log(`[TEST] ${new Date().toISOString()} ${message}`);
}

function addTestResult(name, passed, message = "") {
    testResults.tests.push({ name, passed, message });
    if (passed) {
        testResults.passed++;
        log(`✅ ${name}`);
    } else {
        testResults.failed++;
        log(`❌ ${name}: ${message}`);
    }
}

async function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const url = new URL(path, API_BASE);
        const options = {
            method,
            hostname: url.hostname,
            port: url.port,
            path: url.pathname + url.search,
            headers: {
                "Content-Type": "application/json",
                Accept: "application/json",
            },
        };

        const req = http.request(options, (res) => {
            let body = "";
            res.on("data", (chunk) => (body += chunk));
            res.on("end", () => {
                try {
                    const parsed = body ? JSON.parse(body) : {};
                    resolve({ status: res.statusCode, data: parsed });
                } catch (e) {
                    resolve({ status: res.statusCode, data: body });
                }
            });
        });

        req.on("error", reject);

        if (data) {
            req.write(JSON.stringify(data));
        }
        req.end();
    });
}

async function testHealthCheck() {
    try {
        const response = await makeRequest("GET", "/health");
        const passed = response.status === 200 && response.data.status === "healthy";
        addTestResult("Health Check", passed, passed ? "" : `Status: ${response.status}`);
    } catch (error) {
        addTestResult("Health Check", false, error.message);
    }
}

async function testWorkflowCreation() {
    try {
        const workflowData = {
            title: "Test Workflow",
            definition: "Test workflow to verify step processing",
            workflow_type: "blog-post",
            config: { client_demo: true },
        };

        const response = await makeRequest("POST", "/v1/workflows", workflowData);
        const passed = response.status === 201 && response.data.workflow_id;

        if (passed) {
            // Store workflow ID for further tests
            global.testWorkflowId = response.data.workflow_id;
            addTestResult("Workflow Creation", true);
        } else {
            addTestResult("Workflow Creation", false, `Status: ${response.status}`);
        }
    } catch (error) {
        addTestResult("Workflow Creation", false, error.message);
    }
}

async function testWorkflowStatus() {
    if (!global.testWorkflowId) {
        addTestResult("Workflow Status", false, "No workflow ID available");
        return;
    }

    try {
        const response = await makeRequest("GET", `/v1/workflows/${global.testWorkflowId}`);
        const passed = response.status === 200 && response.data.id === global.testWorkflowId;
        addTestResult("Workflow Status", passed, passed ? "" : `Status: ${response.status}`);
    } catch (error) {
        addTestResult("Workflow Status", false, error.message);
    }
}

async function testStepProcessing() {
    if (!global.testWorkflowId) {
        addTestResult("Step Processing", false, "No workflow ID available");
        return;
    }

    try {
        // Wait for workflow to process through steps
        let attempts = 0;
        const maxAttempts = 30;
        let lastStatus = "";

        while (attempts < maxAttempts) {
            const response = await makeRequest("GET", `/v1/workflows/${global.testWorkflowId}`);

            if (response.status === 200) {
                const workflow = response.data;
                lastStatus = workflow.status;

                if (workflow.status === "completed") {
                    const hasSteps = workflow.steps && workflow.steps.length > 0;
                    const allStepsHaveStatus = hasSteps && workflow.steps.every((step) => step.status);
                    const passed = hasSteps && allStepsHaveStatus;

                    addTestResult(
                        "Step Processing",
                        passed,
                        passed ? `Completed with ${workflow.steps.length} steps` : "Steps missing or invalid",
                    );
                    return;
                }

                if (workflow.status === "failed") {
                    addTestResult("Step Processing", false, "Workflow failed");
                    return;
                }
            }

            attempts++;
            await new Promise((resolve) => setTimeout(resolve, 1000));
        }

        addTestResult("Step Processing", false, `Timeout after ${maxAttempts}s, last status: ${lastStatus}`);
    } catch (error) {
        addTestResult("Step Processing", false, error.message);
    }
}

async function testWebSocket() {
    return new Promise((resolve) => {
        try {
            const ws = new WebSocket(WS_URL);
            let connected = false;

            const timeout = setTimeout(() => {
                if (!connected) {
                    addTestResult("WebSocket Connection", false, "Connection timeout");
                    ws.close();
                    resolve();
                }
            }, 5000);

            ws.on("open", () => {
                connected = true;
                clearTimeout(timeout);
                addTestResult("WebSocket Connection", true);
                ws.close();
                resolve();
            });

            ws.on("error", (error) => {
                clearTimeout(timeout);
                addTestResult("WebSocket Connection", false, error.message);
                resolve();
            });
        } catch (error) {
            addTestResult("WebSocket Connection", false, error.message);
            resolve();
        }
    });
}

async function runAllTests() {
    log("Starting mock API server tests...");
    log("Make sure the mock server is running: node mock-api-server.cjs");

    await testHealthCheck();
    await testWorkflowCreation();
    await testWorkflowStatus();
    await testWebSocket();
    await testStepProcessing(); // Run this last as it takes longest

    log("\n📊 Test Results Summary:");
    log(`✅ Passed: ${testResults.passed}`);
    log(`❌ Failed: ${testResults.failed}`);
    log(`📝 Total: ${testResults.tests.length}`);

    if (testResults.failed > 0) {
        log("\n❌ Failed Tests:");
        testResults.tests.filter((test) => !test.passed).forEach((test) => log(`  - ${test.name}: ${test.message}`));
        process.exit(1);
    } else {
        log("\n🎉 All tests passed! Mock API server is working correctly.");
        process.exit(0);
    }
}

// Handle graceful shutdown
process.on("SIGINT", () => {
    log("Test interrupted");
    process.exit(1);
});

process.on("unhandledRejection", (reason, promise) => {
    log(`Unhandled promise rejection: ${reason}`);
    process.exit(1);
});

// Run tests
runAllTests().catch((error) => {
    log(`Test suite failed: ${error.message}`);
    process.exit(1);
});
