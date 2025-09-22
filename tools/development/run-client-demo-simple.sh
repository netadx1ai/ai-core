#!/bin/bash

# AI-CORE Client Demo Simple Runner
# Sets up environment and runs the improved client demo with MVP design

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

print_header() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════════╗"
    echo "║          🎯 AI-CORE CLIENT DEMO - IMPROVED VERSION              ║"
    echo "║                                                                  ║"
    echo "║  ✅ MVP Demo Design (Gradient + Glass-morphism)                  ║"
    echo "║  ✅ Real EARLY-LAUNCH Integration (Port 8801)                   ║"
    echo "║  ✅ Enhanced UI with Workflow Diagram                           ║"
    echo "║  ✅ Professional Metrics & Branding                             ║"
    echo "╚══════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

setup_environment() {
    echo -e "${BLUE}▶ Setting up environment...${NC}"

    # Navigate to client demo directory
    cd src/client-app-demo

    # Set environment variables
    export CLIENT_APP_HOST=0.0.0.0
    export CLIENT_APP_PORT=8090
    export CLIENT_NAME="EARLY-LAUNCH Client Demo"

    # EARLY-LAUNCH Federation Bridge (Real Integration)
    export AI_CORE_API_URL=http://localhost:8801
    export AI_CORE_API_KEY=demo-client-key-12345

    # Demo mode - real integration by default
    export DEMO_MODE=real

    # MVP Demo Branding (Purple/Blue gradient theme)
    export COMPANY_NAME="Your Company"
    export LOGO_URL=""
    export PRIMARY_COLOR="#667eea"
    export SECONDARY_COLOR="#764ba2"
    export FONT_FAMILY="Segoe UI, Tahoma, Geneva, Verdana, sans-serif"

    # Performance settings
    export REQUEST_TIMEOUT=120
    export MAX_RETRIES=3
    export CONNECTION_POOL_SIZE=10

    # Feature flags
    export ENABLE_WEBSOCKET=true
    export ENABLE_REAL_TIME_UPDATES=true
    export ENABLE_ANALYTICS=false
    export ENABLE_MOCK_FALLBACK=true

    # Health check settings
    export HEALTH_CHECK_INTERVAL=30
    export HEALTH_CHECK_TIMEOUT=5

    # Demo settings
    export DEMO_DURATION_SECONDS=600
    export AUTO_REFRESH_METRICS=true
    export SHOW_CONNECTION_STATUS=true

    # Logging
    export RUST_LOG=info
    export DEBUG_MODE=false

    echo -e "${GREEN}✅ Environment configured${NC}"
}

check_federation_service() {
    echo -e "${BLUE}▶ Checking EARLY-LAUNCH federation service...${NC}"

    if curl -s http://localhost:8801/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Federation service detected on port 8801${NC}"
        echo -e "${GREEN}   Demo will run in REAL integration mode${NC}"
        export INTEGRATION_STATUS="REAL"
    else
        echo -e "${YELLOW}⚠️  Federation service not detected on port 8801${NC}"
        echo -e "${YELLOW}   Demo will run in mock mode with real integration UI${NC}"
        export INTEGRATION_STATUS="MOCK"
    fi
}

build_demo() {
    echo -e "${BLUE}▶ Building client demo...${NC}"

    if [ ! -f "../../target/release/client-app-demo" ]; then
        echo -e "${BLUE}   Building release binary...${NC}"
        cargo build --release > /dev/null 2>&1
        echo -e "${GREEN}✅ Demo built successfully${NC}"
    else
        echo -e "${GREEN}✅ Demo binary already exists${NC}"
    fi
}

run_demo() {
    echo -e "${BLUE}▶ Starting client demo...${NC}"

    echo -e "${GREEN}🚀 Configuration:${NC}"
    echo -e "   • ${BLUE}Demo URL:${NC} http://localhost:$CLIENT_APP_PORT"
    echo -e "   • ${BLUE}Federation API:${NC} $AI_CORE_API_URL"
    echo -e "   • ${BLUE}Integration Mode:${NC} $INTEGRATION_STATUS"
    echo -e "   • ${BLUE}Design Theme:${NC} MVP Demo (Gradient)"
    echo ""

    echo -e "${GREEN}🎨 UI Features:${NC}"
    echo -e "   • ${BLUE}Gradient Background:${NC} Purple to Blue (#667eea → #764ba2)"
    echo -e "   • ${BLUE}Glass-morphism Cards:${NC} Backdrop blur effects"
    echo -e "   • ${BLUE}Workflow Diagram:${NC} Interactive data flow visualization"
    echo -e "   • ${BLUE}Enhanced Metrics:${NC} 8 feature cards with animations"
    echo -e "   • ${BLUE}EARLY-LAUNCH Branding:${NC} Professional badges and indicators"
    echo ""

    echo -e "${GREEN}⚡ Performance Targets:${NC}"
    echo -e "   • ${BLUE}Blog Generation:${NC} ~35.2 seconds"
    echo -e "   • ${BLUE}Quality Score:${NC} 4.32/5.0 average"
    echo -e "   • ${BLUE}Success Rate:${NC} 97.8%"
    echo -e "   • ${BLUE}Cost per Request:${NC} $0.47"
    echo ""

    echo -e "${YELLOW}Starting demo server...${NC}"
    echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
    echo ""

    # Start the demo
    ../../target/release/client-app-demo
}

cleanup() {
    echo -e "\n${BLUE}▶ Cleaning up...${NC}"
    cd ../../
}

trap cleanup EXIT

# Main execution
main() {
    print_header

    # Check if we're in the AI-CORE root directory
    if [ ! -f "Cargo.toml" ] || [ ! -d "src/client-app-demo" ]; then
        echo -e "${RED}❌ Please run this script from the AI-CORE root directory${NC}"
        exit 1
    fi

    setup_environment
    check_federation_service
    build_demo
    echo ""
    run_demo
}

# Parse arguments
case "${1:-}" in
    --help|-h)
        echo "AI-CORE Client Demo Simple Runner"
        echo ""
        echo "This script runs the improved client demo with:"
        echo "  • MVP demo design (gradient theme)"
        echo "  • Real EARLY-LAUNCH integration (port 8801)"
        echo "  • Enhanced UI with workflow diagram"
        echo "  • Professional metrics and branding"
        echo ""
        echo "Usage: $0 [--help]"
        echo ""
        echo "The demo will be available at: http://localhost:8090"
        exit 0
        ;;
    "")
        main
        ;;
    *)
        echo "Unknown option: $1"
        echo "Use --help for usage information"
        exit 1
        ;;
esac
