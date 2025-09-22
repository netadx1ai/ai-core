#!/bin/bash

# AI-CORE Core Services Startup Script
# Starts Federation, Intent Parser, and MCP Manager services for MVP demo

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Service configuration
FEDERATION_PORT=8801
INTENT_PARSER_PORT=8802
MCP_MANAGER_PORT=8803

# Database connection
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/ai_core_dev"
export REDIS_URL="redis://localhost:6379"
export SQLX_OFFLINE=true

echo -e "${BLUE}🚀 Starting AI-CORE Core Services for MVP Demo${NC}"
echo "=================================================="

# Function to check if port is available
check_port() {
    local port=$1
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null ; then
        echo -e "${YELLOW}⚠️  Port $port is already in use${NC}"
        return 1
    fi
    return 0
}

# Function to wait for service to be ready
wait_for_service() {
    local name=$1
    local port=$2
    local max_attempts=30
    local attempt=1

    echo -e "${YELLOW}⏳ Waiting for $name to be ready on port $port...${NC}"

    while [ $attempt -le $max_attempts ]; do
        if curl -s http://localhost:$port/health >/dev/null 2>&1; then
            echo -e "${GREEN}✅ $name is ready!${NC}"
            return 0
        fi
        echo -n "."
        sleep 2
        attempt=$((attempt + 1))
    done

    echo -e "${RED}❌ $name failed to start within 60 seconds${NC}"
    return 1
}

# Function to start a service
start_service() {
    local name=$1
    local path=$2
    local port=$3
    local log_file="logs/${name}.log"

    echo -e "${BLUE}📦 Starting $name...${NC}"

    # Create logs directory if it doesn't exist
    mkdir -p logs

    # Check if port is available
    if ! check_port $port; then
        echo -e "${RED}❌ Cannot start $name - port $port is in use${NC}"
        return 1
    fi

    # Start the service in background
    cd src/services/$path
    echo "Starting in $(pwd)"
    RUST_LOG=info cargo run -- --port $port > ../../../$log_file 2>&1 &
    local pid=$!
    cd - > /dev/null

    # Save PID for cleanup
    echo $pid > "logs/${name}.pid"

    echo -e "${GREEN}🔄 $name started with PID $pid${NC}"

    # Wait for service to be ready
    if wait_for_service "$name" $port; then
        return 0
    else
        echo -e "${RED}❌ $name failed to start${NC}"
        kill $pid 2>/dev/null || true
        return 1
    fi
}

# Function to cleanup services
cleanup() {
    echo -e "\n${YELLOW}🧹 Cleaning up services...${NC}"

    for service in federation intent-parser mcp-manager; do
        if [ -f "logs/${service}.pid" ]; then
            local pid=$(cat "logs/${service}.pid")
            if kill -0 $pid 2>/dev/null; then
                echo -e "${YELLOW}⏹️  Stopping $service (PID: $pid)${NC}"
                kill $pid
                sleep 2
                # Force kill if still running
                if kill -0 $pid 2>/dev/null; then
                    kill -9 $pid 2>/dev/null || true
                fi
            fi
            rm -f "logs/${service}.pid"
        fi
    done

    echo -e "${GREEN}✅ Cleanup complete${NC}"
}

# Trap to cleanup on exit
trap cleanup EXIT INT TERM

# Check if databases are running
echo -e "${BLUE}🔍 Checking database connections...${NC}"
if ! docker ps | grep -q AI-PLATFORM-postgres; then
    echo -e "${RED}❌ PostgreSQL is not running. Please start databases first:${NC}"
    echo "cd infrastructure/docker && docker compose -f docker-compose.dev.yml up -d"
    exit 1
fi

if ! docker ps | grep -q AI-PLATFORM-redis; then
    echo -e "${RED}❌ Redis is not running. Please start databases first:${NC}"
    echo "cd infrastructure/docker && docker compose -f docker-compose.dev.yml up -d"
    exit 1
fi

echo -e "${GREEN}✅ Databases are running${NC}"

# Start core services
echo -e "\n${BLUE}🚀 Starting core services...${NC}"

# Start Federation Service
if start_service "Federation" "federation" $FEDERATION_PORT; then
    echo -e "${GREEN}✅ Federation Service started successfully${NC}"
else
    echo -e "${RED}❌ Failed to start Federation Service${NC}"
    exit 1
fi

# Start Intent Parser Service
if start_service "Intent Parser" "intent-parser" $INTENT_PARSER_PORT; then
    echo -e "${GREEN}✅ Intent Parser Service started successfully${NC}"
else
    echo -e "${RED}❌ Failed to start Intent Parser Service${NC}"
    exit 1
fi

# Start MCP Manager Service
if start_service "MCP Manager" "mcp-manager" $MCP_MANAGER_PORT; then
    echo -e "${GREEN}✅ MCP Manager Service started successfully${NC}"
else
    echo -e "${RED}❌ Failed to start MCP Manager Service${NC}"
    exit 1
fi

echo -e "\n${GREEN}🎉 All core services are running successfully!${NC}"
echo "=================================================="
echo -e "${BLUE}📊 Service Status:${NC}"
echo "  • Federation Service: http://localhost:$FEDERATION_PORT/health"
echo "  • Intent Parser:      http://localhost:$INTENT_PARSER_PORT/health"
echo "  • MCP Manager:        http://localhost:$MCP_MANAGER_PORT/health"
echo ""
echo -e "${BLUE}🔗 Service URLs:${NC}"
echo "  • Federation API:     http://localhost:$FEDERATION_PORT"
echo "  • Intent Parser API:  http://localhost:$INTENT_PARSER_PORT"
echo "  • MCP Manager API:    http://localhost:$MCP_MANAGER_PORT"
echo ""
echo -e "${BLUE}📝 Logs:${NC}"
echo "  • Federation: logs/Federation.log"
echo "  • Intent Parser: logs/intent-parser.log"
echo "  • MCP Manager: logs/mcp-manager.log"
echo ""
echo -e "${YELLOW}💡 Press Ctrl+C to stop all services${NC}"

# Test basic integration
echo -e "\n${BLUE}🧪 Testing basic service integration...${NC}"

# Test Federation health
if curl -s http://localhost:$FEDERATION_PORT/health | grep -q "ok\|healthy"; then
    echo -e "${GREEN}✅ Federation Service health check passed${NC}"
else
    echo -e "${YELLOW}⚠️  Federation Service health check uncertain${NC}"
fi

# Test Intent Parser health
if curl -s http://localhost:$INTENT_PARSER_PORT/health | grep -q "ok\|healthy"; then
    echo -e "${GREEN}✅ Intent Parser health check passed${NC}"
else
    echo -e "${YELLOW}⚠️  Intent Parser health check uncertain${NC}"
fi

# Test MCP Manager health
if curl -s http://localhost:$MCP_MANAGER_PORT/health | grep -q "ok\|healthy"; then
    echo -e "${GREEN}✅ MCP Manager health check passed${NC}"
else
    echo -e "${YELLOW}⚠️  MCP Manager health check uncertain${NC}"
fi

echo -e "\n${GREEN}🚀 Core services are ready for MVP development!${NC}"
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Build and register built-in MCPs"
echo "  2. Create external MCPs (Image, Calendar, Facebook)"
echo "  3. Implement workflow orchestration"
echo "  4. Build demo web interface"

# Keep services running
echo -e "\n${YELLOW}Services are running... Press Ctrl+C to stop${NC}"
wait
