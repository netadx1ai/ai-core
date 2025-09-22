#!/bin/bash

# AI-CORE MVP Quick Start Script
# Builds and runs all core services using Docker for 10-hour MVP demo

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="docker-compose.mvp.yml"
ENV_FILE=".env.mvp"

echo -e "${BLUE}🚀 AI-CORE MVP Quick Start${NC}"
echo "=============================="
echo -e "${PURPLE}Project Root: ${PROJECT_ROOT}${NC}"
echo ""

# Function to check prerequisites
check_prerequisites() {
    echo -e "${BLUE}🔍 Checking prerequisites...${NC}"

    # Check Docker
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker is not installed${NC}"
        exit 1
    fi

    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo -e "${RED}❌ Docker Compose is not installed${NC}"
        exit 1
    fi

    # Check if Docker is running
    if ! docker info &> /dev/null; then
        echo -e "${RED}❌ Docker is not running${NC}"
        exit 1
    fi

    echo -e "${GREEN}✅ All prerequisites met${NC}"
}

# Function to create environment file
create_env_file() {
    if [ ! -f "$ENV_FILE" ]; then
        echo -e "${YELLOW}📝 Creating environment file...${NC}"
        cat > "$ENV_FILE" << 'EOF'
# AI-CORE MVP Environment Variables
# For demo purposes - replace with real values for production

# OpenAI Configuration
OPENAI_API_KEY=demo-openai-key-replace-for-real-demo

# Anthropic Configuration
ANTHROPIC_API_KEY=demo-anthropic-key-replace-for-real-demo

# Google Calendar Configuration
GOOGLE_CLIENT_ID=demo-google-client-id
GOOGLE_CLIENT_SECRET=demo-google-client-secret

# Facebook Integration Configuration
FACEBOOK_ACCESS_TOKEN=demo-facebook-token
FACEBOOK_PAGE_ID=demo-facebook-page-id

# Service Configuration
RUST_LOG=info
NODE_ENV=development
PYTHONUNBUFFERED=1

# Database URLs (using existing dev databases)
DATABASE_URL=postgresql://postgres:postgres@host.docker.internal:5432/ai_core_dev
REDIS_URL=redis://host.docker.internal:6379
MONGODB_URL=mongodb://admin:password@host.docker.internal:27017/ai_core_dev
EOF
        echo -e "${GREEN}✅ Environment file created: $ENV_FILE${NC}"
        echo -e "${YELLOW}⚠️  Please update API keys in $ENV_FILE for full functionality${NC}"
    else
        echo -e "${GREEN}✅ Environment file exists: $ENV_FILE${NC}"
    fi
}

# Function to create simple services
create_simple_services() {
    echo -e "${BLUE}🏗️  Creating simple MVP services...${NC}"

    # Create directories
    mkdir -p src/services/intent-parser-simple/src
    mkdir -p src/services/mcp-manager-simple/src
    mkdir -p src/services/content-mcp-simple/src
    mkdir -p src/services/text-processing-mcp-simple/src
    mkdir -p external-mcps/image-generation
    mkdir -p external-mcps/calendar
    mkdir -p external-mcps/facebook
    mkdir -p demo2/frontend
    mkdir -p infrastructure/nginx

    echo -e "${GREEN}✅ Service directories created${NC}"
}

# Function to build services
build_services() {
    echo -e "${BLUE}🔨 Building services...${NC}"

    # Check if development databases are running
    if ! docker ps | grep -q AI-PLATFORM-postgres; then
        echo -e "${YELLOW}⚠️  Starting development databases first...${NC}"
        cd infrastructure/docker
        docker compose -f docker-compose.dev.yml up -d
        cd ../..
        echo -e "${GREEN}✅ Development databases started${NC}"
        sleep 5
    fi

    # Build MVP services
    docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" build --parallel

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ All services built successfully${NC}"
    else
        echo -e "${RED}❌ Service build failed${NC}"
        exit 1
    fi
}

# Function to start services
start_services() {
    echo -e "${BLUE}🚀 Starting MVP services...${NC}"

    # Start services
    docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" up -d

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ All services started successfully${NC}"
    else
        echo -e "${RED}❌ Failed to start services${NC}"
        exit 1
    fi
}

# Function to wait for services
wait_for_services() {
    echo -e "${BLUE}⏳ Waiting for services to be ready...${NC}"

    services=(
        "federation-simple:8801"
        "intent-parser:8802"
        "mcp-manager:8803"
        "content-mcp:8804"
        "text-processing-mcp:8805"
    )

    for service in "${services[@]}"; do
        name=$(echo "$service" | cut -d':' -f1)
        port=$(echo "$service" | cut -d':' -f2)

        echo -n "  Waiting for $name on port $port... "

        max_attempts=30
        attempt=1

        while [ $attempt -le $max_attempts ]; do
            if curl -s http://localhost:$port/health > /dev/null 2>&1; then
                echo -e "${GREEN}✅${NC}"
                break
            fi

            if [ $attempt -eq $max_attempts ]; then
                echo -e "${YELLOW}⚠️  (timeout, may still be starting)${NC}"
            else
                echo -n "."
                sleep 2
                attempt=$((attempt + 1))
            fi
        done
    done
}

# Function to show service status
show_status() {
    echo ""
    echo -e "${BLUE}📊 Service Status${NC}"
    echo "=================="

    docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" ps

    echo ""
    echo -e "${BLUE}🔗 Service URLs${NC}"
    echo "==============="
    echo -e "  • ${GREEN}Health Monitor:${NC}     http://localhost:8080"
    echo -e "  • ${GREEN}Demo UI:${NC}            http://localhost:3000"
    echo -e "  • ${GREEN}Federation API:${NC}     http://localhost:8801"
    echo -e "  • ${GREEN}Intent Parser:${NC}      http://localhost:8802"
    echo -e "  • ${GREEN}MCP Manager:${NC}        http://localhost:8803"
    echo -e "  • ${GREEN}Content MCP:${NC}        http://localhost:8804"
    echo -e "  • ${GREEN}Text Processing:${NC}    http://localhost:8805"
    echo -e "  • ${GREEN}Image Generation:${NC}   http://localhost:8806"
    echo -e "  • ${GREEN}Calendar MCP:${NC}       http://localhost:8807"
    echo -e "  • ${GREEN}Facebook MCP:${NC}       http://localhost:8808"

    echo ""
    echo -e "${BLUE}📋 Quick Test Commands${NC}"
    echo "======================="
    echo "  curl http://localhost:8801/health"
    echo "  curl http://localhost:8801/v1/mcps"
    echo ""
    echo -e "${BLUE}🛠️  Management Commands${NC}"
    echo "====================="
    echo "  Stop services:    ./scripts/stop-mvp-docker.sh"
    echo "  View logs:        docker compose -f $COMPOSE_FILE logs -f"
    echo "  Restart service:  docker compose -f $COMPOSE_FILE restart <service-name>"
}

# Function to run health checks
run_health_checks() {
    echo -e "${BLUE}🏥 Running health checks...${NC}"

    # Test core endpoints
    if curl -s http://localhost:8801/health | grep -q "healthy\|ok"; then
        echo -e "${GREEN}✅ Federation Service healthy${NC}"
    else
        echo -e "${YELLOW}⚠️  Federation Service may still be starting${NC}"
    fi

    # Test workflow creation
    workflow_response=$(curl -s -X POST http://localhost:8801/v1/workflows \
        -H "Content-Type: application/json" \
        -d '{"intent": "Create a test blog post about AI automation"}' || echo "failed")

    if echo "$workflow_response" | grep -q "workflow_id"; then
        echo -e "${GREEN}✅ Workflow creation successful${NC}"
    else
        echo -e "${YELLOW}⚠️  Workflow creation test pending${NC}"
    fi
}

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}🧹 Cleaning up...${NC}"

    # Stop services if requested
    if [ "$1" = "stop" ]; then
        docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" down
        echo -e "${GREEN}✅ Services stopped${NC}"
    fi
}

# Trap to cleanup on exit
trap 'cleanup' EXIT

# Main execution
main() {
    cd "$PROJECT_ROOT"

    # Parse arguments
    if [ "$1" = "stop" ]; then
        echo -e "${YELLOW}🛑 Stopping MVP services...${NC}"
        cleanup stop
        exit 0
    fi

    if [ "$1" = "restart" ]; then
        echo -e "${YELLOW}🔄 Restarting MVP services...${NC}"
        cleanup stop
        sleep 2
    fi

    # Execute setup steps
    check_prerequisites
    create_env_file
    create_simple_services
    build_services
    start_services
    wait_for_services
    show_status
    run_health_checks

    echo ""
    echo -e "${GREEN}🎉 AI-CORE MVP is now running!${NC}"
    echo -e "${BLUE}Visit http://localhost:3000 for the demo interface${NC}"
    echo -e "${YELLOW}💡 Use 'docker compose -f $COMPOSE_FILE logs -f' to view logs${NC}"
    echo -e "${YELLOW}💡 Use './scripts/start-mvp-docker.sh stop' to stop all services${NC}"
}

# Show help
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "AI-CORE MVP Quick Start Script"
    echo ""
    echo "Usage:"
    echo "  $0                Start all MVP services"
    echo "  $0 stop           Stop all MVP services"
    echo "  $0 restart        Restart all MVP services"
    echo "  $0 --help         Show this help"
    echo ""
    echo "Services included:"
    echo "  • Federation Service (Port 8801)"
    echo "  • Intent Parser (Port 8802)"
    echo "  • MCP Manager (Port 8803)"
    echo "  • Content Generation MCP (Port 8804)"
    echo "  • Text Processing MCP (Port 8805)"
    echo "  • Image Generation MCP (Port 8806)"
    echo "  • Calendar Management MCP (Port 8807)"
    echo "  • Facebook Posting MCP (Port 8808)"
    echo "  • Demo Web Interface (Port 3000)"
    echo "  • Health Monitor (Port 8080)"
    exit 0
fi

# Run main function
main "$@"
