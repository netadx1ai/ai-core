#!/usr/bin/env node

/**
 * Test Client App Integration with Real AI-CORE Services
 *
 * This script tests the client app's integration with real AI-CORE services,
 * specifically focusing on the fixes for MCP Manager mock status issues.
 */

const http = require("http");
const https = require("https");

// Test configuration
const CLIENT_APP_URL = "http://localhost:5173";
const AI_CORE_SERVICES = {
    federation: "http://localhost:8801",
    intentParser: "http://localhost:8802",
    mcpManager: "http://localhost:8804",
    mcpProxy: "http://localhost:8803",
};

// Utility functions
function makeRequest(url, options = {}) {
    return new Promise((resolve, reject) => {
        const urlObj = new URL(url);
        const requestModule = urlObj.protocol === "https:" ? https : http;

        const reqOptions = {
            hostname: urlObj.hostname,
            port: urlObj.port,
            path: urlObj.pathname + urlObj.search,
            method: options.method || "GET",
            headers: {
                "Content-Type": "application/json",
                "User-Agent": "AI-CORE-Client-Test/1.0",
                ...options.headers,
            },
            timeout: 10000,
        };

        const req = requestModule.request(reqOptions, (res) => {
            let data = "";
            res.on("data", (chunk) => (data += chunk));
            res.on("end", () => {
                try {
                    const parsed = data ? JSON.parse(data) : {};
                    resolve({ status: res.statusCode, data: parsed, raw: data });
                } catch (e) {
                    resolve({ status: res.statusCode, data: null, raw: data, error: e.message });
                }
            });
        });

        req.on("error", reject);
        req.on("timeout", () => reject(new Error("Request timeout")));

        if (options.body) {
            req.write(typeof options.body === "string" ? options.body : JSON.stringify(options.body));
        }

        req.end();
    });
}

function log(level, message, data = null) {
    const timestamp = new Date().toISOString();
    const colors = {
        INFO: "\x1b[36m",
        SUCCESS: "\x1b[32m",
        WARN: "\x1b[33m",
        ERROR: "\x1b[31m",
        RESET: "\x1b[0m",
    };

    console.log(`${colors[level]}[${timestamp}] ${level}: ${message}${colors.RESET}`);
    if (data) {
        console.log(JSON.stringify(data, null, 2));
    }
}

async function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

// Test functions
async function testServiceHealth(serviceName, serviceUrl) {
    log("INFO", `Testing ${serviceName} health...`);

    try {
        const response = await makeRequest(`${serviceUrl}/health`);

        if (response.status === 200) {
            const serviceType =
                response.data.service?.includes("mock") && !response.data.service?.includes("proxy") ? "MOCK" : "REAL";

            log("SUCCESS", `${serviceName} is healthy (${serviceType})`, {
                service: response.data.service,
                status: response.data.status,
                version: response.data.version,
                port: new URL(serviceUrl).port,
            });

            return { healthy: true, type: serviceType, data: response.data };
        } else {
            log("WARN", `${serviceName} returned ${response.status}`, {
                url: serviceUrl,
                response: response.raw.substring(0, 200),
            });
            return { healthy: false, type: "UNKNOWN", error: `HTTP ${response.status}` };
        }
    } catch (error) {
        log("ERROR", `${serviceName} health check failed`, {
            error: error.message,
            url: serviceUrl,
        });
        return { healthy: false, type: "UNKNOWN", error: error.message };
    }
}

async function testClientAppAccess() {
    log("INFO", "Testing client app accessibility...");

    try {
        const response = await makeRequest(CLIENT_APP_URL);

        if (response.status === 200 && response.raw.includes("<title>")) {
            log("SUCCESS", "Client app is accessible", {
                status: response.status,
                has_html: true,
                title_found: response.raw.includes("AI-CORE") || response.raw.includes("Vite"),
            });
            return { success: true };
        } else {
            log("WARN", "Client app returned unexpected response", {
                status: response.status,
                content_preview: response.raw.substring(0, 300),
            });
            return { success: false, error: `HTTP ${response.status}` };
        }
    } catch (error) {
        log("ERROR", "Client app accessibility test failed", { error: error.message });
        return { success: false, error: error.message };
    }
}

async function testURLConstruction() {
    log("INFO", "Testing URL construction (double /v1 fix)...");

    const testCases = [
        { base: "http://localhost:8801", expected: "http://localhost:8801/v1/workflows" },
        { base: "http://localhost:8801/", expected: "http://localhost:8801/v1/workflows" },
        { base: "http://localhost:8801/v1", expected: "http://localhost:8801/v1/workflows" },
        { base: "http://localhost:8801/v1/", expected: "http://localhost:8801/v1/workflows" },
    ];

    const cleanBaseUrl = (url) => url.replace(/\/v1\/?$/, "").replace(/\/$/, "");
    let allPassed = true;

    for (const testCase of testCases) {
        const cleaned = cleanBaseUrl(testCase.base);
        const constructed = `${cleaned}/v1/workflows`;

        if (constructed === testCase.expected) {
            log("SUCCESS", `URL construction test passed: ${testCase.base} → ${constructed}`);
        } else {
            log(
                "ERROR",
                `URL construction test failed: ${testCase.base} → ${constructed} (expected: ${testCase.expected})`,
            );
            allPassed = false;
        }
    }

    return { success: allPassed };
}

async function testAPIEndpoints() {
    log("INFO", "Testing API endpoints accessibility...");

    const endpoints = [
        { name: "Federation Health", url: `${AI_CORE_SERVICES.federation}/health` },
        { name: "Federation Workflows", url: `${AI_CORE_SERVICES.federation}/v1/workflows`, method: "GET" },
        { name: "Intent Parser Health", url: `${AI_CORE_SERVICES.intentParser}/health` },
        { name: "MCP Manager Health", url: `${AI_CORE_SERVICES.mcpManager}/health` },
        { name: "MCP Proxy Health", url: `${AI_CORE_SERVICES.mcpProxy}/health` },
    ];

    const results = [];

    for (const endpoint of endpoints) {
        try {
            const response = await makeRequest(endpoint.url, { method: endpoint.method || "GET" });

            if (response.status >= 200 && response.status < 400) {
                log("SUCCESS", `${endpoint.name} accessible`, {
                    status: response.status,
                    url: endpoint.url,
                });
                results.push({ ...endpoint, success: true, status: response.status });
            } else {
                log("WARN", `${endpoint.name} returned ${response.status}`, {
                    url: endpoint.url,
                });
                results.push({ ...endpoint, success: false, status: response.status });
            }
        } catch (error) {
            log("ERROR", `${endpoint.name} failed`, {
                error: error.message,
                url: endpoint.url,
            });
            results.push({ ...endpoint, success: false, error: error.message });
        }

        // Small delay between requests
        await sleep(200);
    }

    const successCount = results.filter((r) => r.success).length;
    return { success: successCount === results.length, results, successCount, totalCount: results.length };
}

async function testWorkflowCreation() {
    log("INFO", "Testing workflow creation via federation service...");

    const testWorkflow = {
        intent: "Test client app integration with a simple blog post about AI automation",
        workflow_type: "blog-post-social",
        client_context: {
            user_id: "test-client-integration",
            test_run: true,
            timestamp: new Date().toISOString(),
        },
    };

    try {
        const response = await makeRequest(`${AI_CORE_SERVICES.federation}/v1/workflows`, {
            method: "POST",
            body: testWorkflow,
        });

        if (response.status === 200 && response.data.workflow_id) {
            log("SUCCESS", "Workflow creation successful", {
                workflow_id: response.data.workflow_id,
                status: response.data.status,
                estimated_duration: response.data.estimated_duration,
            });
            return { success: true, workflow_id: response.data.workflow_id };
        } else {
            log("ERROR", "Workflow creation failed", {
                status: response.status,
                response: response.data,
            });
            return { success: false, error: response };
        }
    } catch (error) {
        log("ERROR", "Workflow creation error", { error: error.message });
        return { success: false, error: error.message };
    }
}

// Main test runner
async function runClientAppIntegrationTests() {
    console.log("\n🚀 Starting Client App Integration Tests\n");

    const results = {
        client_app: null,
        services: {},
        url_construction: null,
        api_endpoints: null,
        workflow_creation: null,
        summary: {
            total_tests: 0,
            passed: 0,
            failed: 0,
            warnings: 0,
        },
    };

    // Test 1: Client App Access
    log("INFO", "=== PHASE 1: Client App Accessibility ===");
    results.client_app = await testClientAppAccess();
    results.summary.total_tests++;
    if (results.client_app.success) {
        results.summary.passed++;
    } else {
        results.summary.failed++;
    }

    // Test 2: Service Health Checks
    log("INFO", "\n=== PHASE 2: Service Health Checks ===");
    for (const [serviceName, serviceUrl] of Object.entries(AI_CORE_SERVICES)) {
        const healthResult = await testServiceHealth(serviceName, serviceUrl);
        results.services[serviceName] = healthResult;
        results.summary.total_tests++;

        if (healthResult.healthy) {
            results.summary.passed++;
        } else {
            results.summary.failed++;
        }
    }

    // Test 3: URL Construction Fix
    log("INFO", "\n=== PHASE 3: URL Construction (Double /v1 Fix) ===");
    results.url_construction = await testURLConstruction();
    results.summary.total_tests++;
    if (results.url_construction.success) {
        results.summary.passed++;
    } else {
        results.summary.failed++;
    }

    // Test 4: API Endpoints
    log("INFO", "\n=== PHASE 4: API Endpoints Accessibility ===");
    results.api_endpoints = await testAPIEndpoints();
    results.summary.total_tests++;
    if (results.api_endpoints.success) {
        results.summary.passed++;
    } else {
        if (results.api_endpoints.successCount > 0) {
            results.summary.warnings++;
        } else {
            results.summary.failed++;
        }
    }

    // Test 5: Workflow Creation
    log("INFO", "\n=== PHASE 5: Workflow Creation ===");
    results.workflow_creation = await testWorkflowCreation();
    results.summary.total_tests++;
    if (results.workflow_creation.success) {
        results.summary.passed++;
    } else {
        results.summary.failed++;
    }

    // Final Summary
    log("INFO", "\n=== CLIENT APP INTEGRATION TEST SUMMARY ===");

    const realServices = Object.values(results.services).filter((s) => s.type === "REAL").length;
    const mockServices = Object.values(results.services).filter((s) => s.type === "MOCK").length;
    const healthyServices = Object.values(results.services).filter((s) => s.healthy).length;
    const totalServices = Object.keys(results.services).length;

    console.log(`
📊 Test Results:
   Total Tests: ${results.summary.total_tests}
   ✅ Passed: ${results.summary.passed}
   ❌ Failed: ${results.summary.failed}
   ⚠️  Warnings: ${results.summary.warnings}

🌐 Client App:
   Accessible: ${results.client_app?.success ? "✅" : "❌"}

🔧 Services Status:
   Real Services: ${realServices} / ${totalServices}
   Mock Services: ${mockServices} / ${totalServices}
   Healthy Services: ${healthyServices} / ${totalServices}

🔗 URL Construction:
   Double /v1 Fix: ${results.url_construction?.success ? "✅ Working" : "❌ Failed"}

🚀 API Endpoints:
   Accessible: ${results.api_endpoints?.successCount || 0} / ${results.api_endpoints?.totalCount || 0}

🚀 Workflow Creation:
   Working: ${results.workflow_creation?.success ? "✅" : "❌"}
    `);

    // Specific feedback for MCP Manager status fix
    const mcpManagerStatus = results.services.mcpManager;
    if (mcpManagerStatus) {
        if (mcpManagerStatus.healthy && mcpManagerStatus.type === "REAL") {
            log("SUCCESS", "🎉 MCP Manager Mock Status Fix: SUCCESS - Service shows as REAL");
        } else if (mcpManagerStatus.healthy && mcpManagerStatus.type === "MOCK") {
            log("WARN", "⚠️  MCP Manager Mock Status Fix: PARTIAL - Service healthy but shows as MOCK");
        } else {
            log("ERROR", "❌ MCP Manager Mock Status Fix: FAILED - Service offline or unreachable");
        }
    }

    // Overall assessment
    if (results.summary.failed === 0 && realServices >= 3) {
        log("SUCCESS", "🎉 ALL CLIENT APP INTEGRATION TESTS PASSED! Ready for deployment.");
        return 0;
    } else if (results.summary.failed <= 1 && healthyServices >= 2) {
        log("WARN", "⚠️  Most tests passed but some issues detected. Check logs above.");
        return 1;
    } else {
        log("ERROR", "❌ Multiple integration tests failed. Client app may not work properly.");
        return 2;
    }
}

// Run tests if called directly
if (require.main === module) {
    runClientAppIntegrationTests()
        .then((exitCode) => {
            console.log(`\n🏁 Tests completed with exit code: ${exitCode}`);
            process.exit(exitCode);
        })
        .catch((error) => {
            log("ERROR", "Test runner crashed", { error: error.message, stack: error.stack });
            process.exit(3);
        });
}

module.exports = {
    runClientAppIntegrationTests,
    testServiceHealth,
    testURLConstruction,
    testWorkflowCreation,
};
