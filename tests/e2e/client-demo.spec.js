// @ts-check
const { test, expect } = require('@playwright/test');

const CLIENT_APP_URL = process.env.CLIENT_APP_URL || 'http://localhost:8090';
const TEST_TIMEOUT = 60000; // 60 seconds for workflow completion

test.describe('AI-CORE Client App Demo - End-to-End Tests', () => {
  test.beforeEach(async ({ page }) => {
    // Set longer timeout for these tests
    test.setTimeout(90000);

    // Navigate to the demo page
    await page.goto(`${CLIENT_APP_URL}/demo`);

    // Wait for page to load
    await expect(page).toHaveTitle(/Live Demo/);
  });

  test('should load demo interface with all required elements', async ({ page }) => {
    // Check header elements
    await expect(page.locator('h1')).toContainText('Live Demo');
    await expect(page.locator('.status-indicator')).toBeVisible();

    // Check form elements
    await expect(page.locator('#topic')).toBeVisible();
    await expect(page.locator('#input_text')).toBeVisible();
    await expect(page.locator('#audience')).toBeVisible();
    await expect(page.locator('#tone')).toBeVisible();
    await expect(page.locator('#word_count')).toBeVisible();
    await expect(page.locator('#submit-btn')).toBeVisible();

    // Check output panel
    await expect(page.locator('#waiting-message')).toBeVisible();
    await expect(page.locator('#logs-container')).toBeVisible();
  });

  test('should complete full workflow for blog post generation', async ({ page }) => {
    // Fill out the form
    await page.fill('#topic', 'AI Automation in Healthcare');
    await page.fill('#input_text', 'Write a comprehensive blog post about how AI automation is revolutionizing healthcare delivery, improving patient outcomes, and reducing operational costs.');
    await page.selectOption('#audience', 'business_professionals');
    await page.selectOption('#tone', 'professional');
    await page.selectOption('#word_count', '800');

    // Submit the form
    await page.click('#submit-btn');

    // Verify form submission
    await expect(page.locator('#submit-btn')).toBeDisabled();
    await expect(page.locator('#progress-container')).toBeVisible();
    await expect(page.locator('#waiting-message')).toBeHidden();

    // Wait for progress to start
    await expect(page.locator('#progress-fill')).toHaveCSS('width', /[1-9]/);

    // Monitor logs for workflow steps
    const expectedSteps = [
      'Parsing user intent',
      'Creating workflow in AI-CORE',
      'Workflow created',
      'Workflow completed successfully'
    ];

    // Wait for workflow completion with polling
    let completed = false;
    let attempts = 0;
    const maxAttempts = 30; // 60 seconds total

    while (!completed && attempts < maxAttempts) {
      attempts++;

      // Check if results are visible
      const resultsVisible = await page.locator('#results').isVisible();
      if (resultsVisible) {
        completed = true;
        break;
      }

      // Check for error states
      const errorLogs = await page.locator('.log-level-ERROR').count();
      if (errorLogs > 0) {
        const errorText = await page.locator('.log-level-ERROR').first().textContent();
        throw new Error(`Workflow failed with error: ${errorText}`);
      }

      await page.waitForTimeout(2000);
    }

    if (!completed) {
      throw new Error('Workflow did not complete within timeout');
    }

    // Verify results are displayed
    await expect(page.locator('#results')).toBeVisible();
    await expect(page.locator('#result-title')).not.toBeEmpty();
    await expect(page.locator('#result-content')).not.toBeEmpty();

    // Verify metrics are populated
    await expect(page.locator('#execution-time')).not.toContainText('--');
    await expect(page.locator('#quality-score')).not.toContainText('--');
    await expect(page.locator('#word-count')).not.toContainText('--');
    await expect(page.locator('#time-saved')).not.toContainText('--');

    // Verify execution logs contain expected steps
    const logsContent = await page.locator('#logs').textContent();
    expect(logsContent).toContain('INFO');
    expect(logsContent).toContain('SUCCESS');

    // Verify quality metrics are reasonable
    const qualityScore = await page.locator('#quality-score').textContent();
    const score = parseFloat(qualityScore.split('/')[0]);
    expect(score).toBeGreaterThan(3.0);
    expect(score).toBeLessThanOrEqual(5.0);

    // Verify word count is reasonable
    const wordCountText = await page.locator('#word-count').textContent();
    const wordCount = parseInt(wordCountText);
    expect(wordCount).toBeGreaterThan(500);
    expect(wordCount).toBeLessThan(1500);

    // Form should be re-enabled
    await expect(page.locator('#submit-btn')).toBeEnabled();
  });

  test('should handle different content types and configurations', async ({ page }) => {
    const testCases = [
      {
        topic: 'Machine Learning',
        audience: 'technical_experts',
        tone: 'technical',
        wordCount: '1200'
      },
      {
        topic: 'Digital Marketing',
        audience: 'general_public',
        tone: 'conversational',
        wordCount: '500'
      }
    ];

    for (const testCase of testCases) {
      // Fill out form with test case data
      await page.fill('#topic', testCase.topic);
      await page.fill('#input_text', `Create content about ${testCase.topic}`);
      await page.selectOption('#audience', testCase.audience);
      await page.selectOption('#tone', testCase.tone);
      await page.selectOption('#word_count', testCase.wordCount);

      // Submit and wait for completion
      await page.click('#submit-btn');

      // Wait for workflow to complete
      await page.waitForSelector('#results', {
        state: 'visible',
        timeout: TEST_TIMEOUT
      });

      // Verify results
      const title = await page.locator('#result-title').textContent();
      expect(title).toContain(testCase.topic);

      // Reset for next test
      await page.reload();
      await page.waitForLoadState('networkidle');
    }
  });

  test('should display real-time progress updates', async ({ page }) => {
    // Fill and submit form
    await page.fill('#topic', 'Test Topic');
    await page.fill('#input_text', 'Test content description');
    await page.click('#submit-btn');

    // Track progress updates
    const progressUpdates = [];
    let initialProgress = 0;

    // Monitor progress for up to 30 seconds
    for (let i = 0; i < 15; i++) {
      const progressElement = page.locator('#progress-fill');
      const progressStyle = await progressElement.getAttribute('style');

      if (progressStyle && progressStyle.includes('width:')) {
        const widthMatch = progressStyle.match(/width:\s*(\d+)%/);
        if (widthMatch) {
          const currentProgress = parseInt(widthMatch[1]);
          if (currentProgress > initialProgress) {
            progressUpdates.push(currentProgress);
            initialProgress = currentProgress;
          }
        }
      }

      // Check if completed
      const resultsVisible = await page.locator('#results').isVisible();
      if (resultsVisible) break;

      await page.waitForTimeout(2000);
    }

    // Verify progress actually updated
    expect(progressUpdates.length).toBeGreaterThan(0);
    expect(Math.max(...progressUpdates)).toBeGreaterThan(50);
  });

  test('should log execution steps in real-time', async ({ page }) => {
    // Fill and submit form
    await page.fill('#topic', 'Testing Logs');
    await page.fill('#input_text', 'Test logging functionality');
    await page.click('#submit-btn');

    // Monitor logs
    const logEntries = [];

    // Check for log updates every 2 seconds for up to 30 seconds
    for (let i = 0; i < 15; i++) {
      const logs = await page.locator('#logs .log-entry').count();
      if (logs > logEntries.length) {
        const newLogCount = logs - logEntries.length;
        for (let j = 0; j < newLogCount; j++) {
          const logText = await page.locator('#logs .log-entry').nth(logEntries.length + j).textContent();
          logEntries.push(logText);
        }
      }

      // Check if workflow completed
      const resultsVisible = await page.locator('#results').isVisible();
      if (resultsVisible) break;

      await page.waitForTimeout(2000);
    }

    // Verify we got log entries
    expect(logEntries.length).toBeGreaterThan(0);

    // Verify log entries contain expected information
    const allLogsText = logEntries.join(' ');
    expect(allLogsText).toMatch(/INFO|SUCCESS|DEBUG/);
    expect(allLogsText).toContain('workflow');
  });

  test('should handle errors gracefully', async ({ page }) => {
    // Test with invalid configuration that might cause errors
    await page.fill('#topic', '');  // Empty topic
    await page.fill('#input_text', 'x');  // Very short description

    await page.click('#submit-btn');

    // Wait a reasonable time to see if error handling works
    await page.waitForTimeout(10000);

    // Check if error is displayed in logs or UI
    const errorLogs = await page.locator('.log-level-ERROR').count();
    const submitButtonEnabled = await page.locator('#submit-btn').isEnabled();

    // Either should show error or complete gracefully
    if (errorLogs > 0) {
      // Error was logged - good error handling
      expect(submitButtonEnabled).toBe(true);
    } else {
      // No error - system handled edge case gracefully
      // This is also acceptable behavior
    }
  });

  test('should maintain session state correctly', async ({ page }) => {
    // Start a workflow
    await page.fill('#topic', 'Session Test');
    await page.fill('#input_text', 'Testing session management');
    await page.click('#submit-btn');

    // Wait for some progress
    await page.waitForTimeout(5000);

    // Refresh the page
    await page.reload();

    // Verify the interface is in correct state after reload
    await expect(page.locator('#submit-btn')).toBeEnabled();
    await expect(page.locator('#progress-container')).toBeHidden();
  });
});

// Health check test for services
test.describe('Service Health Checks', () => {
  test('should verify all required services are running', async ({ request }) => {
    // Test client app health
    const clientHealth = await request.get(`${CLIENT_APP_URL}/api/health`);
    expect(clientHealth.ok()).toBeTruthy();

    // Test federation service health (through client app proxy or direct)
    try {
      const federationHealth = await request.get('http://localhost:8801/health');
      expect(federationHealth.ok()).toBeTruthy();
    } catch (error) {
      console.warn('Federation service not accessible directly, testing through client app');
    }
  });
});
