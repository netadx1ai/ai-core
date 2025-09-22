#!/bin/bash

# AI-CORE Services Startup Script - REAL INTEGRATION
# Startup script for core services with REAL Intent Parser (not mock)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 AI-CORE Services Startup - REAL INTEGRATION${NC}"
echo "Starting core services with REAL Intent Parser for client app integration..."

# Load environment variables from .env file if it exists
if [ -f ".env" ]; then
    echo "Loading environment variables from .env file..."
    export $(grep -v '^#' .env | xargs)
fi

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

# Export environment variables for Intent Parser REAL service
export INTENT_PARSER_PORT=8802
export INTENT_PARSER_HOST=127.0.0.1
export ENVIRONMENT=development
export LOG_LEVEL=info

# Configure LLM - Check for Gemini API key first, then Ollama, then mock
if [ ! -z "$GEMINI_API_KEY" ]; then
    echo "   Using Gemini AI (API key detected)"
    export LLM_PROVIDER=openai
    export LLM_API_KEY="$GEMINI_API_KEY"
    export LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/openai/chat/completions
    export LLM_MODEL=gemini-1.5-flash
elif curl -s http://localhost:11434/api/version >/dev/null 2>&1; then
    echo "   Using Ollama LLM (detected running)"
    export LLM_PROVIDER=ollama
    export LLM_API_KEY=dummy-key-for-ollama
    export LLM_API_URL=http://localhost:11434/api/chat
    export LLM_MODEL=llama2
else
    echo "   Using Mock LLM (no Gemini API key or Ollama detected)"
    export LLM_PROVIDER=mock
    export LLM_API_KEY=mock-key
    export LLM_API_URL=http://localhost:8899/mock
    export LLM_MODEL=mock-model
fi
export DATABASE_URL=sqlite:intent_parser.db
export REDIS_URL=redis://localhost:6379
export MAX_BATCH_SIZE=50

echo -e "${BLUE}🔧 Environment configured for REAL Intent Parser:${NC}"
echo "   Port: $INTENT_PARSER_PORT"
echo "   LLM Provider: $LLM_PROVIDER"
echo "   LLM Model: $LLM_MODEL"
echo "   LLM URL: $LLM_API_URL"

# Start Mock LLM Service if needed (Port 8899)
if [ "$LLM_PROVIDER" = "mock" ]; then
    echo -e "${BLUE}🔧 Starting Mock LLM Service...${NC}"
    python3 -c "
import http.server
import socketserver
import json

class MockLLMHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({'status': 'ok'}).encode())

    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)

        # Mock LLM response
        response = {
            'choices': [{
                'message': {
                    'role': 'assistant',
                    'content': 'Mock intent parsing response',
                    'function_call': {
                        'name': 'create_blog_post',
                        'arguments': json.dumps({
                            'topic': 'AI automation trends',
                            'word_count': 800,
                            'tone': 'professional'
                        })
                    }
                },
                'finish_reason': 'function_call'
            }],
            'usage': {'prompt_tokens': 100, 'completion_tokens': 50, 'total_tokens': 150}
        }

        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(response).encode())

    def log_message(self, format, *args):
        return

with socketserver.TCPServer(('', 8899), MockLLMHandler) as httpd:
    print('Mock LLM serving at port 8899')
    httpd.serve_forever()
" > logs/mock-llm.log 2>&1 &
    MOCK_LLM_PID=$!
    echo "Mock LLM started with PID: $MOCK_LLM_PID"
    sleep 2
fi

# Start REAL Intent Parser Service (Port 8802)
echo -e "${BLUE}🔧 Starting REAL Intent Parser Service...${NC}"
cd src/services/intent-parser
if cargo build --release; then
    RUST_LOG=info INTENT_PARSER_PORT=8802 cargo run --release > ../../../logs/intent-parser.log 2>&1 &
    INTENT_PARSER_PID=$!
    echo "REAL Intent Parser started with PID: $INTENT_PARSER_PID"
    cd ../../..
else
    echo -e "${RED}❌ Failed to build Intent Parser service${NC}"
    [ "$LLM_PROVIDER" = "mock" ] && kill $MOCK_LLM_PID 2>/dev/null
    exit 1
fi

# Start Federation Simple Service (Port 8801)
echo -e "${BLUE}🔧 Starting Federation Simple Service...${NC}"
cd src/services/federation-simple
if cargo build --release; then
    RUST_LOG=info cargo run --release > ../../../logs/federation.log 2>&1 &
    FEDERATION_PID=$!
    echo "Federation service started with PID: $FEDERATION_PID"
    cd ../../..
else
    echo -e "${RED}❌ Failed to build federation service${NC}"
    kill $INTENT_PARSER_PID 2>/dev/null
    exit 1
fi

# Start MCP Manager Mock Service (Port 8803) - Keep as mock for now
echo -e "${BLUE}🔧 Starting Mock MCP Manager...${NC}"
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
                'content': 'Mock content generation result from MCP Manager',
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
" > logs/mcp-manager.log 2>&1 &
MCP_MANAGER_PID=$!
echo "Mock MCP Manager started with PID: $MCP_MANAGER_PID"

# Wait for services to be ready
echo -e "${BLUE}🏥 Checking service health...${NC}"
sleep 5

wait_for_service "http://localhost:8802/health" "REAL Intent Parser"
wait_for_service "http://localhost:8801/health" "Federation Service"
wait_for_service "http://localhost:8803/health" "Mock MCP Manager"

echo ""
echo -e "${GREEN}✅ All AI-CORE services are running with REAL Intent Parser!${NC}"
echo ""
echo -e "${BLUE}📊 Service Status:${NC}"
echo "  • Federation Service: http://localhost:8801 (PID: $FEDERATION_PID) - REAL"
if [ ! -z "$GEMINI_API_KEY" ]; then
    echo "  • Intent Parser:      http://localhost:8802 (PID: $INTENT_PARSER_PID) - REAL with Gemini AI ✨"
elif [ "$LLM_PROVIDER" = "ollama" ]; then
    echo "  • Intent Parser:      http://localhost:8802 (PID: $INTENT_PARSER_PID) - REAL with Ollama ✨"
else
    echo "  • Intent Parser:      http://localhost:8802 (PID: $INTENT_PARSER_PID) - REAL with Mock LLM ✨"
fi
if [ "$LLM_PROVIDER" = "mock" ]; then
    echo "  • Mock LLM Service:   http://localhost:8899 (PID: $MOCK_LLM_PID) - MOCK"
fi
echo "  • MCP Manager:        http://localhost:8803 (PID: $MCP_MANAGER_PID) - MOCK"
echo ""
echo -e "${BLUE}🔌 API Endpoints:${NC}"
echo "  • Health Check:       http://localhost:8801/health"
echo "  • Create Workflow:    POST http://localhost:8801/v1/workflows"
echo "  • Get Workflow:       GET http://localhost:8801/v1/workflows/{id}"
echo "  • Intent Parser:      POST http://localhost:8802/v1/parse"
echo ""
echo -e "${YELLOW}📝 Logs available in:${NC}"
echo "  • Federation: logs/federation.log"
echo "  • Intent Parser: logs/intent-parser.log"
echo "  • MCP Manager: logs/mcp-manager.log"
echo ""
if [ ! -z "$GEMINI_API_KEY" ]; then
    echo -e "${GREEN}🚀 Ready for client app integration with REAL Intent Parser using Gemini AI!${NC}"
else
    echo -e "${GREEN}🚀 Ready for client app integration with REAL Intent Parser!${NC}"
fi
echo ""
if [ "$LLM_PROVIDER" = "mock" ]; then
    echo "To stop services: kill $FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID $MOCK_LLM_PID"
    echo "$FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID $MOCK_LLM_PID" > .ai-core-pids
else
    echo "To stop services: kill $FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID"
    echo "$FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID" > .ai-core-pids
fi
echo "Or run: pkill -f 'federation-simple|intent-parser|python3'"



# Wait for user input to keep running
echo ""
echo -e "${BLUE}Press Ctrl+C to stop all services...${NC}"
trap 'echo -e "\n${YELLOW}Shutting down services...${NC}"; kill $FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID 2>/dev/null; rm -f .ai-core-pids; exit 0' INT

# Keep script running
while true; do
    sleep 1
done
