#!/bin/bash

# AI-CORE Real Integration Deployment Script
# Complete automation for deploying and testing the real AI-CORE integration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLIENT_APP_DIR="$PROJECT_ROOT/src/client-app-integration"
LOGS_DIR="$PROJECT_ROOT/logs"
PIDS_FILE="$PROJECT_ROOT/.ai-core-pids"

# Functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_header() {
    echo -e "\n${PURPLE}🚀 $1${NC}\n"
}

# Cleanup function
cleanup() {
    log_warning "Cleaning up processes..."
    if [ -f "$PIDS_FILE" ]; then
        while read -r pid; do
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
            fi
        done < "$PIDS_FILE"
        rm -f "$PIDS_FILE"
    fi
    pkill -f "federation-simple|intent-parser|demo-content|python3.*8803" 2>/dev/null || true
}

# Check prerequisites
check_prerequisites() {
    log_header "Checking Prerequisites"

    # Check Node.js
    if ! command -v node &> /dev/null; then
        log_error "Node.js is not installed"
        exit 1
    fi

    local node_version=$(node --version | cut -d'v' -f2)
    log_info "Node.js version: $node_version"

    # Check npm
    if ! command -v npm &> /dev/null; then
        log_error "npm is not installed"
        exit 1
    fi

    # Check Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo is not installed"
        exit 1
    fi

    # Check Python3
    if ! command -v python3 &> /dev/null; then
        log_error "Python3 is not installed"
        exit 1
    fi

    # Check environment variables
    if [ -z "$GEMINI_API_KEY" ]; then
        log_warning "GEMINI_API_KEY not set - services will run with limited functionality"
    else
        log_success "Gemini API key detected"
    fi

    log_success "All prerequisites met"
}

# Setup environment
setup_environment() {
    log_header "Setting Up Environment"

    # Create logs directory
    mkdir -p "$LOGS_DIR"

    # Load environment variables
    if [ -f "$PROJECT_ROOT/.env" ]; then
        log_info "Loading environment variables from .env"
        set -a
        source "$PROJECT_ROOT/.env"
        set +a
    fi

    # Setup client app environment
    if [ ! -f "$CLIENT_APP_DIR/.env" ]; then
        log_info "Creating client app environment configuration"
        cat > "$CLIENT_APP_DIR/.env" << EOF
VITE_AI_CORE_API_URL=http://localhost:8801
VITE_AI_CORE_WS_URL=ws://localhost:8801/ws
VITE_AI_CORE_API_KEY=real-api-key
VITE_DEMO_MODE=real
VITE_ENABLE_MOCK_DATA=false
VITE_AUTO_CONNECT=true
VITE_API_TIMEOUT=30000
VITE_API_RETRY_ATTEMPTS=3
VITE_POLL_INTERVAL=1000
EOF
    fi

    log_success "Environment setup complete"
}

# Install dependencies
install_dependencies() {
    log_header "Installing Dependencies"

    # Install client app dependencies
    log_info "Installing client app dependencies..."
    cd "$CLIENT_APP_DIR"
    npm ci --silent
    cd "$PROJECT_ROOT"

    log_success "Dependencies installed"
}

# Start AI-CORE services
start_services() {
    log_header "Starting AI-CORE Services"

    # Make sure startup script is executable
    chmod +x "$PROJECT_ROOT/start-real-services.sh"

    # Start services
    log_info "Starting real AI-CORE services..."
    "$PROJECT_ROOT/start-real-services.sh"

    # Wait for services to be ready
    log_info "Waiting for services to
 initialize..."
    sleep 10

    # Verify services are running
    local services_healthy=true

    if ! curl -sf http://localhost:8801/health >/dev/null 2>&1; then
        log_error "Federation service not responding"
        services_healthy=false
    else
        log_success "Federation service: healthy"
    fi

    if ! curl -sf http://localhost:8802/health >/dev/null 2>&1; then
        log_error "Intent Parser service not responding"
        services_healthy=false
    else
        log_success "Intent Parser service: healthy"
    fi

    if ! curl -sf http://localhost:8803/health >/dev/null 2>&1; then
        log_error "MCP Manager proxy not responding"
        services_healthy=false
    else
        log_success "MCP Manager proxy: healthy"
    fi

    if ! curl -sf http://localhost:8804/health >/dev/null 2>&1; then
        log_error "Demo Content MCP not responding"
        services_healthy=false
    else
        log_success "Demo Content MCP: healthy"
    fi

    if [ "$services_healthy" = false ]; then
        log_error "Some services failed to start properly"
        exit 1
    fi

    log_success "All AI-CORE services are running"
}

# Build and start client app
start_client_app() {
    log_header "Building and Starting Client Application"

    cd "$CLIENT_APP_DIR"

    # Build the application
    log_info "Building client application..."
    npm run build --silent

    # Start development server
    log_info "Starting development server..."
    nohup npm run dev > "$LOGS_DIR/client-app.log" 2>&1 &
    CLIENT_PID=$!
    echo "$CLIENT_PID" >> "$PIDS_FILE"

    # Wait for client app to start
    log_info "Waiting for client app to start..."
    for i in {1..30}; do
        if curl -sf http://localhost:5173 >/dev/null 2>&1; then
            log_success "Client application is running on http://localhost:5173"
            break
        fi
        sleep 1
    done

    if ! curl -sf http://localhost:5173 >/dev/null 2>&1; then
        log_error "Client application failed to start"
        exit 1
    fi

    cd "$PROJECT_ROOT"
}

# Run integration tests
run_integration_tests() {
    log_header "Running Integration Tests"

    # Wait a moment for everything to stabilize
    sleep 5

    # Run the integration test
    log_info "Executing integration test suite..."
    if node "$PROJECT_ROOT/test-real-integration.js"; then
        log_success "All integration tests passed!"
        return 0
    else
        log_error "Integration tests failed"
        return 1
    fi
}

# Display deployment summary
show_deployment_summary() {
    log_header "Deployment Summary"

    echo -e "${CYAN}"
    echo "🎉 AI-CORE Real Integration Deployed Successfully!"
    echo ""
    echo "📊 Services Status:"
    echo "   • Federation Service:  http://localhost:8801 (REAL)"
    echo "   • Intent Parser:       http://localhost:8802 (REAL with Gemini AI)"
    echo "   • Demo Content MCP:    http://localhost:8804 (REAL with AI generation)"
    echo "   • MCP Proxy:          http://localhost:8803 (Port forwarding)"
    echo ""
    echo "🌐 Client Application:"
    echo "   • URL:                http://localhost:5173"
    echo "   • Mode:               Real AI-CORE Integration"
    echo "   • Status:             All services showing REAL status"
    echo ""
    echo "🔧 Management Commands:"
    echo "   • Stop all services:   ./deploy-real-integration.sh stop"
    echo "   • View logs:          tail -f logs/*.log"
    echo "   • Test integration:   node test-real-integration.js"
    echo ""
    echo "📋 Next Steps:"
    echo "   1. Open http://localhost:5173 in your browser"
    echo "   2. Try creating a workflow with real AI generation"
    echo "   3. Check execution logs for complete API interactions"
    echo "   4. Verify all services show 'REAL' status (no more mock warnings)"
    echo ""
    if [ -n "$GEMINI_API_KEY" ]; then
        echo "✨ Gemini AI integration is ACTIVE - you'll get real AI-generated content!"
    else
        echo "⚠️  Gemini API key not configured - using fallback content generation"
    fi
    echo -e "${NC}"
}

# Stop services
stop_services() {
    log_header "Stopping AI-CORE Integration"

    cleanup

    # Stop client app
    if pgrep -f "vite.*5173" >/dev/null; then
        pkill -f "vite.*5173"
        log_info "Client app stopped"
    fi

    # Stop AI-CORE services
    pkill -f "federation-simple|intent-parser|demo-content|python3.*8803|python3.*8804" 2>/dev/null || true

    log_success "All services stopped"
}

# Show status
show_status() {
    log_header "AI-CORE Integration Status"

    echo -e "${CYAN}Checking service status...${NC}\n"

    # Check each service
    check_service_status() {
        local name="$1"
        local url="$2"
        local expected_type="$3"

        if curl -sf "$url" >/dev/null 2>&1; then
            local response=$(curl -s "$url" 2>/dev/null)
            local service_name=$(echo "$response" | grep -o '"service":"[^"]*' | cut -d'"' -f4 2>/dev/null || echo "unknown")
            local status=$(echo "$response" | grep -o '"status":"[^"]*' | cut -d'"' -f4 2>/dev/null || echo "unknown")

            if [[ "$service_name" == *"mock"* && "$service_name" != *"proxy"* ]]; then
                echo -e "   • $name: ${YELLOW}MOCK${NC} ($status)"
            else
                echo -e "   • $name: ${GREEN}REAL${NC} ($status)"
            fi
        else
            echo -e "   • $name: ${RED}OFFLINE${NC}"
        fi
    }

    echo "🔧 AI-CORE Services:"
    check_service_status "Federation Service" "http://localhost:8801/health"
    check_service_status "Intent Parser" "http://localhost:8802/health"
    check_service_status "MCP Manager" "http://localhost:8803/health"
    check_service_status "Demo Content MCP" "http://localhost:8804/health"

    echo ""
    echo "🌐 Client Application:"
    if curl -sf http://localhost:5173 >/dev/null 2>&1; then
        echo -e "   • Client App: ${GREEN}RUNNING${NC} (http://localhost:5173)"
    else
        echo -e "   • Client App: ${RED}OFFLINE${NC}"
    fi

    echo ""
}

# Show help
show_help() {
    echo -e "${CYAN}"
    echo "AI-CORE Real Integration Deployment Script"
    echo ""
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  deploy    Deploy the complete AI-CORE real integration (default)"
    echo "  start     Start all services and client app"
    echo "  stop      Stop all services and client app"
    echo "  restart   Restart all services"
    echo "  status    Show current status of all services"
    echo "  test      Run integration tests"
    echo "  logs      Show recent logs from all services"
    echo "  help      Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  GEMINI_API_KEY    Your Gemini API key for real AI generation"
    echo ""
    echo "Examples:"
    echo "  $0                    # Deploy complete integration"
    echo "  $0 start              # Start services"
    echo "  $0 status             # Check service status"
    echo "  $0 test               # Run integration tests"
    echo -e "${NC}"
}

# Show logs
show_logs() {
    log_header "Recent Logs"

    echo -e "${CYAN}Federation Service:${NC}"
    tail -20 "$LOGS_DIR/federation.log" 2>/dev/null || echo "No federation logs found"

    echo -e "\n${CYAN}Intent Parser:${NC}"
    tail -20 "$LOGS_DIR/intent-parser.log" 2>/dev/null || echo "No intent parser logs found"

    echo -e "\n${CYAN}MCP Manager:${NC}"
    tail -20 "$LOGS_DIR/mcp-manager.log" 2>/dev/null || echo "No MCP manager logs found"

    echo -e "\n${CYAN}Client App:${NC}"
    tail -20 "$LOGS_DIR/client-app.log" 2>/dev/null || echo "No client app logs found"
}

# Main execution
main() {
    local command="${1:-deploy}"

    # Set trap for cleanup on exit
    trap cleanup EXIT

    case "$command" in
        deploy)
            log_header "🚀 AI-CORE Real Integration Deployment"
            check_prerequisites
            setup_environment
            install_dependencies
            start_services
            start_client_app

            if run_integration_tests; then
                show_deployment_summary
                log_success "🎉 Deployment completed successfully!"
            else
                log_error "Deployment completed but integration tests failed"
                exit 1
            fi
            ;;
        start)
            check_prerequisites
            setup_environment
            start_services
            start_client_app
            show_status
            ;;
        stop)
            stop_services
            ;;
        restart)
            stop_services
            sleep 3
            start_services
            start_client_app
            show_status
            ;;
        status)
            show_status
            ;;
        test)
            log_header "Running Integration Tests"
            node "$PROJECT_ROOT/test-real-integration.js"
            ;;
        logs)
            show_logs
            ;;
        help)
            show_help
            ;;
        *)
            log_error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Execute main function with all arguments
main "$@"
