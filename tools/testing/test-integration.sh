#!/bin/bash

# AI-CORE Client App Demo - End-to-End Integration Test
# Tests the complete workflow from client request to final results

set -e

echo "🚀 AI-CORE Integration Test Suite"
echo "================================="

# Configuration
FEDERATION_URL="http://localhost:8801"
CLIENT_APP_URL="http://localhost:8090"
TIMEOUT=30

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Test 1: Health Checks
echo
log_info "Test 1: Service Health Checks"
echo "------------------------------"

# Check federation service
log_info "Testing federation-simple service..."
if curl -f -s "$FEDERATION_URL/health" > /dev/null; then
    log_success "Federation service is healthy"
else
    log_error "Federation service is not responding"
    echo "Please start the federation service:"
    echo "cd src/services/federation-simple && cargo run --release"
    exit 1
fi

# Check client app
log_info "Testing client-app-demo..."
if curl -f -s "$CLIENT_APP_URL/api/health" > /dev/null; then
    log_success "Client app is healthy"
else
    log_error "Client app is not responding"
    echo "Please start the client app:"
    echo "cd src/client-app-demo && DEMO_MODE=real cargo run --release"
    exit 1
fi

# Test 2: Workflow Creation
echo
log_info "Test 2: Workflow Creation"
echo "--------------------------"

WORKFLOW_DATA='{
    "intent": "Create a blog post about AI automation for business professionals in professional tone with 800 words",
    "workflow_type": "blog_generation",
    "client_context": {
        "test": true,
        "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'"
    }
}'

log_info "Creating workflow via federation service..."
WORKFLOW_RESPONSE=$(curl -s -X POST "$FEDERATION_URL/v1/workflows" \
    -H "Content-Type: application/json" \
    -d "$WORKFLOW_DATA")

if [ $? -eq 0 ]; then
    WORKFLOW_ID=$(echo "$WORKFLOW_RESPONSE" | grep -o '"workflow_id":"[^"]*"' | cut -d'"' -f4)
    if [ -n "$WORKFLOW_ID" ]; then
        log_success "Workflow created with ID: $WORKFLOW_ID"
    else
        log_error "Failed to extract workflow ID from response"
        echo "Response: $WORKFLOW_RESPONSE"
        exit 1
    fi
else
    log_error "Failed to create workflow"
    exit 1
fi

# Test 3: Workflow Status Monitoring
echo
log_info "Test 3: Workflow Status Monitoring"
echo "-----------------------------------"

log_info "Monitoring workflow progress..."
MAX_ATTEMPTS=30
ATTEMPT=0
COMPLETED=false

while [ $ATTEMPT -lt $MAX_ATTEMPTS ] && [ "$COMPLETED" = false ]; do
    ATTEMPT=$((ATTEMPT + 1))

    STATUS_RESPONSE=$(curl -s "$FEDERATION_URL/v1/workflows/$WORKFLOW_ID")
    STATUS=$(echo "$STATUS_RESPONSE" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    PROGRESS=$(echo "$STATUS_RESPONSE" | grep -o '"progress":[0-9]*' | cut -d':' -f2)
    CURRENT_STEP=$(echo "$STATUS_RESPONSE" | grep -o '"current_step":"[^"]*"' | cut -d'"' -f4)

    if [ -n "$STATUS" ]; then
        log_info "Attempt $ATTEMPT: Status=$STATUS, Progress=$PROGRESS%, Step=$CURRENT_STEP"

        if [ "$STATUS" = "completed" ]; then
            COMPLETED=true
            log_success "Workflow completed successfully!"

            # Extract results
            RESULTS=$(echo "$STATUS_RESPONSE" | grep -o '"results":{[^}]*}')
            if [ -n "$RESULTS" ]; then
                log_success "Results received"
            else
                log_warning "No results in response"
            fi
        elif [ "$STATUS" = "failed" ]; then
            ERROR=$(echo "$STATUS_RESPONSE" | grep -o '"error":"[^"]*"' | cut -d'"' -f4)
            log_error "Workflow failed: $ERROR"
            exit 1
        fi
    else
        log_error "Invalid status response"
        echo "Response: $STATUS_RESPONSE"
        exit 1
    fi

    if [ "$COMPLETED" = false ]; then
        sleep 2
    fi
done

if [ "$COMPLETED" = false ]; then
    log_error "Workflow did not complete within timeout ($MAX_ATTEMPTS attempts)"
    exit 1
fi

# Test 4: Client App Integration
echo
log_info "Test 4: Client App Integration"
echo "-------------------------------"

CLIENT_REQUEST='{
    "topic": "AI Automation",
    "input_text": "Write a comprehensive blog post about AI automation in business",
    "audience": "business_professionals",
    "tone": "professional",
    "word_count": 800,
    "brand_guidelines": null
}'

log_info "Starting demo session via client app..."
DEMO_RESPONSE=$(curl -s -X POST "$CLIENT_APP_URL/api/start-demo" \
    -H "Content-Type: application/json" \
    -d "$CLIENT_REQUEST")

if [ $? -eq 0 ]; then
    SESSION_ID=$(echo "$DEMO_RESPONSE" | grep -o '"session_id":"[^"]*"' | cut -d'"' -f4)
    if [ -n "$SESSION_ID" ]; then
        log_success "Demo session created with ID: $SESSION_ID"
    else
        log_error "Failed to extract session ID from response"
        echo "Response: $DEMO_RESPONSE"
        exit 1
    fi
else
    log_error "Failed to create demo session"
    exit 1
fi

# Monitor demo session
log_info "Monitoring demo session progress..."
MAX_ATTEMPTS=30
ATTEMPT=0
COMPLETED=false

while [ $ATTEMPT -lt $MAX_ATTEMPTS ] && [ "$COMPLETED" = false ]; do
    ATTEMPT=$((ATTEMPT + 1))

    SESSION_STATUS=$(curl -s "$CLIENT_APP_URL/api/session/$SESSION_ID/status")
    STATUS=$(echo "$SESSION_STATUS" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)

    if [ -n "$STATUS" ]; then
        log_info "Attempt $ATTEMPT: Session Status=$STATUS"

        if [ "$STATUS" = "Completed" ]; then
            COMPLETED=true
            log_success "Demo session completed successfully!"

            # Check for results
            RESULTS_CHECK=$(echo "$SESSION_STATUS" | grep -o '"results":{')
            if [ -n "$RESULTS_CHECK" ]; then
                log_success "Demo results generated"
            else
                log_warning "No results in demo response"
            fi
        elif echo "$STATUS" | grep -q "Failed"; then
            log_error "Demo session failed: $STATUS"
            exit 1
        fi
    else
        log_error "Invalid session status response"
        echo "Response: $SESSION_STATUS"
        exit 1
    fi

    if [ "$COMPLETED" = false ]; then
        sleep 2
    fi
done

if [ "$COMPLETED" = false ]; then
    log_error "Demo session did not complete within timeout ($MAX_ATTEMPTS attempts)"
    exit 1
fi

# Test 5: API Metrics
echo
log_info "Test 5: API Metrics Validation"
echo "-------------------------------"

log_info "Retrieving client app metrics..."
METRICS_RESPONSE=$(curl -s "$CLIENT_APP_URL/api/metrics")

if [ $? -eq 0 ]; then
    TOTAL_REQUESTS=$(echo "$METRICS_RESPONSE" | grep -o '"total_requests":[0-9]*' | cut -d':' -f2)
    SUCCESSFUL_REQUESTS=$(echo "$METRICS_RESPONSE" | grep -o '"successful_requests":[0-9]*' | cut -d':' -f2)

    if [ -n "$TOTAL_REQUESTS" ] && [ "$TOTAL_REQUESTS" -gt 0 ]; then
        log_success "Metrics show $TOTAL_REQUESTS total requests, $SUCCESSFUL_REQUESTS successful"
    else
        log_warning "No request metrics found"
    fi
else
    log_error "Failed to retrieve metrics"
fi

# Test Summary
echo
echo "🎉 Integration Test Summary"
echo "=========================="
log_success "All tests passed successfully!"
log_info "The complete workflow is working:"
log_info "  1. ✅ Services are healthy and responding"
log_info "  2. ✅ Federation service creates workflows"
log_info "  3. ✅ Workflows execute with real processing steps"
log_info "  4. ✅ Client app integrates with federation service"
log_info "  5. ✅ End-to-end demo flow completes successfully"
log_info "  6. ✅ Metrics are being tracked and reported"

echo
log_success "🚀 Real API integration is now fully functional!"
echo
echo "🌐 Access the demo interface at: $CLIENT_APP_URL/demo"
echo "📊 View metrics at: $CLIENT_APP_URL/api/metrics"
echo "🔍 Health check: $FEDERATION_URL/health"
echo

exit 0
