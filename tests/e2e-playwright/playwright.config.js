import { defineConfig, devices } from "@playwright/test";
import dotenv from "dotenv";

// Load environment variables
dotenv.config();

/**
 * Playwright Configuration for AI-CORE E2E Tests
 * Multiple Run Strategies: Stability, Load, Regression Testing
 */
export default defineConfig({
    // Test directory
    testDir: "./tests",

    // Global test timeout
    timeout: 30 * 1000,

    // Expect timeout for assertions
    expect: {
        timeout: 5000,
    },

    // Fail the build on CI if you accidentally left test.only in the source code
    forbidOnly: !!process.env.CI,

    // Retry on CI only
    retries: process.env.CI ? 2 : 1,

    // Opt out of parallel tests on CI
    workers: process.env.CI ? 2 : undefined,

    // Reporter configuration for multiple runs
    reporter: [
        [
            "html",
            {
                outputFolder: "./test-results/html-report",
                open: "never",
            },
        ],
        [
            "json",
            {
                outputFile: "./test-results/test-results.json",
            },
        ],
        [
            "junit",
            {
                outputFile: "./test-results/junit.xml",
            },
        ],
        ["line"],
        // ['./scripts/custom-reporter.js'] // Disabled until reporter is created
    ],

    // Global setup and teardown - disabled until scripts are created
    // globalSetup: './scripts/global-setup.js',
    // globalTeardown: './scripts/global-teardown.js',

    // Output directory for test artifacts
    outputDir: "./test-results/artifacts",

    // Use configuration
    use: {
        // Base URL for the client app
        baseURL: process.env.BASE_URL || "http://localhost:8090",

        // Global test timeout
        actionTimeout: 10 * 1000,
        navigationTimeout: 15 * 1000,

        // Collect trace when retrying the failed test
        trace: "retain-on-failure",

        // Take screenshot on failure
        screenshot: "only-on-failure",

        // Record video on failure
        video: "retain-on-failure",

        // Browser viewport
        viewport: { width: 1280, height: 720 },

        // Ignore HTTPS errors
        ignoreHTTPSErrors: true,

        // Accept downloads
        acceptDownloads: true,

        // Extra HTTP headers
        extraHTTPHeaders: {
            Accept: "application/json",
            "Content-Type": "application/json",
        },
    },

    // Configure projects for major browsers with multiple run strategies
    projects: [
        // === STABILITY TESTS ===
        {
            name: "stability-chromium",
            testDir: "./tests/stability",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@stability/,
            retries: 3,
            timeout: 45 * 1000,
        },
        {
            name: "stability-firefox",
            testDir: "./tests/stability",
            use: { ...devices["Desktop Firefox"] },
            grep: /@stability/,
            retries: 3,
            timeout: 45 * 1000,
        },
        {
            name: "stability-webkit",
            testDir: "./tests/stability",
            use: { ...devices["Desktop Safari"] },
            grep: /@stability/,
            retries: 3,
            timeout: 45 * 1000,
        },

        // === REGRESSION TESTS ===
        {
            name: "regression-chromium",
            testDir: "./tests/regression",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@regression/,
            retries: 2,
            timeout: 30 * 1000,
        },
        {
            name: "regression-firefox",
            testDir: "./tests/regression",
            use: { ...devices["Desktop Firefox"] },
            grep: /@regression/,
            retries: 2,
            timeout: 30 * 1000,
        },

        // === LOAD TESTS ===
        {
            name: "load-test",
            testDir: "./tests/load",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@load/,
            workers: 8, // Increased workers for load testing
            retries: 1,
            timeout: 60 * 1000,
        },

        // === SMOKE TESTS ===
        {
            name: "smoke-chromium",
            testDir: "./tests/smoke",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@smoke/,
            retries: 1,
            timeout: 15 * 1000,
        },

        // === CRITICAL PATH TESTS ===
        {
            name: "critical-chromium",
            testDir: "./tests/critical",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@critical/,
            retries: 3,
            timeout: 60 * 1000,
        },

        // === API INTEGRATION TESTS ===
        {
            name: "api-integration",
            testDir: "./tests/api",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@api/,
            retries: 2,
            timeout: 45 * 1000,
        },

        // === UI TESTS ===
        {
            name: "ui-chromium",
            testDir: "./tests/ui",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@ui/,
            retries: 2,
            timeout: 30 * 1000,
        },
        {
            name: "ui-firefox",
            testDir: "./tests/ui",
            use: { ...devices["Desktop Firefox"] },
            grep: /@ui/,
            retries: 2,
            timeout: 30 * 1000,
        },

        // === MOBILE TESTS ===
        {
            name: "mobile-chrome",
            testDir: "./tests/mobile",
            use: { ...devices["Pixel 5"] },
            grep: /@mobile/,
            retries: 2,
            timeout: 45 * 1000,
        },
        {
            name: "mobile-safari",
            testDir: "./tests/mobile",
            use: { ...devices["iPhone 12"] },
            grep: /@mobile/,
            retries: 2,
            timeout: 45 * 1000,
        },

        // === EDGE CASE TESTS ===
        {
            name: "edge-cases",
            testDir: "./tests/edge-cases",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@edge/,
            retries: 3,
            timeout: 60 * 1000,
        },

        // === PERFORMANCE TESTS ===
        {
            name: "performance",
            testDir: "./tests/performance",
            use: {
                ...devices["Desktop Chrome"],
                channel: "chrome",
            },
            grep: /@performance/,
            retries: 1,
            timeout: 90 * 1000,
        },
    ],

    // Web Server for local development - disabled for external service management
    // webServer: process.env.CI ? undefined : [
    //   {
    //     command: 'cd ../../src/client-app-demo && cargo run --bin client-app-demo',
    //     port: 8090,
    //     timeout: 120 * 1000,
    //     reuseExistingServer: !process.env.CI,
    //     stdout: 'pipe',
    //     stderr: 'pipe',
    //   },
    //   {
    //     command: 'cd ../../src/services/federation-simple && cargo run',
    //     port: 8801,
    //     timeout: 120 * 1000,
    //     reuseExistingServer: !process.env.CI,
    //     stdout: 'pipe',
    //     stderr: 'pipe',
    //   }
    // ],

    // Test metadata for custom reporting
    metadata: {
        testSuite: "AI-CORE E2E Tests",
        version: "1.0.0",
        environment: process.env.NODE_ENV || "test",
        timestamp: new Date().toISOString(),
        testStrategy: "multiple-runs",
        stabilityRuns: 5,
        loadTestRuns: 3,
        regressionRuns: 10,
        coverage: {
            smoke: true,
            regression: true,
            load: true,
            stability: true,
            critical: true,
            api: true,
            ui: true,
            mobile: true,
            performance: true,
            edgeCases: true,
        },
    },
});
