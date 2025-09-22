#!/bin/bash

# AI-CORE Background Services Startup
# Simple background startup for federation service

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Starting AI-CORE Federation Service${NC}"

# Kill existing processes
pkill -f "federation-simple" 2>/dev/null || true
sleep 2

# Create logs directory
mkdir -p logs

# Start Federation Service
echo "Starting federation service..."
cd src/services/federation-simple
nohup cargo run --release > ../../../logs/federation.log 2>&1 &
FEDERATION_PID=$!
cd ../../..

# Wait for service to start
sleep 5

# Test if service is running
if curl -s http://localhost:8801/health >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Federation service running on port 8801${NC}"
    echo "PID: $FEDERATION_PID"
    echo "Health check: http://localhost:8801/health"
    echo "Workflows API: http://localhost:8801/v1/workflows"
    echo "Logs: logs/federation.log"
    echo ""
    echo "To stop: kill $FEDERATION_PID"
    echo "$FEDERATION_PID" > .federation-pid
else
    echo "❌ Federation service failed to start"
    exit 1
fi
