#!/usr/bin/env node

/**
 * Multiple E2E Test Orchestrator for AI-CORE
 *
 * Runs E2E tests multiple times per build to ensure:
 * - Stability across multiple runs
 * - Detection of race conditions
 * - Identification of intermittent failures
 * - Performance consistency validation
 * - Comprehensive error analysis
 */

import { exec, spawn } from 'child_process';
import fs from 'fs-extra';
import path from 'path';
import { fileURLToPath } from 'url';
import { promisify } from 'util';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const execAsync = promisify(exec);

class MultipleE2EOrchestrator {
  constructor() {
    this.config = {
      // Test run configuration
      stabilityRuns: parseInt(process.env.STABILITY_RUNS || '5'),
      regressionRuns: parseInt(process.env.REGRESSION_RUNS || '10'),
      loadTestRuns: parseInt(process.env.LOAD_TEST_RUNS || '3'),

      // Service configuration
      services: {
        clientDemo: { port: 8090, healthPath: '/health' },
        federation: { port: 8801, healthPath: '/health' },
        intentParser: { port: 8802, healthPath: '/health' },
        mcpManager: { port: 8803, healthPath: '/health' }
      },

      // Test configuration
      parallelWorkers: parseInt(process.env.PARALLEL_WORKERS || '2'),
      maxRetries: parseInt(process.env.MAX_RETRIES || '2'),
      timeoutPerRun: parseInt(process.env.TIMEOUT_PER_RUN || '300000'), // 5 minutes
      cooldownBetweenRuns: parseInt(process.env.COOLDOWN_MS || '5000'), // 5 seconds

      // Output configuration
      outputDir: path.join(__dirname, 'test-results'),
      reportDir: path.join(__dirname, 'test-results', 'multiple-runs'),
      keepArtifacts: process.env.KEEP_ARTIFACTS !== 'false'
    };

    this.runId = new Date().toISOString().replace(/[:.]/g, '-');
    this.testResults = {
      metadata: {
        runId: this.runId,
        startTime: new Date().toISOString(),
        config: this.config
      },
      summary: {
        totalCycles: 0,
        totalTests: 0,
        passed: 0,
        failed: 0,
        flaky: 0,
        duration: 0
      },
      cycles: [],
      failures: [],
      stability: {},
      services: {}
    };
  }

  async run() {
    console.log('\n🚀 AI-CORE Multiple E2E Test Orchestrator');
    console.log('==========================================');
    console.log(`📊 Run ID: ${this.runId}`);
    console.log(`🔄 Stability Runs: ${this.config.stabilityRuns}`);
    console.log(`📈 Regression Runs: ${this.config.regressionRuns}`);
    console.log(`⚡ Load Test Runs: ${this.config.loadTestRuns}`);
    console.log(`👥 Parallel Workers: ${this.config.parallelWorkers}`);

    try {
      await this.setup();
      await this.checkServices();
      await this.runTestCycles();
      await this.generateReport();
      await this.cleanup();

      console.log('\n✅ Multiple E2E test execution completed successfully!');
      process.exit(0);

    } catch (error) {
      console.error('\n❌ Multiple E2E test execution failed:', error.message);
      await this.generateErrorReport(error);
      process.exit(1);
    }
  }

  async setup() {
    console.log('\n📋 Setting up test environment...');

    // Ensure directories exist
    await fs.ensureDir(this.config.outputDir);
    await fs.ensureDir(this.config.reportDir);
    await fs.ensureDir(path.join(this.config.reportDir, 'artifacts'));
    await fs.ensureDir(path.join(this.config.reportDir, 'logs'));

    // Install dependencies if needed
    console.log('📦 Checking dependencies...');
    try {
      await execAsync('npx playwright --version', { cwd: __dirname });
    } catch (error) {
      console.log('🔧 Installing Playwright...');
      await execAsync('npm install', { cwd: __dirname });
      await execAsync('npx playwright install', { cwd: __dirname });
    }

    console.log('✅ Environment setup complete');
  }

  async checkServices() {
    console.log('\n🔍 Checking service health...');

    const serviceChecks = Object.entries(this.config.services).map(async ([name, config]) => {
      const url = `http://localhost:${config.port}${config.healthPath}`;

      try {
        const response = await fetch(url, {
          timeout: 5000,
          signal: AbortSignal.timeout(5000)
        });

        if (response.ok) {
          console.log(`  ✅ ${name} (${url}): healthy`);
          return { name, status: 'healthy', url };
        } else {
          console.log(`  ⚠️ ${name} (${url}): unhealthy (${response.status})`);
          return { name, status: 'unhealthy', url, error: `HTTP ${response.status}` };
        }
      } catch (error) {
        console.log(`  ❌ ${name} (${url}): unavailable (${error.message})`);
        return { name, status: 'unavailable', url, error: error.message };
      }
    });

    const results = await Promise.all(serviceChecks);
    this.testResults.services = results.reduce((acc, result) => {
      acc[result.name] = result;
      return acc;
    }, {});

    const unhealthyServices = results.filter(r => r.status !== 'healthy');

    if (unhealthyServices.length > 0) {
      console.log('\n⚠️ Warning: Some services are not healthy:');
      unhealthyServices.forEach(service => {
        console.log(`  - ${service.name}: ${service.status} (${service.error})`);
      });

      if (process.env.REQUIRE_ALL_SERVICES === 'true') {
        throw new Error('All services must be healthy to proceed');
      } else {
        console.log('🤖 Continuing with available services (some tests may fail)...');
      }
    }
  }

  async runTestCycles() {
    console.log('\n🔄 Starting multiple test cycles...');

    const testSuites = [
      {
        name: 'Critical Path Tests',
        command: 'npx playwright test tests/critical --reporter=json',
        runs: Math.min(this.config.stabilityRuns, 3),
        timeout: this.config.timeoutPerRun
      },
      {
        name: 'Stability Tests',
        command: 'npx playwright test tests/stability --reporter=json',
        runs: this.config.stabilityRuns,
        timeout: this.config.timeoutPerRun * 2
      },
      {
        name: 'Regression Tests',
        command: 'npx playwright test tests/regression --reporter=json',
        runs: this.config.regressionRuns,
        timeout: this.config.timeoutPerRun
      },
      {
        name: 'Load Tests',
        command: 'npx playwright test tests/load --reporter=json',
        runs: this.config.loadTestRuns,
        timeout: this.config.timeoutPerRun * 3
      }
    ];

    for (const suite of testSuites) {
      console.log(`\n📝 Running ${suite.name} (${suite.runs} cycles)...`);
      await this.runTestSuite(suite);
    }
  }

  async runTestSuite(suite) {
    const suiteResults = {
      name: suite.name,
      runs: [],
      summary: { passed: 0, failed: 0, total: 0 }
    };

    for (let runNumber = 1; runNumber <= suite.runs; runNumber++) {
      console.log(`\n  🏃‍♂️ ${suite.name} - Run ${runNumber}/${suite.runs}`);

      const runStart = Date.now();
      const outputFile = path.join(
        this.config.reportDir,
        'logs',
        `${suite.name.toLowerCase().replace(/\s+/g, '-')}-run-${runNumber}-${this.runId}.json`
      );

      try {
        // Add output file to command
        const command = `${suite.command} --output-dir="${path.join(this.config.reportDir, 'artifacts', `run-${runNumber}`)}"`;

        const result = await this.executeTestCommand(command, suite.timeout);
        const duration = Date.now() - runStart;

        const runResult = {
          runNumber,
          status: result.exitCode === 0 ? 'passed' : 'failed',
          duration,
          exitCode: result.exitCode,
          stdout: result.stdout,
          stderr: result.stderr,
          timestamp: new Date().toISOString()
        };

        suiteResults.runs.push(runResult);

        if (runResult.status === 'passed') {
          console.log(`    ✅ Run ${runNumber}: PASSED (${Math.round(duration/1000)}s)`);
          suiteResults.summary.passed++;
        } else {
          console.log(`    ❌ Run ${runNumber}: FAILED (${Math.round(duration/1000)}s)`);
          console.log(`       Exit code: ${result.exitCode}`);
          if (result.stderr) {
            console.log(`       Error: ${result.stderr.substring(0, 200)}...`);
          }
          suiteResults.summary.failed++;
        }

        // Save individual run result
        await fs.writeJson(outputFile, runResult, { spaces: 2 });

      } catch (error) {
        console.log(`    💥 Run ${runNumber}: ERROR (${error.message})`);

        const runResult = {
          runNumber,
          status: 'error',
          duration: Date.now() - runStart,
          error: error.message,
          timestamp: new Date().toISOString()
        };

        suiteResults.runs.push(runResult);
        suiteResults.summary.failed++;

        await fs.writeJson(outputFile, runResult, { spaces: 2 });
      }

      // Cooldown between runs
      if (runNumber < suite.runs) {
        console.log(`    ⏳ Cooldown ${this.config.cooldownBetweenRuns}ms...`);
        await this.sleep(this.config.cooldownBetweenRuns);
      }
    }

    suiteResults.summary.total = suiteResults.runs.length;
    suiteResults.summary.successRate = suiteResults.summary.total > 0
      ? (suiteResults.summary.passed / suiteResults.summary.total * 100).toFixed(2)
      : 0;

    console.log(`\n  📊 ${suite.name} Summary:`);
    console.log(`     Success Rate: ${suiteResults.summary.successRate}% (${suiteResults.summary.passed}/${suiteResults.summary.total})`);
    console.log(`     Average Duration: ${Math.round(suiteResults.runs.reduce((sum, r) => sum + r.duration, 0) / suiteResults.runs.length / 1000)}s`);

    // Detect flaky tests
    const flakyThreshold = 0.7; // 70% success rate threshold
    if (suiteResults.summary.successRate < (flakyThreshold * 100) && suiteResults.summary.passed > 0) {
      console.log(`     ⚠️ FLAKY: Success rate below ${flakyThreshold * 100}%`);
      this.testResults.summary.flaky++;
    }

    this.testResults.cycles.push(suiteResults);
    this.testResults.summary.totalCycles++;
    this.testResults.summary.passed += suiteResults.summary.passed;
    this.testResults.summary.failed += suiteResults.summary.failed;
    this.testResults.summary.totalTests += suiteResults.summary.total;
  }

  async executeTestCommand(command, timeout) {
    return new Promise((resolve, reject) => {
      const child = spawn('bash', ['-c', command], {
        cwd: __dirname,
        env: { ...process.env, NODE_ENV: 'test' },
        stdio: ['pipe', 'pipe', 'pipe']
      });

      let stdout = '';
      let stderr = '';

      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      const timeoutId = setTimeout(() => {
        child.kill('SIGTERM');
        reject(new Error(`Test command timed out after ${timeout}ms`));
      }, timeout);

      child.on('close', (exitCode) => {
        clearTimeout(timeoutId);
        resolve({ exitCode, stdout, stderr });
      });

      child.on('error', (error) => {
        clearTimeout(timeoutId);
        reject(error);
      });
    });
  }

  async generateReport() {
    console.log('\n📊 Generating comprehensive report...');

    this.testResults.metadata.endTime = new Date().toISOString();
    this.testResults.metadata.duration = Date.now() - new Date(this.testResults.metadata.startTime).getTime();

    // Calculate final statistics
    this.testResults.summary.successRate = this.testResults.summary.totalTests > 0
      ? (this.testResults.summary.passed / this.testResults.summary.totalTests * 100).toFixed(2)
      : 0;

    this.testResults.summary.duration = this.testResults.metadata.duration;

    // Stability analysis
    this.testResults.stability = this.analyzeStability();

    // Save detailed JSON report
    const reportFile = path.join(this.config.reportDir, `multiple-e2e-report-${this.runId}.json`);
    await fs.writeJson(reportFile, this.testResults, { spaces: 2 });

    // Generate HTML summary
    await this.generateHtmlSummary();

    // Generate CSV for spreadsheet analysis
    await this.generateCsvReport();

    console.log(`\n📄 Reports generated:`);
    console.log(`  📋 JSON Report: ${reportFile}`);
    console.log(`  🌐 HTML Summary: ${path.join(this.config.reportDir, 'summary.html')}`);
    console.log(`  📊 CSV Data: ${path.join(this.config.reportDir, 'results.csv')}`);

    // Print summary to console
    this.printFinalSummary();
  }

  analyzeStability() {
    const stability = {
      overallStability: 0,
      suiteStability: {},
      flakyTests: [],
      consistentTests: [],
      recommendations: []
    };

    // Analyze each test suite
    this.testResults.cycles.forEach(cycle => {
      const successRate = parseFloat(cycle.summary.successRate);

      stability.suiteStability[cycle.name] = {
        successRate,
        totalRuns: cycle.summary.total,
        passedRuns: cycle.summary.passed,
        failedRuns: cycle.summary.failed,
        isStable: successRate >= 80,
        isFlaky: successRate > 0 && successRate < 80
      };

      if (successRate < 80 && successRate > 0) {
        stability.flakyTests.push({
          suite: cycle.name,
          successRate,
          evidence: 'Intermittent failures detected'
        });
      } else if (successRate >= 95) {
        stability.consistentTests.push({
          suite: cycle.name,
          successRate,
          note: 'Highly consistent performance'
        });
      }
    });

    // Calculate overall stability
    const allSuccessRates = Object.values(stability.suiteStability).map(s => s.successRate);
    stability.overallStability = allSuccessRates.length > 0
      ? (allSuccessRates.reduce((sum, rate) => sum + rate, 0) / allSuccessRates.length).toFixed(2)
      : 0;

    // Generate recommendations
    if (stability.overallStability < 90) {
      stability.recommendations.push('Overall stability is below 90%. Consider increasing test timeouts or improving test reliability.');
    }

    if (stability.flakyTests.length > 0) {
      stability.recommendations.push(`${stability.flakyTests.length} test suite(s) show flaky behavior. Investigate and fix intermittent failures.`);
    }

    if (parseFloat(this.testResults.summary.successRate) < 95) {
      stability.recommendations.push('Success rate is below 95%. Review failing tests and improve error handling.');
    }

    return stability;
  }

  async generateHtmlSummary() {
    const html = `
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AI-CORE Multiple E2E Test Report</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { background: linear-gradient(135deg, #667eea, #764ba2); color: white; padding: 30px; border-radius: 10px; margin-bottom: 30px; }
        .summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; margin-bottom: 30px; }
        .card { background: white; padding: 20px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .metric { font-size: 2em; font-weight: bold; margin-bottom: 5px; }
        .metric.success { color: #27ae60; }
        .metric.warning { color: #f39c12; }
        .metric.error { color: #e74c3c; }
        .suite-results { background: white; margin-bottom: 20px; border-radius: 10px; overflow: hidden; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .suite-header { background: #667eea; color: white; padding: 15px 20px; font-weight: bold; }
        .suite-content { padding: 20px; }
        .run-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(80px, 1fr)); gap: 10px; margin: 15px 0; }
        .run-badge { padding: 10px; text-align: center; border-radius: 5px; font-size: 0.9em; }
        .run-passed { background: #d4edda; color: #155724; }
        .run-failed { background: #f8d7da; color: #721c24; }
        .run-error { background: #f5c6cb; color: #721c24; }
        .recommendations { background: #fff3cd; border: 1px solid #ffeaa7; border-radius: 8px; padding: 20px; margin: 20px 0; }
        .timestamp { color: #666; font-size: 0.9em; text-align: right; margin-top: 20px; }
        table { width: 100%; border-collapse: collapse; margin-top: 15px; }
        th, td { padding: 10px; text-align: left; border-bottom: 1px solid #eee; }
        th { background: #f8f9fa; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 AI-CORE Multiple E2E Test Report</h1>
            <p>Run ID: ${this.runId}</p>
            <p>Generated: ${new Date().toLocaleString()}</p>
        </div>

        <div class="summary-grid">
            <div class="card">
                <h3>Total Tests</h3>
                <div class="metric">${this.testResults.summary.totalTests}</div>
                <div>Across ${this.testResults.summary.totalCycles} test suites</div>
            </div>
            <div class="card">
                <h3>Success Rate</h3>
                <div class="metric ${parseFloat(this.testResults.summary.successRate) >= 95 ? 'success' : parseFloat(this.testResults.summary.successRate) >= 80 ? 'warning' : 'error'}">${this.testResults.summary.successRate}%</div>
                <div>${this.testResults.summary.passed}/${this.testResults.summary.totalTests} passed</div>
            </div>
            <div class="card">
                <h3>Stability Score</h3>
                <div class="metric ${parseFloat(this.testResults.stability.overallStability) >= 90 ? 'success' : parseFloat(this.testResults.stability.overallStability) >= 75 ? 'warning' : 'error'}">${this.testResults.stability.overallStability}%</div>
                <div>Cross-run consistency</div>
            </div>
            <div class="card">
                <h3>Duration</h3>
                <div class="metric">${Math.round(this.testResults.summary.duration / 60000)}m</div>
                <div>Total execution time</div>
            </div>
        </div>

        ${this.testResults.cycles.map(cycle => `
        <div class="suite-results">
            <div class="suite-header">${cycle.name}</div>
            <div class="suite-content">
                <p><strong>Success Rate:</strong> ${cycle.summary.successRate}% (${cycle.summary.passed}/${cycle.summary.total})</p>
                <div class="run-grid">
                    ${cycle.runs.map(run => `
                        <div class="run-badge run-${run.status}">
                            Run ${run.runNumber}<br>
                            ${Math.round(run.duration/1000)}s
                        </div>
                    `).join('')}
                </div>
            </div>
        </div>
        `).join('')}

        ${this.testResults.stability.recommendations.length > 0 ? `
        <div class="recommendations">
            <h3>📋 Recommendations</h3>
            <ul>
                ${this.testResults.stability.recommendations.map(rec => `<li>${rec}</li>`).join('')}
            </ul>
        </div>
        ` : ''}

        <div class="card">
            <h3>🔍 Service Health Status</h3>
            <table>
                <thead>
                    <tr><th>Service</th><th>Status</th><th>URL</th></tr>
                </thead>
                <tbody>
                    ${Object.entries(this.testResults.services).map(([name, service]) => `
                        <tr>
                            <td>${name}</td>
                            <td><span class="run-badge run-${service.status === 'healthy' ? 'passed' : 'failed'}">${service.status}</span></td>
                            <td>${service.url}</td>
                        </tr>
                    `).join('')}
                </tbody>
            </table>
        </div>

        <div class="timestamp">
            Report generated: ${new Date().toISOString()}<br>
            Duration: ${Math.round(this.testResults.metadata.duration / 60000)} minutes
        </div>
    </div>
</body>
</html>`;

    await fs.writeFile(path.join(this.config.reportDir, 'summary.html'), html);
  }

  async generateCsvReport() {
    const csvLines = ['Suite,Run,Status,Duration(ms),Timestamp'];

    this.testResults.cycles.forEach(cycle => {
      cycle.runs.forEach(run => {
        csvLines.push(`"${cycle.name}",${run.runNumber},"${run.status}",${run.duration},"${run.timestamp}"`);
      });
    });

    await fs.writeFile(path.join(this.config.reportDir, 'results.csv'), csvLines.join('\n'));
  }

  printFinalSummary() {
    console.log('\n📊 FINAL SUMMARY');
    console.log('================');
    console.log(`🎯 Overall Success Rate: ${this.testResults.summary.successRate}%`);
    console.log(`🔄 Total Test Runs: ${this.testResults.summary.totalTests}`);
    console.log(`✅ Passed: ${this.testResults.summary.passed}`);
    console.log(`❌ Failed: ${this.testResults.summary.failed}`);
    console.log(`⚡ Flaky Suites: ${this.testResults.summary.flaky}`);
    console.log(`📈 Stability Score: ${this.testResults.stability.overallStability}%`);
    console.log(`⏱️  Total Duration: ${Math.round(this.testResults.summary.duration / 60000)} minutes`);

    if (this.testResults.stability.recommendations.length > 0) {
      console.log('\n💡 Key Recommendations:');
      this.testResults.stability.recommendations.forEach((rec, i) => {
        console.log(`  ${i + 1}. ${rec}`);
      });
    }

    // Quality gates
    const qualityGates = {
      successRate: parseFloat(this.testResults.summary.successRate) >= 90,
      stabilityScore: parseFloat(this.testResults.stability.overallStability) >= 80,
      noFlaky: this.testResults.summary.flaky === 0
    };

    const passedGates = Object.values(qualityGates).filter(Boolean).length;
    const totalGates = Object.keys(qualityGates).length;

    console.log(`\n🚦 Quality Gates: ${passedGates}/${totalGates} PASSED`);
    console.log(`   Success Rate ≥90%: ${qualityGates.successRate ? '✅' : '❌'}`);
    console.log(`   Stability Score ≥80%: ${qualityGates.stabilityScore ? '✅' : '❌'}`);
    console.log(`   No Flaky Tests: ${qualityGates.noFlaky ? '✅' : '❌'}`);

    if (passedGates === totalGates) {
      console.log('\n🎉 ALL QUALITY GATES PASSED - READY FOR PRODUCTION!');
    } else {
      console.log('\n⚠️ Some quality gates failed - review recommendations above');
    }
  }

  async generateErrorReport(error) {
    const errorReport = {
      timestamp: new Date().toISOString(),
      runId: this.runId,
      error: {
        message: error.message,
        stack: error.stack
      },
      partialResults: this.testResults,
      environment: {
        node: process.version,
        platform: process.platform,
        cwd: process.cwd()
      }
    };

    await fs.writeJson(
      path.join(this.config.reportDir, `error-report-${this.runId}.json`),
      errorReport,
      { spaces: 2 }
    );
  }

  async cleanup() {
    if (!this.config.keepArtifacts) {
      console.log('\n🧹 Cleaning up temporary artifacts...');

      // Keep only essential files, remove large artifacts
      const artifactsDir = path.join(this.config.reportDir, 'artifacts');
      if (await fs.pathExists(artifactsDir)) {
        const files = await fs.readdir(artifactsDir, { recursive: true });
        let cleanedSize = 0;

        for (const file of files) {
          const filePath = path.join(artifactsDir, file);
          const stats = await fs.stat(filePath).catch(() => null);

          if (stats && stats.size > 1024 * 1024) { // Files > 1MB
            await fs.remove(filePath);
            cleanedSize += stats.size;
          }
        }

        if (cleanedSize > 0) {
          console.log(`   Cleaned ${Math.round(cleanedSize / 1024 / 1024)}MB of artifacts`);
        }
      }
    }
  }

  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// CLI execution
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const orchestrator = new MultipleE2EOrchestrator();
  orchestrator.run();
}

export default MultipleE2EOrchestrator;
