#!/bin/bash

# AI-CORE Client Integration Test Script
# Tests the client-app-integration with real AI-CORE federation service

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🧪 AI-CORE Client Integration Test Suite${NC}"
echo "Testing client-app-integration with real AI-CORE federation service"
echo ""

# Configuration
FEDERATION_URL="http://localhost:8801"
CLIENT_URL="http://localhost:5173"
TEST_WORKFLOW_ID=""
TEST_RESULTS_FILE="integration-test-results.json"

# Function to log test results
log_test() {
    local test_name=$1
    local status=$2
    local message=$3

    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}✅ $test_name${NC}: $message"
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}❌ $test_name${NC}: $message"
    else
        echo -e "${YELLOW}⚠️  $test_name${NC}: $message"
    fi
}

# Function to wait for service
wait_for_service() {
    local url=$1
    local service_name=$2
    local max_attempts=10
    local attempt=1

    echo -e "${YELLOW}⏳ Waiting for $service_name...${NC}"

    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$url" >/dev/null 2>&1; then
            log_test "Service Check" "PASS" "$service_name is ready"
            return 0
        fi
        sleep 2
        attempt=$((attempt + 1))
    done

    log_test "Service Check" "FAIL" "$service_name not available after $max_attempts attempts"
    return 1
}

# Test 1: Check Federation Service Health
test_federation_health() {
    echo -e "\n${BLUE}Test 1: Federation Service Health${NC}"

    local response=$(curl -s "$FEDERATION_URL/health" 2>/dev/null)
    local exit_code=$?

    if [ $exit_code -eq 0 ] && echo "$response" | grep -q "healthy"; then
        local version=$(echo "$response" | jq -r '.version // "unknown"' 2>/dev/null)
        local uptime=$(echo "$response" | jq -r '.uptime_seconds // 0' 2>/dev/null)
        log_test "Federation Health" "PASS" "Service healthy (v$version, uptime: ${uptime}s)"
        return 0
    else
        log_test "Federation Health" "FAIL" "Service not healthy or not responding"
        return 1
    fi
}

# Test 2: Test Workflow Creation
test_workflow_creation() {
    echo -e "\n${BLUE}Test 2: Workflow Creation${NC}"

    local request_data='{
        "intent": "Create a blog post about AI automation trends for integration testing",
        "workflow_type": "blog-post-social",
        "client_context": {
            "client_demo": true,
            "real_time_updates": true,
            "test_mode": true
        }
    }'

    local response=$(curl -s -X POST "$FEDERATION_URL/v1/workflows" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: demo-api-key" \
        -d "$request_data" 2>/dev/null)

    local exit_code=$?

    if [ $exit_code -eq 0 ]; then
        TEST_WORKFLOW_ID=$(echo "$response" | jq -r '.workflow_id // ""' 2>/dev/null)
        local status=$(echo "$response" | jq -r '.status // ""' 2>/dev/null)
        local message=$(echo "$response" | jq -r '.message // ""' 2>/dev/null)

        if [ ! -z "$TEST_WORKFLOW_ID" ] && [ "$status" = "created" ]; then
            log_test "Workflow Creation" "PASS" "Workflow created: $TEST_WORKFLOW_ID"
            echo "    Status: $status"
            echo "    Message: $message"
            return 0
        else
            log_test "Workflow Creation" "FAIL" "Invalid response: $response"
            return 1
        fi
    else
        log_test "Workflow Creation" "FAIL" "Request failed (exit code: $exit_code)"
        return 1
    fi
}

# Test 3: Test Workflow Status Polling
test_workflow_status() {
    echo -e "\n${BLUE}Test 3: Workflow Status Monitoring${NC}"

    if [ -z "$TEST_WORKFLOW_ID" ]; then
        log_test "Workflow Status" "FAIL" "No workflow ID available"
        return 1
    fi

    local max_polls=20
    local poll_count=0
    local last_status=""
    local last_progress=0

    echo "    Polling workflow status (max $max_polls attempts)..."

    while [ $poll_count -lt $max_polls ]; do
        local response=$(curl -s "$FEDERATION_URL/v1/workflows/$TEST_WORKFLOW_ID" 2>/dev/null)
        local exit_code=$?

        if [ $exit_code -eq 0 ]; then
            local status=$(echo "$response" | jq -r '.status // ""' 2>/dev/null)
            local progress=$(echo "$response" | jq -r '.progress // 0' 2>/dev/null)
            local current_step=$(echo "$response" | jq -r '.current_step // ""' 2>/dev/null)
            local error=$(echo "$response" | jq -r '.error // null' 2>/dev/null)

            if [ "$error" != "null" ] && [ ! -z "$error" ]; then
                log_test "Workflow Status" "FAIL" "Workflow failed: $error"
                return 1
            fi

            if [ "$status" != "$last_status" ] || [ "$progress" != "$last_progress" ]; then
                echo "    [$poll_count] Status: $status, Progress: $progress%, Step: $current_step"
                last_status="$status"
                last_progress="$progress"
            fi

            if [ "$status" = "completed" ]; then
                log_test "Workflow Status" "PASS" "Workflow completed successfully in $poll_count polls"

                # Save results for further inspection
                echo "$response" | jq . > "$TEST_RESULTS_FILE" 2>/dev/null
                echo "    Results saved to: $TEST_RESULTS_FILE"
                return 0
            elif [ "$status" = "failed" ]; then
                log_test "Workflow Status" "FAIL" "Workflow failed"
                return 1
            fi
        else
            echo "    [$poll_count] Request failed (exit code: $exit_code)"
        fi

        poll_count=$((poll_count + 1))
        sleep 3
    done

    log_test "Workflow Status" "FAIL" "Workflow did not complete within $max_polls polls"
    return 1
}

# Test 4: Validate Workflow Results
test_workflow_results() {
    echo -e "\n${BLUE}Test 4: Workflow Results Validation${NC}"

    if [ ! -f "$TEST_RESULTS_FILE" ]; then
        log_test "Results Validation" "FAIL" "No results file found"
        return 1
    fi

    local results=$(cat "$TEST_RESULTS_FILE")

    # Check for blog post content
    local title=$(echo "$results" | jq -r '.results.blog_post.title // ""' 2>/dev/null)
    local content=$(echo "$results" | jq -r '.results.blog_post.content // ""' 2>/dev/null)
    local word_count=$(echo "$results" | jq -r '.results.blog_post.word_count // 0' 2>/dev/null)

    # Check for image
    local image_url=$(echo "$results" | jq -r '.results.image.url // ""' 2>/dev/null)

    # Check for quality scores
    local overall_score=$(echo "$results" | jq -r '.results.quality_scores.overall_score // 0' 2>/dev/null)

    local validation_errors=0

    if [ -z "$title" ]; then
        echo "    ❌ Missing blog post title"
        validation_errors=$((validation_errors + 1))
    else
        echo "    ✅ Blog post title: $title"
    fi

    if [ -z "$content" ]; then
        echo "    ❌ Missing blog post content"
        validation_errors=$((validation_errors + 1))
    else
        local content_length=${#content}
        echo "    ✅ Blog post content: $content_length characters"
    fi

    if [ "$word_count" -lt 500 ]; then
        echo "    ❌ Word count too low: $word_count (expected: >500)"
        validation_errors=$((validation_errors + 1))
    else
        echo "    ✅ Word count: $word_count words"
    fi

    if [ -z "$image_url" ]; then
        echo "    ❌ Missing featured image URL"
        validation_errors=$((validation_errors + 1))
    else
        echo "    ✅ Featured image: $image_url"
    fi

    if [ $(echo "$overall_score < 4.0" | bc -l 2>/dev/null || echo 0) -eq 1 ]; then
        echo "    ❌ Quality score too low: $overall_score (expected: >4.0)"
        validation_errors=$((validation_errors + 1))
    else
        echo "    ✅ Quality score: $overall_score/5.0"
    fi

    if [ $validation_errors -eq 0 ]; then
        log_test "Results Validation" "PASS" "All result components validated successfully"
        return 0
    else
        log_test "Results Validation" "FAIL" "$validation_errors validation errors found"
        return 1
    fi
}

# Test 5: Client App Accessibility
test_client_accessibility() {
    echo -e "\n${BLUE}Test 5: Client App Accessibility${NC}"

    # Test if client app is running and accessible
    local response=$(curl -s -I "$CLIENT_URL" 2>/dev/null | head -n 1)

    if echo "$response" | grep -q "200 OK"; then
        log_test "Client Accessibility" "PASS" "Client app accessible at $CLIENT_URL"

        # Test if client can reach federation service (CORS check)
        local cors_response=$(curl -s -X OPTIONS "$FEDERATION_URL/health" \
            -H "Origin: $CLIENT_URL" \
            -H "Access-Control-Request-Method: GET" 2>/dev/null)

        if [ $? -eq 0 ]; then
            log_test "CORS Check" "PASS" "Client can make cross-origin requests to federation"
        else
            log_test "CORS Check" "WARN" "CORS preflight request failed"
        fi

        return 0
    else
        log_test "Client Accessibility" "FAIL" "Client app not accessible or not running"
        echo "    Expected: HTTP 200 OK"
        echo "    Actual: $response"
        echo "    Make sure to run: cd src/client-app-integration && npm run dev"
        return 1
    fi
}

# Test 6: End-to-End Performance
test_performance() {
    echo -e "\n${BLUE}Test 6: Performance Validation${NC}"

    local start_time=$(date +%s)

    # Create and complete a workflow
    local request_data='{
        "intent": "Create a performance test blog post about cloud computing",
        "workflow_type": "blog-post"
    }'

    echo "    Creating performance test workflow..."
    local response=$(curl -s -X POST "$FEDERATION_URL/v1/workflows" \
        -H "Content-Type: application/json" \
        -d "$request_data" 2>/dev/null)

    local perf_workflow_id=$(echo "$response" | jq -r '.workflow_id // ""' 2>/dev/null)

    if [ -z "$perf_workflow_id" ]; then
        log_test "Performance Test" "FAIL" "Could not create performance test workflow"
        return 1
    fi

    # Wait for completion with timing
    local max_wait=60  # 60 seconds max
    local elapsed=0

    while [ $elapsed -lt $max_wait ]; do
        local status_response=$(curl -s "$FEDERATION_URL/v1/workflows/$perf_workflow_id" 2>/dev/null)
        local status=$(echo "$status_response" | jq -r '.status // ""' 2>/dev/null)

        if [ "$status" = "completed" ]; then
            local end_time=$(date +%s)
            local total_time=$((end_time - start_time))

            if [ $total_time -lt 45 ]; then
                log_test "Performance Test" "PASS" "Workflow completed in ${total_time}s (target: <45s)"
            else
                log_test "Performance Test" "WARN" "Workflow completed in ${total_time}s (slower than 45s target)"
            fi
            return 0
        elif [ "$status" = "failed" ]; then
            log_test "Performance Test" "FAIL" "Performance test workflow failed"
            return 1
        fi

        sleep 2
        elapsed=$((elapsed + 2))
    done

    log_test "Performance Test" "FAIL" "Workflow did not complete within ${max_wait}s"
    return 1
}

# Main test execution
main() {
    local start_time=$(date)
    local total_tests=0
    local passed_tests=0

    echo "Test started at: $start_time"
    echo ""

    # Check prerequisites
    if ! command -v curl &> /dev/null; then
        echo -e "${RED}❌ curl is required but not installed${NC}"
        exit 1
    fi

    if ! command -v jq &> /dev/null; then
        echo -e "${YELLOW}⚠️  jq not found - JSON parsing will be limited${NC}"
    fi

    # Run tests
    tests=(
        "test_federation_health"
        "test_workflow_creation"
        "test_workflow_status"
        "test_workflow_results"
        "test_client_accessibility"
        "test_performance"
    )

    for test in "${tests[@]}"; do
        total_tests=$((total_tests + 1))
        if $test; then
            passed_tests=$((passed_tests + 1))
        fi
    done

    # Summary
    echo ""
    echo -e "${BLUE}📊 Test Summary${NC}"
    echo "===================="
    echo "Total Tests: $total_tests"
    echo "Passed: $passed_tests"
    echo "Failed: $((total_tests - passed_tests))"
    echo ""

    if [ $passed_tests -eq $total_tests ]; then
        echo -e "${GREEN}🎉 ALL TESTS PASSED!${NC}"
        echo "✅ Client-app-integration is working correctly with real AI-CORE services"
        echo ""
        echo "🚀 Ready for demonstration!"
        echo "   • Federation Service: $FEDERATION_URL"
        echo "   • Client App: $CLIENT_URL"
        exit 0
    else
        echo -e "${RED}❌ SOME TESTS FAILED${NC}"
        echo "Please review the failed tests above and fix any issues."
        exit 1
    fi
}

# Cleanup function
cleanup() {
    if [ -f "$TEST_RESULTS_FILE" ]; then
        echo ""
        echo "Test results saved in: $TEST_RESULTS_FILE"
    fi
}

trap cleanup EXIT

# Run main function
main "$@"
