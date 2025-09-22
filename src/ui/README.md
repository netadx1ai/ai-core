# AI-CORE Client App

A modern React-based dashboard for managing AI-CORE workflows and services with real-time status monitoring.

## 🎯 Overview

This client app provides a user-friendly interface for:

- **Real-time Service Monitoring**: Check status of AI-CORE services (Federation, Intent Parser, MCP Manager, MCP Proxy)
- **AI-Powered Workflow Creation**: Create workflows using natural language descriptions
- **Workflow Management**: Monitor, control, and track automation workflows
- **Service Health Detection**: Distinguish between REAL and MOCK services
- **Real-time Updates**: Live progress tracking and status updates

## 🚀 Quick Start

### Prerequisites

- Node.js 18+ and npm
- AI-CORE backend services running (Federation, Intent Parser, MCP Manager)

### Installation

```bash
# Navigate to the client app directory
cd AI-CORE/src/ui

# Install dependencies
npm install

# Copy environment configuration
cp .env.example .env

# Start development server
npm run dev
```

The app will be available at `http://localhost:5173`

## 🔧 Configuration

### Environment Variables

Create a `.env` file based on `.env.example`:

```env
# Core API Configuration
VITE_AI_CORE_API_URL=http://localhost:8801

# Service Endpoints
VITE_FEDERATION_URL=http://localhost:8801
VITE_INTENT_PARSER_URL=http://localhost:8802
VITE_MCP_MANAGER_URL=http://localhost:8804
VITE_MCP_PROXY_URL=http://localhost:8803

# Feature Flags
VITE_ENABLE_SERVICE_HEALTH_CHECK=true
VITE_ENABLE_REAL_TIME_UPDATES=true
```

### Service Integration

The client app connects to these AI-CORE services:

| Service       | Port | Purpose                                           |
| ------------- | ---- | ------------------------------------------------- |
| Federation    | 8801 | Main API gateway and workflow orchestration       |
| Intent Parser | 8802 | Natural language processing for workflow creation |
| MCP Manager   | 8804 | Content generation and management                 |
| MCP Proxy     | 8803 | Compatibility layer for MCP services              |

## 📋 Features

### Dashboard

- **Service Status Panel**: Real-time health monitoring with REAL/MOCK/OFFLINE indicators
- **Quick Actions**: Direct access to workflow creation and management
- **System Alerts**: Visual notifications for service issues
- **Auto-refresh**: Service status updates every 30 seconds

### Workflow Manager

- **AI Workflow Creation**: Natural language workflow descriptions
- **Progress Tracking**: Real-time workflow execution monitoring
- **Status Management**: Start, pause, stop, and edit workflows
- **Search and Filter**: Find workflows by status, name, or description

### Service Status Detection

- **REAL Services**: ✅ Green - Fully operational AI-CORE services
- **MOCK Services**: ⚠️ Yellow - Development/testing mode services
- **OFFLINE Services**: ❌ Red - Unavailable or failed services

## 🛠️ Development

### Scripts

```bash
# Development server with hot reload
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview

# Run linting
npm run lint

# Run type checking
npm run type-check
```

### Project Structure

```
src/
├── components/          # Reusable UI components
├── hooks/              # React hooks for state management
├── pages/              # Page components (Dashboard, WorkflowManager)
├── services/           # API services and HTTP clients
├── types/              # TypeScript type definitions
└── utils/              # Utility functions
```

### Key Files

- `src/services/api.ts` - Main API service with AI-CORE integration
- `src/pages/Dashboard.tsx` - Service monitoring dashboard
- `src/pages/WorkflowManager.tsx` - Workflow creation and management
- `vite.config.ts` - Vite configuration with proxy setup

## 🚨 Troubleshooting

### Common Issues

#### ❌ Services Showing as OFFLINE

**Problem**: Dashboard shows all services as offline with red indicators.

**Solutions**:

1. **Check AI-CORE Services**:

    ```bash
    # Verify services are running
    curl http://localhost:8801/health  # Federation
    curl http://localhost:8802/health  # Intent Parser
    curl http://localhost:8804/health  # MCP Manager
    curl http://localhost:8803/health  # MCP Proxy
    ```

2. **Start AI-CORE Services**:

    ```bash
    cd AI-CORE
    ./start-real-services.sh
    ```

3. **Check Ports**:
    ```bash
    # Check if ports are in use
    lsof -i :8801 -i :8802 -i :8803 -i :8804
    ```

#### ⚠️ Services Showing as MOCK

**Problem**: Services are healthy but show as MOCK instead of REAL.

**Solutions**:

1. **Verify Real Services**: Ensure you're using `start-real-services.sh` not mock services
2. **Check Service Response**: Mock services return `service` field containing "mock"
3. **Update Configuration**: Ensure environment variables point to real services

#### 🔄 Double /v1 Path Errors

**Problem**: API calls fail with 404 errors due to `/v1/v1` in URLs.

**Solutions**:

1. **Check Environment Variable**: Remove `/v1` from `VITE_AI_CORE_API_URL`:

    ```env
    # ❌ Wrong
    VITE_AI_CORE_API_URL=http://localhost:8801/v1

    # ✅ Correct
    VITE_AI_CORE_API_URL=http://localhost:8801
    ```

2. **URL Cleaning**: The `cleanBaseUrl()` function automatically removes trailing `/v1`

#### 🚫 CORS Errors

**Problem**: Browser blocks API requests due to CORS policy.

**Solutions**:

1. **Use Proxy**: Vite proxy is configured for development mode
2. **Start with Proxy**: API calls will automatically use proxy in development
3. **Production Setup**: Configure proper CORS headers on AI-CORE services

#### 📱 Workflow Creation Fails

**Problem**: "Create AI Workflow" fails with errors.

**Solutions**:

1. **Check Federation Service**:

    ```bash
    curl -X POST http://localhost:8801/v1/workflows \
      -H "Content-Type: application/json" \
      -d '{"intent": "test workflow", "workflow_type": "blog-post"}'
    ```

2. **Verify Intent Parser**: Ensure Intent Parser service is running and healthy
3. **Check Network**: Open browser dev tools to see actual error responses

#### 🔍 Service Health Check Failures

**Problem**: Health checks time out or fail unexpectedly.

**Solutions**:

1. **Increase Timeout**: Adjust `VITE_API_TIMEOUT` in environment variables
2. **Check Network**: Verify client can reach AI-CORE services
3. **Service Logs**: Check AI-CORE service logs for errors:
    ```bash
    tail -f AI-CORE/logs/federation.log
    tail -f AI-CORE/logs/intent-parser.log
    tail -f AI-CORE/logs/mcp-manager.log
    ```

### Development Issues

#### 📦 Build Failures

```bash
# Clear cache and rebuild
rm -rf node_modules package-lock.json
npm install
npm run build
```

#### 🎨 Styling Issues

```bash
# Rebuild Tailwind CSS
npm run build:css
```

#### 🔧 TypeScript Errors

```bash
# Run type checking
npm run type-check

# Fix common issues
npm run lint --fix
```

## 🧪 Testing

### Manual Testing Checklist

1. **Service Status**:
    - [ ] Dashboard loads without errors
    - [ ] All 4 services show correct status (REAL/MOCK/OFFLINE)
    - [ ] Status refreshes every 30 seconds
    - [ ] Manual refresh button works

2. **Workflow Creation**:
    - [ ] "Create AI Workflow" modal opens
    - [ ] Intent description accepts text input
    - [ ] Workflow type selector works
    - [ ] Workflow creation succeeds with real services
    - [ ] Progress tracking updates in real-time

3. **Navigation**:
    - [ ] Dashboard → Workflows navigation works
    - [ ] All pages load without errors
    - [ ] Theme toggle works (light/dark)

### Integration Testing

```bash
# Run integration test script
cd AI-CORE
node test-real-integration.js
```

### E2E Testing

```bash
# Install Playwright (if available)
npx playwright install

# Run E2E tests
npx playwright test
```

## 📊 Performance

### Optimization

- **Code Splitting**: Vendor chunks separated for better caching
- **Lazy Loading**: Components loaded on demand
- **Service Worker**: Cache static assets (production builds)
- **Image Optimization**: WebP format support with fallbacks

### Monitoring

- **Real-time Updates**: WebSocket connections for live data
- **Error Boundaries**: Graceful error handling and recovery
- **Performance Metrics**: Core Web Vitals tracking
- **Service Health**: Automated monitoring with alerts

## 🚀 Deployment

### Production Build

```bash
# Build for production
npm run build

# Test production build locally
npm run preview

# Deploy dist/ directory to your web server
```

### Docker Deployment

```dockerfile
FROM node:18-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=0 /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### Environment-specific Configuration

```bash
# Development
VITE_AI_CORE_API_URL=http://localhost:8801

# Staging
VITE_AI_CORE_API_URL=https://staging-api.ai-core.com

# Production
VITE_AI_CORE_API_URL=https://api.ai-core.com
```

## 📚 API Reference

### Service Health Check

```typescript
// Check individual service
const status = await apiService.checkServiceHealth("federation", "http://localhost:8801");

// Check all services
const allStatuses = await apiService.checkAllServicesHealth();
```

### Workflow Management

```typescript
// Create workflow from intent
const workflow = await apiService.createWorkflowFromIntent(
    "Create a blog post about AI automation trends",
    "blog-post-social",
);

// Get workflow status
const status = await apiService.getWorkflowStatus(workflowId);
```

## 🤝 Contributing

1. **Setup Development Environment**:

    ```bash
    git clone <repository>
    cd AI-CORE/src/ui
    npm install
    cp .env.example .env
    ```

2. **Make Changes**: Follow existing code patterns and conventions

3. **Test Changes**: Verify all features work with real AI-CORE services

4. **Submit PR**: Include description of changes and testing performed

## 📄 License

This project is part of the AI-CORE system. See the main project license for details.

---

## 🆘 Need Help?

1. **Check Logs**: Browser dev tools → Console/Network tabs
2. **Service Status**: Verify AI-CORE services are running and healthy
3. **Environment**: Ensure `.env` file is configured correctly
4. **Integration Test**: Run `test-real-integration.js` to verify end-to-end functionality

**Quick Health Check**:

```bash
# One-liner to check all services
curl -s http://localhost:8801/health && \
curl -s http://localhost:8802/health && \
curl -s http://localhost:8803/health && \
curl -s http://localhost:8804/health && \
echo "All services OK" || echo "Service check failed"
```
