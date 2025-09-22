#!/bin/bash

# AI-PLATFORM QA Agent Setup and Usage Script
# Comprehensive setup, validation, and execution script for the QA framework

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
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
QA_CONFIG_DIR="${PROJECT_ROOT}/config/qa"
QA_REPORTS_DIR="${PROJECT_ROOT}/target/qa-reports"
QA_LOGS_DIR="${PROJECT_ROOT}/logs/qa"

# Function definitions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${PURPLE}[STEP]${NC} $1"
}

print_banner() {
    echo -e "${CYAN}"
    cat << 'EOF'
    ╔═══════════════════════════════════════════════════════════════╗
    ║                  🚀 AI-PLATFORM QA Agent                         ║
    ║              Quality Assurance Framework                      ║
    ║                                                               ║
    ║  📊 Testing • 🔍 Performance • 🔒 Security • 📈 Monitoring   ║
    ╚═══════════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"
}

show_help() {
    cat << EOF
AI-PLATFORM QA Agent Setup and Usage Script

USAGE:
    $0 [COMMAND] [OPTIONS]

COMMANDS:
    setup           Initialize QA environment and dependencies
    validate        Validate QA environment and configuration
    test            Run comprehensive test suite
    performance     Execute performance testing
    security        Run security testing
    dashboard       Start quality metrics dashboard
    report          Generate comprehensive reports
    monitor         Start continuous monitoring
    clean           Clean up QA artifacts and reports
    help            Show this help message

TESTING OPTIONS:
    --unit          Run unit tests only
    --integration   Run integration tests only
    --e2e           Run end-to-end tests only
    --coverage      Generate test coverage report
    --parallel      Enable parallel test execution
    --verbose       Enable verbose output

PERFORMANCE OPTIONS:
    --load          Run load testing scenarios
    --stress        Run stress testing scenarios
    --benchmark     Run performance benchmarks
    --sla           Validate SLA compliance
    --targets       Specify custom target endpoints

SECURITY OPTIONS:
    --vuln-scan     Run vulnerability scanning
    --pentest       Execute penetration testing
    --compliance    Run compliance validation
    --deps          Audit dependencies for vulnerabilities
    --containers    Scan container images

DASHBOARD OPTIONS:
    --port PORT     Specify dashboard port (default: 8080)
    --host HOST     Specify dashboard host (default: 0.0.0.0)
    --background    Run dashboard in background

REPORT OPTIONS:
    --format FORMAT Report format: html, json, pdf, csv (default: html)
    --timeframe     Report timeframe: 1h, 24h, 7d, 30d (default: 24h)
    --output DIR    Output directory for reports

EXAMPLES:
    $0 setup                                   # Initial setup
    $0 test --coverage --parallel              # Run tests with coverage
    $0 performance --load --targets localhost  # Load test localhost
    $0 security --vuln-scan --deps            # Security scanning
    $0 dashboard --port 9090                   # Start dashboard on port 9090
    $0 report --format pdf --timeframe 7d     # Generate 7-day PDF report

EOF
}

check_dependencies() {
    log_step "Checking dependencies..."

    local deps=("cargo" "docker" "curl" "jq")
    local missing_deps=()

    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing_deps+=("$dep")
        fi
    done

    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_info "Please install the missing dependencies and try again."
        exit 1
    fi

    log_success "All dependencies found"
}

setup_directories() {
    log_step "Setting up directories..."

    local dirs=(
        "$QA_CONFIG_DIR"
        "$QA_REPORTS_DIR"
        "$QA_LOGS_DIR"
        "${PROJECT_ROOT}/target/performance-reports"
        "${PROJECT_ROOT}/target/security-reports"
        "${PROJECT_ROOT}/target/test-reports"
    )

    for dir in "${dirs[@]}"; do
        mkdir -p "$dir"
        log_info "Created directory: $dir"
    done

    log_success "Directory structure created"
}

validate_rust_environment() {
    log_step "Validating Rust environment..."

    cd "$PROJECT_ROOT"

    # Check if QA Agent compiles
    if cargo check -p qa-agent --quiet; then
        log_success "QA Agent compilation successful"
    else
        log_error "QA Agent compilation failed"
        exit 1
    fi

    # Build QA binaries
    log_info "Building QA Agent binaries..."
    if cargo build -p qa-agent --release --quiet; then
        log_success "QA Agent binaries built successfully"
    else
        log_error "Failed to build QA Agent binaries"
        exit 1
    fi
}

check_services() {
    log_step "Checking AI-PLATFORM services availability..."

    local services=(
        "http://localhost:8000/api/v1/health:API Gateway"
        "http://localhost:8001/api/v1/health:Intent Parser"
        "http://localhost:8002/api/v1/health:MCP Manager"
        "http://localhost:8003/api/v1/health:Federation Service"
    )

    local available_services=0
    local total_services=${#services[@]}

    for service in "${services[@]}"; do
        local url="${service%%:*}"
        local name="${service##*:}"

        if curl -s -f "$url" > /dev/null 2>&1; then
            log_success "$name is available"
            ((available_services++))
        else
            log_warning "$name is not available at $url"
        fi
    done

    log_info "Services available: $available_services/$total_services"

    if [ $available_services -eq 0 ]; then
        log_warning "No services are running. Some tests may fail."
        log_info "Consider starting the AI-PLATFORM services first:"
        log_info "  docker-compose up -d"
    fi
}

run_setup() {
    log_step "Running QA Agent setup..."

    check_dependencies
    setup_directories
    validate_rust_environment
    check_services

    # Copy default configuration if it doesn't exist
    if [ ! -f "$QA_CONFIG_DIR/qa.toml" ]; then
        log_info "Configuration file not found. Please ensure qa.toml exists in $QA_CONFIG_DIR"
    else
        log_success "Configuration file found"
    fi

    log_success "QA Agent setup completed successfully!"
    log_info ""
    log_info "Next steps:"
    log_info "1. Review configuration: $QA_CONFIG_DIR/qa.toml"
    log_info "2. Run validation: $0 validate"
    log_info "3. Execute tests: $0 test"
    log_info "4. Start dashboard: $0 dashboard"
}

run_validation() {
    log_step "Validating QA environment..."

    cd "$PROJECT_ROOT"

    # Validate configuration
    if [ -f "$QA_CONFIG_DIR/qa.toml" ]; then
        log_success "Configuration file exists"
    else
        log_error "Configuration file missing: $QA_CONFIG_DIR/qa.toml"
        exit 1
    fi

    # Test QA orchestrator
    log_info "Testing QA orchestrator..."
    if cargo run --bin qa-orchestrator --quiet -- --help > /dev/null 2>&1; then
        log_success "QA orchestrator is functional"
    else
        log_error "QA orchestrator test failed"
        exit 1
    fi

    # Test other binaries
    local binaries=("performance-tester" "security-scanner" "quality-dashboard")
    for binary in "${binaries[@]}"; do
        log_info "Testing $binary..."
        if cargo run --bin "$binary" --quiet -- --help > /dev/null 2>&1; then
            log_success "$binary is functional"
        else
            log_warning "$binary test failed"
        fi
    done

    log_success "QA environment validation completed"
}

run_tests() {
    local test_type="all"
    local coverage=false
    local parallel=false
    local verbose=false

    # Parse test options
    while [[ $# -gt 0 ]]; do
        case $1 in
            --unit)
                test_type="unit"
                shift
                ;;
            --integration)
                test_type="integration"
                shift
                ;;
            --e2e)
                test_type="e2e"
                shift
                ;;
            --coverage)
                coverage=true
                shift
                ;;
            --parallel)
                parallel=true
                shift
                ;;
            --verbose)
                verbose=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done

    log_step "Running tests (type: $test_type)..."

    cd "$PROJECT_ROOT"

    local cargo_args=()
    if [ "$parallel" = true ]; then
        cargo_args+=("--")
        cargo_args+=("--test-threads=4")
    fi

    if [ "$verbose" = true ]; then
        cargo_args+=("--verbose")
    fi

    case $test_type in
        "unit")
            log_info "Running unit tests..."
            cargo test --lib "${cargo_args[@]}"
            ;;
        "integration")
            log_info "Running integration tests..."
            cargo test --test '*' "${cargo_args[@]}"
            ;;
        "e2e")
            log_info "Running end-to-end tests..."
            cargo run --bin qa-orchestrator -- run-e2e-tests
            ;;
        "all")
            log_info "Running comprehensive test suite..."
            cargo run --bin qa-orchestrator -- run-full-suite \
                --config "$QA_CONFIG_DIR/qa.toml" \
                --output "$QA_REPORTS_DIR"
            ;;
    esac

    if [ "$coverage" = true ]; then
        log_info "Generating coverage report..."
        if command -v cargo-tarpaulin &> /dev/null; then
            cargo tarpaulin --out Html --output-dir "$QA_REPORTS_DIR/coverage"
            log_success "Coverage report generated: $QA_REPORTS_DIR/coverage/tarpaulin-report.html"
        else
            log_warning "cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
        fi
    fi

    log_success "Test execution completed"
}

run_performance() {
    local test_type="load"
    local targets=""

    # Parse performance options
    while [[ $# -gt 0 ]]; do
        case $1 in
            --load)
                test_type="load"
                shift
                ;;
            --stress)
                test_type="stress"
                shift
                ;;
            --benchmark)
                test_type="benchmark"
                shift
                ;;
            --sla)
                test_type="sla"
                shift
                ;;
            --targets)
                targets="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    log_step "Running performance tests (type: $test_type)..."

    cd "$PROJECT_ROOT"

    local perf_args=(
        "--config" "$QA_CONFIG_DIR/qa.toml"
        "--output" "${PROJECT_ROOT}/target/performance-reports"
    )

    if [ -n "$targets" ]; then
        perf_args+=("--target" "$targets")
    fi

    case $test_type in
        "load")
            cargo run --bin performance-tester -- load-test "${perf_args[@]}"
            ;;
        "stress")
            cargo run --bin performance-tester -- stress-test "${perf_args[@]}"
            ;;
        "benchmark")
            cargo run --bin performance-tester -- benchmark "${perf_args[@]}"
            ;;
        "sla")
            cargo run --bin performance-tester -- validate-sla "${perf_args[@]}"
            ;;
    esac

    log_success "Performance testing completed"
}

run_security() {
    local scan_type="vuln-scan"

    # Parse security options
    while [[ $# -gt 0 ]]; do
        case $1 in
            --vuln-scan)
                scan_type="vuln-scan"
                shift
                ;;
            --pentest)
                scan_type="penetration-test"
                shift
                ;;
            --compliance)
                scan_type="compliance-check"
                shift
                ;;
            --deps)
                scan_type="dependency-audit"
                shift
                ;;
            --containers)
                scan_type="container-scan"
                shift
                ;;
            *)
                shift
                ;;
        esac
    done

    log_step "Running security tests (type: $scan_type)..."

    cd "$PROJECT_ROOT"

    local security_args=(
        "--config" "$QA_CONFIG_DIR/qa.toml"
        "--output" "${PROJECT_ROOT}/target/security-reports"
    )

    case $scan_type in
        "vuln-scan")
            cargo run --bin security-scanner -- vuln-scan --target "http://localhost:8000" "${security_args[@]}"
            ;;
        "penetration-test")
            cargo run --bin security-scanner -- penetration-test --target "http://localhost:8000" "${security_args[@]}"
            ;;
        "compliance-check")
            cargo run --bin security-scanner -- compliance-check --standard "owasp" "${security_args[@]}"
            ;;
        "dependency-audit")
            cargo run --bin security-scanner -- dependency-audit --manifest "Cargo.toml" "${security_args[@]}"
            ;;
        "container-scan")
            if docker images -q AI-PLATFORM/api-gateway &> /dev/null; then
                cargo run --bin security-scanner -- container-scan --image "AI-PLATFORM/api-gateway" "${security_args[@]}"
            else
                log_warning "No AI-PLATFORM container images found to scan"
            fi
            ;;
    esac

    log_success "Security testing completed"
}

start_dashboard() {
    local port="8080"
    local host="0.0.0.0"
    local background=false

    # Parse dashboard options
    while [[ $# -gt 0 ]]; do
        case $1 in
            --port)
                port="$2"
                shift 2
                ;;
            --host)
                host="$2"
                shift 2
                ;;
            --background)
                background=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done

    log_step "Starting quality dashboard..."

    cd "$PROJECT_ROOT"

    local dashboard_args=(
        "--config" "$QA_CONFIG_DIR/qa.toml"
        "--host" "$host"
        "--port" "$port"
    )

    log_info "Quality dashboard will be available at: http://$host:$port"

    if [ "$background" = true ]; then
        log_info "Starting dashboard in background..."
        nohup cargo run --bin quality-dashboard -- "${dashboard_args[@]}" > "$QA_LOGS_DIR/dashboard.log" 2>&1 &
        local pid=$!
        echo $pid > "$QA_LOGS_DIR/dashboard.pid"
        log_success "Dashboard started in background (PID: $pid)"
        log_info "Logs: $QA_LOGS_DIR/dashboard.log"
        log_info "To stop: kill $pid"
    else
        log_info "Starting dashboard in foreground (Ctrl+C to stop)..."
        cargo run --bin quality-dashboard -- "${dashboard_args[@]}"
    fi
}

generate_reports() {
    local format="html"
    local timeframe="24h"
    local output_dir="$QA_REPORTS_DIR"

    # Parse report options
    while [[ $# -gt 0 ]]; do
        case $1 in
            --format)
                format="$2"
                shift 2
                ;;
            --timeframe)
                timeframe="$2"
                shift 2
                ;;
            --output)
                output_dir="$2"
                shift 2
                ;;
            *)
                shift
                ;;
        esac
    done

    log_step "Generating reports (format: $format, timeframe: $timeframe)..."

    cd "$PROJECT_ROOT"

    mkdir -p "$output_dir"

    # Generate comprehensive QA report
    cargo run --bin qa-orchestrator -- report \
        --format "$format" \
        --timeframe "$timeframe" \
        --output "$output_dir" \
        --config "$QA_CONFIG_DIR/qa.toml"

    log_success "Reports generated in: $output_dir"

    if [ "$format" = "html" ]; then
        local report_file="$output_dir/qa_comprehensive_report.html"
        if [ -f "$report_file" ]; then
            log_info "Open report: file://$report_file"
        fi
    fi
}

start_monitoring() {
    log_step "Starting continuous monitoring..."

    cd "$PROJECT_ROOT"

    log_info "Continuous monitoring will run every 30 minutes"
    log_info "Press Ctrl+C to stop monitoring"

    cargo run --bin qa-orchestrator -- continuous \
        --config "$QA_CONFIG_DIR/qa.toml" \
        --interval "30m" \
        --alerts
}

clean_artifacts() {
    log_step "Cleaning QA artifacts..."

    local dirs_to_clean=(
        "$QA_REPORTS_DIR"
        "${PROJECT_ROOT}/target/performance-reports"
        "${PROJECT_ROOT}/target/security-reports"
        "${PROJECT_ROOT}/target/test-reports"
        "$QA_LOGS_DIR"
    )

    for dir in "${dirs_to_clean[@]}"; do
        if [ -d "$dir" ]; then
            rm -rf "$dir"
            log_info "Cleaned: $dir"
        fi
    done

    # Stop background dashboard if running
    if [ -f "$QA_LOGS_DIR/dashboard.pid" ]; then
        local pid=$(cat "$QA_LOGS_DIR/dashboard.pid")
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid"
            log_info "Stopped background dashboard (PID: $pid)"
        fi
        rm -f "$QA_LOGS_DIR/dashboard.pid"
    fi

    log_success "Cleanup completed"
}

# Main script logic
main() {
    if [ $# -eq 0 ]; then
        print_banner
        show_help
        exit 0
    fi

    local command="$1"
    shift

    case $command in
        "setup")
            print_banner
            run_setup
            ;;
        "validate")
            run_validation
            ;;
        "test")
            run_tests "$@"
            ;;
        "performance")
            run_performance "$@"
            ;;
        "security")
            run_security "$@"
            ;;
        "dashboard")
            start_dashboard "$@"
            ;;
        "report")
            generate_reports "$@"
            ;;
        "monitor")
            start_monitoring
            ;;
        "clean")
            clean_artifacts
            ;;
        "help"|"--help"|"-h")
            show_help
            ;;
        *)
            log_error "Unknown command: $command"
            log_info "Use '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Execute main function with all arguments
main "$@"
