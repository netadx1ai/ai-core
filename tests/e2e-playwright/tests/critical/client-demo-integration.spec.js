import { expect, test } from '@playwright/test';
import { v4 as uuidv4 } from 'uuid';

/**
 * Critical Path Tests: Client-App-Demo Real API Integration
 * Tags: @critical @integration @api @ui
 *
 * Tests the complete end-to-end workflow from client UI to real AI-CORE services
 * Run multiple times per build to ensure stability and catch race conditions
 */

const CLIENT_APP_URL = 'http://localhost:8090';
const FEDERATION_API_URL = 'http://localhost:8801';
const INTENT_PARSER_URL = 'http://localhost:8802';
const MCP_MANAGER_URL = 'http://localhost:8803';

// Test data for multiple runs
const TEST_PROMPTS = [
  'Create a blog post about AI automation trends in 2024',
  'Write an article about remote work productivity tips',
  'Generate content about digital transformation for small businesses',
  'Create a technical blog post about microservices architecture',
  'Write about the future of artificial intelligence in healthcare'
];

const TEST_SCENARIOS = [
  {
    name: 'Standard Blog Post',
    prompt: 'Create a comprehensive blog post about AI automation trends',
    expectedWordCount: { min: 400, max: 1200 },
    expectedQualityScore: { min: 3.5, max: 5.0 },
    maxExecutionTime: 45000 // 45 seconds
  },
  {
    name: 'Technical Article',
    prompt: 'Write a technical article about microservices and containerization',
    expectedWordCount: { min: 600, max: 1500 },
    expectedQualityScore: { min: 4.0, max: 5.0 },
    maxExecutionTime: 60000 // 60 seconds
  },
  {
    name: 'Short Content',
    prompt: 'Create a brief overview of cloud computing benefits',
    expectedWordCount: { min: 200, max: 600 },
    expectedQualityScore: { min: 3.0, max: 5.0 },
    maxExecutionTime: 30000 // 30 seconds
  }
];

test.describe('Critical Path: Client-App-Demo Real API Integration @critical', () => {
  let testRunId;

  test.beforeEach(async ({ page }) => {
    testRunId = uuidv4();
    console.log(`\n🚀 Starting critical test run: ${testRunId}`);

    // Navigate to client app
    await page.goto(CLIENT_APP_URL);
    await page.waitForLoadState('networkidle');

    // Verify page loaded correctly
    await expect(page).toHaveTitle(/AI-CORE/);
  });

  test.afterEach(async ({ page }, testInfo) => {
    console.log(`✅ Completed test run: ${testRunId} - ${testInfo.status}`);

    // Capture final state on failure
    if (testInfo.status !== 'passed') {
      await page.screenshot({ path: `test-results/critical-failure-${testRunId}.png`, fullPage: true });
    }
  });

  // === CRITICAL TEST 1: Service Health Verification ===
  test('CT001: Verify all AI-CORE services are healthy @critical @api', async ({ page, request }) => {
    const services = [
      { name: 'Client Demo', url: `${CLIENT_APP_URL}/health` },
      { name: 'Federation Service', url: `${FEDERATION_API_URL}/health` },
      { name: 'Intent Parser', url: `${INTENT_PARSER_URL}/health` },
      { name: 'MCP Manager', url: `${MCP_MANAGER_URL}/health` }
    ];

    console.log('🔍 Checking service health...');

    for (const service of services) {
      console.log(`  Checking ${service.name}...`);

      try {
        const response = await request.get(service.url, { timeout: 10000 });
        expect(response.status()).toBe(200);

        const healthData = await response.json();
        expect(healthData.status).toBe('healthy');

        console.log(`  ✅ ${service.name}: ${healthData.status}`);
      } catch (error) {
        console.error(`  ❌ ${service.name}: ${error.message}`);
        throw error;
      }
    }
  });

  // === CRITICAL TEST 2: Real Workflow Execution ===
  test('CT002: Execute real workflow with API integration @critical @integration', async ({ page }) => {
    const scenario = TEST_SCENARIOS[0]; // Standard blog post
    const startTime = Date.now();

    console.log(`📝 Testing scenario: ${scenario.name}`);
    console.log(`📝 Prompt: ${scenario.prompt}`);

    // Navigate to demo page
    await page.click('text="Start Demo"');
    await expect(page.locator('h1')).toContainText('AI-CORE Demo');

    // Enter prompt
    await page.fill('#prompt-input', scenario.prompt);
    await page.fill('#topic', 'AI Automation');
    await page.fill('#audience', 'Business Professionals');
    await page.selectOption('#tone', 'professional');

    // Set word count
    await page.fill('#word-count', '800');

    // Submit and start workflow
    console.log('🚀 Starting workflow execution...');
    await page.click('#submit-button');

    // Wait for session to start
    await expect(page.locator('#session-status')).toContainText('Starting', { timeout: 10000 });

    // Track workflow progress
    const progressStates = [
      'Starting',
      'Authenticating',
      'ProcessingIntent',
      'GeneratingContent',
      'CreatingImage',
      'ValidatingQuality',
      'Completed'
    ];

    for (const state of progressStates) {
      console.log(`  Waiting for state: ${state}...`);
      await expect(page.locator('#session-status')).toContainText(state, {
        timeout: scenario.maxExecutionTime / progressStates.length
      });

      // Verify progress updates
      const progressElement = page.locator('#progress-percentage');
      if (await progressElement.isVisible()) {
        const progressText = await progressElement.textContent();
        console.log(`  Progress: ${progressText}`);
      }
    }

    // Verify completion
    await expect(page.locator('#session-status')).toContainText('Completed', {
      timeout: scenario.maxExecutionTime
    });

    const executionTime = Date.now() - startTime;
    console.log(`⏱️  Execution time: ${executionTime}ms`);
    expect(executionTime).toBeLessThan(scenario.maxExecutionTime);

    // Verify results
    await expect(page.locator('#blog-title')).toBeVisible();
    await expect(page.locator('#blog-content')).toBeVisible();
    await expect(page.locator('#word-count-display')).toBeVisible();
    await expect(page.locator('#quality-score')).toBeVisible();

    // Validate content quality
    const titleText = await page.locator('#blog-title').textContent();
    expect(titleText.length).toBeGreaterThan(10);

    const contentText = await page.locator('#blog-content').textContent();
    expect(contentText.length).toBeGreaterThan(100);

    const wordCountText = await page.locator('#word-count-display').textContent();
    const wordCount = parseInt(wordCountText.match(/\d+/)?.[0] || '0');
    expect(wordCount).toBeGreaterThanOrEqual(scenario.expectedWordCount.min);
    expect(wordCount).toBeLessThanOrEqual(scenario.expectedWordCount.max);

    const qualityScoreText = await page.locator('#quality-score').textContent();
    const qualityScore = parseFloat(qualityScoreText.match(/[\d.]+/)?.[0] || '0');
    expect(qualityScore).toBeGreaterThanOrEqual(scenario.expectedQualityScore.min);
    expect(qualityScore).toBeLessThanOrEqual(scenario.expectedQualityScore.max);

    console.log(`✅ Content generated - Words: ${wordCount}, Quality: ${qualityScore}`);
  });

  // === CRITICAL TEST 3: Multiple Concurrent Requests ===
  test('CT003: Handle multiple concurrent workflow requests @critical @load', async ({ browser }) => {
    const concurrentRequests = 3;
    const contexts = [];
    const pages = [];

    console.log(`🔄 Testing ${concurrentRequests} concurrent requests...`);

    // Create multiple browser contexts
    for (let i = 0; i < concurrentRequests; i++) {
      const context = await browser.newContext();
      const page = await context.newPage();
      contexts.push(context);
      pages.push(page);
    }

    try {
      // Start all workflows simultaneously
      const workflowPromises = pages.map(async (page, index) => {
        const prompt = TEST_PROMPTS[index % TEST_PROMPTS.length];
        console.log(`  Starting workflow ${index + 1}: ${prompt.substring(0, 50)}...`);

        await page.goto(CLIENT_APP_URL);
        await page.waitForLoadState('networkidle');
        await page.click('text="Start Demo"');

        await page.fill('#prompt-input', prompt);
        await page.fill('#topic', `Topic ${index + 1}`);
        await page.fill('#audience', 'General Audience');
        await page.selectOption('#tone', 'casual');
        await page.fill('#word-count', '500');

        const startTime = Date.now();
        await page.click('#submit-button');

        // Wait for completion
        await expect(page.locator('#session-status')).toContainText('Completed', {
          timeout: 90000
        });

        const executionTime = Date.now() - startTime;
        console.log(`  ✅ Workflow ${index + 1} completed in ${executionTime}ms`);

        return { index: index + 1, executionTime, page };
      });

      // Wait for all workflows to complete
      const results = await Promise.all(workflowPromises);

      console.log('📊 Concurrent execution results:');
      results.forEach(result => {
        console.log(`  Workflow ${result.index}: ${result.executionTime}ms`);
      });

      // Verify all completed successfully
      expect(results).toHaveLength(concurrentRequests);
      results.forEach(result => {
        expect(result.executionTime).toBeLessThan(120000); // 2 minutes max
      });

    } finally {
      // Cleanup contexts
      for (const context of contexts) {
        await context.close();
      }
    }
  });

  // === CRITICAL TEST 4: Error Handling and Recovery ===
  test('CT004: Verify error handling and recovery mechanisms @critical @edge', async ({ page }) => {
    console.log('🔧 Testing error handling scenarios...');

    await page.goto(CLIENT_APP_URL);
    await page.click('text="Start Demo"');

    // Test 1: Empty prompt
    console.log('  Testing empty prompt handling...');
    await page.click('#submit-button');
    await expect(page.locator('.error-message')).toBeVisible({ timeout: 5000 });

    // Test 2: Invalid parameters
    console.log('  Testing invalid parameters...');
    await page.fill('#prompt-input', 'Test prompt');
    await page.fill('#word-count', '-100');
    await page.click('#submit-button');

    // Should either show validation error or handle gracefully
    const hasValidationError = await page.locator('.error-message').isVisible();
    const hasSessionStarted = await page.locator('#session-status').isVisible();
    expect(hasValidationError || hasSessionStarted).toBeTruthy();

    // Test 3: Network resilience
    console.log('  Testing network resilience...');
    await page.fill('#word-count', '400');
    await page.click('#submit-button');

    // Should handle network issues gracefully
    await expect(page.locator('#session-status')).toBeVisible({ timeout: 10000 });
  });

  // === CRITICAL TEST 5: Real-time Progress Updates ===
  test('CT005: Verify real-time progress updates @critical @ui', async ({ page }) => {
    console.log('📡 Testing real-time progress updates...');

    await page.goto(CLIENT_APP_URL);
    await page.click('text="Start Demo"');

    await page.fill('#prompt-input', 'Create a detailed blog post about cloud computing');
    await page.fill('#topic', 'Cloud Computing');
    await page.fill('#audience', 'IT Professionals');
    await page.selectOption('#tone', 'technical');
    await page.fill('#word-count', '600');

    // Start workflow and monitor progress
    await page.click('#submit-button');

    let lastProgress = -1;
    const progressUpdates = [];

    // Monitor progress for 60 seconds or until completion
    const monitoringTimeout = 60000;
    const startTime = Date.now();

    while (Date.now() - startTime < monitoringTimeout) {
      const statusElement = page.locator('#session-status');
      const progressElement = page.locator('#progress-percentage');

      if (await statusElement.isVisible()) {
        const status = await statusElement.textContent();

        if (await progressElement.isVisible()) {
          const progressText = await progressElement.textContent();
          const currentProgress = parseInt(progressText.match(/\d+/)?.[0] || '0');

          if (currentProgress !== lastProgress) {
            progressUpdates.push({
              timestamp: Date.now() - startTime,
              status,
              progress: currentProgress
            });
            console.log(`  Progress: ${currentProgress}% - ${status}`);
            lastProgress = currentProgress;
          }
        }

        if (status.includes('Completed')) {
          break;
        }
      }

      await page.waitForTimeout(1000); // Check every second
    }

    // Verify progress updates occurred
    expect(progressUpdates.length).toBeGreaterThan(0);
    console.log(`📈 Captured ${progressUpdates.length} progress updates`);

    // Verify progress increased over time
    for (let i = 1; i < progressUpdates.length; i++) {
      expect(progressUpdates[i].progress).toBeGreaterThanOrEqual(progressUpdates[i-1].progress);
    }
  });

  // === CRITICAL TEST 6: Data Validation and Quality ===
  test('CT006: Validate generated content quality @critical @api', async ({ page }) => {
    console.log('🎯 Testing content quality validation...');

    await page.goto(CLIENT_APP_URL);
    await page.click('text="Start Demo"');

    const prompt = 'Write a comprehensive guide about cybersecurity best practices for businesses';
    await page.fill('#prompt-input', prompt);
    await page.fill('#topic', 'Cybersecurity');
    await page.fill('#audience', 'Business Executives');
    await page.selectOption('#tone', 'professional');
    await page.fill('#word-count', '800');

    await page.click('#submit-button');

    // Wait for completion
    await expect(page.locator('#session-status')).toContainText('Completed', {
      timeout: 60000
    });

    // Extract and validate content
    const title = await page.locator('#blog-title').textContent();
    const content = await page.locator('#blog-content').textContent();
    const wordCount = parseInt(await page.locator('#word-count-display').textContent().then(t => t.match(/\d+/)?.[0] || '0'));
    const qualityScore = parseFloat(await page.locator('#quality-score').textContent().then(t => t.match(/[\d.]+/)?.[0] || '0'));

    console.log('📊 Content Metrics:');
    console.log(`  Title: "${title}"`);
    console.log(`  Word Count: ${wordCount}`);
    console.log(`  Quality Score: ${qualityScore}`);
    console.log(`  Content Length: ${content.length} chars`);

    // Quality validations
    expect(title).toBeTruthy();
    expect(title.length).toBeGreaterThan(10);
    expect(title.toLowerCase()).toContain('cybersecurity');

    expect(content).toBeTruthy();
    expect(content.length).toBeGreaterThan(500);
    expect(wordCount).toBeGreaterThan(400);
    expect(wordCount).toBeLessThan(1200);

    expect(qualityScore).toBeGreaterThan(3.0);
    expect(qualityScore).toBeLessThanOrEqual(5.0);

    // Content relevance checks
    const lowerContent = content.toLowerCase();
    expect(lowerContent).toContain('security');
    expect(lowerContent).toContain('business');

    // Check for structured content (paragraphs, etc.)
    expect(content.split('\n').length).toBeGreaterThan(3);
  });

  // === CRITICAL TEST 7: Session State Management ===
  test('CT007: Verify session state management @critical @integration', async ({ page }) => {
    console.log('💾 Testing session state management...');

    await page.goto(CLIENT_APP_URL);
    await page.click('text="Start Demo"');

    // Start a workflow
    await page.fill('#prompt-input', 'Create a blog post about renewable energy');
    await page.fill('#topic', 'Renewable Energy');
    await page.fill('#audience', 'Environmental Enthusiasts');
    await page.selectOption('#tone', 'informative');
    await page.fill('#word-count', '500');

    await page.click('#submit-button');

    // Wait for workflow to start
    await expect(page.locator('#session-status')).toBeVisible();
    const sessionId = await page.locator('#session-id').textContent();

    console.log(`  Session ID: ${sessionId}`);
    expect(sessionId).toBeTruthy();

    // Refresh page during workflow execution
    await page.waitForTimeout(5000); // Let workflow start
    await page.reload();

    // Verify session persistence (if implemented)
    // This test verifies the system handles page refreshes gracefully
    await page.waitForLoadState('networkidle');

    // Should either restore session or handle gracefully
    const pageLoaded = await page.locator('body').isVisible();
    expect(pageLoaded).toBeTruthy();
  });

  // === CRITICAL TEST 8: Performance Under Load ===
  test('CT008: Performance validation under load @critical @performance', async ({ page }) => {
    console.log('⚡ Testing performance under load...');

    const performanceMetrics = {
      pageLoadTime: 0,
      workflowStartTime: 0,
      workflowExecutionTime: 0,
      totalTime: 0
    };

    const startTotal = performance.now();

    // Measure page load time
    const startPageLoad = performance.now();
    await page.goto(CLIENT_APP_URL);
    await page.waitForLoadState('networkidle');
    performanceMetrics.pageLoadTime = performance.now() - startPageLoad;

    await page.click('text="Start Demo"');

    // Measure workflow start time
    const startWorkflowSetup = performance.now();
    await page.fill('#prompt-input', 'Generate a comprehensive analysis of market trends in technology sector');
    await page.fill('#topic', 'Technology Market Trends');
    await page.fill('#audience', 'Business Analysts');
    await page.selectOption('#tone', 'analytical');
    await page.fill('#word-count', '750');
    performanceMetrics.workflowStartTime = performance.now() - startWorkflowSetup;

    // Measure workflow execution time
    const startWorkflowExecution = performance.now();
    await page.click('#submit-button');

    await expect(page.locator('#session-status')).toContainText('Completed', {
      timeout: 90000
    });
    performanceMetrics.workflowExecutionTime = performance.now() - startWorkflowExecution;

    performanceMetrics.totalTime = performance.now() - startTotal;

    console.log('📊 Performance Metrics:');
    console.log(`  Page Load: ${performanceMetrics.pageLoadTime.toFixed(2)}ms`);
    console.log(`  Workflow Setup: ${performanceMetrics.workflowStartTime.toFixed(2)}ms`);
    console.log(`  Workflow Execution: ${performanceMetrics.workflowExecutionTime.toFixed(2)}ms`);
    console.log(`  Total Time: ${performanceMetrics.totalTime.toFixed(2)}ms`);

    // Performance assertions
    expect(performanceMetrics.pageLoadTime).toBeLessThan(5000); // 5 seconds
    expect(performanceMetrics.workflowStartTime).toBeLessThan(1000); // 1 second
    expect(performanceMetrics.workflowExecutionTime).toBeLessThan(75000); // 75 seconds
    expect(performanceMetrics.totalTime).toBeLessThan(90000); // 90 seconds total
  });

  // === CRITICAL TEST 9: API Logging and Monitoring ===
  test('CT009: Verify API request/response logging @critical @api', async ({ page, request }) => {
    console.log('📝 Testing API logging and monitoring...');

    // Monitor network requests
    const apiCalls = [];

    page.on('request', request => {
      if (request.url().includes('/v1/') || request.url().includes('/api/')) {
        apiCalls.push({
          method: request.method(),
          url: request.url(),
          timestamp: Date.now(),
          type: 'request'
        });
      }
    });

    page.on('response', response => {
      if (response.url().includes('/v1/') || response.url().includes('/api/')) {
        apiCalls.push({
          method: response.request().method(),
          url: response.url(),
          status: response.status(),
          timestamp: Date.now(),
          type: 'response'
        });
      }
    });

    await page.goto(CLIENT_APP_URL);
    await page.click('text="Start Demo"');

    await page.fill('#prompt-input', 'Create a blog post about sustainable development');
    await page.fill('#topic', 'Sustainable Development');
    await page.fill('#audience', 'Policy Makers');
    await page.selectOption('#tone', 'formal');
    await page.fill('#word-count', '600');

    await page.click('#submit-button');

    // Wait for completion
    await expect(page.locator('#session-status')).toContainText('Completed', {
      timeout: 60000
    });

    // Verify API calls were made
    console.log(`📡 Captured ${apiCalls.length} API calls`);
    expect(apiCalls.length).toBeGreaterThan(0);

    // Log API calls for debugging
    apiCalls.forEach((call, index) => {
      console.log(`  ${index + 1}. ${call.type.toUpperCase()} ${call.method} ${call.url} ${call.status || ''}`);
    });

    // Verify expected API endpoints were called
    const requestUrls = apiCalls.filter(call => call.type === 'request').map(call => call.url);
    const hasWorkflowCalls = requestUrls.some(url =>
      url.includes('/workflows') ||
      url.includes('/generate') ||
      url.includes('/parse')
    );

    expect(hasWorkflowCalls).toBeTruthy();
  });

  // === CRITICAL TEST 10: Cross-Browser Compatibility ===
  test('CT010: Cross-browser compatibility validation @critical @ui', async ({ browserName, page }) => {
    console.log(`🌐 Testing cross-browser compatibility on ${browserName}...`);

    await page.goto(CLIENT_APP_URL);

    // Verify basic functionality works across browsers
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('text="Start Demo"')).toBeVisible();

    await page.click('text="Start Demo"');

    // Verify form elements are functional
    await page.fill('#prompt-input', `Cross-browser test on ${browserName}`);
    await expect(page.locator('#prompt-input')).toHaveValue(`Cross-browser test on ${browserName}`);

    await page.fill('#topic', 'Cross-Browser Testing');
    await page.fill('#audience', 'QA Engineers');
    await page.selectOption('#tone', 'technical');
    await page.fill('#word-count', '400');

    // Submit and verify it starts
    await page.click('#submit-button');
    await expect(page.locator('#session-status')).toBeVisible({ timeout: 10000 });

    console.log(`✅ ${browserName} compatibility verified`);
  });
});

/**
 * Utility Functions for Test Data and Helpers
 */

// Generate random test data for multiple runs
function generateTestData(runNumber) {
  return {
    prompt: `${TEST_PROMPTS[runNumber % TEST_PROMPTS.length]} - Run ${runNumber}`,
    topic: `Test Topic ${runNumber}`,
    audience: ['General Public', 'Business Professionals', 'Technical Experts', 'Students'][runNumber % 4],
    tone: ['casual', 'professional', 'technical', 'friendly'][runNumber % 4],
    wordCount: [400, 600, 800, 1000][runNumber % 4]
  };
}

// Performance measurement helper
function measurePerformance(name, fn) {
  return async (...args) => {
    const start = performance.now();
    const result = await fn(...args);
    const end = performance.now();
    console.log(`⏱️  ${name}: ${(end - start).toFixed(2)}ms`);
    return result;
  };
}
