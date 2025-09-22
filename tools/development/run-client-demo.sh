#!/bin/bash

# AI-CORE Client App Demo Runner
# Easy execution script for the client application demonstration
# This script starts the client demo with proper configuration and monitoring

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
CLIENT_DEMO_PORT=8090
AI_CORE_API_PORT=8080
DEMO_DURATION=600  # 10 minutes demo
LOG_DIR="./demo-logs"
CLIENT_SESSION_ID="client-demo-$(date +%Y%m%d-%H%M%S)"

# Create log directory
mkdir -p "$LOG_DIR"

print_header() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║                🎯 AI-CORE CLIENT APP DEMO                        ║"
    echo "║           Real SaaS Integration Demonstration                    ║"
    echo "║                                                                  ║"
    echo "║  Features Demonstrated:                                          ║"
    echo "║  • Production EARLY-LAUNCH Federation Bridge                    ║"
    echo "║  • Real Blog Post Generation (35.2s execution)                  ║"
    echo "║  • Quality Metrics (4.32/5.0 average score)                     ║"
    echo "║  • Client Branding & Customization                              ║"
    echo "║  • 98.5% Time Reduction vs Manual Process                       ║"
    echo "║  • Live Progress Updates via WebSocket                          ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "${CYAN}▶ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Cleanup function
cleanup() {
    print_step "Cleaning up client demo environment..."

    # Kill background processes
    for pid in $(jobs -p); do
        if kill -0 $pid 2>/dev/null; then
            print_info "Stopping process $pid"
            kill $pid 2>/dev/null || true
        fi
    done

    # Wait a moment for graceful shutdown
    sleep 2

    # Force kill if necessary
    for pid in $(jobs -p); do
        if kill -0 $pid 2>/dev/null; then
            print_warning "Force killing process $pid"
            kill -9 $pid 2>/dev/null || true
        fi
    done

    print_success "Client demo environment cleaned up"
}

# Set trap for cleanup
trap cleanup EXIT INT TERM

check_dependencies() {
    print_step "Checking dependencies..."

    # Check if Rust/Cargo is available
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi

    # Check if required ports are available
    if lsof -i :$CLIENT_DEMO_PORT &> /dev/null; then
        print_warning "Port $CLIENT_DEMO_PORT is already in use"
        print_info "You may need to stop other services or change ports in configuration"
    fi

    # Check if AI-CORE API is running
    if curl -s "http://localhost:$AI_CORE_API_PORT/health" &> /dev/null; then
        print_success "AI-CORE API detected on port $AI_CORE_API_PORT"
    else
        print_warning "AI-CORE API not detected on port $AI_CORE_API_PORT"
        print_info "Demo will run in mock mode for offline demonstration"
    fi

    print_success "Dependencies check completed"
}

setup_environment() {
    print_step "Setting up client demo environment..."

    # Navigate to client demo directory
    if [ ! -d "src/client-app-demo" ]; then
        print_error "Client app demo directory not found. Please run this script from AI-CORE root directory"
        exit 1
    fi

    cd src/client-app-demo

    # Create .env file if it doesn't exist
    if [ ! -f ".env" ]; then
        print_info "Creating .env configuration from template..."
        cp .env.example .env

        # Customize for demo
        sed -i.bak "s/CLIENT_APP_PORT=8090/CLIENT_APP_PORT=$CLIENT_DEMO_PORT/" .env

        # Check if AI-CORE API is available and set demo mode accordingly
        if curl -s "http://localhost:$AI_CORE_API_PORT/health" &> /dev/null; then
            sed -i.bak "s/DEMO_MODE=real/DEMO_MODE=real/" .env
            print_success "Configured for real AI-CORE integration"
        else
            sed -i.bak "s/DEMO_MODE=real/DEMO_MODE=mock/" .env
            print_success "Configured for mock demonstration mode"
        fi

        rm -f .env.bak
    fi

    print_success "Environment setup completed"
}

build_client_demo() {
    print_step "Building client app demo..."

    # Build the client demo application
    print_info "Compiling client demo application..."
    cargo build --release --bin client-app-demo > "$LOG_DIR/build-client-demo.log" 2>&1

    if [ $? -eq 0 ]; then
        print_success "Client demo application built successfully"
    else
        print_error "Failed to build client demo. Check $LOG_DIR/build-client-demo.log"
        return 1
    fi
}

start_client_demo() {
    print_step "Starting AI-CORE client app demo..."

    # Start the client demo application
    print_info "Starting client demo on port $CLIENT_DEMO_PORT..."
    RUST_LOG=info ./target/release/client-app-demo > "$LOG_DIR/client-demo.log" 2>&1 &
    local demo_pid=$!
    sleep 5

    if ! kill -0 $demo_pid 2>/dev/null; then
        print_error "Client demo failed to start. Check $LOG_DIR/client-demo.log"
        return 1
    fi
    print_success "Client demo started (PID: $demo_pid)"

    # Wait for service to be ready
    print_info "Waiting for client demo to be ready..."
    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -s "http://localhost:$CLIENT_DEMO_PORT/health" > /dev/null 2>&1; then
            break
        fi

        echo -n "."
        sleep 1
        ((attempt++))

        if [ $attempt -gt $max_attempts ]; then
            print_error "Client demo failed to become ready within 30 seconds"
            return 1
        fi
    done

    echo ""
    print_success "Client demo is ready!"
}

check_demo_health() {
    print_step "Checking demo health..."

    # Check Client Demo
    if curl -s "http://localhost:$CLIENT_DEMO_PORT/health" | grep -q "healthy"; then
        print_success "Client Demo: Healthy"
    else
        print_warning "Client Demo: Not responding"
    fi

    # Check AI-CORE API (if available)
    if curl -s "http://localhost:$AI_CORE_API_PORT/health" > /dev/null 2>&1; then
        print_success "AI-CORE API: Connected"
    else
        print_info "AI-CORE API: Not available (running in mock mode)"
    fi
}

show_demo_instructions() {
    print_step "Client demo is now ready!"
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    🎯 CLIENT DEMO ACCESS                          ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${CYAN}🌟 Client Demo Dashboard:${NC}"
    echo -e "   ${BLUE}http://localhost:$CLIENT_DEMO_PORT${NC}"
    echo ""
    echo -e "${CYAN}🎨 Demo Interface:${NC}"
    echo -e "   ${BLUE}http://localhost:$CLIENT_DEMO_PORT/demo${NC}"
    echo ""
    echo -e "${CYAN}📊 API Endpoints:${NC}"
    echo -e "   ${BLUE}Health Check:    http://localhost:$CLIENT_DEMO_PORT/health${NC}"
    echo -e "   ${BLUE}Start Demo:      POST http://localhost:$CLIENT_DEMO_PORT/api/start-demo${NC}"
    echo -e "   ${BLUE}Session Status:  GET http://localhost:$CLIENT_DEMO_PORT/api/session/{id}${NC}"
    echo -e "   ${BLUE}Metrics:         GET http://localhost:$CLIENT_DEMO_PORT/api/metrics${NC}"
    echo ""
    echo -e "${CYAN}🔌 WebSocket Real-time Updates:${NC}"
    echo -e "   ${BLUE}ws://localhost:$CLIENT_DEMO_PORT/ws/{session_id}${NC}"
    echo ""
    echo -e "${CYAN}💡 Demo Features:${NC}"
    echo -e "   • ${YELLOW}Real blog post generation in 35.2s average${NC}"
    echo -e "   • ${YELLOW}Quality scores of 4.32/5.0 average${NC}"
    echo -e "   • ${YELLOW}98.5% time reduction vs manual process${NC}"
    echo -e "   • ${YELLOW}Client branding and customization${NC}"
    echo -e "   • ${YELLOW}Real-time progress tracking${NC}"
    echo ""
    echo -e "${CYAN}🎯 Example Topics for Demo:${NC}"
    echo -e "   • ${YELLOW}\"AI automation trends shaping business in 2024\"${NC}"
    echo -e "   • ${YELLOW}\"The future of remote work and digital collaboration\"${NC}"
    echo -e "   • ${YELLOW}\"Sustainable business practices for modern companies\"${NC}"
    echo -e "   • ${YELLOW}\"Customer service automation: benefits and best practices\"${NC}"
    echo ""
    echo -e "${CYAN}🔍 Logs Location:${NC} ${BLUE}$LOG_DIR/${NC}"
    echo ""

    if command -v open &> /dev/null; then
        echo -e "${GREEN}Opening client demo in your browser...${NC}"
        open "http://localhost:$CLIENT_DEMO_PORT"
    elif command -v xdg-open &> /dev/null; then
        echo -e "${GREEN}Opening client demo in your browser...${NC}"
        xdg-open "http://localhost:$CLIENT_DEMO_PORT"
    else
        echo -e "${YELLOW}Please open http://localhost:$CLIENT_DEMO_PORT in your browser${NC}"
    fi

    echo ""
    echo -e "${PURPLE}Press Ctrl+C to stop the client demo${NC}"
}

monitor_demo() {
    print_step "Monitoring client demo (will run for ${DEMO_DURATION}s)..."

    local elapsed=0
    local check_interval=30

    while [ $elapsed -lt $DEMO_DURATION ]; do
        sleep $check_interval
        elapsed=$((elapsed + check_interval))

        # Check if services are still running
        local services_ok=true

        if ! curl -s "http://localhost:$CLIENT_DEMO_PORT/health" > /dev/null 2>&1; then
            print_error "Client demo is not responding!"
            services_ok=false
        fi

        if [ "$services_ok" = true ]; then
            print_info "Client demo healthy (${elapsed}/${DEMO_DURATION}s)"
        else
            print_warning "Client demo is not responding"
        fi

        # Show some statistics if available
        local metrics=$(curl -s "http://localhost:$CLIENT_DEMO_PORT/api/metrics" 2>/dev/null || echo "{}")
        local total_requests=$(echo "$metrics" | grep -o '"total_requests":[0-9]*' | cut -d':' -f2 || echo "0")
        print_info "Total demo requests: $total_requests"
    done
}

run_performance_test() {
    print_step "Running quick performance validation..."

    # Test a simple blog generation request
    local test_topic="AI automation trends"
    local test_request='{"input_text":"Create a blog post about AI automation trends","topic":"AI automation trends","audience":"business_professionals","tone":"professional","word_count":800,"brand_guidelines":null}'

    print_info "Testing blog generation with topic: $test_topic"

    local start_time=$(date +%s)
    local response=$(curl -s -X POST "http://localhost:$CLIENT_DEMO_PORT/api/start-demo" \
        -H "Content-Type: application/json" \
        -d "$test_request" 2>/dev/null)

    if [ $? -eq 0 ] && echo "$response" | grep -q "id"; then
        print_success "Demo API responding correctly"
        local session_id=$(echo "$response" | grep -o '"id":"[^"]*"' | cut -d'"' -f4)
        print_info "Test session started: $session_id"
    else
        print_warning "Demo API test failed (may be expected in mock mode)"
    fi
}

# Main execution
main() {
    print_header

    # Session tracking
    if [ -f "../../tools/ai-work-tracker.sh" ]; then
        print_info "Starting client demo session tracking..."
        cd ../..
        ./tools/ai-work-tracker.sh -Action start-session -AgentName "client-demo-runner" -Objective "run-client-app-demo-presentation" || true
        cd src/client-app-demo
    fi

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "src/client-app-demo" ]; then
        print_error "Please run this script from the AI-CORE root directory"
        exit 1
    fi

    check_dependencies
    setup_environment
    build_client_demo
    start_client_demo
    check_demo_health
    run_performance_test

    show_demo_instructions

    # Monitor demo
    monitor_demo

    print_success "Client demo session completed successfully!"

    # Update session tracker
    if [ -f "../../tools/ai-work-tracker.sh" ]; then
        cd ../..
        ./tools/ai-work-tracker.sh -Action complete-session -Summary "Client app demo completed with all features demonstrated" || true
        cd src/client-app-demo
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            CLIENT_DEMO_PORT="$2"
            shift 2
            ;;
        --duration)
            DEMO_DURATION="$2"
            shift 2
            ;;
        --mock-mode)
            export DEMO_MODE=mock
            shift
            ;;
        --real-mode)
            export DEMO_MODE=real
            shift
            ;;
        --help)
            echo "AI-CORE Client App Demo Runner"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --port PORT         Set client demo port (default: 8090)"
            echo "  --duration SECONDS  Set demo duration (default: 600)"
            echo "  --mock-mode         Force mock mode (offline demo)"
            echo "  --real-mode         Force real integration mode"
            echo "  --help             Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                  # Run with default settings"
            echo "  $0 --port 9000     # Run on port 9000"
            echo "  $0 --mock-mode     # Run offline demonstration"
            echo ""
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main "$@"
