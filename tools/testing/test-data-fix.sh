#!/bin/bash

# AI-CORE Data Structure Fix Integration Test
# Tests the complete workflow from API call to client display

set -e

echo "🧪 AI-CORE Data Structure Fix - Integration Test"
echo "=============================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
FEDERATION_URL="http://localhost:8801"
CLIENT_URL="http://localhost:5173"
TEST_INTENT="Write a blog post about AI automation benefits"

# Function to check service health
check_service() {
    local service_name=$1
    local url=$2

    echo -n "Checking ${service_name}... "
    if curl -s "${url}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ Running${NC}"
        return 0
    else
        echo -e "${RED}❌ Not available${NC}"
        return 1
    fi
}

# Function to wait for workflow completion
wait_for_completion() {
    local workflow_id=$1
    local max_attempts=30
    local attempt=0

    echo "⏳ Waiting for workflow completion..."

    while [ $attempt -lt $max_attempts ]; do
        local response=$(curl -s "${FEDERATION_URL}/v1/workflows/${workflow_id}")
        local status=$(echo "$response" | jq -r '.status')
        local progress=$(echo "$response" | jq -r '.progress')

        echo -n "Attempt $((attempt + 1))/$max_attempts - Status: ${status}, Progress: ${progress}%"

        if [ "$status" = "completed" ]; then
            echo -e " ${GREEN}✅ Complete${NC}"
            return 0
        elif [ "$status" = "failed" ]; then
            echo -e " ${RED}❌ Failed${NC}"
            return 1
        else
            echo -e " ${YELLOW}⏳ In progress${NC}"
        fi

        sleep 2
        attempt=$((attempt + 1))
    done

    echo -e "${RED}❌ Timeout waiting for workflow completion${NC}"
    return 1
}

# Function to test data structure mapping
test_data_structure() {
    local workflow_id=$1

    echo ""
    echo "🔍 Testing Data Structure Mapping"
    echo "--------------------------------"

    # Get workflow status
    local response=$(curl -s "${FEDERATION_URL}/v1/workflows/${workflow_id}")

    # Check if response has expected structure
    local blog_post_content=$(echo "$response" | jq -r '.results.blog_post.content // empty')
    local blog_post_title=$(echo "$response" | jq -r '.results.blog_post.title // empty')
    local word_count=$(echo "$response" | jq -r '.results.blog_post.word_count // 0')
    local quality_score=$(echo "$response" | jq -r '.results.quality_scores.overall_score // 0')
    local image_url=$(echo "$response" | jq -r '.results.image.url // empty')

    # Test results
    echo -n "Blog post content: "
    if [ -n "$blog_post_content" ] && [ "$blog_post_content" != "null" ]; then
        echo -e "${GREEN}✅ Present (${#blog_post_content} chars)${NC}"
    else
        echo -e "${RED}❌ Missing${NC}"
        return 1
    fi

    echo -n "Blog post title: "
    if [ -n "$blog_post_title" ] && [ "$blog_post_title" != "null" ]; then
        echo -e "${GREEN}✅ Present${NC}"
        echo "   Title: ${blog_post_title}"
    else
        echo -e "${RED}❌ Missing${NC}"
        return 1
    fi

    echo -n "Word count: "
    if [ "$word_count" -gt 0 ]; then
        echo -e "${GREEN}✅ ${word_count} words${NC}"
    else
        echo -e "${RED}❌ Missing or zero${NC}"
        return 1
    fi

    echo -n "Quality score: "
    if [ "$quality_score" != "0" ] && [ "$quality_score" != "null" ]; then
        echo -e "${GREEN}✅ ${quality_score}/5.0${NC}"
    else
        echo -e "${RED}❌ Missing${NC}"
        return 1
    fi

    echo -n "Featured image: "
    if [ -n "$image_url" ] && [ "$image_url" != "null" ]; then
        echo -e "${GREEN}✅ Present${NC}"
        echo "   URL: ${image_url}"
    else
        echo -e "${RED}❌ Missing${NC}"
        return 1
    fi

    return 0
}

# Function to test client transformation
test_client_transformation() {
    echo ""
    echo "🔧 Testing Client Data Transformation"
    echo "-----------------------------------"

    # Run the transformation test
    cd src/client-app-integration
    if node test-transformation.cjs > /tmp/transform_test.log 2>&1; then
        echo -e "${GREEN}✅ Transformation test passed${NC}"

        # Show key results
        if grep -q "Content Length: [0-9]" /tmp/transform_test.log; then
            local content_length=$(grep "Content Length:" /tmp/transform_test.log | awk '{print $3}')
            echo "   Content length: ${content_length} characters"
        fi

        if grep -q "Word Count: [0-9]" /tmp/transform_test.log; then
            local word_count=$(grep "Word Count:" /tmp/transform_test.log | awk '{print $3}')
            echo "   Word count: ${word_count}"
        fi

        if grep -q "Quality Score: [0-9]" /tmp/transform_test.log; then
            local quality_score=$(grep "Quality Score:" /tmp/transform_test.log | awk '{print $3}')
            echo "   Quality score: ${quality_score}"
        fi

        cd - > /dev/null
        return 0
    else
        echo -e "${RED}❌ Transformation test failed${NC}"
        cat /tmp/transform_test.log
        cd - > /dev/null
        return 1
    fi
}

# Function to verify client build
test_client_build() {
    echo ""
    echo "🏗️  Testing Client Build"
    echo "----------------------"

    cd src/client-app-integration
    if npm run build > /tmp/build_test.log 2>&1; then
        echo -e "${GREEN}✅ Client build successful${NC}"

        # Show bundle size
        if grep -q "dist/assets/index-.*\.js" /tmp/build_test.log; then
            local bundle_info=$(grep "dist/assets/index-.*\.js" /tmp/build_test.log)
            echo "   Bundle: ${bundle_info}"
        fi

        cd - > /dev/null
        return 0
    else
        echo -e "${RED}❌ Client build failed${NC}"
        cat /tmp/build_test.log
        cd - > /dev/null
        return 1
    fi
}

# Main test execution
main() {
    echo "📋 Pre-flight Checks"
    echo "------------------"

    # Check required tools
    for tool in curl jq node npm; do
        if ! command -v $tool > /dev/null 2>&1; then
            echo -e "${RED}❌ Required tool not found: ${tool}${NC}"
            exit 1
        fi
    done
    echo -e "${GREEN}✅ All required tools available${NC}"

    # Check services
    echo ""
    echo "🌐 Service Health Checks"
    echo "----------------------"

    check_service "Federation Service" "$FEDERATION_URL"

    echo ""
    echo "🚀 Starting Integration Test"
    echo "==========================="

    # Create workflow
    echo "📝 Creating new workflow..."
    local create_response=$(curl -s -X POST "${FEDERATION_URL}/v1/workflows" \
        -H "Content-Type: application/json" \
        -d "{
            \"intent\": \"${TEST_INTENT}\",
            \"workflow_type\": \"blog-post-generation\",
            \"client_context\": {
                \"user_id\": \"test_user_fix_validation\",
                \"session_id\": \"test_session_$(date +%s)\"
            }
        }")

    local workflow_id=$(echo "$create_response" | jq -r '.workflow_id')

    if [ "$workflow_id" = "null" ] || [ -z "$workflow_id" ]; then
        echo -e "${RED}❌ Failed to create workflow${NC}"
        echo "Response: $create_response"
        exit 1
    fi

    echo -e "${GREEN}✅ Workflow created: ${workflow_id}${NC}"

    # Wait for completion
    if ! wait_for_completion "$workflow_id"; then
        echo -e "${RED}❌ Workflow did not complete successfully${NC}"
        exit 1
    fi

    # Test data structure
    if ! test_data_structure "$workflow_id"; then
        echo -e "${RED}❌ Data structure test failed${NC}"
        exit 1
    fi

    # Test client transformation
    if ! test_client_transformation; then
        echo -e "${RED}❌ Client transformation test failed${NC}"
        exit 1
    fi

    # Test client build
    if ! test_client_build; then
        echo -e "${RED}❌ Client build test failed${NC}"
        exit 1
    fi

    # Summary
    echo ""
    echo "🎉 Integration Test Results"
    echo "========================="
    echo -e "${GREEN}✅ All tests passed!${NC}"
    echo ""
    echo "📊 Test Summary:"
    echo "  • Workflow creation: ✅ Success"
    echo "  • Workflow execution: ✅ Complete"
    echo "  • Data structure mapping: ✅ Correct"
    echo "  • Client transformation: ✅ Working"
    echo "  • Client build: ✅ Successful"
    echo ""
    echo "🎯 Data Structure Fix Status: ✅ RESOLVED"
    echo ""
    echo "The client app should now correctly display:"
    echo "  • Blog post content (HTML)"
    echo "  • Word count and metadata"
    echo "  • Quality scores"
    echo "  • Featured images"
    echo "  • Real-time execution logs"
    echo ""
    echo "🚀 Ready for production deployment!"

    # Cleanup
    rm -f /tmp/transform_test.log /tmp/build_test.log
}

# Execute main function
main "$@"
