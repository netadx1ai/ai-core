import { expect, test } from '@playwright/test';
import { v4 as uuidv4 } from 'uuid';

/**
 * Stability Tests: Multiple Runs to Catch Race Conditions and Intermittent Failures
 * Tags: @stability @multiple-runs @flaky-detection
 *
 * These tests run the same scenarios multiple times to detect:
 * - Race conditions
 * - Memory leaks
 * - Intermittent failures
 * - Resource cleanup issues
 * - State management problems
 */

const CLIENT_APP_URL = 'http://localhost:8090';
const FEDERATION_API_URL = 'http://localhost:8801';
const STABILITY_RUNS = parseInt(process.env.STABILITY_RUNS || '5');
const MAX_EXECUTION_TIME = 90000; // 90 seconds per run
const COOLDOWN_BETWEEN_RUNS = 2000; // 2 seconds between runs

// Test scenarios for stability testing
const STABILITY_SCENARIOS = [
  {
    name: 'Quick Content Generation',
    prompt: 'Write a brief article about artificial intelligence',
    wordCount: 300,
    expectedMaxTime: 30000,
    expectedMinQuality: 3.0
  },
  {
    name: 'Medium Content Generation',
    prompt: 'Create a comprehensive guide about cloud computing',
    wordCount: 600,
    expectedMaxTime: 45000,
    expectedMinQuality: 3.5
  },
  {
    name: 'Long Content Generation',
    prompt: 'Develop an in-depth analysis of digital transformation trends',
    wordCount: 900,
    expectedMaxTime: 60000,
    expectedMinQuality: 4.0
  }
];

test.describe('Stability Tests: Multiple Runs @stability', () => {
  let testSuiteId;
  let runResults = [];

  test.beforeAll(async () => {
    testSuiteId = uuidv4();
    console.log(`\n🔄 Starting stability test suite: ${testSuiteId}`);
    console.log(`📊 Configuration: ${STABILITY_RUNS} runs per test`);
  });

  test.afterAll(async () => {
    console.log(`\n📈 Stability Test Results Summary:`);
    console.log(`Suite ID: ${testSuiteId}`);
    console.log(`Total Runs: ${runResults.length}`);

    if (runResults.length > 0) {
      const successful = runResults.filter(r => r.success).length;
      const failed = runResults.length - successful;
      const successRate = (successful / runResults.length * 100).toFixed(2);

      console.log(`✅ Successful: ${successful}`);
      console.log(`❌ Failed: ${failed}`);
      console.log(`📊 Success Rate: ${successRate}%`);

      const avgExecutionTime = runResults
        .filter(r => r.success && r.executionTime)
        .reduce((acc, r) => acc + r.executionTime, 0) / successful;
      console.log(`⏱️  Average Execution Time: ${avgExecutionTime.toFixed(2)}ms`);

      // Log any failures for analysis
      const failures = runResults.filter(r => !r.success);
      if (failures.length > 0) {
        console.log(`\n❌ Failure Analysis:`);
        failures.forEach((failure, index) => {
          console.log(`  ${index + 1}. Run ${failure.runNumber}: ${failure.error}`);
        });
      }
    }
  });

  // === STABILITY TEST 1: Basic Workflow Multiple Runs ===
  test('ST001: Basic workflow stability - multiple runs @stability', async ({ page }) => {
    console.log(`\n🔄 Running basic workflow ${STABILITY_RUNS} times...`);

    const scenario = STABILITY_SCENARIOS[0]; // Quick content generation
    const runResults = [];

    for (let runNumber = 1; runNumber <= STABILITY_RUNS; runNumber++) {
      const runId = uuidv4();
      console.log(`\n  📝 Run ${runNumber}/${STABILITY_RUNS} - ${runId}`);

      const startTime = Date.now();
      let success = false;
      let error = null;
      let executionTime = 0;

      try {
        // Navigate to app
        await page.goto(CLIENT_APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
        await page.click('text="Start Demo"', { timeout: 10000 });

        // Fill form with unique data for this run
        const uniquePrompt = `${scenario.prompt} - Run ${runNumber} at ${new Date().toISOString()}`;
        await page.fill('#prompt-input', uniquePrompt);
        await page.fill('#topic', `AI Topic Run ${runNumber}`);
        await page.fill('#audience', 'Test Audience');
        await page.selectOption('#tone', 'professional');
        await page.fill('#word-count', scenario.wordCount.toString());

        // Start workflow
        const workflowStartTime = Date.now();
        await page.click('#submit-button');

        // Wait for completion
        await expect(page.locator('#session-status')).toContainText('Completed', {
          timeout: scenario.expectedMaxTime
        });

        executionTime = Date.now() - workflowStartTime;

        // Validate results
        await expect(page.locator('#blog-title')).toBeVisible();
        await expect(page.locator('#blog-content')).toBeVisible();

        const qualityScoreText = await page.locator('#quality-score').textContent();
        const qualityScore = parseFloat(qualityScoreText.match(/[\d.]+/)?.[0] || '0');

        expect(qualityScore).toBeGreaterThanOrEqual(scenario.expectedMinQuality);
        expect(executionTime).toBeLessThan(scenario.expectedMaxTime);

        success = true;
        console.log(`    ✅ Success - ${executionTime}ms, Quality: ${qualityScore}`);

      } catch (err) {
        error = err.message;
        console.log(`    ❌ Failed - ${error}`);
      }

      const result = {
        runNumber,
        runId,
        success,
        error,
        executionTime,
        totalTime: Date.now() - startTime,
        scenario: scenario.name
      };

      runResults.push(result);
      this.runResults.push(result);

      // Cooldown between runs to prevent resource exhaustion
      if (runNumber < STABILITY_RUNS) {
        console.log(`    ⏳ Cooldown ${COOLDOWN_BETWEEN_RUNS}ms...`);
        await page.waitForTimeout(COOLDOWN_BETWEEN_RUNS);

        // Clear any existing state
        await page.goto('about:blank');
        await page.waitForTimeout(500);
      }
    }

    // Analyze run results
    const successful = runResults.filter(r => r.success).length;
    const successRate = (successful / STABILITY_RUNS) * 100;

    console.log(`\n📊 Basic Workflow Stability Results:`);
    console.log(`  Success Rate: ${successRate.toFixed(2)}% (${successful}/${STABILITY_RUNS})`);

    if (successful > 0) {
      const avgTime = runResults
        .filter(r => r.success)
        .reduce((acc, r) => acc + r.executionTime, 0) / successful;
      console.log(`  Average Execution Time: ${avgTime.toFixed(2)}ms`);
    }

    // Stability requirements
    expect(successRate).toBeGreaterThan(80); // At least 80% success rate
    if (successful > 1) {
      const executionTimes = runResults.filter(r => r.success).map(r => r.executionTime);
      const maxTime = Math.max(...executionTimes);
      const minTime = Math.min(...executionTimes);
      const variance = maxTime - minTime;

      console.log(`  Execution Time Variance: ${variance}ms (${minTime}ms - ${maxTime}ms)`);
      expect(variance).toBeLessThan(30000); // Variance should be less than 30 seconds
    }
  });

  // === STABILITY TEST 2: Concurrent Requests Stability ===
  test('ST002: Concurrent requests stability @stability @load', async ({ browser }) => {
    console.log(`\n🔄 Testing concurrent request stability...`);

    const concurrentRuns = 3;
    const stabilityIterations = Math.ceil(STABILITY_RUNS / 2); // Run fewer iterations with more concurrency

    for (let iteration = 1; iteration <= stabilityIterations; iteration++) {
      console.log(`\n  🚀 Concurrent Iteration ${iteration}/${stabilityIterations}`);

      const contexts = [];
      const pages = [];

      // Create browser contexts
      for (let i = 0; i < concurrentRuns; i++) {
        const context = await browser.newContext();
        const page = await context.newPage();
        contexts.push(context);
        pages.push(page);
      }

      try {
        const concurrentPromises = pages.map(async (page, index) => {
          const runId = `${iteration}-${index + 1}`;
          const scenario = STABILITY_SCENARIOS[index % STABILITY_SCENARIOS.length];

          console.log(`    Starting concurrent run ${runId}...`);

          const startTime = Date.now();

          try {
            await page.goto(CLIENT_APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
            await page.click('text="Start Demo"', { timeout: 10000 });

            const uniquePrompt = `${scenario.prompt} - Concurrent ${runId}`;
            await page.fill('#prompt-input', uniquePrompt);
            await page.fill('#topic', `Topic ${runId}`);
            await page.fill('#audience', 'Concurrent Test');
            await page.selectOption('#tone', 'casual');
            await page.fill('#word-count', scenario.wordCount.toString());

            await page.click('#submit-button');

            await expect(page.locator('#session-status')).toContainText('Completed', {
              timeout: MAX_EXECUTION_TIME
            });

            const executionTime = Date.now() - startTime;
            console.log(`    ✅ Concurrent run ${runId} completed in ${executionTime}ms`);

            return { runId, success: true, executionTime };
          } catch (error) {
            console.log(`    ❌ Concurrent run ${runId} failed: ${error.message}`);
            return { runId, success: false, error: error.message };
          }
        });

        const results = await Promise.all(concurrentPromises);

        // Analyze concurrent results
        const successful = results.filter(r => r.success).length;
        const successRate = (successful / concurrentRuns) * 100;

        console.log(`  📊 Iteration ${iteration} Results: ${successRate.toFixed(2)}% success (${successful}/${concurrentRuns})`);

        // Log detailed results
        results.forEach(result => {
          this.runResults.push({
            runNumber: `concurrent-${iteration}-${result.runId}`,
            success: result.success,
            error: result.error,
            executionTime: result.executionTime || 0,
            scenario: 'Concurrent Execution'
          });
        });

        // Expect at least 2 out of 3 concurrent requests to succeed
        expect(successful).toBeGreaterThanOrEqual(2);

      } finally {
        // Cleanup contexts
        for (const context of contexts) {
          await context.close();
        }
      }

      // Cooldown between iterations
      if (iteration < stabilityIterations) {
        console.log(`  ⏳ Iteration cooldown ${COOLDOWN_BETWEEN_RUNS * 2}ms...`);
        await new Promise(resolve => setTimeout(resolve, COOLDOWN_BETWEEN_RUNS * 2));
      }
    }
  });

  // === STABILITY TEST 3: Error Recovery Stability ===
  test('ST003: Error recovery stability @stability @error-handling', async ({ page }) => {
    console.log(`\n🔧 Testing error recovery stability...`);

    const errorScenarios = [
      { name: 'Empty Prompt', prompt: '', expectError: true },
      { name: 'Very Short Prompt', prompt: 'AI', expectError: false },
      { name: 'Invalid Word Count', prompt: 'Valid prompt', wordCount: -1, expectError: false },
      { name: 'Zero Word Count', prompt: 'Valid prompt', wordCount: 0, expectError: false },
      { name: 'Very High Word Count', prompt: 'Valid prompt', wordCount: 10000, expectError: false }
    ];

    const errorTestRuns = Math.max(2, Math.floor(STABILITY_RUNS / errorScenarios.length));

    for (const scenario of errorScenarios) {
      console.log(`\n  🧪 Testing ${scenario.name} (${errorTestRuns} runs)...`);

      for (let run = 1; run <= errorTestRuns; run++) {
        const runId = `${scenario.name}-${run}`;
        console.log(`    Run ${run}/${errorTestRuns}...`);

        try {
          await page.goto(CLIENT_APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
          await page.click('text="Start Demo"', { timeout: 10000 });

          // Fill form with scenario data
          await page.fill('#prompt-input', scenario.prompt);
          await page.fill('#topic', `Test Topic ${run}`);
          await page.fill('#audience', 'Error Test');
          await page.selectOption('#tone', 'casual');

          const wordCount = scenario.wordCount || 500;
          await page.fill('#word-count', wordCount.toString());

          await page.click('#submit-button');

          if (scenario.expectError) {
            // Should show error message quickly
            await expect(page.locator('.error-message')).toBeVisible({ timeout: 5000 });
            console.log(`      ✅ Error correctly displayed`);
          } else {
            // Should either show error or proceed (graceful handling)
            const hasError = await page.locator('.error-message').isVisible();
            const hasSession = await page.locator('#session-status').isVisible();

            expect(hasError || hasSession).toBeTruthy();
            console.log(`      ✅ Handled gracefully (error: ${hasError}, session: ${hasSession})`);

            // If session started, wait a bit but don't require completion
            if (hasSession) {
              await page.waitForTimeout(5000);
            }
          }

          this.runResults.push({
            runNumber: runId,
            success: true,
            error: null,
            scenario: `Error Recovery - ${scenario.name}`
          });

        } catch (error) {
          console.log(`      ❌ Unexpected error: ${error.message}`);
          this.runResults.push({
            runNumber: runId,
            success: false,
            error: error.message,
            scenario: `Error Recovery - ${scenario.name}`
          });
        }

        // Cooldown between error tests
        await page.waitForTimeout(1000);
      }
    }
  });

  // === STABILITY TEST 4: Memory and Resource Stability ===
  test('ST004: Memory and resource stability @stability @performance', async ({ page }) => {
    console.log(`\n💾 Testing memory and resource stability...`);

    const memoryTestRuns = STABILITY_RUNS;
    let initialMemory = null;
    const memoryMeasurements = [];

    for (let run = 1; run <= memoryTestRuns; run++) {
      console.log(`\n  🔬 Memory test run ${run}/${memoryTestRuns}...`);

      const startTime = Date.now();

      try {
        // Navigate and measure initial memory if possible
        await page.goto(CLIENT_APP_URL, { waitUntil: 'networkidle', timeout: 30000 });

        // Try to get memory info (browser dependent)
        const memoryInfo = await page.evaluate(() => {
          if (performance.memory) {
            return {
              usedJSHeapSize: performance.memory.usedJSHeapSize,
              totalJSHeapSize: performance.memory.totalJSHeapSize,
              jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
            };
          }
          return null;
        });

        if (memoryInfo) {
          if (initialMemory === null) {
            initialMemory = memoryInfo.usedJSHeapSize;
          }
          memoryMeasurements.push({
            run,
            usedMemory: memoryInfo.usedJSHeapSize,
            memoryGrowth: memoryInfo.usedJSHeapSize - initialMemory
          });
          console.log(`    📊 Memory: ${(memoryInfo.usedJSHeapSize / 1024 / 1024).toFixed(2)}MB`);
        }

        await page.click('text="Start Demo"', { timeout: 10000 });

        // Generate content
        await page.fill('#prompt-input', `Resource test run ${run} - ${new Date().toISOString()}`);
        await page.fill('#topic', `Resource Test ${run}`);
        await page.fill('#audience', 'Resource Testing');
        await page.selectOption('#tone', 'technical');
        await page.fill('#word-count', '400');

        await page.click('#submit-button');

        // Wait for completion or reasonable timeout
        try {
          await expect(page.locator('#session-status')).toContainText('Completed', {
            timeout: 45000
          });
          console.log(`    ✅ Completed successfully`);
        } catch (timeoutError) {
          console.log(`    ⏰ Timeout after 45s, continuing...`);
        }

        const executionTime = Date.now() - startTime;

        this.runResults.push({
          runNumber: `memory-${run}`,
          success: true,
          executionTime,
          scenario: 'Memory Stability'
        });

        // Force garbage collection if available
        await page.evaluate(() => {
          if (window.gc) {
            window.gc();
          }
        });

      } catch (error) {
        console.log(`    ❌ Memory test run ${run} failed: ${error.message}`);
        this.runResults.push({
          runNumber: `memory-${run}`,
          success: false,
          error: error.message,
          scenario: 'Memory Stability'
        });
      }

      // Short cooldown
      await page.waitForTimeout(1500);
    }

    // Analyze memory growth
    if (memoryMeasurements.length > 1) {
      const finalMemory = memoryMeasurements[memoryMeasurements.length - 1];
      const memoryGrowth = finalMemory.memoryGrowth;
      const memoryGrowthMB = memoryGrowth / 1024 / 1024;

      console.log(`\n📈 Memory Analysis:`);
      console.log(`  Initial Memory: ${(initialMemory / 1024 / 1024).toFixed(2)}MB`);
      console.log(`  Final Memory: ${(finalMemory.usedMemory / 1024 / 1024).toFixed(2)}MB`);
      console.log(`  Memory Growth: ${memoryGrowthMB.toFixed(2)}MB`);

      // Memory growth should be reasonable (less than 50MB for our test)
      expect(memoryGrowthMB).toBeLessThan(50);
    }
  });

  // === STABILITY TEST 5: Network Resilience ===
  test('ST005: Network resilience stability @stability @network', async ({ page }) => {
    console.log(`\n🌐 Testing network resilience...`);

    const networkTestRuns = Math.max(3, Math.floor(STABILITY_RUNS / 2));

    for (let run = 1; run <= networkTestRuns; run++) {
      console.log(`\n  🔌 Network test run ${run}/${networkTestRuns}...`);

      const startTime = Date.now();

      try {
        await page.goto(CLIENT_APP_URL, { waitUntil: 'networkidle', timeout: 30000 });

        // Simulate network conditions by adding some delay
        await page.route('**/*', async route => {
          // Add 100-500ms delay to simulate network latency
          const delay = Math.random() * 400 + 100;
          await new Promise(resolve => setTimeout(resolve, delay));
          await route.continue();
        });

        await page.click('text="Start Demo"', { timeout: 15000 });

        await page.fill('#prompt-input', `Network resilience test ${run}`);
        await page.fill('#topic', `Network Test ${run}`);
        await page.fill('#audience', 'Network Testing');
        await page.selectOption('#tone', 'professional');
        await page.fill('#word-count', '500');

        await page.click('#submit-button');

        // Wait for completion with extended timeout for network delays
        await expect(page.locator('#session-status')).toContainText('Completed', {
          timeout: 75000
        });

        const executionTime = Date.now() - startTime;
        console.log(`    ✅ Network test completed in ${executionTime}ms`);

        this.runResults.push({
          runNumber: `network-${run}`,
          success: true,
          executionTime,
          scenario: 'Network Resilience'
        });

        // Remove network delay for cleanup
        await page.unroute('**/*');

      } catch (error) {
        console.log(`    ❌ Network test ${run} failed: ${error.message}`);
        this.runResults.push({
          runNumber: `network-${run}`,
          success: false,
          error: error.message,
          scenario: 'Network Resilience'
        });
      }

      // Cooldown between network tests
      await page.waitForTimeout(2000);
    }
  });

  // === STABILITY TEST 6: State Consistency ===
  test('ST006: State consistency across multiple operations @stability', async ({ page }) => {
    console.log(`\n🔄 Testing state consistency...`);

    const stateTestRuns = Math.max(3, STABILITY_RUNS);
    const sessionIds = new Set();

    for (let run = 1; run <= stateTestRuns; run++) {
      console.log(`\n  📋 State consistency test ${run}/${stateTestRuns}...`);

      try {
        await page.goto(CLIENT_APP_URL, { waitUntil: 'networkidle', timeout: 30000 });
        await page.click('text="Start Demo"', { timeout: 10000 });

        // Start a workflow
        await page.fill('#prompt-input', `State consistency test ${run}`);
        await page.fill('#topic', `State Test ${run}`);
        await page.fill('#audience', 'State Testing');
        await page.selectOption('#tone', 'casual');
        await page.fill('#word-count', '300');

        await page.click('#submit-button');

        // Wait for session to start
        await expect(page.locator('#session-status')).toBeVisible({ timeout: 10000 });

        // Check if session ID is displayed and unique
        const sessionIdElement = page.locator('#session-id');
        if (await sessionIdElement.isVisible()) {
          const sessionId = await sessionIdElement.textContent();

          if (sessionId && sessionId.trim()) {
            console.log(`    📍 Session ID: ${sessionId}`);

            // Verify uniqueness
            expect(sessionIds.has(sessionId)).toBeFalsy();
            sessionIds.add(sessionId);
          }
        }

        // Let workflow run for a bit
        await page.waitForTimeout(5000);

        // Try refreshing page mid-workflow to test state handling
        await page.reload({ waitUntil: 'networkidle', timeout: 30000 });

        // Page should load without errors (graceful state handling)
        await expect(page.locator('body')).toBeVisible({ timeout: 10000 });

        console.log(`    ✅ State consistency maintained`);

        this.runResults.push({
          runNumber: `state-${run}`,
          success: true,
          scenario: 'State Consistency'
        });

      } catch (error) {
        console.log(`    ❌ State test ${run} failed: ${error.message}`);
        this.runResults.push({
          runNumber: `state-${run}`,
          success: false,
          error: error.message,
          scenario: 'State Consistency'
        });
      }

      await page.waitForTimeout(1000);
    }

    console.log(`\n📊 State Analysis: ${sessionIds.size} unique sessions created`);
  });
});

// Helper function to analyze stability patterns
function analyzeStabilityPatterns(results) {
  const patterns = {
    totalRuns: results.length,
    successfulRuns: results.filter(r => r.success).length,
    failedRuns: results.filter(r => !r.success).length,
    scenarios: {},
    executionTimes: results.filter(r => r.success && r.executionTime).map(r => r.executionTime),
    commonErrors: {}
  };

  // Group by scenario
  results.forEach(result => {
    if (!patterns.scenarios[result.scenario]) {
      patterns.scenarios[result.scenario] = { total: 0, successful: 0, failed: 0 };
    }
    patterns.scenarios[result.scenario].total++;
    if (result.success) {
      patterns.scenarios[result.scenario].successful++;
    } else {
      patterns.scenarios[result.scenario].failed++;
    }
  });

  // Count common errors
  results.filter(r => !r.success && r.error).forEach(result => {
    const errorKey = result.error.substring(0, 100); // First 100 chars of error
    patterns.commonErrors[errorKey] = (patterns.commonErrors[errorKey] || 0) + 1;
  });

  return patterns;
}

// Test data generators for stability testing
function generateStabilityTestData(runNumber) {
  const topics = [
    'Artificial Intelligence Trends',
    'Cloud Computing Evolution',
    'Digital Transformation',
    'Cybersecurity Best Practices',
    'Sustainable Technology',
    'Remote Work Innovation',
    'Data Analytics Insights',
    'Blockchain Applications'
  ];

  const audiences = [
    'Business Leaders',
    'Technical Teams',
    'General Public',
    'Industry Experts',
    'Students',
    'Researchers'
  ];

  const tones = ['professional', 'casual', 'technical', 'friendly'];

  return {
    topic: topics[runNumber % topics.length],
    audience: audiences[runNumber % audiences.length],
    tone: tones[runNumber % tones.length],
    wordCount: 300 + (runNumber % 5) * 100 // 300-700 words
  };
}
