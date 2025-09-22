#!/bin/bash

# AI-CORE Continuity Prompt Generator
# Automatically generates AI agent handoff prompts from session data
# Version: 1.0
# Created: 2025-09-14T10:03:59+00:00

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SESSIONS_DIR="$PROJECT_ROOT/dev-works/sessions"
TEMPLATES_DIR="$PROJECT_ROOT/templates"
OUTPUT_DIR="$PROJECT_ROOT/dev-works/continuity-prompts"

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${BLUE}[GENERATE]${NC} $1"
}

# Function to get current UTC timestamp
get_timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%S+00:00"
}

# Function to get session ID from timestamp
get_session_id() {
    date -u +"%Y%m%d%H%M%S"
}

# Function to extract session data
extract_session_data() {
    local session_file="$1"
    local temp_data="$OUTPUT_DIR/.temp_session_data.json"

    # Parse session file and extract key information
    cat > "$temp_data" << EOF
{
    "session_file": "$(basename "$session_file")",
    "timestamp": "$(get_timestamp)",
    "session_id": "$(get_session_id)",
    "objective": "$(grep -m1 "## 🎯 SESSION OBJECTIVE" "$session_file" -A5 | tail -n+3 | head -n1 | sed 's/^[[:space:]]*//' || echo 'Not specified')",
    "status": "$(grep -m1 "Status:" "$session_file" | sed 's/.*Status:[[:space:]]*//' || echo 'Unknown')",
    "work_dir": "$(grep -m1 "Working Directory:" "$session_file" | sed 's/.*Working Directory:[[:space:]]*//' || echo 'AI-CORE/')",
    "progress": "$(grep -m1 "Progress:" "$session_file" | sed 's/.*Progress:[[:space:]]*//' || echo 'Unknown')",
    "last_action": "$(grep -A1 "Last Action" "$session_file" | tail -n1 | sed 's/^[[:space:]]*//' || echo 'Not specified')",
    "next_step": "$(grep -A1 "Next Step" "$session_file" | tail -n1 | sed 's/^[[:space:]]*//' || echo 'Not specified')"
}
EOF

    echo "$temp_data"
}

# Function to generate continuity prompt
generate_prompt() {
    local session_file="$1"
    local agent_type="${2:-coordinator}"
    local output_file="$3"

    print_header "Generating continuity prompt from $(basename "$session_file")"

    # Extract session data
    local data_file
    data_file=$(extract_session_data "$session_file")

    # Read template
    local template_file="$TEMPLATES_DIR/ai-agent-continuity-prompt.md"
    if [[ ! -f "$template_file" ]]; then
        print_error "Template file not found: $template_file"
        return 1
    fi

    # Extract data from JSON
    local session_id objective status work_dir progress last_action next_step
    session_id=$(jq -r '.session_id' "$data_file")
    objective=$(jq -r '.objective' "$data_file")
    status=$(jq -r '.status' "$data_file")
    work_dir=$(jq -r '.work_dir' "$data_file")
    progress=$(jq -r '.progress' "$data_file")
    last_action=$(jq -r '.last_action' "$data_file")
    next_step=$(jq -r '.next_step' "$data_file")

    # Generate the prompt by replacing placeholders
    cp "$template_file" "$output_file"

    # Replace placeholders with actual data
    sed -i.bak \
        -e "s/\[Generate: ACTIVE-YYYYMMDDHHMMSS-task-name\]/ACTIVE-${session_id}-continuation/" \
        -e "s/AI-CORE\/\[current-working-path\]/${work_dir//\//\\/}/" \
        -e "s/\[backend-specialist|frontend-expert|infrastructure-agent|coordinator|etc.\]/$agent_type/" \
        -e "s/\[Specific task name\]/$objective/" \
        -e "s/\[X%\] complete/$progress/" \
        -e "s/\[Specific last action taken\]/$last_action/" \
        -e "s/\[Immediate next action needed\]/$next_step/" \
        -e "s/2025-09-14T10:03:59+00:00/$(get_timestamp)/" \
        "$output_file"

    # Add session-specific context from original file
    print_status "Adding session-specific context..."

    # Extract completed tasks
    local completed_tasks
    completed_tasks=$(grep -A20 "### ✅ COMPLETED TASKS" "$session_file" | grep "^- \[x\]" | head -10 || echo "- [x] Session initialized")

    # Extract current files being worked on
    local current_files
    current_files=$(grep -A10 "Current File" "$session_file" | grep -E "^\s*-\s*\`.*\`" | head -5 || echo "- \`$work_dir\` - main working area")

    # Extract build/test status
    local build_status
    build_status=$(grep -i "build.*status" "$session_file" | tail -1 | sed 's/.*://' || echo "unknown")

    # Update the template with extracted data
    awk -v completed_tasks="$completed_tasks" -v current_files="$current_files" -v build_status="$build_status" '
    /### ✅ COMPLETED TASKS/ {
        print $0
        print completed_tasks
        next
    }
    /\*\*Current File\(s\)\*\*:/ {
        print $0
        print current_files
        next
    }
    /Build Status:/ {
        print "Build Status: " build_status
        next
    }
    { print }
    ' "$output_file" > "$output_file.tmp" && mv "$output_file.tmp" "$output_file"

    # Add continuation prompt section
    cat >> "$output_file" << EOF

## 🚀 GENERATED CONTINUATION PROMPT

**FOR NEXT AI AGENT:**

"Continue the work on $objective in the AI-CORE repository.

**CURRENT CONTEXT**:
- Working in: \`$work_dir\`
- Last completed: $last_action
- Currently implementing: $objective ($progress)
- Next priority: $next_step

**TECHNICAL STATE**:
- Build status: $build_status
- Session: ACTIVE-${session_id}-continuation
- Original session: $(basename "$session_file")

**IMMEDIATE ACTION NEEDED**:
$next_step

**VALIDATION**:
Run these commands to verify state before starting:
\`\`\`bash
cd AI-CORE/
cargo build --release
cargo test --workspace
./tools/ai-work-tracker.sh -Action start-session -AgentName "$agent_type" -Objective "continuation-$objective"
\`\`\`

Please continue from where the previous agent left off, following the AI-CORE project standards in AGENTS.md and maintaining session continuity."

---

**Generated**: $(get_timestamp)
**Source Session**: $(basename "$session_file")
**Target Agent**: $agent_type
**Auto-generated by**: generate-continuity-prompt.sh v1.0

EOF

    # Clean up temp file
    rm -f "$data_file" "$output_file.bak"

    print_status "Continuity prompt generated: $output_file"
}

# Function to list available sessions
list_sessions() {
    print_header "Available Sessions"

    if [[ ! -d "$SESSIONS_DIR" ]]; then
        print_error "Sessions directory not found: $SESSIONS_DIR"
        return 1
    fi

    echo "Active Sessions:"
    find "$SESSIONS_DIR" -name "ACTIVE-*.md" -exec basename {} \; | sort

    echo -e "\nRecent Completed Sessions:"
    find "$SESSIONS_DIR" -name "COMPLETED-*.md" -exec basename {} \; | sort -r | head -5
}

# Function to show usage
show_usage() {
    cat << EOF
AI-CORE Continuity Prompt Generator

USAGE:
    $0 [OPTIONS] <session-file> [agent-type] [output-file]

OPTIONS:
    -h, --help          Show this help message
    -l, --list          List available sessions
    -a, --auto          Auto-detect active session and generate prompt
    -t, --template      Show template structure
    -v, --validate      Validate generated prompt

ARGUMENTS:
    session-file        Path to session file (relative to dev-works/sessions/)
    agent-type          Target agent type (default: coordinator)
                       Options: backend-specialist, frontend-expert, infrastructure-agent,
                               coordinator, mvp-implementer, testing-specialist, etc.
    output-file         Output file path (default: auto-generated in continuity-prompts/)

EXAMPLES:
    # Generate from specific session
    $0 ACTIVE-20250912163337-client-app-integration.md frontend-expert

    # Auto-detect active session
    $0 --auto backend-specialist

    # List available sessions
    $0 --list

    # Generate with custom output file
    $0 COMPLETED-session.md coordinator /path/to/output.md

AGENT TYPES:
    - backend-specialist     (Rust/systems work)
    - frontend-expert        (React/TypeScript/UI)
    - infrastructure-agent   (DevOps/deployment)
    - coordinator           (project management)
    - mvp-implementer       (rapid prototyping)
    - testing-specialist    (QA/testing)
    - documentation-expert  (docs/specs)
    - performance-optimizer (benchmarking/tuning)
    - security-auditor      (security review)
    - integration-specialist (API/service integration)
    - data-architect        (database/analytics)
    - ui-ux-designer       (design/user experience)
    - devops-engineer      (CI/CD/automation)

EOF
}

# Function to validate prompt
validate_prompt() {
    local prompt_file="$1"

    print_header "Validating continuity prompt: $(basename "$prompt_file")"

    local issues=0

    # Check required sections
    local required_sections=(
        "SESSION CONTEXT"
        "WORK STATUS SUMMARY"
        "COMPLETED TASKS"
        "CURRENTLY WORKING ON"
        "TECHNICAL CONTEXT"
        "IMMEDIATE NEXT ACTIONS"
        "VALIDATION CHECKLIST"
        "CONTINUATION PROMPT"
    )

    for section in "${required_sections[@]}"; do
        if ! grep -q "$section" "$prompt_file"; then
            print_error "Missing required section: $section"
            ((issues++))
        fi
    done

    # Check for unfilled placeholders
    local placeholders
    placeholders=$(grep -o '\[.*\]' "$prompt_file" | sort -u)
    if [[ -n "$placeholders" ]]; then
        print_warning "Found unfilled placeholders:"
        echo "$placeholders" | sed 's/^/  /'
        ((issues++))
    fi

    # Check file paths
    if ! grep -q "AI-CORE/" "$prompt_file"; then
        print_warning "No AI-CORE file paths found"
        ((issues++))
    fi

    # Summary
    if [[ $issues -eq 0 ]]; then
        print_status "✅ Prompt validation passed"
        return 0
    else
        print_error "❌ Prompt validation failed with $issues issues"
        return 1
    fi
}

# Function to auto-detect active session
auto_detect_session() {
    local active_session
    active_session=$(find "$SESSIONS_DIR" -name "ACTIVE-*.md" | head -1)

    if [[ -z "$active_session" ]]; then
        print_error "No active session found"
        return 1
    fi

    echo "$active_session"
}

# Main execution
main() {
    case "${1:-}" in
        -h|--help)
            show_usage
            exit 0
            ;;
        -l|--list)
            list_sessions
            exit 0
            ;;
        -a|--auto)
            local agent_type="${2:-coordinator}"
            local active_session
            active_session=$(auto_detect_session)
            if [[ -n "$active_session" ]]; then
                local output_file="$OUTPUT_DIR/continuity-prompt-$(get_session_id).md"
                generate_prompt "$active_session" "$agent_type" "$output_file"
                validate_prompt "$output_file"
            fi
            exit 0
            ;;
        -t|--template)
            if [[ -f "$TEMPLATES_DIR/ai-agent-continuity-prompt.md" ]]; then
                head -50 "$TEMPLATES_DIR/ai-agent-continuity-prompt.md"
                echo -e "\n... (showing first 50 lines of template)"
            else
                print_error "Template not found"
            fi
            exit 0
            ;;
        -v|--validate)
            if [[ -n "${2:-}" ]]; then
                validate_prompt "$2"
            else
                print_error "Please specify prompt file to validate"
                exit 1
            fi
            exit 0
            ;;
        "")
            print_error "No arguments provided"
            show_usage
            exit 1
            ;;
        *)
            # Generate prompt from session file
            local session_file="$1"
            local agent_type="${2:-coordinator}"
            local output_file="${3:-$OUTPUT_DIR/continuity-prompt-$(get_session_id).md}"

            # Handle relative path
            if [[ ! -f "$session_file" ]]; then
                session_file="$SESSIONS_DIR/$session_file"
            fi

            if [[ ! -f "$session_file" ]]; then
                print_error "Session file not found: $session_file"
                exit 1
            fi

            generate_prompt "$session_file" "$agent_type" "$output_file"
            validate_prompt "$output_file"

            print_status "Generated continuity prompt ready for use!"
            print_status "File: $output_file"
            ;;
    esac
}

# Check dependencies
if ! command -v jq &> /dev/null; then
    print_error "jq is required but not installed. Please install jq."
    exit 1
fi

# Run main function
main "$@"
