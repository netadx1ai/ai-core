#!/bin/bash

# AI-CORE Working Demo Script
# Simple demonstration of the fixed MVP services

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
DEMO_PORT=8080
CONTENT_MCP_PORT=8081
LOG_DIR="./demo-logs"
DEMO_PIDS=()

# Create log directory
mkdir -p "$LOG_DIR"

print_header() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║                    🚀 AI-CORE WORKING DEMO                       ║"
    echo "║                                                                  ║"
    echo "║  Fixed and Working MVP Components:                               ║"
    echo "║  • Demo Orchestrator (Complete UI + WebSocket)                  ║"
    echo "║  • Content MCP Server (Working API)                             ║"
    echo "║  • Real-time Progress Updates                                    ║"
    echo "║  • Cost Tracking & Federation Demo                              ║"
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
    print_step "Cleaning up demo..."

    for pid in "${DEMO_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            print_info "Stopping process $pid"
            kill "$pid" 2>/dev/null || true
        fi
    done

    # Wait for graceful shutdown
    sleep 2

    # Force kill any remaining processes
    pkill mvp-demo || true
    pkill demo-content-mcp || true

    print_success "Demo cleanup completed"
}

# Set trap for cleanup
trap cleanup EXIT INT TERM

check_build() {
    print_step "Checking if services are built..."

    if [ ! -f "./target/release/mvp-demo" ]; then
        print_info "Building demo orchestrator..."
        cargo build --release --bin mvp-demo > "$LOG_DIR/build-demo.log" 2>&1 || {
            print_error "Demo orchestrator build failed. Check $LOG_DIR/build-demo.log"
            return 1
        }
    fi

    if [ ! -f "./target/release/demo-content-mcp" ]; then
        print_info "Building content MCP..."
        cargo build --release --bin demo-content-mcp > "$LOG_DIR/build-content-mcp.log" 2>&1 || {
            print_error "Content MCP build failed. Check $LOG_DIR/build-content-mcp.log"
            return 1
        }
    fi

    print_success "All services are built"
}

start_services() {
    print_step "Starting AI-CORE services..."

    # Check if ports are available
    if lsof -i :$CONTENT_MCP_PORT &> /dev/null; then
        print_warning "Port $CONTENT_MCP_PORT is in use, attempting to free it..."
        pkill demo-content-mcp || true
        sleep 2
    fi

    if lsof -i :$DEMO_PORT &> /dev/null; then
        print_warning "Port $DEMO_PORT is in use, attempting to free it..."
        pkill mvp-demo || true
        sleep 2
    fi

    # Start Content MCP Server
    print_info "Starting Content MCP Server on port $CONTENT_MCP_PORT..."
    RUST_LOG=demo_content_mcp=info ./target/release/demo-content-mcp > "$LOG_DIR/content-mcp.log" 2>&1 &
    local content_mcp_pid=$!
    DEMO_PIDS+=($content_mcp_pid)

    # Wait for Content MCP to be ready
    local attempts=0
    while [ $attempts -lt 10 ]; do
        if curl -s "http://localhost:$CONTENT_MCP_PORT/health" > /dev/null 2>&1; then
            print_success "Content MCP Server ready (PID: $content_mcp_pid)"
            break
        fi
        sleep 1
        ((attempts++))
    done

    if [ $attempts -eq 10 ]; then
        print_error "Content MCP Server failed to start"
        return 1
    fi

    # Start Demo Orchestrator
    print_info "Starting Demo Orchestrator on port $DEMO_PORT..."
    RUST_LOG=ai_core_mvp_demo=info ./target/release/mvp-demo > "$LOG_DIR/demo-orchestrator.log" 2>&1 &
    local demo_pid=$!
    DEMO_PIDS+=($demo_pid)

    # Wait for Demo Orchestrator to be ready
    local attempts=0
    while [ $attempts -lt 15 ]; do
        if curl -s "http://localhost:$DEMO_PORT/api/v1/health" > /dev/null 2>&1; then
            print_success "Demo Orchestrator ready (PID: $demo_pid)"
            break
        fi
        sleep 1
        ((attempts++))
    done

    if [ $attempts -eq 15 ]; then
        print_error "Demo Orchestrator failed to start"
        return 1
    fi

    print_success "All services started successfully!"
}

test_services() {
    print_step "Testing service health..."

    # Test Content MCP
    local content_health=$(curl -s "http://localhost:$CONTENT_MCP_PORT/health" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    if [ "$content_health" = "healthy" ]; then
        print_success "Content MCP: Healthy"
    else
        print_warning "Content MCP: Not responding properly"
    fi

    # Test Demo Orchestrator
    local demo_health=$(curl -s "http://localhost:$DEMO_PORT/api/v1/health" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    if [ "$demo_health" = "healthy" ]; then
        print_success "Demo Orchestrator: Healthy"
    else
        print_warning "Demo Orchestrator: Not responding properly"
    fi

    # Test Content Generation
    print_info "Testing content generation..."
    local content_response=$(curl -s -X POST "http://localhost:$CONTENT_MCP_PORT/v1/content/generate" \
        -H "Content-Type: application/json" \
        -d '{"content_type": "blog_post", "topic": "AI automation test"}')

    if echo "$content_response" | grep -q '"status":"completed"'; then
        print_success "Content generation working"
    else
        print_warning "Content generation may have issues"
    fi
}

run_demo_scenario() {
    print_step "Running automated demo scenario..."

    local demo_request='{"input": "Create a blog post about AI automation trends for business users"}'
    local response=$(curl -s -X POST "http://localhost:$DEMO_PORT/api/v1/demo/start" \
        -H "Content-Type: application/json" \
        -d "$demo_request")

    if echo "$response" | grep -q '"workflow_id"'; then
        local workflow_id=$(echo "$response" | grep -o '"workflow_id":"[^"]*"' | cut -d'"' -f4)
        print_success "Demo scenario started with workflow ID: $workflow_id"

        print_info "Monitoring workflow progress..."
        for i in {1..30}; do
            local status=$(curl -s "http://localhost:$DEMO_PORT/api/v1/workflows/$workflow_id" | \
                grep -o '"status":"[^"]*"' | cut -d'"' -f4)

            case "$status" in
                "Completed")
                    print_success "Workflow completed successfully!"
                    return 0
                    ;;
                "Failed")
                    print_error "Workflow failed"
                    return 1
                    ;;
                "Executing"|"Planning"|"Parsing")
                    echo -n "."
                    ;;
            esac

            sleep 2
        done
        echo ""
        print_warning "Workflow still running after 60 seconds"
    else
        print_error "Failed to start demo scenario"
        return 1
    fi
}

show_demo_info() {
    print_step "Demo is ready!"
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    🎯 DEMO ACCESS POINTS                          ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${CYAN}🌟 Main Demo Dashboard:${NC}"
    echo -e "   ${BLUE}http://localhost:$DEMO_PORT${NC}"
    echo ""
    echo -e "${CYAN}🔌 API Endpoints:${NC}"
    echo -e "   ${BLUE}Demo API:        http://localhost:$DEMO_PORT/api/v1/${NC}"
    echo -e "   ${BLUE}Content MCP:     http://localhost:$CONTENT_MCP_PORT/v1/${NC}"
    echo -e "   ${BLUE}Health Checks:   http://localhost:$DEMO_PORT/api/v1/health${NC}"
    echo ""
    echo -e "${CYAN}💡 Try These Commands:${NC}"
    echo -e "   ${YELLOW}curl -X POST http://localhost:$DEMO_PORT/api/v1/demo/start \\${NC}"
    echo -e "   ${YELLOW}  -H \"Content-Type: application/json\" \\${NC}"
    echo -e "   ${YELLOW}  -d '{\"input\": \"Create a blog post about AI automation\"}'${NC}"
    echo ""
    echo -e "${CYAN}📊 Real-time Updates:${NC}"
    echo -e "   ${BLUE}WebSocket: ws://localhost:$DEMO_PORT/ws/{workflow_id}${NC}"
    echo ""
    echo -e "${CYAN}🔍 Logs:${NC}"
    echo -e "   ${BLUE}Demo Orchestrator: $LOG_DIR/demo-orchestrator.log${NC}"
    echo -e "   ${BLUE}Content MCP:       $LOG_DIR/content-mcp.log${NC}"
    echo ""

    # Try to open browser
    if command -v open &> /dev/null; then
        print_info "Opening demo dashboard in your browser..."
        open "http://localhost:$DEMO_PORT"
    elif command -v xdg-open &> /dev/null; then
        print_info "Opening demo dashboard in your browser..."
        xdg-open "http://localhost:$DEMO_PORT"
    else
        echo -e "${GREEN}Please open http://localhost:$DEMO_PORT in your browser${NC}"
    fi

    echo ""
    echo -e "${PURPLE}Press Ctrl+C to stop the demo${NC}"
}

# Main execution
main() {
    print_header

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "src/demo" ]; then
        print_error "Please run this script from the AI-CORE root directory"
        exit 1
    fi

    check_build
    start_services
    test_services
    run_demo_scenario
    show_demo_info

    # Keep demo running
    print_step "Demo running... Press Ctrl+C to stop"
    while true; do
        sleep 10

        # Check if services are still running
        local services_ok=true
        for pid in "${DEMO_PIDS[@]}"; do
            if ! kill -0 "$pid" 2>/dev/null; then
                print_warning "A service has stopped unexpectedly"
                services_ok=false
                break
            fi
        done

        if [ "$services_ok" = true ]; then
            # Quick health check
            if ! curl -s "http://localhost:$DEMO_PORT/api/v1/health" > /dev/null 2>&1; then
                print_warning "Demo orchestrator not responding"
            fi
        fi
    done
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            DEMO_PORT="$2"
            shift 2
            ;;
        --content-port)
            CONTENT_MCP_PORT="$2"
            shift 2
            ;;
        --help)
            echo "AI-CORE Working Demo Script"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --port PORT           Set demo port (default: 8080)"
            echo "  --content-port PORT   Set content MCP port (default: 8081)"
            echo "  --help               Show this help message"
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
