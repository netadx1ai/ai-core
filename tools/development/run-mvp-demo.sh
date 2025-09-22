#!/bin/bash

# AI-CORE MVP Demo Runner
# Complete demonstration script for the AI-CORE platform MVP
# This script orchestrates the entire demo environment and workflow

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
INTENT_PARSER_PORT=8083
API_GATEWAY_PORT=8000
DEMO_DURATION=300  # 5 minutes demo
LOG_DIR="./demo-logs"
DEMO_SESSION_ID="mvp-demo-$(date +%Y%m%d-%H%M%S)"

# Create log directory
mkdir -p "$LOG_DIR"

print_header() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║                    🚀 AI-CORE MVP DEMO                           ║"
    echo "║                Complete Intelligent Automation Platform          ║"
    echo "║                                                                  ║"
    echo "║  Features Demonstrated:                                          ║"
    echo "║  • Natural Language → AI Intent Parsing                         ║"
    echo "║  • Real-time Workflow Orchestration                             ║"
    echo "║  • Content Generation via MCP Servers                           ║"
    echo "║  • Federation & Multi-client Coordination                       ║"
    echo "║  • Cost Optimization & Tracking                                 ║"
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
    print_step "Cleaning up demo environment..."

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

    print_success "Demo environment cleaned up"
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
    for port in $DEMO_PORT $CONTENT_MCP_PORT $INTENT_PARSER_PORT $API_GATEWAY_PORT; do
        if lsof -i :$port &> /dev/null; then
            print_warning "Port $port is already in use"
            print_info "You may need to stop other services or change ports in configuration"
        fi
    done

    print_success "Dependencies check completed"
}

build_services() {
    print_step "Building AI-CORE services..."

    # Build all services in parallel
    print_info "Building demo orchestrator..."
    cargo build --release --bin mvp-demo > "$LOG_DIR/build-demo.log" 2>&1 &
    local demo_build_pid=$!

    print_info "Building content MCP server..."
    cargo build --release --bin demo-content-mcp > "$LOG_DIR/build-content-mcp.log" 2>&1 &
    local content_build_pid=$!

    print_info "Building intent parser..."
    cargo build --release --bin intent-parser > "$LOG_DIR/build-intent-parser.log" 2>&1 &
    local intent_build_pid=$!

    print_info "Building API gateway..."
    cargo build --release --bin api-gateway > "$LOG_DIR/build-api-gateway.log" 2>&1 &
    local gateway_build_pid=$!

    # Wait for builds to complete
    wait $demo_build_pid || {
        print_error "Demo orchestrator build failed. Check $LOG_DIR/build-demo.log"
        return 1
    }

    wait $content_build_pid || {
        print_error "Content MCP build failed. Check $LOG_DIR/build-content-mcp.log"
        return 1
    }

    wait $intent_build_pid || {
        print_warning "Intent parser build failed (continuing without it). Check $LOG_DIR/build-intent-parser.log"
    }

    wait $gateway_build_pid || {
        print_warning "API gateway build failed (continuing without it). Check $LOG_DIR/build-api-gateway.log"
    }

    print_success "Services built successfully"
}

start_services() {
    print_step "Starting AI-CORE services..."

    # Start Content MCP Server
    print_info "Starting Content MCP Server on port $CONTENT_MCP_PORT..."
    RUST_LOG=demo_content_mcp=info ./target/release/demo-content-mcp > "$LOG_DIR/content-mcp.log" 2>&1 &
    local content_mcp_pid=$!
    sleep 3

    if ! kill -0 $content_mcp_pid 2>/dev/null; then
        print_error "Content MCP Server failed to start. Check $LOG_DIR/content-mcp.log"
        return 1
    fi
    print_success "Content MCP Server started (PID: $content_mcp_pid)"

    # Start Intent Parser (if available)
    if [ -f "./target/release/intent-parser" ]; then
        print_info "Starting Intent Parser on port $INTENT_PARSER_PORT..."
        RUST_LOG=intent_parser=info ./target/release/intent-parser > "$LOG_DIR/intent-parser.log" 2>&1 &
        local intent_parser_pid=$!
        sleep 2

        if kill -0 $intent_parser_pid 2>/dev/null; then
            print_success "Intent Parser started (PID: $intent_parser_pid)"
        else
            print_warning "Intent Parser failed to start (demo will continue)"
        fi
    fi

    # Start API Gateway (if available)
    if [ -f "./target/release/api-gateway" ]; then
        print_info "Starting API Gateway on port $API_GATEWAY_PORT..."
        RUST_LOG=ai_core_api_gateway=info ./target/release/api-gateway > "$LOG_DIR/api-gateway.log" 2>&1 &
        local gateway_pid=$!
        sleep 2

        if kill -0 $gateway_pid 2>/dev/null; then
            print_success "API Gateway started (PID: $gateway_pid)"
        else
            print_warning "API Gateway failed to start (demo will continue)"
        fi
    fi

    # Start Demo Orchestrator
    print_info "Starting Demo Orchestrator on port $DEMO_PORT..."
    RUST_LOG=ai_core_mvp_demo=info ./target/release/mvp-demo > "$LOG_DIR/demo-orchestrator.log" 2>&1 &
    local demo_pid=$!
    sleep 5

    if ! kill -0 $demo_pid 2>/dev/null; then
        print_error "Demo Orchestrator failed to start. Check $LOG_DIR/demo-orchestrator.log"
        return 1
    fi
    print_success "Demo Orchestrator started (PID: $demo_pid)"

    # Wait for services to be ready
    print_info "Waiting for services to be ready..."
    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -s "http://localhost:$DEMO_PORT/api/v1/health" > /dev/null 2>&1; then
            break
        fi

        echo -n "."
        sleep 1
        ((attempt++))

        if [ $attempt -gt $max_attempts ]; then
            print_error "Demo services failed to become ready within 30 seconds"
            return 1
        fi
    done

    echo ""
    print_success "All services are ready!"
}

check_service_health() {
    print_step "Checking service health..."

    # Check Demo Orchestrator
    if curl -s "http://localhost:$DEMO_PORT/api/v1/health" | grep -q "healthy"; then
        print_success "Demo Orchestrator: Healthy"
    else
        print_warning "Demo Orchestrator: Not responding"
    fi

    # Check Content MCP
    if curl -s "http://localhost:$CONTENT_MCP_PORT/health" | grep -q "healthy"; then
        print_success "Content MCP: Healthy"
    else
        print_warning "Content MCP: Not responding"
    fi

    # Check Intent Parser (if running)
    if curl -s "http://localhost:$INTENT_PARSER_PORT/health" > /dev/null 2>&1; then
        print_success "Intent Parser: Healthy"
    else
        print_info "Intent Parser: Not running (optional for demo)"
    fi

    # Check API Gateway (if running)
    if curl -s "http://localhost:$API_GATEWAY_PORT/health" > /dev/null 2>&1; then
        print_success "API Gateway: Healthy"
    else
        print_info "API Gateway: Not running (optional for demo)"
    fi
}

run_automated_demo() {
    print_step "Running automated demo scenarios..."

    # Demo scenarios to run
    local scenarios=(
        "Create a blog post about AI automation trends and schedule it on our WordPress site and LinkedIn"
        "Generate a social media campaign about our new product launch for Twitter, LinkedIn, and Facebook"
        "Create marketing content for Client A and publish it using Client B's premium publishing service"
    )

    for i in "${!scenarios[@]}"; do
        local scenario="${scenarios[$i]}"
        print_info "Running scenario $((i+1)): $scenario"

        # Start demo workflow
        local response=$(curl -s -X POST "http://localhost:$DEMO_PORT/api/v1/demo/start" \
            -H "Content-Type: application/json" \
            -d "{\"input\": \"$scenario\"}")

        if [ $? -eq 0 ]; then
            local workflow_id=$(echo "$response" | grep -o '"workflow_id":"[^"]*"' | cut -d'"' -f4)
            if [ -n "$workflow_id" ]; then
                print_success "Started workflow: $workflow_id"

                # Monitor progress for 30 seconds
                local monitor_time=30
                print_info "Monitoring progress for ${monitor_time}s..."

                for ((j=0; j<monitor_time; j++)); do
                    local status=$(curl -s "http://localhost:$DEMO_PORT/api/v1/workflows/$workflow_id" | \
                        grep -o '"status":"[^"]*"' | cut -d'"' -f4)

                    if [ "$status" = "Completed" ]; then
                        print_success "Workflow completed!"
                        break
                    elif [ "$status" = "Failed" ]; then
                        print_error "Workflow failed!"
                        break
                    fi

                    echo -n "."
                    sleep 1
                done
                echo ""
            else
                print_warning "Failed to extract workflow ID from response"
            fi
        else
            print_warning "Failed to start demo scenario"
        fi

        echo ""
    done
}

show_demo_instructions() {
    print_step "Demo is now ready!"
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
    echo -e "${CYAN}📊 WebSocket Real-time Updates:${NC}"
    echo -e "   ${BLUE}ws://localhost:$DEMO_PORT/ws/{workflow_id}${NC}"
    echo ""
    echo -e "${CYAN}💡 Try These Demo Scenarios:${NC}"
    echo -e "   • ${YELLOW}\"Create a blog post about AI automation and publish it to WordPress\"${NC}"
    echo -e "   • ${YELLOW}\"Generate a social media campaign for our product launch\"${NC}"
    echo -e "   • ${YELLOW}\"Create marketing content using federated client systems\"${NC}"
    echo ""
    echo -e "${CYAN}🔍 Logs Location:${NC} ${BLUE}$LOG_DIR/${NC}"
    echo ""

    if command -v open &> /dev/null; then
        echo -e "${GREEN}Opening demo dashboard in your browser...${NC}"
        open "http://localhost:$DEMO_PORT"
    elif command -v xdg-open &> /dev/null; then
        echo -e "${GREEN}Opening demo dashboard in your browser...${NC}"
        xdg-open "http://localhost:$DEMO_PORT"
    else
        echo -e "${YELLOW}Please open http://localhost:$DEMO_PORT in your browser${NC}"
    fi

    echo ""
    echo -e "${PURPLE}Press Ctrl+C to stop the demo${NC}"
}

monitor_demo() {
    print_step "Monitoring demo (will run for ${DEMO_DURATION}s)..."

    local elapsed=0
    local check_interval=30

    while [ $elapsed -lt $DEMO_DURATION ]; do
        sleep $check_interval
        elapsed=$((elapsed + check_interval))

        # Check if services are still running
        local services_ok=true

        if ! curl -s "http://localhost:$DEMO_PORT/api/v1/health" > /dev/null 2>&1; then
            print_error "Demo orchestrator is not responding!"
            services_ok=false
        fi

        if ! curl -s "http://localhost:$CONTENT_MCP_PORT/health" > /dev/null 2>&1; then
            print_error "Content MCP is not responding!"
            services_ok=false
        fi

        if [ "$services_ok" = true ]; then
            print_info "All services healthy (${elapsed}/${DEMO_DURATION}s)"
        else
            print_warning "Some services are not responding"
        fi

        # Show some statistics if available
        local workflows=$(curl -s "http://localhost:$DEMO_PORT/api/v1/scenarios" | grep -o '"id"' | wc -l || echo "0")
        print_info "Available scenarios: $workflows"
    done
}

# Main execution
main() {
    print_header

    # Session tracking
    if [ -f "./tools/ai-work-tracker.sh" ]; then
        print_info "Starting demo session tracking..."
        ./tools/ai-work-tracker.sh -Action start-session -AgentName "mvp-demo-runner" -Objective "run-complete-mvp-demonstration" || true
    fi

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "src/demo" ]; then
        print_error "Please run this script from the AI-CORE root directory"
        exit 1
    fi

    check_dependencies
    build_services
    start_services
    check_service_health

    # Run automated scenarios first
    print_info "Running automated demo scenarios..."
    run_automated_demo

    show_demo_instructions

    # Monitor demo
    monitor_demo

    print_success "Demo completed successfully!"

    # Update session tracker
    if [ -f "./tools/ai-work-tracker.sh" ]; then
        ./tools/ai-work-tracker.sh -Action complete-session -Summary "MVP demo completed with all core features demonstrated" || true
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            DEMO_PORT="$2"
            shift 2
            ;;
        --duration)
            DEMO_DURATION="$2"
            shift 2
            ;;
        --automated-only)
            AUTOMATED_ONLY=true
            shift
            ;;
        --help)
            echo "AI-CORE MVP Demo Runner"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --port PORT         Set demo port (default: 8080)"
            echo "  --duration SECONDS  Set demo duration (default: 300)"
            echo "  --automated-only    Run automated scenarios only"
            echo "  --help             Show this help message"
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
