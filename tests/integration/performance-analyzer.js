#!/usr/bin/env node

/**
 * AI-CORE Performance Analyzer
 * Real-time performance monitoring, analytics, and reporting system
 *
 * Provides comprehensive performance metrics, bottleneck analysis,
 * and production readiness validation for all MCP services
 */

const http = require('http');
const fs = require('fs');
const path = require('path');
const { EventEmitter } = require('events');

class PerformanceAnalyzer extends EventEmitter {
    constructor() {
        super();

        this.services = {
            'demo-content-mcp': { port: 8804, name: 'Content Generation' },
            'text-processing-mcp': { port: 8805, name: 'Text Processing' },
            'image-generation-internal-mcp': { port: 8806, name: 'Image Generation (Internal)' },
            'mcp-orchestrator': { port: 8807, name: 'Workflow Orchestrator' },
            'image-generation-external-mcp': { port: 8091, name: 'Image Generation (External)' },
            'calendar-management-mcp': { port: 8092, name: 'Calendar Management' },
            'facebook-posting-mcp': { port: 8093, name: 'Facebook Posting' }
        };

        this.metrics = {
            services: {},
            global: {
                totalRequests: 0,
                totalErrors: 0,
                averageResponseTime: 0,
                requestsPerSecond: 0,
                startTime: Date.now(),
                lastUpdate: Date.now()
            },
            performance: {
                responseTimes: [],
                throughput: [],
                errorRates: [],
                memoryUsage: [],
                cpuUsage: []
            }
        };

        this.thresholds = {
            responseTime: {
                excellent: 100,    // < 100ms
                good: 500,         // < 500ms
                acceptable: 2000,  // < 2s
                poor: 5000         // < 5s
            },
            errorRate: {
                excellent: 0.1,    // < 0.1%
                good: 1.0,         // < 1%
                acceptable: 5.0,   // < 5%
                poor: 10.0         // < 10%
            },
            throughput: {
                minimum: 10,       // 10 requests/second
                good: 50,          // 50 requests/second
                excellent: 100     // 100+ requests/second
            }
        };

        this.monitoring = {
            interval: null,
            isRunning: false,
            updateFrequency: 5000 // 5 seconds
        };

        this.reports = {
            directory: path.join(__dirname, 'reports'),
            latestReport: null,
            historicalData: []
        };

        // Initialize service metrics
        Object.keys(this.services).forEach(serviceId => {
            this.metrics.services[serviceId] = {
                name: this.services[serviceId].name,
                port: this.services[serviceId].port,
                requests: 0,
                errors: 0,
                totalResponseTime: 0,
                averageResponseTime: 0,
                minResponseTime: Infinity,
                maxResponseTime: 0,
                lastResponseTime: 0,
                status: 'unknown',
                uptime: 0,
                lastHealthCheck: null,
                errorRate: 0,
                throughput: 0,
                performance: {
                    p50: 0,
                    p95: 0,
                    p99: 0
                },
                responseTimes: []
            };
        });

        this.ensureReportsDirectory();
        this.logFile = path.join(__dirname, 'logs', 'performance.log');
    }

    ensureReportsDirectory() {
        if (!fs.existsSync(this.reports.directory)) {
            fs.mkdirSync(this.reports.directory, { recursive: true });
        }

        const logsDir = path.join(__dirname, 'logs');
        if (!fs.existsSync(logsDir)) {
            fs.mkdirSync(logsDir, { recursive: true });
        }
    }

    log(message, level = 'INFO') {
        const timestamp = new Date().toISOString();
        const logMessage = `[${timestamp}] [${level}] ${message}\n`;

        console.log(logMessage.trim());
        fs.appendFileSync(this.logFile, logMessage);
    }

    async checkServiceHealth(serviceId) {
        const service = this.services[serviceId];
        const serviceMetrics = this.metrics.services[serviceId];

        return new Promise((resolve) => {
            const startTime = Date.now();
            const options = {
                hostname: 'localhost',
                port: service.port,
                path: '/health',
                method: 'GET',
                timeout: 5000
            };

            const req = http.request(options, (res) => {
                const responseTime = Date.now() - startTime;
                let data = '';

                res.on('data', chunk => data += chunk);
                res.on('end', () => {
                    const isHealthy = res.statusCode === 200;

                    serviceMetrics.lastResponseTime = responseTime;
                    serviceMetrics.lastHealthCheck = Date.now();
                    serviceMetrics.status = isHealthy ? 'healthy' : 'unhealthy';

                    if (isHealthy) {
                        this.updateServiceMetrics(serviceId, responseTime, true);
                    } else {
                        this.updateServiceMetrics(serviceId, responseTime, false);
                    }

                    resolve({
                        serviceId,
                        healthy: isHealthy,
                        responseTime,
                        statusCode: res.statusCode,
                        data: data
                    });
                });
            });

            req.on('error', (error) => {
                const responseTime = Date.now() - startTime;
                serviceMetrics.status = 'error';
                serviceMetrics.lastHealthCheck = Date.now();
                this.updateServiceMetrics(serviceId, responseTime, false);

                resolve({
                    serviceId,
                    healthy: false,
                    responseTime,
                    error: error.message
                });
            });

            req.on('timeout', () => {
                req.destroy();
                const responseTime = Date.now() - startTime;
                serviceMetrics.status = 'timeout';
                this.updateServiceMetrics(serviceId, responseTime, false);

                resolve({
                    serviceId,
                    healthy: false,
                    responseTime,
                    error: 'Timeout'
                });
            });

            req.end();
        });
    }

    updateServiceMetrics(serviceId, responseTime, success) {
        const serviceMetrics = this.metrics.services[serviceId];

        // Update request counts
        serviceMetrics.requests++;
        this.metrics.global.totalRequests++;

        if (!success) {
            serviceMetrics.errors++;
            this.metrics.global.totalErrors++;
        }

        // Update response times
        serviceMetrics.totalResponseTime += responseTime;
        serviceMetrics.averageResponseTime = serviceMetrics.totalResponseTime / serviceMetrics.requests;
        serviceMetrics.minResponseTime = Math.min(serviceMetrics.minResponseTime, responseTime);
        serviceMetrics.maxResponseTime = Math.max(serviceMetrics.maxResponseTime, responseTime);
        serviceMetrics.lastResponseTime = responseTime;

        // Store response time for percentile calculations
        serviceMetrics.responseTimes.push(responseTime);
        if (serviceMetrics.responseTimes.length > 1000) {
            serviceMetrics.responseTimes = serviceMetrics.responseTimes.slice(-1000);
        }

        // Calculate percentiles
        this.calculatePercentiles(serviceId);

        // Update error rate
        serviceMetrics.errorRate = (serviceMetrics.errors / serviceMetrics.requests) * 100;

        // Update global metrics
        this.updateGlobalMetrics();
    }

    calculatePercentiles(serviceId) {
        const serviceMetrics = this.metrics.services[serviceId];
        const times = [...serviceMetrics.responseTimes].sort((a, b) => a - b);

        if (times.length === 0) return;

        serviceMetrics.performance.p50 = times[Math.floor(times.length * 0.5)];
        serviceMetrics.performance.p95 = times[Math.floor(times.length * 0.95)];
        serviceMetrics.performance.p99 = times[Math.floor(times.length * 0.99)];
    }

    updateGlobalMetrics() {
        const now = Date.now();
        const uptimeSeconds = (now - this.metrics.global.startTime) / 1000;

        // Calculate global average response time
        let totalResponseTime = 0;
        let totalRequests = 0;

        Object.values(this.metrics.services).forEach(service => {
            totalResponseTime += service.totalResponseTime;
            totalRequests += service.requests;
        });

        this.metrics.global.averageResponseTime = totalRequests > 0 ? totalResponseTime / totalRequests : 0;
        this.metrics.global.requestsPerSecond = totalRequests / uptimeSeconds;
        this.metrics.global.lastUpdate = now;
    }

    async performLoadTest(options = {}) {
        const {
            duration = 30000,           // 30 seconds
            concurrentRequests = 10,    // 10 concurrent requests
            requestInterval = 1000,     // 1 second between request waves
            targetServices = Object.keys(this.services)
        } = options;

        this.log(`🚀 Starting load test - Duration: ${duration}ms, Concurrent: ${concurrentRequests}`);

        const startTime = Date.now();
        const endTime = startTime + duration;
        const loadTestResults = {
            startTime,
            endTime,
            duration,
            concurrentRequests,
            totalRequests: 0,
            successfulRequests: 0,
            failedRequests: 0,
            averageResponseTime: 0,
            maxResponseTime: 0,
            minResponseTime: Infinity,
            requestsPerSecond: 0,
            services: {}
        };

        // Initialize service results
        targetServices.forEach(serviceId => {
            loadTestResults.services[serviceId] = {
                requests: 0,
                successes: 0,
                failures: 0,
                totalResponseTime: 0,
                averageResponseTime: 0,
                responseTimes: []
            };
        });

        // Load testing loop
        while (Date.now() < endTime) {
            const promises = [];

            // Create concurrent requests to all target services
            for (let i = 0; i < concurrentRequests; i++) {
                targetServices.forEach(serviceId => {
                    promises.push(this.performLoadTestRequest(serviceId, loadTestResults));
                });
            }

            await Promise.all(promises);
            await new Promise(resolve => setTimeout(resolve, requestInterval));
        }

        // Calculate final statistics
        this.calculateLoadTestStats(loadTestResults);

        this.log(`✅ Load test completed - ${loadTestResults.totalRequests} requests, ${loadTestResults.requestsPerSecond.toFixed(2)} req/s`);

        return loadTestResults;
    }

    async performLoadTestRequest(serviceId, results) {
        const startTime = Date.now();

        try {
            const healthResult = await this.checkServiceHealth(serviceId);
            const responseTime = Date.now() - startTime;

            results.totalRequests++;
            results.services[serviceId].requests++;
            results.services[serviceId].totalResponseTime += responseTime;
            results.services[serviceId].responseTimes.push(responseTime);

            if (healthResult.healthy) {
                results.successfulRequests++;
                results.services[serviceId].successes++;
            } else {
                results.failedRequests++;
                results.services[serviceId].failures++;
            }

            // Update min/max response times
            results.maxResponseTime = Math.max(results.maxResponseTime, responseTime);
            results.minResponseTime = Math.min(results.minResponseTime, responseTime);

        } catch (error) {
            results.totalRequests++;
            results.failedRequests++;
            results.services[serviceId].requests++;
            results.services[serviceId].failures++;
        }
    }

    calculateLoadTestStats(results) {
        const durationSeconds = results.duration / 1000;
        results.requestsPerSecond = results.totalRequests / durationSeconds;

        let totalResponseTime = 0;
        Object.values(results.services).forEach(service => {
            if (service.requests > 0) {
                service.averageResponseTime = service.totalResponseTime / service.requests;
                totalResponseTime += service.totalResponseTime;
            }
        });

        results.averageResponseTime = results.totalRequests > 0 ? totalResponseTime / results.totalRequests : 0;
    }

    async startMonitoring() {
        if (this.monitoring.isRunning) {
            this.log('⚠️ Monitoring is already running');
            return;
        }

        this.log('📊 Starting performance monitoring...');
        this.monitoring.isRunning = true;

        this.monitoring.interval = setInterval(async () => {
            await this.collectMetrics();
            this.emit('metrics-updated', this.getMetricsSummary());
        }, this.monitoring.updateFrequency);

        this.log(`✅ Performance monitoring started (${this.monitoring.updateFrequency}ms interval)`);
    }

    async stopMonitoring() {
        if (!this.monitoring.isRunning) {
            this.log('⚠️ Monitoring is not running');
            return;
        }

        if (this.monitoring.interval) {
            clearInterval(this.monitoring.interval);
            this.monitoring.interval = null;
        }

        this.monitoring.isRunning = false;
        this.log('🛑 Performance monitoring stopped');
    }

    async collectMetrics() {
        const promises = Object.keys(this.services).map(serviceId =>
            this.checkServiceHealth(serviceId)
        );

        try {
            const results = await Promise.all(promises);

            // Update service uptime and throughput
            Object.keys(this.services).forEach(serviceId => {
                const serviceMetrics = this.metrics.services[serviceId];
                const now = Date.now();

                if (serviceMetrics.status === 'healthy') {
                    serviceMetrics.uptime += this.monitoring.updateFrequency;
                }

                // Calculate throughput (requests per second)
                const uptimeSeconds = serviceMetrics.uptime / 1000;
                serviceMetrics.throughput = uptimeSeconds > 0 ? serviceMetrics.requests / uptimeSeconds : 0;
            });

            this.storeMetricsSnapshot();

        } catch (error) {
            this.log(`❌ Error collecting metrics: ${error.message}`, 'ERROR');
        }
    }

    storeMetricsSnapshot() {
        const snapshot = {
            timestamp: Date.now(),
            global: { ...this.metrics.global },
            services: {}
        };

        Object.keys(this.metrics.services).forEach(serviceId => {
            snapshot.services[serviceId] = {
                ...this.metrics.services[serviceId],
                responseTimes: [] // Don't store full response time arrays in snapshots
            };
        });

        this.metrics.performance.responseTimes.push(snapshot);

        // Keep only the last 1000 snapshots
        if (this.metrics.performance.responseTimes.length > 1000) {
            this.metrics.performance.responseTimes = this.metrics.performance.responseTimes.slice(-1000);
        }
    }

    getMetricsSummary() {
        const summary = {
            timestamp: Date.now(),
            global: { ...this.metrics.global },
            services: {},
            healthCheck: {
                healthy: 0,
                unhealthy: 0,
                total: Object.keys(this.services).length
            },
            performance: {
                averageResponseTime: 0,
                bestPerformingService: null,
                worstPerformingService: null,
                overallStatus: 'unknown'
            }
        };

        let totalResponseTime = 0;
        let totalRequests = 0;
        let bestService = null;
        let worstService = null;

        Object.keys(this.services).forEach(serviceId => {
            const service = this.metrics.services[serviceId];
            summary.services[serviceId] = {
                name: service.name,
                port: service.port,
                status: service.status,
                averageResponseTime: service.averageResponseTime,
                errorRate: service.errorRate,
                throughput: service.throughput,
                requests: service.requests,
                performance: service.performance
            };

            // Count healthy/unhealthy services
            if (service.status === 'healthy') {
                summary.healthCheck.healthy++;
            } else {
                summary.healthCheck.unhealthy++;
            }

            // Find best and worst performing services
            if (service.requests > 0) {
                totalResponseTime += service.totalResponseTime;
                totalRequests += service.requests;

                if (!bestService || service.averageResponseTime < bestService.averageResponseTime) {
                    bestService = { id: serviceId, ...service };
                }
                if (!worstService || service.averageResponseTime > worstService.averageResponseTime) {
                    worstService = { id: serviceId, ...service };
                }
            }
        });

        summary.performance.averageResponseTime = totalRequests > 0 ? totalResponseTime / totalRequests : 0;
        summary.performance.bestPerformingService = bestService ? {
            id: bestService.id,
            name: bestService.name,
            responseTime: bestService.averageResponseTime
        } : null;
        summary.performance.worstPerformingService = worstService ? {
            id: worstService.id,
            name: worstService.name,
            responseTime: worstService.averageResponseTime
        } : null;

        // Determine overall status
        if (summary.healthCheck.healthy === summary.healthCheck.total) {
            if (summary.performance.averageResponseTime < this.thresholds.responseTime.excellent) {
                summary.performance.overallStatus = 'excellent';
            } else if (summary.performance.averageResponseTime < this.thresholds.responseTime.good) {
                summary.performance.overallStatus = 'good';
            } else {
                summary.performance.overallStatus = 'acceptable';
            }
        } else if (summary.healthCheck.healthy > summary.healthCheck.total * 0.8) {
            summary.performance.overallStatus = 'degraded';
        } else {
            summary.performance.overallStatus = 'critical';
        }

        return summary;
    }

    async generateReport(options = {}) {
        const {
            includeLoadTest = false,
            loadTestDuration = 30000,
            format = 'json'
        } = options;

        this.log('📋 Generating performance report...');

        const report = {
            metadata: {
                generatedAt: new Date().toISOString(),
                reportVersion: '1.0.0',
                generatedBy: 'AI-CORE Performance Analyzer',
                platform: process.platform,
                nodeVersion: process.version
            },
            summary: this.getMetricsSummary(),
            detailed: {
                services: { ...this.metrics.services },
                thresholds: this.thresholds,
                historical: this.metrics.performance.responseTimes.slice(-100) // Last 100 snapshots
            },
            loadTest: null,
            recommendations: []
        };

        // Run load test if requested
        if (includeLoadTest) {
            this.log('🚀 Running load test for report...');
            report.loadTest = await this.performLoadTest({ duration: loadTestDuration });
        }

        // Generate recommendations
        report.recommendations = this.generateRecommendations(report.summary);

        // Save report
        const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
        const filename = `performance-report-${timestamp}.${format}`;
        const filepath = path.join(this.reports.directory, filename);

        if (format === 'json') {
            fs.writeFileSync(filepath, JSON.stringify(report, null, 2));
        } else if (format === 'txt') {
            fs.writeFileSync(filepath, this.formatTextReport(report));
        }

        this.reports.latestReport = filepath;
        this.log(`📄 Report generated: ${filepath}`);

        return report;
    }

    generateRecommendations(summary) {
        const recommendations = [];

        // Check response times
        Object.entries(summary.services).forEach(([serviceId, service]) => {
            if (service.averageResponseTime > this.thresholds.responseTime.poor) {
                recommendations.push({
                    type: 'performance',
                    severity: 'high',
                    service: serviceId,
                    issue: 'High response time',
                    current: `${service.averageResponseTime.toFixed(2)}ms`,
                    threshold: `${this.thresholds.responseTime.poor}ms`,
                    suggestion: 'Investigate service performance, check for bottlenecks, consider scaling'
                });
            }

            if (service.errorRate > this.thresholds.errorRate.poor) {
                recommendations.push({
                    type: 'reliability',
                    severity: 'high',
                    service: serviceId,
                    issue: 'High error rate',
                    current: `${service.errorRate.toFixed(2)}%`,
                    threshold: `${this.thresholds.errorRate.poor}%`,
                    suggestion: 'Review error logs, fix service issues, implement better error handling'
                });
            }

            if (service.throughput < this.thresholds.throughput.minimum) {
                recommendations.push({
                    type: 'scalability',
                    severity: 'medium',
                    service: serviceId,
                    issue: 'Low throughput',
                    current: `${service.throughput.toFixed(2)} req/s`,
                    threshold: `${this.thresholds.throughput.minimum} req/s`,
                    suggestion: 'Consider optimizing service or adding more instances'
                });
            }
        });

        // Global recommendations
        if (summary.performance.overallStatus === 'critical') {
            recommendations.push({
                type: 'system',
                severity: 'critical',
                issue: 'System-wide performance issues',
                suggestion: 'Immediate attention required - multiple services are unhealthy'
            });
        }

        return recommendations;
    }

    formatTextReport(report) {
        let text = `AI-CORE Performance Analysis Report\n`;
        text += `Generated: ${report.metadata.generatedAt}\n`;
        text += `${'='.repeat(60)}\n\n`;

        // Summary
        text += `SYSTEM OVERVIEW\n`;
        text += `${'-'.repeat(30)}\n`;
        text += `Overall Status: ${report.summary.performance.overallStatus.toUpperCase()}\n`;
        text += `Healthy Services: ${report.summary.healthCheck.healthy}/${report.summary.healthCheck.total}\n`;
        text += `Average Response Time: ${report.summary.performance.averageResponseTime.toFixed(2)}ms\n`;
        text += `Total Requests: ${report.summary.global.totalRequests}\n`;
        text += `Total Errors: ${report.summary.global.totalErrors}\n\n`;

        // Service Details
        text += `SERVICE DETAILS\n`;
        text += `${'-'.repeat(30)}\n`;
        Object.entries(report.summary.services).forEach(([serviceId, service]) => {
            const statusIcon = service.status === 'healthy' ? '🟢' : '🔴';
            text += `${statusIcon} ${service.name} (Port ${service.port})\n`;
            text += `   Status: ${service.status}\n`;
            text += `   Avg Response: ${service.averageResponseTime.toFixed(2)}ms\n`;
            text += `   Error Rate: ${service.errorRate.toFixed(2)}%\n`;
            text += `   Throughput: ${service.throughput.toFixed(2)} req/s\n`;
            text += `   Requests: ${service.requests}\n\n`;
        });

        // Load Test Results
        if (report.loadTest) {
            text += `LOAD TEST RESULTS\n`;
            text += `${'-'.repeat(30)}\n`;
            text += `Duration: ${report.loadTest.duration / 1000}s\n`;
            text += `Total Requests: ${report.loadTest.totalRequests}\n`;
            text += `Success Rate: ${(report.loadTest.successfulRequests / report.loadTest.totalRequests * 100).toFixed(1)}%\n`;
            text += `Requests/Second: ${report.loadTest.requestsPerSecond.toFixed(2)}\n`;
            text += `Avg Response Time: ${report.loadTest.averageResponseTime.toFixed(2)}ms\n\n`;
        }

        // Recommendations
        if (report.recommendations.length > 0) {
            text += `RECOMMENDATIONS\n`;
            text += `${'-'.repeat(30)}\n`;
            report.recommendations.forEach((rec, index) => {
                const severityIcon = rec.severity === 'high' ? '🔴' : rec.severity === 'medium' ? '🟡' : '🟢';
                text += `${severityIcon} ${rec.issue}\n`;
                text += `   Service: ${rec.service || 'System-wide'}\n`;
                text += `   Current: ${rec.current || 'N/A'}\n`;
                text += `   Suggestion: ${rec.suggestion}\n\n`;
            });
        }

        return text;
    }

    async cleanup() {
        await this.stopMonitoring();
        this.log('🧹 Performance analyzer cleanup completed');
    }
}

// CLI Interface
if (require.main === module) {
    const analyzer = new PerformanceAnalyzer();
    const command = process.argv[2] || 'help';

    (async () => {
        try {
            switch (command) {
                case 'monitor':
                    await analyzer.startMonitoring();

                    // Keep monitoring until interrupted
                    process.on('SIGINT', async () => {
                        console.log('\n🛑 Stopping monitoring...');
                        await analyzer.stopMonitoring();
                        process.exit(0);
                    });

                    // Keep process alive
                    setInterval(() => {}, 1000);
                    break;

                case 'report':
                    const includeLoadTest = process.argv.includes('--load-test');
                    const format = process.argv.includes('--format=txt') ? 'txt' : 'json';

                    const report = await analyzer.generateReport({
                        includeLoadTest,
                        format
                    });

                    console.log(`📄 Report generated with ${report.recommendations.length} recommendations`);
                    process.exit(0);
                    break;

                case 'load-test':
                    const duration = parseInt(process.argv[3]) || 30000;
                    const concurrent = parseInt(process.argv[4]) || 10;

                    const results = await analyzer.performLoadTest({
                        duration,
                        concurrentRequests: concurrent
                    });

                    console.log(`📊 Load test completed: ${results.requestsPerSecond.toFixed(2)} req/s`);
                    process.exit(0);
                    break;

                case 'status':
                    const summary = analyzer.getMetricsSummary();
                    console.log(JSON.stringify(summary, null, 2));
                    process.exit(0);
                    break;

                case 'help':
                default:
                    console.log(`AI-CORE Performance Analyzer

Usage:
  node performance-analyzer.js monitor                # Start real-time monitoring
  node performance-analyzer.js report [--load-test]   # Generate performance report
  node performance-analyzer.js load-test [duration] [concurrent] # Run load test
  node performance-analyzer.js status                 # Show current metrics
  node performance-analyzer.js help                   # Show this help

Examples:
  node performance-analyzer.js monitor
  node performance-analyzer.js report --load-test --format=txt
  node performance-analyzer.js load-test 60000 20

Reports: tests/reports/
Logs: tests/logs/performance.log`);
                    process.exit(0);
            }
        } catch (error) {
            console.error(`Fatal error: ${error.message}`);
            process.exit(1);
        }
    })();
}

module.exports = PerformanceAnalyzer;
