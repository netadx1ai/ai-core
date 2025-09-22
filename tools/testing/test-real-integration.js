#!/usr/bin/env node

/**
 * Test Real AI-CORE Integration
 *
 * This script tests the complete integration between the client app and real AI-CORE services.
 * It verifies service connectivity, workflow creation, and data transformation.
 */

const http = require("http");
const https = require("https");

// Test configuration
const SERVICES = {
    federation: "http://localhost:8801",
    intentParser: "http://localhost:8802",
    mcpManager: "http://localhost:8804",
    mcpDirect: "http://localhost:8804",
    clientApp: "http://localhost:5173",
};

const TEST_WORKFLOW = {
    intent: "Write a blog post about AI-powered automation in healthcare",
    workflow_type: "blog-post-social",
    client_context: {
        user_id: "test-integration-user",
        test_run: true,
    },
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
                ...options.headers,
            },
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
            });
            return { healthy: true, type: serviceType, data: response.data };
        } else {
            log("WARN", `${serviceName} returned ${response.status}`, response.data);
            return { healthy: false, type: "UNKNOWN", error: `HTTP ${response.status}` };
        }
    } catch (error) {
        log("ERROR", `${serviceName} health check failed`, { error: error.message });
        return { healthy: false, type: "UNKNOWN", error: error.message };
    }
}

async function testWorkflowCreation() {
    log("INFO", "Testing workflow creation...");

    try {
        const response = await makeRequest(`${SERVICES.federation}/v1/workflows`, {
            method: "POST",
            body: TEST_WORKFLOW,
        });

        if (response.status === 200 && response.data.workflow_id) {
            log("SUCCESS", "Workflow created successfully", {
                workflow_id: response.data.workflow_id,
                status: response.data.status,
                estimated_duration: response.data.estimated_duration,
            });
            return { success: true, workflow_id: response.data.workflow_id };
        } else {
            log("ERROR", "Workflow creation failed", response);
            return { success: false, error: response };
        }
    } catch (error) {
        log("ERROR", "Workflow creation error", { error: error.message });
        return { success: false, error: error.message };
    }
}

async function testWorkflowProgress(workflowId, maxWaitTime = 120000) {
    log("INFO", `Testing workflow progress for ${workflowId}...`);

    const startTime = Date.now();
    let lastProgress = -1;

    while (Date.now() - startTime < maxWaitTime) {
        try {
            const response = await makeRequest(`${SERVICES.federation}/v1/workflows/${workflowId}`);

            if (response.status === 200) {
                const { status, progress, current_step, results, error } = response.data;

                if (progress !== lastProgress) {
                    log("INFO", `Workflow progress: ${progress}% - ${current_step}`, {
                        status,
                        progress,
                        current_step,
                    });
                    lastProgress = progress;
                }

                if (status === "completed" && results) {
                    log("SUCCESS", "Workflow completed with results!", {
                        blog_post_title: results.blog_post?.title,
                        word_count: results.blog_post?.word_count,
                        quality_score: results.quality_scores?.overall_score,
                        has_image: !!results.image,
                        execution_time_ms: results.metrics?.execution_time_ms,
                    });
                    return { success: true, results };
                } else if (status === "failed" || error) {
                    log("ERROR", "Workflow failed", { status, error });
                    return { success: false, error: error || "Workflow failed" };
                }

                await sleep(2000); // Wait 2 seconds before next check
            } else {
                log("WARN", `Workflow status check returned ${response.status}`, response.data);
                await sleep(5000);
            }
        } catch (error) {
            log("WARN", "Error checking workflow progress", { error: error.message });
            await sleep(5000);
        }
    }

    log("WARN", "Workflow did not complete within timeout");
    return { success: false, error: "Timeout" };
}

async function testClientAppConnectivity() {
    log("INFO", "Testing client app connectivity...");

    try {
        const response = await makeRequest(SERVICES.clientApp);

        if (response.status === 200 && response.raw.includes("<title>")) {
            log("SUCCESS", "Client app is running and accessible");
            return { success: true };
        } else {
            log("WARN", "Client app returned unexpected response", {
                status: response.status,
                content_preview: response.raw.substring(0, 200),
            });
            return { success: false, error: `HTTP ${response.status}` };
        }
    } catch (error) {
        log("ERROR", "Client app connectivity test failed", { error: error.message });
        return { success: false, error: error.message };
    }
}

async function testDataTransformation(workflowResults) {
    log("INFO", "Testing data transformation compatibility...");

    try {
        // Simulate the transformation that the client app performs
        const blogPost = workflowResults.blog_post;
        const qualityScores = workflowResults.quality_scores;
        const image = workflowResults.image;

        const transformed = {
            content: {
                title: blogPost?.title,
                content: blogPost?.content,
                word_count: blogPost?.word_count || 0,
                seo_keywords: blogPost?.seo_keywords || [],
                featured_image_url: image?.url,
            },
            images: image
                ? [
                      {
                          url: image.url,
                          alt_text: image.alt_text,
                          width: image.width || 0,
                          height: image.height || 0,
                      },
                  ]
                : [],
            quality_score: qualityScores?.overall_score || 0,
        };

        const hasValidContent = !!(transformed.content.title && transformed.content.content);
        const hasValidQuality = transformed.quality_score > 0;
        const hasValidImage = transformed.images.length > 0;

        log("SUCCESS", "Data transformation test completed", {
            has_valid_content: hasValidContent,
            has_valid_quality: hasValidQuality,
            has_valid_image: hasValidImage,
            transformation_preview: {
                title_length: transformed.content.title?.length || 0,
                content_length: transformed.content.content?.length || 0,
                quality_score: transformed.quality_score,
                image_count: transformed.images.length,
            },
        });

        return {
            success: true,
            valid: hasValidContent && hasValidQuality,
            transformed,
        };
    } catch (error) {
        log("ERROR", "Data transformation test failed", { error: error.message });
        return { success: false, error: error.message };
    }
}

// Main test runner
async function runIntegrationTests() {
    console.log("\n🚀 Starting Real AI-CORE Integration Tests\n");

    const results = {
        services: {},
        workflow: null,
        client_app: null,
        data_transformation: null,
        summary: {
            total_tests: 0,
            passed: 0,
            failed: 0,
            warnings: 0,
        },
    };

    // Test 1: Service Health Checks
    log("INFO", "=== PHASE 1: Service Health Checks ===");
    for (const [serviceName, serviceUrl] of Object.entries(SERVICES)) {
        if (serviceName === "clientApp") continue; // Test separately

        const healthResult = await testServiceHealth(serviceName, serviceUrl);
        results.services[serviceName] = healthResult;
        results.summary.total_tests++;

        if (healthResult.healthy) {
            results.summary.passed++;
        } else {
            results.summary.failed++;
        }
    }

    // Test 2: Client App Connectivity
    log("INFO", "\n=== PHASE 2: Client App Connectivity ===");
    results.client_app = await testClientAppConnectivity();
    results.summary.total_tests++;
    if (results.client_app.success) {
        results.summary.passed++;
    } else {
        results.summary.failed++;
    }

    // Test 3: Workflow Creation and Execution
    log("INFO", "\n=== PHASE 3: Workflow Creation and Execution ===");
    const workflowCreation = await testWorkflowCreation();
    results.summary.total_tests++;

    if (workflowCreation.success) {
        results.summary.passed++;

        const workflowProgress = await testWorkflowProgress(workflowCreation.workflow_id);
        results.summary.total_tests++;

        if (workflowProgress.success) {
            results.summary.passed++;
            results.workflow = {
                created: true,
                completed: true,
                workflow_id: workflowCreation.workflow_id,
                results: workflowProgress.results,
            };

            // Test 4: Data Transformation
            log("INFO", "\n=== PHASE 4: Data Transformation ===");
            const transformationResult = await testDataTransformation(workflowProgress.results);
            results.data_transformation = transformationResult;
            results.summary.total_tests++;

            if (transformationResult.success && transformationResult.valid) {
                results.summary.passed++;
            } else {
                results.summary.warnings++;
            }
        } else {
            results.summary.failed++;
            results.workflow = {
                created: true,
                completed: false,
                error: workflowProgress.error,
            };
        }
    } else {
        results.summary.failed++;
        results.workflow = {
            created: false,
            error: workflowCreation.error,
        };
    }

    // Final Summary
    log("INFO", "\n=== INTEGRATION TEST SUMMARY ===");

    const realServices = Object.values(results.services).filter((s) => s.type === "REAL").length;
    const mockServices = Object.values(results.services).filter((s) => s.type === "MOCK").length;
    const healthyServices = Object.values(results.services).filter((s) => s.healthy).length;

    console.log(`
📊 Test Results:
   Total Tests: ${results.summary.total_tests}
   ✅ Passed: ${results.summary.passed}
   ❌ Failed: ${results.summary.failed}
   ⚠️  Warnings: ${results.summary.warnings}

🔧 Services Status:
   Real Services: ${realServices}
   Mock Services: ${mockServices}
   Healthy Services: ${healthyServices}/${Object.keys(results.services).length}

🚀 Workflow Execution:
   Created: ${results.workflow?.created ? "✅" : "❌"}
   Completed: ${results.workflow?.completed ? "✅" : "❌"}

📱 Client App:
   Accessible: ${results.client_app?.success ? "✅" : "❌"}

🔄 Data Transformation:
   Compatible: ${results.data_transformation?.valid ? "✅" : "❌"}
    `);

    if (results.summary.failed === 0 && realServices >= 2) {
        log("SUCCESS", "🎉 ALL INTEGRATION TESTS PASSED! Real AI-CORE integration is working properly.");
        return 0;
    } else if (results.summary.failed === 0) {
        log("WARN", "⚠️  Tests passed but some services are still in mock mode.");
        return 1;
    } else {
        log("ERROR", "❌ Some integration tests failed. Check the logs above for details.");
        return 2;
    }
}

// Run tests if called directly
if (require.main === module) {
    runIntegrationTests()
        .then((exitCode) => process.exit(exitCode))
        .catch((error) => {
            log("ERROR", "Test runner crashed", { error: error.message });
            process.exit(3);
        });
}

module.exports = { runIntegrationTests, testServiceHealth, testWorkflowCreation };
