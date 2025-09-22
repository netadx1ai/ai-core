#!/bin/bash

# AI-CORE Client App Integration - Build and Run Script
# Production-ready automation for build, test, and deployment

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_NAME="AI-CORE Client App Integration"
VERSION="1.0.0"
NODE_MIN_VERSION="16.0.0"
BUILD_DIR="dist"
TEST_RESULTS_DIR="test-results"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."

    # Check Node.js
    if ! command -v node &> /dev/null; then
        print_error "Node.js is not installed. Please install Node.js $NODE_MIN_VERSION or higher."
        exit 1
    fi

    NODE_VERSION=$(node -v | sed 's/v//')
    print_status "Node.js version: $NODE_VERSION"

    # Check npm
    if ! command -v npm &> /dev/null; then
        print_error "npm is not installed. Please install npm."
        exit 1
    fi

    NPM_VERSION=$(npm -v)
    print_status "npm version: $NPM_VERSION"

    print_success "Prerequisites check passed"
}

# Function to install dependencies
install_dependencies() {
    print_status "Installing dependencies..."

    if [ ! -f "package.json" ]; then
        print_error "package.json not found. Are you in the correct directory?"
        exit 1
    fi

    npm ci --silent
    print_success "Dependencies installed successfully"
}

# Function to run linting
run_lint() {
    print_status "Running ESLint..."

    if npm run lint; then
        print_success "Linting passed"
    else
        print_error "Linting failed. Please fix the issues before proceeding."
        exit 1
    fi
}

# Function to run type checking
run_type_check() {
    print_status "Running TypeScript type checking..."

    if npx tsc --noEmit; then
        print_success "Type checking passed"
    else
        print_error "Type checking failed. Please fix the type errors."
        exit 1
    fi
}

# Function to run unit tests
run_unit_tests() {
    print_status "Running unit tests with Jest..."

    # Create test results directory
    mkdir -p "$TEST_RESULTS_DIR"

    if npm run test -- --coverage --watchAll=false --passWithNoTests; then
        print_success "Unit tests passed"
    else
        print_error "Unit tests failed"
        exit 1
    fi
}

# Function to build the application
build_app() {
    print_status "Building application for production..."

    # Clean previous build
    if [ -d "$BUILD_DIR" ]; then
        rm -rf "$BUILD_DIR"
        print_status "Cleaned previous build"
    fi

    # Build the app
    if npm run build; then
        print_success "Build completed successfully"

        # Display build info
        if [ -d "$BUILD_DIR" ]; then
            BUILD_SIZE=$(du -sh "$BUILD_DIR" | cut -f1)
            print_status "Build size: $BUILD_SIZE"

            # List main build files
            print_status "Build files:"
            find "$BUILD_DIR" -type f -name "*.js" -o -name "*.css" -o -name "*.html" | head -10
        fi
    else
        print_error "Build failed"
        exit 1
    fi
}

# Function to run E2E tests
run_e2e_tests() {
    print_status "Running E2E tests with Playwright..."

    # Install Playwright browsers if needed
    if ! npx playwright install --dry-run &> /dev/null; then
        print_status "Installing Playwright browsers..."
        npx playwright install
    fi

    # Start dev server in background for E2E tests
    print_status "Starting development server for E2E tests..."
    npm run dev &
    DEV_SERVER_PID=$!

    # Wait for server to be ready
    print_status "Waiting for server to start..."
    for i in {1..30}; do
        if curl -s http://localhost:5173 > /dev/null 2>&1; then
            print_success "Development server is ready"
            break
        fi

        if [ $i -eq 30 ]; then
            print_error "Development server failed to start"
            kill $DEV_SERVER_PID 2>/dev/null || true
            exit 1
        fi

        sleep 2
    done

    # Run E2E tests
    if npm run test:e2e; then
        print_success "E2E tests passed"
    else
        print_error "E2E tests failed"
        E2E_FAILED=true
    fi

    # Kill dev server
    kill $DEV_SERVER_PID 2>/dev/null || true
    print_status "Development server stopped"

    if [ "$E2E_FAILED" = true ]; then
        exit 1
    fi
}

# Function to run the preview server
run_preview() {
    print_status "Starting preview server..."

    if [ ! -d "$BUILD_DIR" ]; then
        print_error "Build directory not found. Please run build first."
        exit 1
    fi

    print_success "Preview server starting at http://localhost:4173"
    print_status "Press Ctrl+C to stop the server"

    npm run preview
}

# Function to run development server
run_dev() {
    print_status "Starting development server..."

    print_success "Development server starting at http://localhost:5173"
    print_status "Press Ctrl+C to stop the server"

    npm run dev
}

# Function to run continuous integration pipeline
run_ci() {
    print_status "Running continuous integration pipeline..."

    check_prerequisites
    install_dependencies
    run_lint
    run_type_check
    run_unit_tests
    build_app
    run_e2e_tests

    print_success "✅ All CI checks passed! Application is ready for deployment."
}

# Function to create Docker build
build_docker() {
    print_status "Building Docker image..."

    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed. Please install Docker to build container images."
        exit 1
    fi

    # Create Dockerfile if it doesn't exist
    if [ ! -f "Dockerfile" ]; then
        create_dockerfile
    fi

    DOCKER_TAG="ai-core-client-integration:$VERSION"

    if docker build -t "$DOCKER_TAG" .; then
        print_success "Docker image built successfully: $DOCKER_TAG"
    else
        print_error "Docker build failed"
        exit 1
    fi
}

# Function to create Dockerfile
create_dockerfile() {
    print_status "Creating Dockerfile..."

    cat > Dockerfile << 'EOF'
# Multi-stage build for production
FROM node:18-alpine AS builder

WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

# Production stage
FROM nginx:alpine

# Copy custom nginx config
COPY nginx.conf /etc/nginx/nginx.conf

# Copy built app
COPY --from=builder /app/dist /usr/share/nginx/html

# Add health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:80 || exit 1

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
EOF

    print_success "Dockerfile created"
}

# Function to clean build artifacts
clean() {
    print_status "Cleaning build artifacts..."

    rm -rf "$BUILD_DIR"
    rm -rf "$TEST_RESULTS_DIR"
    rm -rf "coverage"
    rm -rf "node_modules/.cache"
    rm -rf "playwright-report"

    print_success "Clean completed"
}

# Function to show help
show_help() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  dev         Start development server"
    echo "  build       Build application for production"
    echo "  preview     Preview production build"
    echo "  test        Run unit tests"
    echo "  test:e2e    Run E2E tests"
    echo "  lint        Run ESLint"
    echo "  typecheck   Run TypeScript type checking"
    echo "  ci          Run complete CI pipeline"
    echo "  docker      Build Docker image"
    echo "  clean       Clean build artifacts"
    echo "  install     Install dependencies"
    echo "  help        Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 dev                Start development"
    echo "  $0 ci                 Run full CI pipeline"
    echo "  $0 build && $0 preview   Build and preview"
    echo "  $0 docker             Build Docker container"
}

# Function to display banner
show_banner() {
    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                                                              ║"
    echo "║        🚀 AI-CORE Client App Integration Builder            ║"
    echo "║                                                              ║"
    echo "║        Production-ready build and deployment automation      ║"
    echo "║        Version: $VERSION                                  ║"
    echo "║                                                              ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo ""
}

# Main execution
main() {
    show_banner

    case "${1:-help}" in
        "dev")
            check_prerequisites
            install_dependencies
            run_dev
            ;;
        "build")
            check_prerequisites
            install_dependencies
            run_lint
            run_type_check
            build_app
            ;;
        "preview")
            run_preview
            ;;
        "test")
            check_prerequisites
            install_dependencies
            run_unit_tests
            ;;
        "test:e2e")
            check_prerequisites
            install_dependencies
            build_app
            run_e2e_tests
            ;;
        "lint")
            check_prerequisites
            install_dependencies
            run_lint
            ;;
        "typecheck")
            check_prerequisites
            install_dependencies
            run_type_check
            ;;
        "ci")
            run_ci
            ;;
        "docker")
            check_prerequisites
            install_dependencies
            build_app
            build_docker
            ;;
        "clean")
            clean
            ;;
        "install")
            check_prerequisites
            install_dependencies
            ;;
        "help"|"--help"|"-h")
            show_help
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
