#!/bin/bash

# AI-CORE Services Startup Script
# Simple startup for core services needed for client app integration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 AI-CORE Services Startup${NC}"
echo "Starting core services for client app integration..."

# Function to check if port is in use
check_port() {
    if lsof -Pi :$1 -sTCP:LISTEN -t >/dev/null ; then
        echo -e "${YELLOW}⚠️  Port $1 is already in use${NC}"
        return 1
    fi
    return 0
}

# Function to wait for service to be ready
wait_for_service() {
    local url=$1
    local service_name=$2
    local max_attempts=30
    local attempt=1

    echo -e "${YELLOW}⏳ Waiting for $service_name to be ready...${NC}"

    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$url" >/dev/null 2>&1; then
            echo -e "${GREEN}✅ $service_name is ready!${NC}"
            return 0
        fi

        echo "   Attempt $attempt/$max_attempts - waiting..."
        sleep 2
        attempt=$((attempt + 1))
    done

    echo -e "${RED}❌ $service_name failed to start after $max_attempts attempts${NC}"
    return 1
}

# Kill any existing services
echo -e "${YELLOW}🧹 Cleaning up existing processes...${NC}"
pkill -f "federation-simple" 2>/dev/null || true
pkill -f "intent-parser" 2>/dev/null || true
pkill -f "mcp-manager" 2>/dev/null || true
sleep 2

# Check required ports
REQUIRED_PORTS=(8801 8802 8803)
for port in "${REQUIRED_PORTS[@]}"; do
    if ! check_port $port; then
        echo -e "${RED}❌ Port $port is in use. Please free it first.${NC}"
        exit 1
    fi
done

# Create logs directory
mkdir -p logs

# Start Federation Simple Service (Port 8801)
echo -e "${BLUE}🔧 Starting Federation Simple Service...${NC}"
cd src/services/federation-simple
cargo build --release
if [ $? -eq 0 ]; then
    RUST_LOG=info cargo run --release > ../../../logs/federation.log 2>&1 &
    FEDERATION_PID=$!
    echo "Federation service started with PID: $FEDERATION_PID"
    cd ../../..
else
    echo -e "${RED}❌ Failed to build federation service${NC}"
    exit 1
fi

# Start MCP Manager Simple Service (Port 8803) - Mock version
echo -e "${BLUE}🔧 Starting Mock MCP Manager...${NC}"
cat > logs/mcp-manager.log 2>&1 &
python3 -c "
import http.server
import socketserver
import json
from urllib.parse import urlparse, parse_qs

class MockMCPHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': 'healthy', 'service': 'mcp-manager-mock'}).encode())
        else:
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'message': 'Mock MCP Manager', 'path': self.path}).encode())

    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)

        # Mock MCP execution response
        response = {
            'execution_id': 'mock_exec_123',
            'status': 'completed',
            'result': {
                'content': 'Mock content generation result',
                'quality_score': 4.5,
                'execution_time_ms': 2000
            }
        }

        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(response).encode())

    def log_message(self, format, *args):
        return  # Suppress default logging

with socketserver.TCPServer(('', 8803), MockMCPHandler) as httpd:
    print('Mock MCP Manager serving at port 8803')
    httpd.serve_forever()
" &
MCP_MANAGER_PID=$!
echo "Mock MCP Manager started with PID: $MCP_MANAGER_PID"

# Start Intent Parser Mock Service (Port 8802)
echo -e "${BLUE}🔧 Starting Mock Intent Parser...${NC}"
python3 -c "
import http.server
import socketserver
import json

class MockIntentHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': 'healthy', 'service': 'intent-parser-mock'}).encode())
        else:
            self.send_response(404)
            self.end_headers()

    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)

        try:
            request_data = json.loads(post_data.decode())
        except:
            request_data = {}

        # Mock intent parsing response
        response = {
            'intent': {
                'type': 'blog_generation',
                'topic': request_data.get('input', 'AI automation trends'),
                'parameters': {
                    'word_count': 800,
                    'tone': 'professional',
                    'include_image': True
                }
            },
            'confidence': 0.95,
            'processing_time_ms': 500
        }

        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(response).encode())

    def log_message(self, format, *args):
        return  # Suppress default logging

with socketserver.TCPServer(('', 8802), MockIntentHandler) as httpd:
    print('Mock Intent Parser serving at port 8802')
    httpd.serve_forever()
" &
INTENT_PARSER_PID=$!
echo "Mock Intent Parser started with PID: $INTENT_PARSER_PID"

# Wait for services to be ready
sleep 3

# Check service health
echo -e "${BLUE}🏥 Checking service health...${NC}"
wait_for_service "http://localhost:8801/health" "Federation Service"
wait_for_service "http://localhost:8802/health" "Intent Parser"
wait_for_service "http://localhost:8803/health" "MCP Manager"

echo ""
echo -e "${GREEN}✅ All AI-CORE services are running!${NC}"
echo ""
echo -e "${BLUE}📊 Service Status:${NC}"
echo "  • Federation Service: http://localhost:8801 (PID: $FEDERATION_PID)"
echo "  • Intent Parser:      http://localhost:8802 (PID: $INTENT_PARSER_PID)"
echo "  • MCP Manager:        http://localhost:8803 (PID: $MCP_MANAGER_PID)"
echo ""
echo -e "${BLUE}🔌 API Endpoints:${NC}"
echo "  • Health Check:       http://localhost:8801/health"
echo "  • Create Workflow:    POST http://localhost:8801/v1/workflows"
echo "  • Get Workflow:       GET http://localhost:8801/v1/workflows/{id}"
echo ""
echo -e "${YELLOW}📝 Logs available in:${NC}"
echo "  • Federation: logs/federation.log"
echo "  • Intent Parser: Console output"
echo "  • MCP Manager: Console output"
echo ""
echo -e "${GREEN}🚀 Ready for client app integration!${NC}"
echo ""
echo "To stop services: kill $FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID"
echo "Or run: pkill -f 'federation-simple|python3'"

# Save PIDs for cleanup
echo "$FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID" > .ai-core-pids

# Wait for user input to keep running
echo ""
echo -e "${BLUE}Press Ctrl+C to stop all services...${NC}"
trap 'echo -e "\n${YELLOW}Shutting down services...${NC}"; kill $FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID 2>/dev/null; rm -f .ai-core-pids; exit 0' INT

# Keep script running
while true; do
    sleep 1
done
