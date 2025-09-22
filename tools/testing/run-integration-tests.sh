#!/bin/bash

# AI-PLATFORM Microservices Integration Test Runner
# Comprehensive test execution script for all microservices
# Usage: ./scripts/run-integration-tests.sh [options]

set -euo pipefail

# Default configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LOG_LEVEL="${LOG_LEVEL:-info}"
TEST_TIMEOUT="${TEST_TIMEOUT:-1800}"
PARALLEL_TESTS="${PARALLEL_TESTS:-true}"
SKIP_PERFORMANCE="${SKIP_PERFORMANCE:-false}"
GENERATE_REPORTS="${GENERATE_REPORTS:-true}"
CLEANUP_ON_EXIT="${CLEANUP_ON_EXIT:-true}"
DRY_RUN="${DRY_RUN:-false}"
FAIL_FAST="${FAIL_FAST:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Test configuration
SERVICES=(
    "service-discovery"
    "intent-parser"
    "mcp-manager"
    "federation"
    "file-storage"
    "event-streaming"
    "notification"
    "data-processing"
)

# Function to print colored output
print_color() {
    local color="$1"
    local message="$2"
    echo -e "${color}${message}${NC}"
}

# Function to print section headers
print_section() {
    local title="$1"
    echo ""
    print_color "$CYAN" "═══════════════════════════════════════════════════════════════"
    print_color "$CYAN" "  $title"
    print_color "$CYAN" "═══════════════════════════════════════════════════════════════"
    echo ""
}

# Function to print step info
print_step() {
    local step="$1"
    print_color "$BLUE" "🔧 $step"
}

# Function to print success message
print_success() {
    local message="$1"
    print_color "$GREEN" "✅ $message"
}

# Function to print warning message
print_warning() {
    local message="$1"
    print_color "$YELLOW" "⚠️  $message"
}

# Function to print error message
print_error() {
    local message="$1"
    print_color "$RED" "❌ $message"
}

# Function to show usage
show_usage() {
    cat << EOF
AI-PLATFORM Microservices Integration Test Runner

Usage: $0 [OPTIONS]

Options:
  -h, --help                Show this help message
  -d, --dry-run            Show what would be tested without executing
  -v, --verbose            Enable verbose logging
  -q, --quiet              Quiet mode (errors only)
  -f, --fail-fast          Stop on first test failure
  -p, --parallel           Run tests in parallel (default: true)
  -s, --sequential         Run tests sequentially
  --skip-performance       Skip performance validation tests
  --skip-reports           Skip report generation
  --no-cleanup             Don't cleanup test environment on exit
  --timeout SECONDS        Test timeout in seconds (default: 1800)
  --services SERVICE_LIST  Comma-separated list of services to test
  --scenarios SCENARIO_LIST Comma-separated list of test scenarios
  --config CONFIG_FILE     Use custom configuration file
  --report-format FORMAT   Report format: json,html,both (default: both)

Test Scenarios:
  basic_health_checks      Basic health checks for all services
  cross_service_communication Cross-service API communication
  end_to_end_workflows     Complete workflow execution testing
  event_streaming_integration Event publishing and consumption
  federation_coordination  MCP federation and client coordination
  service_discovery_integration Service registration and discovery
  performance_validation  Throughput and latency validation
  load_testing            High-load stress testing
  chaos_testing           Failure injection and recovery

Examples:
  $0                                    # Run all tests
  $0 --services intent-parser,mcp-manager  # Test specific services
  $0 --scenarios basic_health_checks    # Run specific scenarios
  $0 --dry-run                         # Preview test execution
  $0 --fail-fast --verbose             # Stop on first failure with verbose output

Environment Variables:
  LOG_LEVEL                Logging level (debug,info,warn,error)
  TEST_TIMEOUT             Test timeout in seconds
  PARALLEL_TESTS           Enable parallel execution (true/false)
  SKIP_PERFORMANCE         Skip performance tests (true/false)
  CLEANUP_ON_EXIT          Cleanup on exit (true/false)
  TEST_BASE_URL            Base URL for services
  TEST_POSTGRES_URL        PostgreSQL connection string
  TEST_MONGODB_URL         MongoDB connection string
  TEST_CLICKHOUSE_URL      ClickHouse connection string
  TEST_REDIS_URL           Redis connection string
  TEST_KAFKA_BROKERS       Kafka broker list

EOF
}

# Function to parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -d|--dry-run)
                DRY_RUN="true"
                shift
                ;;
            -v|--verbose)
                LOG_LEVEL="debug"
                shift
                ;;
            -q|--quiet)
                LOG_LEVEL="error"
                shift
                ;;
            -f|--fail-fast)
                FAIL_FAST="true"
                shift
                ;;
            -p|--parallel)
                PARALLEL_TESTS="true"
                shift
                ;;
            -s|--sequential)
                PARALLEL_TESTS="false"
                shift
                ;;
            --skip-performance)
                SKIP_PERFORMANCE="true"
                shift
                ;;
            --skip-reports)
                GENERATE_REPORTS="false"
                shift
                ;;
            --no-cleanup)
                CLEANUP_ON_EXIT="false"
                shift
                ;;
            --timeout)
                TEST_TIMEOUT="$2"
                shift 2
                ;;
            --services)
                SERVICES_FILTER="$2"
                shift 2
                ;;
            --scenarios)
                SCENARIOS_FILTER="$2"
                shift 2
                ;;
            --config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --report-format)
                REPORT_FORMAT="$2"
                shift 2
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Function to check prerequisites
check_prerequisites() {
    print_step "Checking prerequisites"

    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        print_error "Not in AI-PLATFORM project root directory"
        exit 1
    fi

    # Check if Rust toolchain is available
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo not found. Please install Rust toolchain."
        exit 1
    fi

    # Check Rust version
    local rust_version
    rust_version=$(rustc --version | cut -d' ' -f2)
    print_color "$GREEN" "  Rust version: $rust_version"

    # Check if required services binaries exist or can be built
    print_step "Checking service binaries"
    for service in "${SERVICES[@]}"; do
        local binary_name
        case $service in
            "service-discovery")
                binary_name="service-discovery-server"
                ;;
            "intent-parser")
                binary_name="intent-parser"
                ;;
            "mcp-manager")
                binary_name="mcp-manager"
                ;;
            "federation")
                binary_name="federation-server"
                ;;
            "file-storage")
                binary_name="file-storage-server"
                ;;
            "event-streaming")
                binary_name="event-streaming-server"
                ;;
            "notification")
                binary_name="notification-server"
                ;;
            "data-processing")
                binary_name="data-processing-server"
                ;;
        esac

        if [[ -f "$PROJECT_ROOT/target/debug/$binary_name" ]]; then
            print_color "$GREEN" "  ✓ $service ($binary_name)"
        else
            print_color "$YELLOW" "  ⚠ $service binary not found (will build)"
        fi
    done
}

# Function to setup test environment
setup_test_environment() {
    print_step "Setting up test environment"

    # Set environment variables
    export RUST_LOG="${LOG_LEVEL}"
    export RUST_BACKTRACE="1"
    export TEST_TIMEOUT="${TEST_TIMEOUT}"

    # Create test directories
    mkdir -p "$PROJECT_ROOT/test-results"
    mkdir -p "$PROJECT_ROOT/test-reports"
    mkdir -p "$PROJECT_ROOT/test-logs"

    # Setup test database URLs
    export TEST_POSTGRES_URL="${TEST_POSTGRES_URL:-postgres://localhost:5432/ai_core_test}"
    export TEST_MONGODB_URL="${TEST_MONGODB_URL:-mongodb://localhost:27017/ai_core_test}"
    export TEST_CLICKHOUSE_URL="${TEST_CLICKHOUSE_URL:-tcp://localhost:9000/ai_core_test}"
    export TEST_REDIS_URL="${TEST_REDIS_URL:-redis://localhost:6379/1}"
    export TEST_KAFKA_BROKERS="${TEST_KAFKA_BROKERS:-localhost:9092}"

    print_success "Test environment configured"
}

# Function to build all services
build_services() {
    print_step "Building all services"

    if [[ "$DRY_RUN" == "true" ]]; then
        print_color "$YELLOW" "DRY RUN: Would build all services"
        return 0
    fi

    # Build in release mode for better performance
    if cargo build --release --workspace; then
        print_success "All services built successfully"
    else
        print_error "Build failed"
        exit 1
    fi
}

# Function to run integration tests
run_integration_tests() {
    print_section "Running Integration Tests"

    local test_args=(
        "--timeout" "$TEST_TIMEOUT"
        "--log-level" "$LOG_LEVEL"
    )

    if [[ "$DRY_RUN" == "true" ]]; then
        test_args+=("--dry-run")
    fi

    if [[ "$PARALLEL_TESTS" == "true" ]]; then
        test_args+=("--parallel")
    fi

    if [[ "$SKIP_PERFORMANCE" == "true" ]]; then
        test_args+=("--skip-performance")
    fi

    if [[ "$FAIL_FAST" == "true" ]]; then
        test_args+=("--fail-fast")
    fi

    if [[ -n "${SERVICES_FILTER:-}" ]]; then
        test_args+=("--services" "$SERVICES_FILTER")
    fi

    if [[ -n "${SCENARIOS_FILTER:-}" ]]; then
        test_args+=("--scenarios" "$SCENARIOS_FILTER")
    fi

    if [[ -n "${CONFIG_FILE:-}" ]]; then
        test_args+=("--config" "$CONFIG_FILE")
    fi

    if [[ "$GENERATE_REPORTS" == "true" ]]; then
        test_args+=("--json-report" "--html-report")
        test_args+=("--report" "$PROJECT_ROOT/test-reports/integration-report.json")
    fi

    # Run the integration test runner
    print_step "Executing integration test suite"

    cd "$PROJECT_ROOT"

    if cargo run --bin integration-test-runner -- "${test_args[@]}"; then
        print_success "Integration tests completed successfully"
        return 0
    else
        print_error "Integration tests failed"
        return 1
    fi
}

# Function to run performance tests
run_performance_tests() {
    if [[ "$SKIP_PERFORMANCE" == "true" ]]; then
        return 0
    fi

    print_section "Running Performance Tests"

    if [[ "$DRY_RUN" == "true" ]]; then
        print_color "$YELLOW" "DRY RUN: Would run performance tests"
        return 0
    fi

    local perf_args=(
        "performance"
        "--target-rps" "10000"
        "--duration" "60"
        "--timeout" "$TEST_TIMEOUT"
    )

    if cargo run --bin integration-test-runner -- "${perf_args[@]}"; then
        print_success "Performance tests completed successfully"
        return 0
    else
        print_error "Performance tests failed"
        return 1
    fi
}

# Function to generate test reports
generate_reports() {
    if [[ "$GENERATE_REPORTS" == "false" ]]; then
        return 0
    fi

    print_section "Generating Test Reports"

    local report_dir="$PROJECT_ROOT/test-reports"
    local timestamp=$(date '+%Y%m%d_%H%M%S')

    if [[ -f "$report_dir/integration-report.json" ]]; then
        # Archive the report with timestamp
        cp "$report_dir/integration-report.json" "$report_dir/integration-report_$timestamp.json"
        print_success "JSON report saved: integration-report_$timestamp.json"
    fi

    if [[ -f "$report_dir/integration-test-report.html" ]]; then
        cp "$report_dir/integration-test-report.html" "$report_dir/integration-report_$timestamp.html"
        print_success "HTML report saved: integration-report_$timestamp.html"
    fi

    # Generate summary report
    cat > "$report_dir/test-summary.txt" << EOF
AI-PLATFORM Microservices Integration Test Summary
Generated: $(date)
═══════════════════════════════════════════════

Test Configuration:
  Services Tested: ${SERVICES_FILTER:-"All services"}
  Scenarios: ${SCENARIOS_FILTER:-"All scenarios"}
  Parallel Execution: $PARALLEL_TESTS
  Performance Tests: $(if [[ "$SKIP_PERFORMANCE" == "true" ]]; then echo "Skipped"; else echo "Included"; fi)
  Timeout: ${TEST_TIMEOUT}s
  Log Level: $LOG_LEVEL

Environment:
  Platform: $(uname -s)
  Rust Version: $(rustc --version | cut -d' ' -f2)
  Test Runner Version: 1.0.0

Results:
  Detailed results available in JSON and HTML reports
  Check integration-report_$timestamp.json for full details
  View integration-report_$timestamp.html in browser for visual report

EOF

    print_success "Test summary generated: test-summary.txt"
}

# Function to cleanup test environment
cleanup_test_environment() {
    if [[ "$CLEANUP_ON_EXIT" == "false" ]]; then
        return 0
    fi

    print_section "Cleaning Up Test Environment"

    if [[ "$DRY_RUN" == "true" ]]; then
        print_color "$YELLOW" "DRY RUN: Would cleanup test environment"
        return 0
    fi

    # Stop any running test services
    print_step "Stopping test services"
    pkill -f "target/debug.*-server" || true
    pkill -f "integration-test" || true

    # Clean up temporary test files
    print_step "Removing temporary files"
    find "$PROJECT_ROOT" -name "test_*" -type f -delete 2>/dev/null || true

    print_success "Test environment cleaned up"
}

# Function to display test summary
display_summary() {
    print_section "Test Execution Summary"

    local end_time=$(date '+%Y-%m-%d %H:%M:%S')
    local duration=$((SECONDS))

    echo "Test execution completed at: $end_time"
    echo "Total duration: ${duration}s"
    echo ""

    if [[ -f "$PROJECT_ROOT/test-reports/integration-report.json" ]]; then
        print_color "$GREEN" "📊 Reports generated:"
        echo "  • JSON: test-reports/integration-report.json"
        echo "  • HTML: test-reports/integration-test-report.html"
        echo "  • Summary: test-reports/test-summary.txt"
        echo ""
    fi

    print_color "$CYAN" "For detailed results, check the generated reports in test-reports/ directory"
}

# Function to handle script exit
cleanup_on_exit() {
    local exit_code=$?

    if [[ $exit_code -ne 0 ]]; then
        print_error "Script exited with error code: $exit_code"
    fi

    cleanup_test_environment
    exit $exit_code
}

# Main execution function
main() {
    local start_time=$(date '+%Y-%m-%d %H:%M:%S')

    print_section "AI-PLATFORM Microservices Integration Test Runner"
    print_color "$CYAN" "Started at: $start_time"

    # Parse command line arguments
    parse_args "$@"

    # Set up exit handler
    trap cleanup_on_exit EXIT

    # Execute test pipeline
    check_prerequisites
    setup_test_environment

    if [[ "$DRY_RUN" != "true" ]]; then
        build_services
    fi

    local test_result=0
    run_integration_tests || test_result=$?
    run_performance_tests || test_result=$?

    generate_reports
    display_summary

    if [[ $test_result -eq 0 ]]; then
        print_success "All tests completed successfully! 🎉"
    else
        print_error "Some tests failed. Check the reports for details."
        exit 1
    fi
}

# Execute main function with all arguments
main "$@"
