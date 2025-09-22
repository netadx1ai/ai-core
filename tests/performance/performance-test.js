/**
 * AI-CORE Performance Testing Suite with K6
 * FAANG-Enhanced Testing Infrastructure - DevOps Agent Implementation T7.1
 *
 * Comprehensive performance testing covering:
 * - Load testing (normal expected load)
 * - Stress testing (beyond normal capacity)
 * - Spike testing (sudden load increases)
 * - Endurance testing (extended periods)
 * - API response time validation
 * - Resource utilization monitoring
 * - Performance regression detection
 */

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Rate, Trend, Counter, Gauge } from 'k6/metrics';
import { randomString, randomIntBetween } from 'https://jslib.k6.io/k6-utils/1.2.0/index.js';

// ============================================================================
// Configuration and Environment Variables
// ============================================================================

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8002';
const TEST_TYPE = __ENV.TEST_TYPE || 'load';
const DURATION = __ENV.DURATION || '5m';
const VUS = parseInt(__ENV.VUS || '10');
const API_VERSION = __ENV.API_VERSION || 'v1';

// Performance thresholds based on AI-CORE requirements
const PERFORMANCE_THRESHOLDS = {
  // Response time thresholds (milliseconds)
  auth_login: { p95: 500, p99: 800 },
  user_creation: { p95: 300, p99: 500 },
  data_generation: { p95: 1000, p99: 2000 },
  health_check: { p95: 100, p99: 200 },

  // Success rate thresholds (percentage)
  success_rate_threshold: 99.0,

  // Concurrency limits
  max_concurrent_users: 1000,
  max_requests_per_second: 500
};

// ============================================================================
// Custom Metrics
// ============================================================================

// Response time metrics
const authResponseTime = new Trend('auth_response_time', true);
const userCreationTime = new Trend('user_creation_time', true);
const dataGenerationTime = new Trend('data_generation_time', true);
const healthCheckTime = new Trend('health_check_time', true);

// Success/failure rates
const authSuccessRate = new Rate('auth_success_rate');
const userCreationSuccessRate = new Rate('user_creation_success_rate');
const dataGenerationSuccessRate = new Rate('data_generation_success_rate');

// Business metrics
const testUsersCreated = new Counter('test_users_created');
const testDataGenerated = new Counter('test_data_generated');
const apiErrors = new Counter('api_errors');

// System metrics
const activeConnections = new Gauge('active_connections');
const memoryUsage = new Gauge('memory_usage_mb');
const cpuUsage = new Gauge('cpu_usage_percent');

// ============================================================================
// Test Configuration Profiles
// ============================================================================

const testProfiles = {
  load: {
    stages: [
      { duration: '2m', target: Math.floor(VUS * 0.1) }, // Ramp up to 10%
      { duration: '5m', target: Math.floor(VUS * 0.5) }, // Ramp up to 50%
      { duration: '10m', target: VUS }, // Target load
      { duration: '5m', target: Math.floor(VUS * 0.5) }, // Ramp down to 50%
      { duration: '2m', target: 0 }, // Ramp down to 0
    ],
    thresholds: {
      http_req_duration: ['p(95)<500', 'p(99)<800'],
      http_req_failed: ['rate<0.01'], // <1% failure rate
      auth_response_time: ['p(95)<500'],
      user_creation_time: ['p(95)<300'],
      auth_success_rate: ['rate>0.99'],
    }
  },

  stress: {
    stages: [
      { duration: '2m', target: VUS }, // Ramp up quickly
      { duration: '5m', target: VUS * 2 }, // Beyond normal capacity
      { duration: '10m', target: VUS * 3 }, // Well beyond capacity
      { duration: '5m', target: VUS * 2 }, // Scale back
      { duration: '2m', target: 0 }, // Recovery
    ],
    thresholds: {
      http_req_duration: ['p(95)<1000', 'p(99)<2000'],
      http_req_failed: ['rate<0.05'], // <5% failure rate acceptable under stress
      auth_response_time: ['p(95)<1000'],
    }
  },

  spike: {
    stages: [
      { duration: '1m', target: Math.floor(VUS * 0.1) }, // Low load
      { duration: '30s', target: VUS * 5 }, // Sudden spike
      { duration: '1m', target: Math.floor(VUS * 0.1) }, // Back to low
      { duration: '30s', target: VUS * 5 }, // Another spike
      { duration: '1m', target: 0 }, // Recovery
    ],
    thresholds: {
      http_req_duration: ['p(95)<2000'],
      http_req_failed: ['rate<0.1'], // <10% failure rate during spikes
    }
  },

  endurance: {
    stages: [
      { duration: '5m', target: VUS }, // Ramp up
      { duration: '30m', target: VUS }, // Steady load for extended period
      { duration: '5m', target: 0 }, // Ramp down
    ],
    thresholds: {
      http_req_duration: ['p(95)<600', 'p(99)<1000'],
      http_req_failed: ['rate<0.02'], // <2% failure rate
      auth_success_rate: ['rate>0.98'],
    }
  }
};

// ============================================================================
// Test Options Configuration
// ============================================================================

export let options = {
  ...testProfiles[TEST_TYPE],
  ext: {
    loadimpact: {
      projectID: 3569253,
      name: `AI-CORE ${TEST_TYPE.toUpperCase()} Test - ${new Date().toISOString()}`
    }
  },
  summaryTrendStats: ['min', 'med', 'avg', 'p(90)', 'p(95)', 'p(99)', 'max'],
  summaryTimeUnit: 'ms',
};

// ============================================================================
// Test Data Generation
// ============================================================================

function generateTestUser() {
  const roles = ['admin', 'manager', 'developer', 'tester', 'user', 'viewer'];
  const environments = ['testing', 'staging', 'integration'];

  return {
    username: `perf_user_${randomString(8)}`,
    email: `perf${randomIntBetween(1000, 9999)}@test.aicore.dev`,
    password: `TestPass${randomIntBetween(100, 999)}!`,
    firstName: `TestUser${randomIntBetween(1, 1000)}`,
    lastName: `LastName${randomIntBetween(1, 1000)}`,
    role: roles[randomIntBetween(0, roles.length - 1)],
    testEnvironment: environments[randomIntBetween(0, environments.length - 1)],
    expiresHours: randomIntBetween(1, 24)
  };
}

function generateWorkflowData() {
  const categories = ['authentication', 'data-entry', 'approval', 'reporting'];
  const complexities = ['simple', 'moderate', 'complex'];

  return {
    name: `perf_workflow_${randomString(10)}`,
    category: categories[randomIntBetween(0, categories.length - 1)],
    complexity: complexities[randomIntBetween(0, complexities.length - 1)],
    steps: randomIntBetween(3, 15),
    estimatedDuration: randomIntBetween(30, 300)
  };
}

// ============================================================================
// Authentication Helper Functions
// ============================================================================

let authToken = null;

function authenticate() {
  const loginData = {
    username: 'perf_test_admin',
    password: 'PerformanceTest123!'
  };

  const response = http.post(`${BASE_URL}/api/${API_VERSION}/auth/login`,
    JSON.stringify(loginData), {
    headers: {
      'Content-Type': 'application/json',
      'User-Agent': 'K6-Performance-Test/1.0'
    },
    tags: { name: 'auth_login' }
  });

  const success = check(response, {
    'auth login status is 200': (r) => r.status === 200,
    'auth login has token': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.token && body.token.length > 0;
      } catch {
        return false;
      }
    }
  });

  if (success && response.status === 200) {
    try {
      const body = JSON.parse(response.body);
      authToken = body.token;
    } catch (e) {
      console.error('Failed to parse auth response:', e);
    }
  }

  authSuccessRate.add(success);
  authResponseTime.add(response.timings.duration);

  return authToken;
}

function getAuthHeaders() {
  return {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${authToken}`,
    'User-Agent': 'K6-Performance-Test/1.0'
  };
}

// ============================================================================
// Test Scenario Functions
// ============================================================================

function testHealthCheck() {
  const response = http.get(`${BASE_URL}/api/${API_VERSION}/health`, {
    headers: { 'User-Agent': 'K6-Performance-Test/1.0' },
    tags: { name: 'health_check' }
  });

  const success = check(response, {
    'health check status is 200': (r) => r.status === 200,
    'health check response time < 200ms': (r) => r.timings.duration < 200,
    'health check has status': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.status === 'healthy';
      } catch {
        return false;
      }
    }
  });

  if (!success) {
    apiErrors.add(1);
  }

  healthCheckTime.add(response.timings.duration);
  return success;
}

function testUserCreation() {
  if (!authToken) {
    console.error('No auth token available for user creation');
    return false;
  }

  const userData = generateTestUser();
  const response = http.post(`${BASE_URL}/api/${API_VERSION}/test-users`,
    JSON.stringify(userData), {
    headers: getAuthHeaders(),
    tags: { name: 'user_creation' }
  });

  const success = check(response, {
    'user creation status is 201': (r) => r.status === 201,
    'user creation response time < 500ms': (r) => r.timings.duration < 500,
    'user creation returns user ID': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.userId && body.userId.length > 0;
      } catch {
        return false;
      }
    }
  });

  if (success) {
    testUsersCreated.add(1);
  } else {
    apiErrors.add(1);
  }

  userCreationSuccessRate.add(success);
  userCreationTime.add(response.timings.duration);

  return success;
}

function testDataGeneration() {
  if (!authToken) {
    console.error('No auth token available for data generation');
    return false;
  }

  const generationRequest = {
    templateId: 'user-profile-comprehensive',
    count: randomIntBetween(1, 10),
    format: 'json'
  };

  const response = http.post(`${BASE_URL}/api/${API_VERSION}/test-data/generate`,
    JSON.stringify(generationRequest), {
    headers: getAuthHeaders(),
    tags: { name: 'data_generation' }
  });

  const success = check(response, {
    'data generation status is 200': (r) => r.status === 200,
    'data generation response time < 2000ms': (r) => r.timings.duration < 2000,
    'data generation returns generation ID': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body.generationId && body.generationId.length > 0;
      } catch {
        return false;
      }
    }
  });

  if (success) {
    testDataGenerated.add(1);
  } else {
    apiErrors.add(1);
  }

  dataGenerationSuccessRate.add(success);
  dataGenerationTime.add(response.timings.duration);

  return success;
}

function testWorkflowExecution() {
  if (!authToken) {
    return false;
  }

  const workflowData = generateWorkflowData();
  const response = http.post(`${BASE_URL}/api/${API_VERSION}/workflows/execute`,
    JSON.stringify(workflowData), {
    headers: getAuthHeaders(),
    tags: { name: 'workflow_execution' }
  });

  const success = check(response, {
    'workflow execution status is 200 or 202': (r) => r.status === 200 || r.status === 202,
    'workflow execution response time < 1000ms': (r) => r.timings.duration < 1000
  });

  if (!success) {
    apiErrors.add(1);
  }

  return success;
}

function testSystemMetrics() {
  const response = http.get(`${BASE_URL}/api/${API_VERSION}/system/metrics`, {
    headers: getAuthHeaders(),
    tags: { name: 'system_metrics' }
  });

  if (response.status === 200) {
    try {
      const metrics = JSON.parse(response.body);
      if (metrics.memory) memoryUsage.add(metrics.memory);
      if (metrics.cpu) cpuUsage.add(metrics.cpu);
      if (metrics.connections) activeConnections.add(metrics.connections);
    } catch (e) {
      // Ignore parsing errors for metrics
    }
  }

  return response.status === 200;
}

// ============================================================================
// Main Test Execution Function
// ============================================================================

export default function() {
  // Ensure authentication
  if (!authToken) {
    authenticate();
    if (!authToken) {
      console.error('Authentication failed, skipping test iteration');
      return;
    }
  }

  // Test scenarios with different weights based on real usage patterns
  const scenarios = [
    { name: 'Health Check', func: testHealthCheck, weight: 30 },
    { name: 'User Creation', func: testUserCreation, weight: 20 },
    { name: 'Data Generation', func: testDataGeneration, weight: 25 },
    { name: 'Workflow Execution', func: testWorkflowExecution, weight: 20 },
    { name: 'System Metrics', func: testSystemMetrics, weight: 5 }
  ];

  // Select scenario based on weight distribution
  const totalWeight = scenarios.reduce((sum, scenario) => sum + scenario.weight, 0);
  const random = randomIntBetween(1, totalWeight);
  let currentWeight = 0;
  let selectedScenario = scenarios[0];

  for (const scenario of scenarios) {
    currentWeight += scenario.weight;
    if (random <= currentWeight) {
      selectedScenario = scenario;
      break;
    }
  }

  // Execute selected scenario
  group(selectedScenario.name, () => {
    selectedScenario.func();
  });

  // Random think time between requests (simulate real user behavior)
  sleep(randomIntBetween(1, 5));
}

// ============================================================================
// Setup and Teardown Functions
// ============================================================================

export function setup() {
  console.log(`🚀 Starting ${TEST_TYPE.toUpperCase()} performance test`);
  console.log(`📊 Configuration:`);
  console.log(`   - Base URL: ${BASE_URL}`);
  console.log(`   - Duration: ${DURATION}`);
  console.log(`   - Virtual Users: ${VUS}`);
  console.log(`   - API Version: ${API_VERSION}`);

  // Warm-up request to ensure system is ready
  const warmupResponse = http.get(`${BASE_URL}/api/${API_VERSION}/health`);
  if (warmupResponse.status !== 200) {
    console.error(`❌ System not ready - health check failed with status: ${warmupResponse.status}`);
    return null;
  }

  console.log('✅ System ready - starting performance test');
  return { startTime: Date.now() };
}

export function teardown(data) {
  if (data && data.startTime) {
    const duration = Date.now() - data.startTime;
    console.log(`🏁 Performance test completed in ${Math.round(duration / 1000)}s`);
  }

  console.log('📊 Test Summary:');
  console.log(`   - Test Type: ${TEST_TYPE.toUpperCase()}`);
  console.log(`   - Duration: ${DURATION}`);
  console.log(`   - Virtual Users: ${VUS}`);
  console.log('✅ Teardown complete');
}

// ============================================================================
// Error Handling and Monitoring
// ============================================================================

export function handleSummary(data) {
  const summary = {
    testType: TEST_TYPE,
    timestamp: new Date().toISOString(),
    duration: DURATION,
    virtualUsers: VUS,
    baseUrl: BASE_URL,
    metrics: {
      http_reqs: data.metrics.http_reqs?.values?.count || 0,
      http_req_failed: data.metrics.http_req_failed?.values?.rate || 0,
      http_req_duration: {
        avg: data.metrics.http_req_duration?.values?.avg || 0,
        p95: data.metrics.http_req_duration?.values?.['p(95)'] || 0,
        p99: data.metrics.http_req_duration?.values?.['p(99)'] || 0,
        max: data.metrics.http_req_duration?.values?.max || 0
      },
      custom_metrics: {
        auth_success_rate: data.metrics.auth_success_rate?.values?.rate || 0,
        user_creation_success_rate: data.metrics.user_creation_success_rate?.values?.rate || 0,
        data_generation_success_rate: data.metrics.data_generation_success_rate?.values?.rate || 0,
        test_users_created: data.metrics.test_users_created?.values?.count || 0,
        test_data_generated: data.metrics.test_data_generated?.values?.count || 0,
        api_errors: data.metrics.api_errors?.values?.count || 0
      }
    },
    thresholds: data.thresholds || {}
  };

  return {
    'performance-summary.json': JSON.stringify(summary, null, 2),
    stdout: textSummary(data, { indent: ' ', enableColors: true })
  };
}

function textSummary(data, { indent = '', enableColors = false } = {}) {
  const colors = {
    green: enableColors ? '\x1b[32m' : '',
    red: enableColors ? '\x1b[31m' : '',
    yellow: enableColors ? '\x1b[33m' : '',
    blue: enableColors ? '\x1b[34m' : '',
    reset: enableColors ? '\x1b[0m' : ''
  };

  return `
${indent}${colors.blue}====== AI-CORE Performance Test Results ======${colors.reset}
${indent}Test Type: ${TEST_TYPE.toUpperCase()}
${indent}Duration: ${DURATION}
${indent}Virtual Users: ${VUS}
${indent}
${indent}${colors.blue}HTTP Metrics:${colors.reset}
${indent}  Total Requests: ${data.metrics.http_reqs?.values?.count || 0}
${indent}  Failed Requests: ${((data.metrics.http_req_failed?.values?.rate || 0) * 100).toFixed(2)}%
${indent}  Avg Response Time: ${(data.metrics.http_req_duration?.values?.avg || 0).toFixed(2)}ms
${indent}  P95 Response Time: ${(data.metrics.http_req_duration?.values?.['p(95)'] || 0).toFixed(2)}ms
${indent}  P99 Response Time: ${(data.metrics.http_req_duration?.values?.['p(99)'] || 0).toFixed(2)}ms
${indent}
${indent}${colors.blue}Business Metrics:${colors.reset}
${indent}  Auth Success Rate: ${((data.metrics.auth_success_rate?.values?.rate || 0) * 100).toFixed(2)}%
${indent}  Test Users Created: ${data.metrics.test_users_created?.values?.count || 0}
${indent}  Test Data Generated: ${data.metrics.test_data_generated?.values?.count || 0}
${indent}  API Errors: ${data.metrics.api_errors?.values?.count || 0}
${indent}
${indent}${colors.blue}Threshold Status:${colors.reset}
${Object.entries(data.thresholds || {}).map(([key, value]) =>
  `${indent}  ${key}: ${value.ok ? colors.green + '✓ PASS' + colors.reset : colors.red + '✗ FAIL' + colors.reset}`
).join('\n')}
${indent}
${indent}${colors.blue}===============================================${colors.reset}
`;
}
