#!/bin/bash

# AI-CORE Real Services Startup Script
# Background startup for REAL Intent Parser with Gemini AI

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 AI-CORE Real Services Startup${NC}"

# Load environment variables from .env file if it exists
if [ -f ".env" ]; then
    echo "Loading environment variables from .env file..."
    set -a
    source .env
    set +a
fi

# Kill any existing services
echo -e "${YELLOW}🧹 Cleaning up existing processes...${NC}"
pkill -f "federation-simple" 2>/dev/null || true
pkill -f "intent-parser" 2>/dev/null || true
pkill -f "python3.*8803" 2>/dev/null || true
pkill -f "python3.*8899" 2>/dev/null || true
sleep 3

# Create logs directory
mkdir -p logs

# Export environment variables for Intent Parser REAL service
export INTENT_PARSER_PORT=8802
export INTENT_PARSER_HOST=127.0.0.1
export ENVIRONMENT=development
export LOG_LEVEL=info

# Configure LLM - Check for Gemini API key first
if [ ! -z "$GEMINI_API_KEY" ]; then
    echo -e "${GREEN}   Using Gemini AI (API key detected)${NC}"
    export LLM_PROVIDER=gemini
    export LLM_API_KEY="$GEMINI_API_KEY"
    export LLM_API_URL=https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent
    export LLM_MODEL=gemini-2.0-flash
else
    echo -e "${YELLOW}   No Gemini API key found - services may have limited functionality${NC}"
    export LLM_PROVIDER=mock
    export LLM_API_KEY=mock-key
    export LLM_API_URL=http://localhost:8899/mock
    export LLM_MODEL=mock-model
fi

export DATABASE_URL=sqlite:intent_parser.db
export REDIS_URL=redis://localhost:6379
export MAX_BATCH_SIZE=50

echo -e "${BLUE}🔧 Configuration:${NC}"
echo "   Intent Parser Port: $INTENT_PARSER_PORT"
echo "   LLM Provider: $LLM_PROVIDER"
echo "   LLM Model: $LLM_MODEL"

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
    httpd.serve_forever()
" > logs/mock-llm.log 2>&1 &
    MOCK_LLM_PID=$!
    echo "Mock LLM started with PID: $MOCK_LLM_PID"
    sleep 2
fi

# Start REAL Intent Parser Service (Port 8802)
echo -e "${BLUE}🔧 Starting REAL Intent Parser Service...${NC}"
cd src/services/intent-parser
if cargo build --release --quiet; then
    nohup cargo run --release > ../../../logs/intent-parser.log 2>&1 &
    INTENT_PARSER_PID=$!
    echo "REAL Intent Parser started with PID: $INTENT_PARSER_PID"
    cd ../../..
else
    echo -e "${RED}❌ Failed to build Intent Parser service${NC}"
    [ "$LLM_PROVIDER" = "mock" ] && kill $MOCK_LLM_PID 2>/dev/null
    exit 1
fi

# Start Federation Service (Port 8801)
echo -e "${BLUE}🔧 Starting Federation Service...${NC}"
cd src/services/federation-simple
if cargo build --release --quiet; then
    nohup cargo run --release > ../../../logs/federation.log 2>&1 &
    FEDERATION_PID=$!
    echo "Federation service started with PID: $FEDERATION_PID"
    cd ../../..
else
    echo -e "${RED}❌ Failed to build Federation service${NC}"
    kill $INTENT_PARSER_PID 2>/dev/null
    [ "$LLM_PROVIDER" = "mock" ] && kill $MOCK_LLM_PID 2>/dev/null
    exit 1
fi

# Start REAL Demo Content MCP (Port 8804, proxied to 8803)
echo -e "${BLUE}🔧 Starting REAL Demo Content MCP Service...${NC}"
cd src/services/demo-content-mcp
if cargo build --release --quiet; then
    nohup cargo run --release > ../../../logs/mcp-manager.log 2>&1 &
    MCP_MANAGER_PID=$!
    echo "Demo Content MCP started with PID: $MCP_MANAGER_PID (port 8804)"
    cd ../../..

    # Start port forwarder from 8803 to 8804 for compatibility
    echo -e "${BLUE}🔀 Setting up port forwarding 8803 → 8804...${NC}"
    nohup python3 -c "
import http.server
import socketserver
import urllib.request
import json

class ProxyHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        try:
            if self.path == '/health':
                # Custom health response for compatibility
                response = {
                    'status': 'healthy',
                    'service': 'demo-content-mcp-proxy',
                    'upstream_port': 8804,
                    'proxy_port': 8803
                }
                self.send_response(200)
                self.send_header('Content-Type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode())
            else:
                self.proxy_request()
        except Exception as e:
            self.send_error(500, str(e))

    def do_POST(self):
        try:
            self.proxy_request()
        except Exception as e:
            self.send_error(500, str(e))

    def proxy_request(self):
        url = f'http://localhost:8804{self.path}'
        if self.command == 'POST':
            content_length = int(self.headers.get('Content-Length', 0))
            post_data = self.rfile.read(content_length)
            req = urllib.request.Request(url, data=post_data, method='POST')
            for header, value in self.headers.items():
                if header.lower() not in ['host', 'connection']:
                    req.add_header(header, value)
        else:
            req = urllib.request.Request(url)

        try:
            response = urllib.request.urlopen(req)
            self.send_response(response.status)
            for header, value in response.headers.items():
                self.send_header(header, value)
            self.end_headers()
            self.wfile.write(response.read())
        except Exception:
            self.send_error(502, 'Bad Gateway')

    def log_message(self, format, *args):
        pass

with socketserver.TCPServer(('', 8803), ProxyHandler) as httpd:
    httpd.serve_forever()
" > logs/mcp-proxy.log 2>&1 &
    PROXY_PID=$!
    echo "Port proxy started with PID: $PROXY_PID"
else
    echo -e "${RED}❌ Failed to build Demo Content MCP service${NC}"
    kill $INTENT_PARSER_PID $FEDERATION_PID 2>/dev/null
    [ "$LLM_PROVIDER" = "mock" ] && kill $MOCK_LLM_PID 2>/dev/null
    exit 1
fi

# Wait for services to start
echo -e "${BLUE}⏳ Waiting for services to initialize...${NC}"
sleep 5

# Function to wait for service
wait_for_service() {
    local url=$1
    local service_name=$2
    local max_attempts=15
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$url" >/dev/null 2>&1; then
            echo -e "${GREEN}✅ $service_name is ready!${NC}"
            return 0
        fi
        sleep 2
        attempt=$((attempt + 1))
    done

    echo -e "${RED}❌ $service_name failed to start${NC}"
    return 1
}

# Check services
echo -e "${BLUE}🏥 Checking service health...${NC}"
wait_for_service "http://localhost:8802/health" "Intent Parser"
wait_for_service "http://localhost:8801/health" "Federation Service"
wait_for_service "http://localhost:8804/health" "Demo Content MCP"
wait_for_service "http://localhost:8803/health" "MCP Proxy"

echo ""
echo -e "${GREEN}✅ All services are running!${NC}"
echo ""
echo -e "${BLUE}📊 Service Status:${NC}"
echo "  • Federation Service: http://localhost:8801 (PID: $FEDERATION_PID) - REAL"
if [ ! -z "$GEMINI_API_KEY" ]; then
    echo "  • Intent Parser:      http://localhost:8802 (PID: $INTENT_PARSER_PID) - REAL with Gemini AI ✨"
else
    echo "  • Intent Parser:      http://localhost:8802 (PID: $INTENT_PARSER_PID) - REAL with Mock LLM ✨"
fi
echo "  • Demo Content MCP:   http://localhost:8804 (PID: $MCP_MANAGER_PID) - REAL ✨"
echo "  • MCP Proxy:          http://localhost:8803 (PID: $PROXY_PID) - PORT FORWARD"
if [ "$LLM_PROVIDER" = "mock" ]; then
    echo "  • Mock LLM Service:   http://localhost:8899 (PID: $MOCK_LLM_PID) - MOCK"
fi

echo ""
echo -e "${BLUE}🔌 API Endpoints:${NC}"
echo "  • Health Check:       http://localhost:8801/health"
echo "  • Create Workflow:    POST http://localhost:8801/v1/workflows"
echo "  • Intent Parser:      POST http://localhost:8802/v1/parse"
echo "  • Demo Content MCP:   POST http://localhost:8804/generate"
echo "  • MCP Proxy:          POST http://localhost:8803/ (forwards to 8804)"

echo ""
echo -e "${BLUE}📝 Logs:${NC}"
echo "  • Federation: logs/federation.log"
echo "  • Intent Parser: logs/intent-parser.log"
echo "  • MCP Manager: logs/mcp-manager.log"
if [ "$LLM_PROVIDER" = "mock" ]; then
    echo "  • Mock LLM: logs/mock-llm.log"
fi

echo ""
if [ ! -z "$GEMINI_API_KEY" ]; then
    echo -e "${GREEN}🚀 Ready for client app integration with REAL Intent Parser using Gemini AI!${NC}"
else
    echo -e "${GREEN}🚀 Ready for client app integration with REAL Intent Parser!${NC}"
fi

# Save PIDs for cleanup
if [ "$LLM_PROVIDER" = "mock" ]; then
    echo "$FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID $PROXY_PID $MOCK_LLM_PID" > .ai-core-pids
else
    echo "$FEDERATION_PID $INTENT_PARSER_PID $MCP_MANAGER_PID $PROXY_PID" > .ai-core-pids
fi

echo ""
echo "To stop all services: pkill -f 'federation-simple|intent-parser|python3'"
echo "Or kill processes: $(cat .ai-core-pids)"
echo ""
echo -e "${GREEN}Services are running in background. Check logs for details.${NC}"
