#!/bin/bash

# AI-CORE E2E Test Suite - Multiple Run Executor
# Runs comprehensive E2E tests multiple times per build with detailed reporting

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TIMESTAMP=$(date -u +"%Y%m%d-%H%M%S")
RUN_ID="e2e-${TIMESTAMP}"
LOG_FILE="${SCRIPT_DIR}/test-results/execution-${RUN_ID}.log"

# Default configuration (can be overridden by environment)
STABILITY_RUNS=${STABILITY_RUNS:-5}
REGRESSION_RUNS=${REGRESSION_RUNS:-10}
LOAD_TEST_RUNS=${LOAD_TEST_RUNS:-3}
PARALLEL_WORKERS=${PARALLEL_WORKERS:-2}
REQUIRE_ALL_SERVICES=${REQUIRE_ALL_SERVICES:-false}
KEEP_ARTIFACTS=${KEEP_ARTIFACTS:-true}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

    case "$level" in
        INFO)  echo -e "${GREEN}[INFO]${NC}  ${timestamp} - $message" | tee -a "$LOG_FILE" ;;
        WARN)  echo -e "${YELLOW}[WARN]${NC}  ${timestamp} - $message" | tee -a "$LOG_FILE" ;;
        ERROR) echo -e "${RED}[ERROR]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE" ;;
        DEBUG) echo -e "${BLUE}[DEBUG]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE" ;;
        *)     echo -e "${CYAN}[${level}]${NC} ${timestamp} - $message" | tee -a "$LOG_FILE" ;;
    esac
}

# Print banner
print_banner() {
    echo -e "${PURPLE}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                   AI-CORE E2E Test Suite                    ║"
    echo "║                Multiple Runs with Reporting                 ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo -e "📊 ${CYAN}Run ID:${NC} $RUN_ID"
    echo -e "🔄 ${CYAN}Stability Runs:${NC} $STABILITY_RUNS"
    echo -e "📈 ${CYAN}Regression Runs:${NC} $REGRESSION_RUNS"
    echo -e "⚡ ${CYAN}Load Test Runs:${NC} $LOAD_TEST_RUNS"
    echo -e "👥 ${CYAN}Parallel Workers:${NC} $PARALLEL_WORKERS"
    echo ""
}

# Check prerequisites
check_prerequisites() {
    log "INFO" "Checking prerequisites..."

    # Check Node.js
    if ! command -v node &> /dev/null; then
        log "ERROR" "Node.js is not installed"
        exit 1
    fi

    local node_version=$(node --version)
    log "INFO" "Node.js version: $node_version"

    # Check npm
    if ! command -v npm &> /dev/null; then
        log "ERROR" "npm is not installed"
        exit 1
    fi

    # Ensure test directory exists and navigate to it
    if [[ ! -d "$SCRIPT_DIR" ]]; then
        log "ERROR" "Test directory not found: $SCRIPT_DIR"
        exit 1
    fi

    cd "$SCRIPT_DIR"

    # Create necessary directories
    mkdir -p test-results/{logs,artifacts,reports}

    # Check if package.json exists
    if [[ ! -f "package.json" ]]; then
        log "ERROR" "package.json not found in $SCRIPT_DIR"
        exit 1
    fi

    log "INFO" "Prerequisites check completed"
}

# Install dependencies
install_dependencies() {
    log "INFO" "Installing dependencies..."

    # Install npm packages
    npm install --silent >> "$LOG_FILE" 2>&1

    # Install Playwright browsers
    npx playwright install --with-deps >> "$LOG_FILE" 2>&1

    # Verify Playwright installation
    local playwright_version=$(npx playwright --version 2>/dev/null || echo "unknown")
    log "INFO" "Playwright version: $playwright_version"

    log "INFO" "Dependencies installed successfully"
}

# Check service health
check_services() {
    log "INFO" "Checking service health..."

    local services=(
        "Client Demo:http://localhost:8090/health"
        "Federation Service:http://localhost:8801/health"
        "Intent Parser:http://localhost:8802/health"
        "MCP Manager:http://localhost:8803/health"
    )

    local healthy_count=0
    local total_count=${#services[@]}

    for service_info in "${services[@]}"; do
        local service_name="${service_info%%:*}"
        local service_url="${service_info#*:}"

        if curl -s --max-time 5 "$service_url" > /dev/null 2>&1; then
            log "INFO" "✅ $service_name is healthy ($service_url)"
            ((healthy_count++))
        else
            log "WARN" "⚠️  $service_name is not responding ($service_url)"
        fi
    done

    log "INFO" "Service health: $healthy_count/$total_count services available"

    if [[ "$REQUIRE_ALL_SERVICES" == "true" && $healthy_count -lt $total_count ]]; then
        log "ERROR" "Not all services are healthy and REQUIRE_ALL_SERVICES=true"
        exit 1
    fi

    if [[ $healthy_count -eq 0 ]]; then
        log "ERROR" "No services are available - cannot proceed with tests"
        exit 1
    fi
}

# Run specific test type multiple times
run_test_type() {
    local test_type="$1"
    local runs="$2"
    local parallel="$3"
    local timeout="$4"

    log "INFO" "Starting $test_type tests ($runs runs, parallel: $parallel)"

    local passed=0
    local failed=0
    local total_duration=0

    for ((i=1; i<=runs; i++)); do
        log "INFO" "🏃‍♂️ $test_type - Run $i/$runs"

        local run_start=$(date +%s)
        local output_dir="test-results/artifacts/${test_type}-run-${i}"
        mkdir -p "$output_dir"

        local workers_flag=""
        if [[ "$parallel" == "true" ]]; then
            workers_flag="--workers=$PARALLEL_WORKERS"
        else
            workers_flag="--workers=1"
        fi

        # Construct Playwright command
        local cmd="npx playwright test tests/${test_type} --reporter=json --timeout=${timeout} ${workers_flag} --output-dir='${output_dir}'"

        # Run the test
        local exit_code=0
        if timeout "${timeout}s" bash -c "$cmd" >> "$LOG_FILE" 2>&1; then
            local duration=$(($(date +%s) - run_start))
            log "INFO" "    ✅ Run $i: PASSED (${duration}s)"
            ((passed++))
            ((total_duration+=duration))
        else
            exit_code=$?
            local duration=$(($(date +%s) - run_start))
            log "WARN" "    ❌ Run $i: FAILED (${duration}s, exit code: $exit_code)"
            ((failed++))
            ((total_duration+=duration))
        fi

        # Brief cooldown between runs
        if [[ $i -lt $runs ]]; then
            sleep 2
        fi
    done

    local success_rate=$(( (passed * 100) / runs ))
    local avg_duration=$(( total_duration / runs ))

    log "INFO" "📊 $test_type Summary: $success_rate% success ($passed/$runs), avg ${avg_duration}s"

    # Write summary to results file
    cat >> "test-results/summary-${RUN_ID}.txt" << EOF

=== $test_type TESTS ===
Total Runs: $runs
Passed: $passed
Failed: $failed
Success Rate: $success_rate%
Average Duration: ${avg_duration}s
Total Duration: ${total_duration}s

EOF

    return $((failed > 0 ? 1 : 0))
}

# Execute complete test suite
execute_test_suite() {
    log "INFO" "🚀 Starting complete E2E test suite execution"

    local suite_start=$(date +%s)
    local total_phases=0
    local passed_phases=0

    # Initialize summary file
    cat > "test-results/summary-${RUN_ID}.txt" << EOF
AI-CORE E2E Test Suite Results
==============================
Run ID: $RUN_ID
Started: $(date -u)
Configuration:
- Stability Runs: $STABILITY_RUNS
- Regression Runs: $REGRESSION_RUNS
- Load Test Runs: $LOAD_TEST_RUNS
- Parallel Workers: $PARALLEL_WORKERS

EOF

    # Test phases with configuration
    local test_phases=(
        "critical:3:true:60"
        "stability:${STABILITY_RUNS}:false:90"
        "regression:${REGRESSION_RUNS}:true:45"
        "load:${LOAD_TEST_RUNS}:true:120"
    )

    for phase_config in "${test_phases[@]}"; do
        IFS=':' read -r test_type runs parallel timeout <<< "$phase_config"
        ((total_phases++))

        log "INFO" "📝 Phase: $test_type tests"

        if run_test_type "$test_type" "$runs" "$parallel" "$timeout"; then
            log "INFO" "✅ $test_type phase PASSED"
            ((passed_phases++))
        else
            log "WARN" "❌ $test_type phase FAILED"
        fi

        # Cooldown between phases
        sleep 5
    done

    local suite_duration=$(($(date +%s) - suite_start))
    local suite_success_rate=$(( (passed_phases * 100) / total_phases ))

    # Update summary file
    cat >> "test-results/summary-${RUN_ID}.txt" << EOF

=== OVERALL RESULTS ===
Total Phases: $total_phases
Passed Phases: $passed_phases
Failed Phases: $((total_phases - passed_phases))
Phase Success Rate: $suite_success_rate%
Total Suite Duration: ${suite_duration}s ($(( suite_duration / 60 ))m $(( suite_duration % 60 ))s)
Completed: $(date -u)

EOF

    log "INFO" "🏁 Test suite completed - $suite_success_rate% phase success rate"

    return $((passed_phases == total_phases ? 0 : 1))
}

# Generate comprehensive reports
generate_reports() {
    log "INFO" "📊 Generating comprehensive reports..."

    # Run the report generator if it exists
    if [[ -f "scripts/generate-comprehensive-report.js" ]]; then
        log "INFO" "Running comprehensive report generator..."
        node scripts/generate-comprehensive-report.js >> "$LOG_FILE" 2>&1 || log "WARN" "Report generator failed"
    fi

    # Create a simple HTML summary if the comprehensive one doesn't exist
    local html_report="test-results/reports/quick-summary-${RUN_ID}.html"
    mkdir -p "$(dirname "$html_report")"

    cat > "$html_report" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>AI-CORE E2E Test Results</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        .header { text-align: center; color: #333; border-bottom: 2px solid #667eea; padding-bottom: 20px; margin-bottom: 30px; }
        .metric { display: inline-block; margin: 10px 20px; padding: 15px; background: #f8f9fa; border-radius: 5px; min-width: 120px; text-align: center; }
        .success { border-left: 4px solid #28a745; }
        .warning { border-left: 4px solid #ffc107; }
        .error { border-left: 4px solid #dc3545; }
        .summary { margin: 20px 0; padding: 20px; background: #e9ecef; border-radius: 5px; }
        pre { background: #f8f9fa; padding: 15px; border-radius: 5px; overflow-x: auto; }
        .timestamp { text-align: right; color: #666; font-size: 0.9em; margin-top: 30px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 AI-CORE E2E Test Results</h1>
            <p>Multiple Run Analysis</p>
        </div>

        <div class="summary">
            <h3>Test Summary</h3>
            <p>This report shows results from multiple test runs designed to catch race conditions and intermittent failures.</p>
        </div>

        <div style="text-align: center;">
            <div class="metric success">
                <h4>Run ID</h4>
                <p>RUN_ID_PLACEHOLDER</p>
            </div>
            <div class="metric">
                <h4>Total Duration</h4>
                <p>DURATION_PLACEHOLDER</p>
            </div>
            <div class="metric">
                <h4>Test Phases</h4>
                <p>PHASES_PLACEHOLDER</p>
            </div>
        </div>

        <h3>Detailed Results</h3>
        <pre>SUMMARY_CONTENT_PLACEHOLDER</pre>

        <div class="timestamp">
            Generated: TIMESTAMP_PLACEHOLDER
        </div>
    </div>
</body>
</html>
EOF

    # Replace placeholders with actual data
    sed -i.bak "s/RUN_ID_PLACEHOLDER/$RUN_ID/g" "$html_report"
    sed -i.bak "s/TIMESTAMP_PLACEHOLDER/$(date)/g" "$html_report"
    sed -i.bak "s/DURATION_PLACEHOLDER/$(( ($(date +%s) - $(stat -c %Y "$LOG_FILE" 2>/dev/null || echo $(date +%s))) / 60 ))m/g" "$html_report"
    sed -i.bak "s/PHASES_PLACEHOLDER/4/g" "$html_report"

    # Insert summary content
    if [[ -f "test-results/summary-${RUN_ID}.txt" ]]; then
        # Escape the content for HTML
        local summary_content=$(cat "test-results/summary-${RUN_ID}.txt" | sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')
        sed -i.bak "s/SUMMARY_CONTENT_PLACEHOLDER/$summary_content/g" "$html_report"
    fi

    rm -f "$html_report.bak"

    log "INFO" "📄 Reports generated:"
    log "INFO" "  - Summary: test-results/summary-${RUN_ID}.txt"
    log "INFO" "  - HTML: $html_report"
    log "INFO" "  - Log: $LOG_FILE"
}

# Cleanup function
cleanup() {
    if [[ "$KEEP_ARTIFACTS" != "true" ]]; then
        log "INFO" "🧹 Cleaning up large artifacts..."
        find test-results/artifacts -name "*.webm" -delete 2>/dev/null || true
        find test-results/artifacts -name "*.png" -size +1M -delete 2>/dev/null || true
        log "INFO" "Cleanup completed"
    fi
}

# Print final summary
print_final_summary() {
    log "INFO" "📋 Final Summary:"

    if [[ -f "test-results/summary-${RUN_ID}.txt" ]]; then
        echo ""
        echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${CYAN}║                        FINAL RESULTS                         ║${NC}"
        echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════╝${NC}"

        # Extract key metrics from summary
        local summary_file="test-results/summary-${RUN_ID}.txt"

        echo ""
        tail -n 20 "$summary_file"
        echo ""
    fi

    echo -e "${GREEN}✅ E2E Test Suite Completed${NC}"
    echo -e "📊 Run ID: $RUN_ID"
    echo -e "📁 Results: test-results/"
    echo -e "📄 Full log: $LOG_FILE"
    echo ""
}

# Error handler
error_handler() {
    local exit_code=$?
    log "ERROR" "Test execution failed with exit code $exit_code"

    # Still try to generate reports on failure
    generate_reports

    echo ""
    echo -e "${RED}❌ E2E Test Suite Failed${NC}"
    echo -e "📄 Check logs: $LOG_FILE"
    echo ""

    exit $exit_code
}

# Main execution function
main() {
    # Set up error handling
    trap error_handler ERR

    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --stability-runs)
                STABILITY_RUNS="$2"
                shift 2
                ;;
            --regression-runs)
                REGRESSION_RUNS="$2"
                shift 2
                ;;
            --load-runs)
                LOAD_TEST_RUNS="$2"
                shift 2
                ;;
            --workers)
                PARALLEL_WORKERS="$2"
                shift 2
                ;;
            --require-services)
                REQUIRE_ALL_SERVICES="true"
                shift
                ;;
            --no-artifacts)
                KEEP_ARTIFACTS="false"
                shift
                ;;
            --help)
                echo "Usage: $0 [options]"
                echo "Options:"
                echo "  --stability-runs N      Number of stability test runs (default: 5)"
                echo "  --regression-runs N     Number of regression test runs (default: 10)"
                echo "  --load-runs N          Number of load test runs (default: 3)"
                echo "  --workers N            Number of parallel workers (default: 2)"
                echo "  --require-services     Require all services to be healthy"
                echo "  --no-artifacts        Don't keep test artifacts"
                echo "  --help                Show this help message"
                exit 0
                ;;
            *)
                log "WARN" "Unknown option: $1"
                shift
                ;;
        esac
    done

    # Execute the test suite
    print_banner
    check_prerequisites
    install_dependencies
    check_services

    if execute_test_suite; then
        generate_reports
        cleanup
        print_final_summary

        log "INFO" "🎉 All tests completed successfully!"
        exit 0
    else
        generate_reports
        cleanup
        print_final_summary

        log "ERROR" "❌ Some tests failed - review results"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"
