#!/usr/bin/env node

/**
 * AI-CORE Analytics Dashboard
 * Professional real-time analytics and business intelligence platform
 *
 * Features:
 * - Real-time performance monitoring
 * - Business KPI tracking
 * - Interactive visualizations
 * - Export capabilities
 * - Alert systems
 */

const express = require('express');
const http = require('http');
const socketIo = require('socket.io');
const path = require('path');
const fs = require('fs').promises;

class AnalyticsDashboard {
    constructor() {
        this.app = express();
        this.server = http.createServer(this.app);
        this.io = socketIo(this.server, {
            cors: {
                origin: "*",
                methods: ["GET", "POST"]
            }
        });

        this.metrics = {
            realTime: {
                activeWorkflows: 0,
                completedWorkflows: 0,
                failedWorkflows: 0,
                averageResponseTime: 0,
                throughputPerMinute: 0,
                errorRate: 0,
                serviceHealth: 100
            },
            historical: {
                hourlyStats: [],
                dailyStats: [],
                weeklyStats: []
            },
            business: {
                totalProcessingTime: 0,
                estimatedCostSavings: 0,
                qualityScore: 0,
                clientSatisfaction: 0,
                roi: 0
            },
            services: {},
            alerts: [],
            performanceTrends: []
        };

        this.setupRoutes();
        this.setupSocketHandlers();
        this.startMetricsCollection();
    }

    setupRoutes() {
        this.app.use(express.json());
        this.app.use(express.static(path.join(__dirname, 'public')));

        // Main dashboard route
        this.app.get('/', (req, res) => {
            res.sendFile(path.join(__dirname, 'dashboard.html'));
        });

        // API Routes
        this.app.get('/api/metrics/current', (req, res) => {
            res.json({
                timestamp: new Date().toISOString(),
                metrics: this.metrics.realTime,
                business: this.metrics.business,
                services: this.metrics.services
            });
        });

        this.app.get('/api/metrics/historical/:period', (req, res) => {
            const { period } = req.params;
            const data = this.metrics.historical[`${period}Stats`] || [];
            res.json({
                period,
                data,
                summary: this.calculatePeriodSummary(data)
            });
        });

        this.app.get('/api/alerts', (req, res) => {
            res.json({
                alerts: this.metrics.alerts,
                summary: {
                    critical: this.metrics.alerts.filter(a => a.severity === 'critical').length,
                    warning: this.metrics.alerts.filter(a => a.severity === 'warning').length,
                    info: this.metrics.alerts.filter(a => a.severity === 'info').length
                }
            });
        });

        this.app.post('/api/metrics/update', (req, res) => {
            const { source, metrics } = req.body;
            this.updateMetrics(source, metrics);
            res.json({ success: true, timestamp: new Date().toISOString() });
        });

        // Export endpoints
        this.app.get('/api/export/:format', async (req, res) => {
            try {
                const { format } = req.params;
                const exportData = await this.exportMetrics(format);

                switch (format.toLowerCase()) {
                    case 'json':
                        res.setHeader('Content-Type', 'application/json');
                        res.setHeader('Content-Disposition', `attachment; filename="analytics-${Date.now()}.json"`);
                        res.send(JSON.stringify(exportData, null, 2));
                        break;
                    case 'csv':
                        res.setHeader('Content-Type', 'text/csv');
                        res.setHeader('Content-Disposition', `attachment; filename="analytics-${Date.now()}.csv"`);
                        res.send(this.convertToCSV(exportData));
                        break;
                    default:
                        res.status(400).json({ error: 'Unsupported format' });
                }
            } catch (error) {
                res.status(500).json({ error: error.message });
            }
        });
    }

    setupSocketHandlers() {
        this.io.on('connection', (socket) => {
            console.log(`Analytics client connected: ${socket.id}`);

            // Send initial data
            socket.emit('initialData', {
                metrics: this.metrics.realTime,
                business: this.metrics.business,
                services: this.metrics.services,
                alerts: this.metrics.alerts
            });

            socket.on('subscribe', (channels) => {
                channels.forEach(channel => {
                    socket.join(channel);
                });
            });

            socket.on('disconnect', () => {
                console.log(`Analytics client disconnected: ${socket.id}`);
            });
        });
    }

    updateMetrics(source, newMetrics) {
        // Update real-time metrics
        Object.assign(this.metrics.realTime, newMetrics.realTime || {});

        // Update service-specific metrics
        if (newMetrics.serviceMetrics) {
            this.metrics.services[source] = {
                ...this.metrics.services[source],
                ...newMetrics.serviceMetrics,
                lastUpdate: new Date().toISOString()
            };
        }

        // Calculate business metrics
        this.calculateBusinessMetrics();

        // Check for alerts
        this.checkAlerts();

        // Broadcast updates
        this.io.emit('metricsUpdate', {
            source,
            timestamp: new Date().toISOString(),
            metrics: this.metrics.realTime,
            business: this.metrics.business,
            services: this.metrics.services
        });
    }

    calculateBusinessMetrics() {
        const { realTime } = this.metrics;

        // Calculate cost savings (assuming $50/hour manual work)
        const manualTimeHours = (realTime.completedWorkflows * 2); // 2 hours per workflow manual
        const automatedTimeHours = (this.metrics.business.totalProcessingTime / 3600); // Convert seconds to hours
        const timeSaved = Math.max(0, manualTimeHours - automatedTimeHours);
        this.metrics.business.estimatedCostSavings = timeSaved * 50;

        // Calculate ROI (simple calculation)
        const platformCost = 1000; // Monthly platform cost
        this.metrics.business.roi = ((this.metrics.business.estimatedCostSavings - platformCost) / platformCost) * 100;

        // Calculate quality score based on error rate
        this.metrics.business.qualityScore = Math.max(0, 100 - (realTime.errorRate * 10));

        // Calculate client satisfaction (mock calculation)
        this.metrics.business.clientSatisfaction = Math.min(100,
            (this.metrics.business.qualityScore * 0.6) +
            (Math.max(0, 100 - realTime.averageResponseTime) * 0.4)
        );
    }

    checkAlerts() {
        const now = new Date().toISOString();
        const { realTime, business } = this.metrics;

        // Clear old alerts (older than 1 hour)
        const oneHourAgo = new Date(Date.now() - 60 * 60 * 1000).toISOString();
        this.metrics.alerts = this.metrics.alerts.filter(alert => alert.timestamp > oneHourAgo);

        // Check for new alerts
        const newAlerts = [];

        if (realTime.errorRate > 10) {
            newAlerts.push({
                id: `error-rate-${Date.now()}`,
                severity: 'critical',
                type: 'performance',
                message: `Error rate is ${realTime.errorRate.toFixed(1)}% (threshold: 10%)`,
                timestamp: now,
                metric: 'errorRate',
                value: realTime.errorRate
            });
        }

        if (realTime.averageResponseTime > 5000) {
            newAlerts.push({
                id: `response-time-${Date.now()}`,
                severity: 'warning',
                type: 'performance',
                message: `Average response time is ${realTime.averageResponseTime}ms (threshold: 5000ms)`,
                timestamp: now,
                metric: 'responseTime',
                value: realTime.averageResponseTime
            });
        }

        if (business.qualityScore < 80) {
            newAlerts.push({
                id: `quality-score-${Date.now()}`,
                severity: 'warning',
                type: 'quality',
                message: `Quality score dropped to ${business.qualityScore.toFixed(1)} (threshold: 80)`,
                timestamp: now,
                metric: 'qualityScore',
                value: business.qualityScore
            });
        }

        // Add new alerts
        this.metrics.alerts.push(...newAlerts);

        // Broadcast alerts if any
        if (newAlerts.length > 0) {
            this.io.emit('newAlerts', newAlerts);
        }
    }

    startMetricsCollection() {
        // Simulate real-time data collection
        setInterval(() => {
            this.collectSystemMetrics();
        }, 5000); // Every 5 seconds

        // Historical data aggregation
        setInterval(() => {
            this.aggregateHistoricalData();
        }, 60000); // Every minute

        console.log('📊 Metrics collection started');
    }

    async collectSystemMetrics() {
        try {
            // In a real implementation, this would collect from actual services
            // For now, we'll simulate realistic metrics based on current state

            const variance = () => (Math.random() - 0.5) * 0.1; // ±5% variance

            this.metrics.realTime.averageResponseTime = Math.max(100,
                this.metrics.realTime.averageResponseTime * (1 + variance())
            );

            this.metrics.realTime.throughputPerMinute = Math.max(0,
                this.metrics.realTime.throughputPerMinute * (1 + variance())
            );

            this.metrics.realTime.serviceHealth = Math.min(100, Math.max(80,
                this.metrics.realTime.serviceHealth * (1 + variance() * 0.5)
            ));

            // Update service metrics
            const services = ['workflow-engine', 'mcp-manager', 'federation-service', 'intent-parser'];
            services.forEach(service => {
                if (!this.metrics.services[service]) {
                    this.metrics.services[service] = {
                        status: 'healthy',
                        responseTime: 200,
                        uptime: 99.9,
                        requestCount: 0,
                        errorCount: 0
                    };
                }

                const serviceMetrics = this.metrics.services[service];
                serviceMetrics.responseTime = Math.max(50, serviceMetrics.responseTime * (1 + variance()));
                serviceMetrics.uptime = Math.min(100, Math.max(95, serviceMetrics.uptime * (1 + variance() * 0.1)));
                serviceMetrics.requestCount += Math.floor(Math.random() * 5);

                if (Math.random() < 0.01) { // 1% chance of error
                    serviceMetrics.errorCount++;
                }

                serviceMetrics.status = serviceMetrics.uptime > 98 ? 'healthy' :
                                      serviceMetrics.uptime > 90 ? 'degraded' : 'unhealthy';
            });

            this.calculateBusinessMetrics();
            this.checkAlerts();

        } catch (error) {
            console.error('Error collecting metrics:', error);
        }
    }

    aggregateHistoricalData() {
        const now = new Date();
        const timestamp = now.toISOString();

        const dataPoint = {
            timestamp,
            metrics: { ...this.metrics.realTime },
            business: { ...this.metrics.business }
        };

        // Add to hourly stats
        this.metrics.historical.hourlyStats.push(dataPoint);

        // Keep only last 24 hours
        const twentyFourHoursAgo = new Date(now.getTime() - 24 * 60 * 60 * 1000);
        this.metrics.historical.hourlyStats = this.metrics.historical.hourlyStats
            .filter(stat => new Date(stat.timestamp) > twentyFourHoursAgo);

        // Aggregate daily stats (every hour)
        if (now.getMinutes() === 0) {
            const hourlyAverage = this.calculateAverage(this.metrics.historical.hourlyStats.slice(-60));
            this.metrics.historical.dailyStats.push({
                timestamp,
                ...hourlyAverage
            });

            // Keep only last 30 days
            const thirtyDaysAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
            this.metrics.historical.dailyStats = this.metrics.historical.dailyStats
                .filter(stat => new Date(stat.timestamp) > thirtyDaysAgo);
        }
    }

    calculateAverage(dataPoints) {
        if (dataPoints.length === 0) return { metrics: {}, business: {} };

        const avg = {
            metrics: {},
            business: {}
        };

        // Calculate averages for metrics
        const metricKeys = Object.keys(dataPoints[0].metrics);
        metricKeys.forEach(key => {
            avg.metrics[key] = dataPoints.reduce((sum, dp) => sum + dp.metrics[key], 0) / dataPoints.length;
        });

        // Calculate averages for business metrics
        const businessKeys = Object.keys(dataPoints[0].business);
        businessKeys.forEach(key => {
            avg.business[key] = dataPoints.reduce((sum, dp) => sum + dp.business[key], 0) / dataPoints.length;
        });

        return avg;
    }

    calculatePeriodSummary(data) {
        if (data.length === 0) return {};

        const latest = data[data.length - 1];
        const earliest = data[0];

        return {
            latest: latest,
            earliest: earliest,
            trend: {
                workflows: latest.metrics.completedWorkflows - earliest.metrics.completedWorkflows,
                errorRate: latest.metrics.errorRate - earliest.metrics.errorRate,
                responseTime: latest.metrics.averageResponseTime - earliest.metrics.averageResponseTime,
                qualityScore: latest.business.qualityScore - earliest.business.qualityScore
            },
            averages: this.calculateAverage(data)
        };
    }

    async exportMetrics(format) {
        const exportData = {
            exportTime: new Date().toISOString(),
            realTimeMetrics: this.metrics.realTime,
            businessMetrics: this.metrics.business,
            serviceMetrics: this.metrics.services,
            historicalData: this.metrics.historical,
            alerts: this.metrics.alerts,
            summary: {
                totalWorkflows: this.metrics.realTime.completedWorkflows + this.metrics.realTime.failedWorkflows,
                successRate: this.metrics.realTime.completedWorkflows /
                    (this.metrics.realTime.completedWorkflows + this.metrics.realTime.failedWorkflows) * 100,
                averageQuality: this.metrics.business.qualityScore,
                estimatedSavings: this.metrics.business.estimatedCostSavings,
                roi: this.metrics.business.roi
            }
        };

        return exportData;
    }

    convertToCSV(data) {
        const headers = [
            'Timestamp', 'Completed Workflows', 'Failed Workflows', 'Error Rate',
            'Avg Response Time', 'Quality Score', 'Cost Savings', 'ROI'
        ];

        const rows = [headers.join(',')];

        // Add current metrics
        rows.push([
            data.exportTime,
            data.realTimeMetrics.completedWorkflows,
            data.realTimeMetrics.failedWorkflows,
            data.realTimeMetrics.errorRate.toFixed(2),
            data.realTimeMetrics.averageResponseTime,
            data.businessMetrics.qualityScore.toFixed(2),
            data.businessMetrics.estimatedCostSavings.toFixed(2),
            data.businessMetrics.roi.toFixed(2)
        ].join(','));

        // Add historical data
        data.historicalData.hourlyStats.forEach(stat => {
            rows.push([
                stat.timestamp,
                stat.metrics.completedWorkflows,
                stat.metrics.failedWorkflows,
                stat.metrics.errorRate.toFixed(2),
                stat.metrics.averageResponseTime,
                stat.business.qualityScore.toFixed(2),
                stat.business.estimatedCostSavings.toFixed(2),
                stat.business.roi.toFixed(2)
            ].join(','));
        });

        return rows.join('\n');
    }

    start(port = 8095) {
        this.server.listen(port, () => {
            console.log(`📊 Analytics Dashboard running on http://localhost:${port}`);
            console.log(`🔗 WebSocket server ready for real-time updates`);
            console.log(`📈 Metrics collection active`);
        });
    }

    // Integration with Hour 5 testing framework
    async integrateWithTestingFramework() {
        try {
            // Connect to the testing framework from Hour 5
            const testResults = await this.loadTestResults();

            if (testResults) {
                this.updateMetrics('testing-framework', {
                    realTime: {
                        completedWorkflows: testResults.successfulTests || 0,
                        failedWorkflows: testResults.failedTests || 0,
                        averageResponseTime: testResults.averageResponseTime || 0,
                        errorRate: testResults.errorRate || 0
                    },
                    serviceMetrics: testResults.serviceMetrics || {}
                });

                console.log('✅ Integrated with Hour 5 testing framework');
            }
        } catch (error) {
            console.log('ℹ️  Testing framework integration will be available when tests run');
        }
    }

    async loadTestResults() {
        try {
            const testResultsPath = path.join(__dirname, '../test-results.json');
            const data = await fs.readFile(testResultsPath, 'utf8');
            return JSON.parse(data);
        } catch (error) {
            return null;
        }
    }
}

// Create and start the dashboard
const dashboard = new AnalyticsDashboard();

// Integrate with testing framework
dashboard.integrateWithTestingFramework();

// Start the server
dashboard.start(8095);

// Export for testing
module.exports = AnalyticsDashboard;
