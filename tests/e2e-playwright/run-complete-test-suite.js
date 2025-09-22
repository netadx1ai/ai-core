#!/usr/bin/env node

/**
 * Complete E2E Test Suite Runner for AI-CORE
 *
 * This is the master orchestrator that runs the complete E2E test suite
 * multiple times per build to ensure comprehensive validation:
 *
 * - Multiple runs to detect race conditions and intermittent failures
 * - Comprehensive reporting with HTML, JSON, and CSV outputs
 * - Service health monitoring and dependency verification
 * - Performance analysis across multiple test executions
 * - Failure pattern analysis and stability scoring
 * - Quality gates for production readiness assessment
 */

import { exec, spawn } from 'child_process';
import fs from 'fs-extra';
import path from 'path';
import { fileURLToPath } from 'url';
import { promisify } from 'util';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const execAsync = promisify(exec);

class CompleteE2ETestSuite {
  constructor() {
    this.startTime = Date.now();
    this.runId = new Date().toISOString().replace(/[:.]/g, '-');

    this.config = {
      // Test execution configuration
      testRounds: {
        smoke: { runs: 2, timeout: '30s', parallel: false },
        critical: { runs: 5, timeout: '60s', parallel: true },
        stability: { runs: 10, timeout: '90s', parallel: false },
        regression: { runs: 15, timeout: '45s', parallel: true },
        load: { runs: 3, timeout: '120s', parallel: true },
        performance: { runs: 5, timeout: '180s', parallel: false }
      },

      // Service endpoints to validate
      services: {
        'Client Demo': 'http://localhost:8090',
        'Federation Service': 'http://localhost:8801',
        'Intent Parser': 'http://localhost:8802',
        'MCP Manager': 'http://localhost:8803'
      },

      // Output configuration
      outputDir: path.join(__dirname, 'test-results', 'complete-suite'),
      maxLogSize: 10 * 1024 * 1024, // 10MB max per log file
      keepArtifacts: process.env.KEEP_ALL_ARTIFACTS === 'true',

      // Quality gates
      qualityGates: {
        minSuccessRate: 90,
        maxFlakyRate: 5,
        minStabilityScore: 85,
        maxAvgExecutionTime: 60000 // 60 seconds
      }
    };

    this.results = {
      metadata: {
        runId: this.runId,
        startTime: new Date().toISOString(),
        version: '1.0.0',
        environment: process.env.NODE_ENV || 'test',
        playwright_version: null,
        node_version: process.version
      },
      execution: {
        totalRounds: 0,
        completedRounds: 0,
        totalTests: 0,
        totalDuration: 0,
        phases: []
      },
      summary: {
        passed: 0,
        failed: 0,
        skipped: 0,
        flaky: 0,
        successRate: 0,
        stabilityScore: 0
      },
      services: {},
      performance: {
        averageExecutionTime: 0,
        medianExecutionTime: 0,
        p95ExecutionTime: 0,
        slowestTests: [],
        fastestTests: []
      },
      stability: {
        consistentTests: [],
        flakyTests: [],
        failurePatterns: {},
        raceConditions: []
      },
      qualityGates: {
        passed: 0,
        total: 0,
        results: {}
      },
      artifacts: {
        reports: [],
        screenshots: [],
        videos: [],
        logs: []
      },
      recommendations: []
    };
  }

  async run() {
    console.log('\n🚀 AI-CORE Complete E2E Test Suite');
    console.log('===================================');
    console.log(`📊 Run ID: ${this.runId}`);
    console.log(`🏗️ Environment: ${this.results.metadata.environment}`);
    console.log(`📅 Started: ${this.results.metadata.startTime}`);

    try {
      await this.initialize();
      await this.validateEnvironment();
      await this.executeTestSuite();
      await this.analyzeResults();
      await this.generateReports();
      await this.evaluateQualityGates();

      const success = this.results.qualityGates.passed === this.results.qualityGates.total;

      if (success) {
        console.log('\n🎉 ALL TESTS PASSED - PRODUCTION READY!');
        process.exit(0);
      } else {
        console.log('\n⚠️ QUALITY GATES FAILED - REVIEW REQUIRED');
        process.exit(1);
      }

    } catch (error) {
      console.error('\n💥 Test suite execution failed:', error.message);
      await this.handleError(error);
      process.exit(1);
    }
  }

  async initialize() {
    console.log('\n📋 Initializing test environment...');

    // Ensure output directories exist
    await fs.ensureDir(this.config.outputDir);
    await fs.ensureDir(path.join(this.config.outputDir, 'logs'));
    await fs.ensureDir(path.join(this.config.outputDir, 'artifacts'));
    await fs.ensureDir(path.join(this.config.outputDir, 'reports'));

    // Get Playwright version
    try {
      const { stdout } = await execAsync('npx playwright --version');
      this.results.metadata.playwright_version = stdout.trim();
    } catch (error) {
      console.log('⚠️ Playwright not found, installing...');
      await execAsync('npm install');
      await execAsync('npx playwright install');
      const { stdout } = await execAsync('npx playwright --version');
      this.results.metadata.playwright_version = stdout.trim();
    }

    console.log(`✅ Environment initialized`);
    console.log(`   Node.js: ${process.version}`);
    console.log(`   Playwright: ${this.results.metadata.playwright_version}`);
  }

  async validateEnvironment() {
    console.log('\n🔍 Validating service dependencies...');

    const serviceChecks = Object.entries(this.config.services).map(async ([name, url]) => {
      const healthUrl = `${url}/health`;

      try {
        const response = await fetch(healthUrl, {
          timeout: 5000,
          signal: AbortSignal.timeout(5000)
        });

        const status = response.ok ? 'healthy' : 'degraded';
        const responseTime = response.ok ? Date.now() : null;

        console.log(`  ${status === 'healthy' ? '✅' : '⚠️'} ${name}: ${status} (${healthUrl})`);

        return {
          name,
          url,
          status,
          responseTime,
          error: response.ok ? null : `HTTP ${response.status}`
        };
      } catch (error) {
        console.log(`  ❌ ${name}: unavailable (${error.message})`);
        return {
          name,
          url,
          status: 'unavailable',
          error: error.message
        };
      }
    });

    const serviceResults = await Promise.all(serviceChecks);
    this.results.services = serviceResults.reduce((acc, result) => {
      acc[result.name] = result;
      return acc;
    }, {});

    const healthyServices = serviceResults.filter(s => s.status === 'healthy').length;
    const totalServices = serviceResults.length;

    console.log(`📊 Service Health: ${healthyServices}/${totalServices} services available`);

    if (healthyServices < totalServices && process.env.REQUIRE_ALL_SERVICES === 'true') {
      throw new Error('Not all required services are available');
    }
  }

  async executeTestSuite() {
    console.log('\n🏃‍♂️ Executing test suite...');

    const testRounds = Object.entries(this.config.testRounds);
    this.results.execution.totalRounds = testRounds.length;

    for (const [roundName, roundConfig] of testRounds) {
      console.log(`\n📝 Starting ${roundName.toUpperCase()} tests (${roundConfig.runs} runs)...`);

      const phaseStartTime = Date.now();
      const phaseResult = {
        name: roundName,
        config: roundConfig,
        startTime: new Date().toISOString(),
        runs: [],
        summary: {
          total: 0,
          passed: 0,
          failed: 0,
          duration: 0,
          successRate: 0
        }
      };

      // Execute multiple runs for this test round
      for (let runNumber = 1; runNumber <= roundConfig.runs; runNumber++) {
        console.log(`\n  🔄 ${roundName} - Run ${runNumber}/${roundConfig.runs}`);

        const runResult = await this.executeTestRun(roundName, runNumber, roundConfig);
        phaseResult.runs.push(runResult);

        // Update phase summary
        phaseResult.summary.total++;
        if (runResult.status === 'passed') {
          phaseResult.summary.passed++;
        } else {
          phaseResult.summary.failed++;
        }
        phaseResult.summary.duration += runResult.duration;

        // Brief cooldown between runs
        if (runNumber < roundConfig.runs) {
          await this.sleep(2000);
        }
      }

      // Calculate phase metrics
      phaseResult.summary.successRate = phaseResult.summary.total > 0
        ? (phaseResult.summary.passed / phaseResult.summary.total * 100).toFixed(2)
        : 0;

      phaseResult.endTime = new Date().toISOString();
      phaseResult.totalDuration = Date.now() - phaseStartTime;

      console.log(`\n  📊 ${roundName.toUpperCase()} Results:`);
      console.log(`     Success Rate: ${phaseResult.summary.successRate}% (${phaseResult.summary.passed}/${phaseResult.summary.total})`);
      console.log(`     Average Duration: ${Math.round(phaseResult.summary.duration / phaseResult.summary.total / 1000)}s`);
      console.log(`     Total Phase Time: ${Math.round(phaseResult.totalDuration / 1000)}s`);

      this.results.execution.phases.push(phaseResult);
      this.results.execution.completedRounds++;

      // Update overall totals
      this.results.summary.passed += phaseResult.summary.passed;
      this.results.summary.failed += phaseResult.summary.failed;
      this.results.execution.totalTests += phaseResult.summary.total;
      this.results.execution.totalDuration += phaseResult.totalDuration;
    }
  }

  async executeTestRun(testType, runNumber, config) {
    const runStartTime = Date.now();
    const outputDir = path.join(this.config.outputDir, 'artifacts', `${testType}-run-${runNumber}`);

    // Construct Playwright command
    let command = `npx playwright test tests/${testType}`;
    command += ` --reporter=json`;
    command += ` --output-dir="${outputDir}"`;
    command += ` --timeout=${this.parseTimeout(config.timeout)}`;

    if (config.parallel) {
      command += ` --workers=2`;
    } else {
      command += ` --workers=1`;
    }

    try {
      const { exitCode, stdout, stderr } = await this.runCommand(command, {
        timeout: this.parseTimeout(config.timeout) + 30000,
        cwd: __dirname
      });

      const duration = Date.now() - runStartTime;
      const status = exitCode === 0 ? 'passed' : 'failed';

      console.log(`    ${status === 'passed' ? '✅' : '❌'} Run ${runNumber}: ${status.toUpperCase()} (${Math.round(duration/1000)}s)`);

      // Log any errors
      if (stderr && status === 'failed') {
        console.log(`       Error preview: ${stderr.substring(0, 150)}...`);
      }

      // Save run logs
      const logFile = path.join(this.config.outputDir, 'logs', `${testType}-run-${runNumber}.log`);
      await fs.writeFile(logFile, JSON.stringify({
        testType,
        runNumber,
        status,
        duration,
        exitCode,
        stdout: stdout.length > this.config.maxLogSize ? stdout.substring(0, this.config.maxLogSize) + '...[truncated]' : stdout,
        stderr: stderr.length > this.config.maxLogSize ? stderr.substring(0, this.config.maxLogSize) + '...[truncated]' : stderr,
        timestamp: new Date().toISOString()
      }, null, 2));

      return {
        runNumber,
        status,
        duration,
        exitCode,
        timestamp: new Date().toISOString(),
        logFile: path.relative(this.config.outputDir, logFile)
      };

    } catch (error) {
      const duration = Date.now() - runStartTime;
      console.log(`    💥 Run ${runNumber}: ERROR (${error.message})`);

      return {
        runNumber,
        status: 'error',
        duration,
        error: error.message,
        timestamp: new Date().toISOString()
      };
    }
  }

  async analyzeResults() {
    console.log('\n📈 Analyzing test results...');

    // Calculate overall success rate
    this.results.summary.successRate = this.results.execution.totalTests > 0
      ? (this.results.summary.passed / this.results.execution.totalTests * 100).toFixed(2)
      : 0;

    // Analyze performance
    this.analyzePerformance();

    // Analyze stability
    this.analyzeStability();

    // Generate recommendations
    this.generateRecommendations();

    console.log(`✅ Analysis complete`);
  }

  analyzePerformance() {
    const allDurations = [];

    this.results.execution.phases.forEach(phase => {
      phase.runs.forEach(run => {
        if (run.status === 'passed' && run.duration) {
          allDurations.push(run.duration);
        }
      });
    });

    if (allDurations.length > 0) {
      allDurations.sort((a, b) => a - b);

      this.results.performance.averageExecutionTime = Math.round(
        allDurations.reduce((sum, d) => sum + d, 0) / allDurations.length
      );

      this.results.performance.medianExecutionTime = Math.round(
        allDurations[Math.floor(allDurations.length / 2)]
      );

      this.results.performance.p95ExecutionTime = Math.round(
        allDurations[Math.floor(allDurations.length * 0.95)]
      );

      // Find slowest and fastest tests
      const sortedPhases = this.results.execution.phases
        .map(phase => ({
          name: phase.name,
          avgDuration: phase.summary.duration / phase.summary.total,
          successRate: parseFloat(phase.summary.successRate)
        }))
        .sort((a, b) => b.avgDuration - a.avgDuration);

      this.results.performance.slowestTests = sortedPhases.slice(0, 3);
      this.results.performance.fastestTests = sortedPhases.slice(-3).reverse();
    }
  }

  analyzeStability() {
    // Calculate stability score based on consistency across runs
    let totalStabilityPoints = 0;
    let totalPhases = 0;

    this.results.execution.phases.forEach(phase => {
      const successRate = parseFloat(phase.summary.successRate);
      totalPhases++;

      if (successRate >= 95) {
        totalStabilityPoints += 100;
        this.results.stability.consistentTests.push({
          name: phase.name,
          successRate,
          runs: phase.summary.total
        });
      } else if (successRate >= 80) {
        totalStabilityPoints += 75;
      } else if (successRate > 0) {
        totalStabilityPoints += 50;
        this.results.stability.flakyTests.push({
          name: phase.name,
          successRate,
          runs: phase.summary.total,
          issue: 'Intermittent failures detected'
        });
      } else {
        totalStabilityPoints += 0;
      }

      // Detect potential race conditions (high variance in execution times)
      if (phase.runs.length > 2) {
        const durations = phase.runs.filter(r => r.status === 'passed').map(r => r.duration);
        if (durations.length > 1) {
          const variance = this.calculateVariance(durations);
          if (variance > 10000) { // High variance indicates potential timing issues
            this.results.stability.raceConditions.push({
              name: phase.name,
              variance: Math.round(variance),
              evidence: 'High execution time variance detected'
            });
          }
        }
      }
    });

    this.results.summary.stabilityScore = totalPhases > 0
      ? (totalStabilityPoints / totalPhases).toFixed(2)
      : 0;

    // Count flaky tests
    this.results.summary.flaky = this.results.stability.flakyTests.length;
  }

  generateRecommendations() {
    const recommendations = [];

    // Success rate recommendations
    if (parseFloat(this.results.summary.successRate) < this.config.qualityGates.minSuccessRate) {
      recommendations.push({
        priority: 'HIGH',
        category: 'Reliability',
        issue: `Success rate ${this.results.summary.successRate}% is below target ${this.config.qualityGates.minSuccessRate}%`,
        action: 'Investigate and fix failing tests to improve overall reliability',
        impact: 'Critical for production deployment'
      });
    }

    // Stability recommendations
    if (this.results.stability.flakyTests.length > 0) {
      recommendations.push({
        priority: 'HIGH',
        category: 'Stability',
        issue: `${this.results.stability.flakyTests.length} test types show flaky behavior`,
        action: 'Fix intermittent failures by improving waits, selectors, and error handling',
        impact: 'Flaky tests reduce CI/CD reliability'
      });
    }

    // Performance recommendations
    if (this.results.performance.averageExecutionTime > this.config.qualityGates.maxAvgExecutionTime) {
      recommendations.push({
        priority: 'MEDIUM',
        category: 'Performance',
        issue: `Average execution time ${Math.round(this.results.performance.averageExecutionTime/1000)}s exceeds target ${this.config.qualityGates.maxAvgExecutionTime/1000}s`,
        action: 'Optimize slow tests or increase timeout values appropriately',
        impact: 'Long test times slow development cycles'
      });
    }

    // Race condition recommendations
    if (this.results.stability.raceConditions.length > 0) {
      recommendations.push({
        priority: 'HIGH',
        category: 'Concurrency',
        issue: `${this.results.stability.raceConditions.length} tests show signs of race conditions`,
        action: 'Review tests with high execution time variance and add proper synchronization',
        impact: 'Race conditions cause unpredictable test failures'
      });
    }

    this.results.recommendations = recommendations;
  }

  async generateReports() {
    console.log('\n📊 Generating comprehensive reports...');

    // Update metadata
    this.results.metadata.endTime = new Date().toISOString();
    this.results.metadata.totalDuration = Date.now() - this.startTime;

    // Save detailed JSON report
    const jsonReport = path.join(this.config.outputDir, 'reports', `complete-test-report-${this.runId}.json`);
    await fs.writeJson(jsonReport, this.results, { spaces: 2 });
    this.results.artifacts.reports.push(path.relative(this.config.outputDir, jsonReport));

    // Generate HTML dashboard
    await this.generateHtmlDashboard();

    // Generate CSV summary
    await this.generateCsvSummary();

    // Generate executive summary
    await this.generateExecutiveSummary();

    console.log(`✅ Reports generated in: ${this.config.outputDir}/reports/`);
  }

  async generateHtmlDashboard() {
    const html = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AI-CORE Complete E2E Test Dashboard</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f7fa; color: #333; }
        .header { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 2rem; text-align: center; }
        .header h1 { font-size: 2.5rem; margin-bottom: 0.5rem; }
        .header p { font-size: 1.1rem; opacity: 0.9; }
        .container { max-width: 1200px; margin: 0 auto; padding: 2rem; }
        .metrics-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 1.5rem; margin-bottom: 2rem; }
        .metric-card { background: white; padding: 1.5rem; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); border-left: 4px solid #667eea; }
        .metric-card h3 { color: #667eea; font-size: 0.9rem; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 0.5rem; }
        .metric-value { font-size: 2.5rem; font-weight: bold; margin-bottom: 0.5rem; }
        .metric-value.success { color: #10b981; }
        .metric-value.warning { color: #f59e0b; }
        .metric-value.error { color: #ef4444; }
        .metric-label { color: #6b7280; font-size: 0.9rem; }
        .section { background: white; border-radius: 12px; margin-bottom: 2rem; overflow: hidden; box-shadow: 0 4px 6px rgba(0,0,0,0.1); }
        .section-header { background: #667eea; color: white; padding: 1rem 1.5rem; font-size: 1.1rem; font-weight: 600; }
        .section-content { padding: 1.5rem; }
        .phase-grid { display: grid; gap: 1rem; }
        .phase-card { border: 1px solid #e5e7eb; border-radius: 8px; padding: 1rem; }
        .phase-header { display: flex; justify-content: between; align-items: center; margin-bottom: 1rem; }
        .phase-name { font-weight: 600; font-size: 1.1rem; }
        .success-rate { font-weight: bold; }
        .success-rate.high { color: #10b981; }
        .success-rate.medium { color: #f59e0b; }
        .success-rate.low { color: #ef4444; }
        .runs-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(50px, 1fr)); gap: 0.5rem; margin-top: 1rem; }
        .run-badge { padding: 0.5rem; text-align: center; border-radius: 4px; font-size: 0.8rem; font-weight: 500; }
        .run-passed { background: #dcfce7; color: #166534; }
        .run-failed { background: #fee2e2; color: #991b1b; }
        .run-error { background: #fef3c7; color: #92400e; }
        .recommendations { background: #fffbeb; border: 1px solid #fed7aa; border-radius: 8px; padding: 1rem; margin: 1rem 0; }
        .recommendation { margin: 0.5rem 0; padding: 0.5rem; background: white; border-radius: 4px; border-left: 4px solid #f59e0b; }
        .recommendation.high { border-left-color: #ef4444; }
        .recommendation.medium { border-left-color: #f59e0b; }
        .recommendation.low { border-left-color: #10b981; }
        .quality-gates { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem; margin: 1rem 0; }
        .gate-card { padding: 1rem; border-radius: 8px; text-align: center; }
        .gate-passed { background: #dcfce7; border: 1px solid #bbf7d0; }
        .gate-failed { background: #fee2e2; border: 1px solid #fecaca; }
        .timestamp { text-align: center; margin-top: 2rem; color: #6b7280; font-size: 0.9rem; }
        table { width: 100%; border-collapse: collapse; margin-top: 1rem; }
        th, td { padding: 0.75rem; text-align: left; border-bottom: 1px solid #e5e7eb; }
        th { background: #f9fafb; font-weight: 600; }
    </style>
</head>
<body>
    <div class="header">
        <h1>🚀 AI-CORE E2E Test Dashboard</h1>
        <p>Complete Test Suite Results - Run ID: ${this.runId}</p>
        <p>Generated: ${new Date().toLocaleString()}</p>
    </div>

    <div class="container">
        <!-- Key Metrics -->
        <div class="metrics-grid">
            <div class="metric-card">
                <h3>Success Rate</h3>
                <div class="metric-value ${parseFloat(this.results.summary.successRate) >= 90 ? 'success' : parseFloat(this.results.summary.successRate) >= 75 ? 'warning' : 'error'}">${this.results.summary.successRate}%</div>
                <div class="metric-label">${this.results.summary.passed}/${this.results.execution.totalTests} tests passed</div>
            </div>
            <div class="metric-card">
                <h3>Stability Score</h3>
                <div class="metric-value ${parseFloat(this.results.summary.stabilityScore) >= 85 ? 'success' : parseFloat(this.results.summary.stabilityScore) >= 70 ? 'warning' : 'error'}">${this.results.summary.stabilityScore}/100</div>
                <div class="metric-label">Cross-run consistency</div>
            </div>
            <div class="metric-card">
                <h3>Execution Time</h3>
                <div class="metric-value">${Math.round(this.results.metadata.totalDuration / 60000)}m</div>
                <div class="metric-label">Total suite duration</div>
            </div>
            <div class="metric-card">
                <h3>Test Coverage</h3>
                <div class="metric-value">${this.results.execution.totalRounds}</div>
                <div class="metric-label">Test phases executed</div>
            </div>
        </div>

        <!-- Test Phase Results -->
        <div class="section">
            <div class="section-header">📊 Test Phase Results</div>
            <div class="section-content">
                <div class="phase-grid">
                    ${this.results.execution.phases.map(phase => {
                      const successRate = parseFloat(phase.summary.successRate);
                      const rateClass = successRate >= 90 ? 'high' : successRate >= 75 ? 'medium' : 'low';

                      return `
                        <div class="phase-card">
                            <div class="phase-header">
                                <div class="phase-name">${phase.name.toUpperCase()}</div>
                                <div class="success-rate ${rateClass}">${phase.summary.successRate}%</div>
                            </div>
                            <div>Runs: ${phase.summary.passed}/${phase.summary.total} passed</div>
                            <div>Avg Duration: ${Math.round(phase.summary.duration / phase.summary.total / 1000)}s</div>
                            <div class="runs-grid">
                                ${phase.runs.map(run => `
                                    <div class="run-badge run-${run.status}">
                                        ${run.runNumber}
                                    </div>
                                `).join('')}
                            </div>
                        </div>
                      `;
                    }).join('')}
                </div>
            </div>
        </div>

        <!-- Performance Analysis -->
        <div class="section">
            <div class="section-header">⚡ Performance Analysis</div>
            <div class="section-content">
                <div class="metrics-grid">
                    <div>
                        <h4>Execution Times</h4>
                        <p><strong>Average:</strong> ${Math.round(this.results.performance.averageExecutionTime / 1000)}s</p>
                        <p><strong>Median:</strong> ${Math.round(this.results.performance.medianExecutionTime / 1000)}s</p>
                        <p><strong>95th Percentile:</strong> ${Math.round(this.results.performance.p95ExecutionTime / 1000)}s</p>
                    </div>
                    <div>
                        <h4>Slowest Test Types</h4>
                        ${this.results.performance.slowestTests.map(test => `
                            <p><strong>${test.name}:</strong> ${Math.round(test.avgDuration / 1000)}s avg</p>
                        `).join('')}
                    </div>
                    <div>
                        <h4>Fastest Test Types</h4>
                        ${this.results.performance.fastestTests.map(test => `
                            <p><strong>${test.name}:</strong> ${Math.round(test.avgDuration / 1000)}s avg</p>
                        `).join('')}
                    </div>
                </div>
            </div>
        </div>

        <!-- Service Health -->
        <div class="section">
            <div class="section-header">🔍 Service Health Status</div>
            <div class="section-content">
                <table>
                    <thead>
                        <tr>
                            <th>Service</th>
                            <th>Status</th>
                            <th>URL</th>
                            <th>Response Time</th>
                        </tr>
                    </thead>
                    <tbody>
                        ${Object.entries(this.results.services).map(([name, service]) => `
                            <tr>
                                <td>${name}</td>
                                <td>
                                    <span class="run-badge run-${service.status === 'healthy' ? 'passed' : 'failed'}">
                                        ${service.status}
                                    </span>
                                </td>
                                <td>${service.url}</td>
                                <td>${service.responseTime ? service.responseTime + 'ms' : 'N/A'}</td>
                            </tr>
                        `).join('')}
                    </tbody>
                </table>
            </div>
        </div>

        <!-- Recommendations -->
        ${this.results.recommendations.length > 0 ? `
        <div class="section">
            <div class="section-header">💡 Recommendations</div>
            <div class="section-content">
                ${this.results.recommendations.map(rec => `
                    <div class="recommendation ${rec.priority.toLowerCase()}">
                        <h4>${rec.category} - ${rec.priority} Priority</h4>
                        <p><strong>Issue:</strong> ${rec.issue}</p>
                        <p><strong>Action:</strong> ${rec.action}</p>
                        <p><strong>Impact:</strong> ${rec.impact}</p>
                    </div>
                `).join('')}
            </div>
        </div>
        ` : ''}

        <div class="timestamp">
            <p>Report generated: ${new Date().toISOString()}</p>
            <p>Total execution time: ${Math.round(this.results.metadata.totalDuration / 60000)} minutes</p>
            <p>AI-CORE E2E Test Suite v${this.results.metadata.version}</p>
        </div>
    </div>
</body>
</html>`;

    await fs.writeFile(path.join(this.config.outputDir, 'reports', 'dashboard.html'), html);
    this.results.artifacts.reports.push('reports/dashboard.html');
  }

  async generateCsvSummary() {
    const csvLines = ['Phase,Run,Status,Duration(ms),Success Rate,Timestamp'];

    this.results.execution.phases.forEach(phase => {
      phase.runs.forEach(run => {
        csvLines.push([
          `"${phase.name}"`,
          run.runNumber,
          `"${run.status}"`,
          run.duration || 0,
          `"${phase.summary.successRate}%"`,
          `"${run.timestamp}"`
        ].join(','));
      });
    });

    await fs.writeFile(path.join(this.config.outputDir, 'reports', 'test-results.csv'), csvLines.join('\n'));
    this.results.artifacts.reports.push('reports/test-results.csv');
  }

  async generateExecutiveSummary() {
    const summary = `
# AI-CORE E2E Test Suite - Executive Summary

**Run ID:** ${this.runId}
**Generated:** ${new Date().toLocaleString()}
**Duration:** ${Math.round(this.results.metadata.totalDuration / 60000)} minutes

## 🎯 Key Results

- **Success Rate:** ${this.results.summary.successRate}% (${this.results.summary.passed}/${this.results.execution.totalTests} tests passed)
- **Stability Score:** ${this.results.summary.stabilityScore}/100 (cross-run consistency)
- **Test Coverage:** ${this.results.execution.totalRounds} test phases executed
- **Average Execution:** ${Math.round(this.results.performance.averageExecutionTime / 1000)}s per test

## 📊 Test Phases Summary

${this.results.execution.phases.map(phase => `
### ${phase.name.toUpperCase()}
- **Success Rate:** ${phase.summary.successRate}%
- **Runs:** ${phase.summary.passed}/${phase.summary.total} passed
- **Average Duration:** ${Math.round(phase.summary.duration / phase.summary.total / 1000)}s
`).join('')}

## 🔍 Service Health

${Object.entries(this.results.services).map(([name, service]) => `
- **${name}:** ${service.status} (${service.url})
`).join('')}

## ⚠️ Issues Identified

${this.results.recommendations.length > 0 ? this.results.recommendations.map(rec => `
### ${rec.category} - ${rec.priority} Priority
**Issue:** ${rec.issue}
**Action:** ${rec.action}
`).join('') : 'No critical issues identified.'}

## 🚦 Production Readiness

${this.results.summary.successRate >= 90 && this.results.summary.stabilityScore >= 85 && this.results.summary.flaky <= 1
  ? '✅ **READY FOR PRODUCTION** - All quality gates passed'
  : '⚠️ **REVIEW REQUIRED** - Some quality gates failed'
}

---
*Generated by AI-CORE E2E Test Suite v${this.results.metadata.version}*
`;

    await fs.writeFile(path.join(this.config.outputDir, 'reports', 'executive-summary.md'), summary);
    this.results.artifacts.reports.push('reports/executive-summary.md');
  }

  async evaluateQualityGates() {
    console.log('\n🚦 Evaluating quality gates...');

    const gates = [
      {
        name: 'Success Rate',
        condition: parseFloat(this.results.summary.successRate) >= this.config.qualityGates.minSuccessRate,
        actual: `${this.results.summary.successRate}%`,
        target: `≥${this.config.qualityGates.minSuccessRate}%`
      },
      {
        name: 'Stability Score',
        condition: parseFloat(this.results.summary.stabilityScore) >= this.config.qualityGates.minStabilityScore,
        actual: `${this.results.summary.stabilityScore}/100`,
        target: `≥${this.config.qualityGates.minStabilityScore}/100`
      },
      {
        name: 'Flaky Tests',
        condition: this.results.summary.flaky <= this.config.qualityGates.maxFlakyRate,
        actual: this.results.summary.flaky,
        target: `≤${this.config.qualityGates.maxFlakyRate}`
      },
      {
        name: 'Performance',
        condition: this.results.performance.averageExecutionTime <= this.config.qualityGates.maxAvgExecutionTime,
        actual: `${Math.round(this.results.performance.averageExecutionTime / 1000)}s`,
        target: `≤${this.config.qualityGates.maxAvgExecutionTime / 1000}s`
      }
    ];

    this.results.qualityGates.total = gates.length;
    this.results.qualityGates.passed = 0;

    console.log('\n  Quality Gate Results:');
    gates.forEach(gate => {
      const status = gate.condition ? '✅ PASS' : '❌ FAIL';
      console.log(`    ${status} ${gate.name}: ${gate.actual} (target: ${gate.target})`);

      this.results.qualityGates.results[gate.name] = {
        passed: gate.condition,
        actual: gate.actual,
        target: gate.target
      };

      if (gate.condition) {
        this.results.qualityGates.passed++;
      }
    });

    console.log(`\n  📊 Quality Gates: ${this.results.qualityGates.passed}/${this.results.qualityGates.total} PASSED`);
  }

  async handleError(error) {
    const errorReport = {
      timestamp: new Date().toISOString(),
      runId: this.runId,
      error: {
        message: error.message,
        stack: error.stack
      },
      partialResults: this.results,
      environment: process.env
    };

    await fs.writeJson(
      path.join(this.config.outputDir, `error-report-${this.runId}.json`),
      errorReport,
      { spaces: 2 }
    );
  }

  // Utility methods
  parseTimeout(timeoutStr) {
    const match = timeoutStr.match(/^(\d+)([sm])$/);
    if (!match) return 30000;

    const [, num, unit] = match;
    return unit === 's' ? parseInt(num) * 1000 : parseInt(num) * 60000;
  }

  runCommand(command, options = {}) {
    return new Promise((resolve, reject) => {
      const child = spawn('bash', ['-c', command], {
        cwd: options.cwd || __dirname,
        env: { ...process.env, NODE_ENV: 'test' }
      });

      let stdout = '';
      let stderr = '';

      child.stdout?.on('data', (data) => stdout += data.toString());
      child.stderr?.on('data', (data) => stderr += data.toString());

      const timeout = options.timeout || 300000;
      const timer = setTimeout(() => {
        child.kill('SIGTERM');
        reject(new Error(`Command timed out after ${timeout}ms`));
      }, timeout);

      child.on('close', (exitCode) => {
        clearTimeout(timer);
        resolve({ exitCode, stdout, stderr });
      });

      child.on('error', (error) => {
        clearTimeout(timer);
        reject(error);
      });
    });
  }

  calculateVariance(numbers) {
    if (numbers.length === 0) return 0;
    const mean = numbers.reduce((sum, n) => sum + n, 0) / numbers.length;
    const squaredDiffs = numbers.map(n => Math.pow(n - mean, 2));
    return Math.sqrt(squaredDiffs.reduce((sum, n) => sum + n, 0) / numbers.length);
  }

  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// CLI execution
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const suite = new CompleteE2ETestSuite();

  // Handle process signals
  process.on('SIGINT', async () => {
    console.log('\n⚠️ Test suite interrupted by user');
    await suite.handleError(new Error('Test suite interrupted by SIGINT'));
    process.exit(1);
  });

  process.on('SIGTERM', async () => {
    console.log('\n⚠️ Test suite terminated');
    await suite.handleError(new Error('Test suite terminated by SIGTERM'));
    process.exit(1);
  });

  suite.run();
}

export default CompleteE2ETestSuite;
