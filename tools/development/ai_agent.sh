#!/bin/bash

# AI-CORE Simple Agent Prompt System
# Usage: ./ai_agent.sh [agent_name] [task_description]

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

# Project paths
PROJECT_ROOT="/Volumes/T7Shield/HPZ620/AI-CORE"
AGENTS_DIR="${PROJECT_ROOT}/dev-works/dev-agents"
PROMPTS_DIR="${AGENTS_DIR}/prompts"

# Available agents (8 core agents)
AGENTS=(
    "architect_agent"
    "backend_agent"
    "frontend_agent"
    "database_agent"
    "security_agent"
    "integration_agent"
    "devops_agent"
    "qa_agent"
)

print_status() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to generate AI agent prompt
generate_agent_prompt() {
    local agent=$1
    local task=$2

    print_status $BLUE "🤖 Generating AI prompt for ${agent}..."

    cat << EOF

=== AI AGENT TASK PROMPT ===

AGENT: ${agent}
TASK: ${task}
PROJECT: AI-CORE Intelligent Automation Platform

CRITICAL REQUIREMENTS:
1. BUILD/RUN/TEST/FIX CYCLE - You MUST complete this cycle 100% before moving to next step:
   - BUILD: Compile/validate your code works
   - RUN: Execute and verify functionality
   - TEST: Run all tests and ensure 100% pass rate
   - FIX: Address any failures before proceeding

2. USE PROJECT CONTEXT:
   - Read your specific prompt: ${PROMPTS_DIR}/${agent}.md
   - Check CLAUDE.md for project standards
   - Follow shared workspace protocols in dev-works/dev-agentsshared-workspace/

3. QUALITY GATES:
   - All code must compile without errors
   - All tests must pass
   - Follow project coding standards
   - Document your work properly

4. COORDINATION:
   - Check dependencies in dev-works/dev-agentsAGENT_COORDINATION_GUIDE.md
   - Update progress in shared workspace
   - Coordinate with other agents as needed

YOUR TASK: ${task}

STEPS TO EXECUTE:
1. Read your agent prompt file: ${PROMPTS_DIR}/${agent}.md
2. Understand the task requirements
3. Plan your implementation approach
4. Execute BUILD/RUN/TEST/FIX cycle
5. Document deliverables
6. Update progress status

Remember: No task is complete until BUILD/RUN/TEST all pass 100%

=== END PROMPT ===

EOF
}

# Function to list available agents
list_agents() {
    print_status $BLUE "📋 Available AI Agents:"
    for agent in "${AGENTS[@]}"; do
        echo "  - ${agent}"
    done
}

# Function to show agent info
show_agent_info() {
    local agent=$1

    if [[ -f "${PROMPTS_DIR}/${agent}.md" ]]; then
        print_status $GREEN "📖 Agent: ${agent}"
        print_status $YELLOW "Prompt file: ${PROMPTS_DIR}/${agent}.md"
        print_status $BLUE "Sample usage: ./ai_agent.sh ${agent} 'implement user authentication'"
    else
        print_status $RED "❌ Agent prompt file not found: ${PROMPTS_DIR}/${agent}.md"
    fi
}

# Function to start agent with task
start_agent() {
    local agent=$1
    local task=$2

    # Validate agent exists
    if [[ ! " ${AGENTS[@]} " =~ " ${agent} " ]]; then
        print_status $RED "❌ Unknown agent: ${agent}"
        list_agents
        exit 1
    fi

    # Check if prompt file exists
    if [[ ! -f "${PROMPTS_DIR}/${agent}.md" ]]; then
        print_status $RED "❌ Agent prompt file not found: ${PROMPTS_DIR}/${agent}.md"
        exit 1
    fi

    # Generate and display the prompt
    generate_agent_prompt "$agent" "$task"

    print_status $GREEN "✅ AI prompt generated for ${agent}"
    print_status $YELLOW "Copy the prompt above and paste it to your AI assistant"
    print_status $BLUE "The AI will read the agent prompt file and execute the task following BUILD/RUN/TEST/FIX cycle"
}

# Function to show help
show_help() {
    cat << EOF
🤖 AI-CORE Simple Agent Prompt System

USAGE:
    ./ai_agent.sh [agent_name] [task_description]
    ./ai_agent.sh [command]

COMMANDS:
    list                    List all available agents
    info [agent_name]       Show agent information
    help                    Show this help

AGENTS:
$(for agent in "${AGENTS[@]}"; do echo "    ${agent}"; done)

EXAMPLES:
    ./ai_agent.sh architect_agent "design the user authentication system"
    ./ai_agent.sh backend_agent "implement REST API for user management"
    ./ai_agent.sh frontend_agent "create login form component"
    ./ai_agent.sh qa_agent "write integration tests for auth flow"

PARALLEL TASK COMMANDS:
    # Start foundation phase (run these in parallel AI sessions)
    ./ai_agent.sh architect_agent "create system architecture and API contracts"
    ./ai_agent.sh devops_agent "setup development infrastructure"

    # Start data phase (after architect completes)
    ./ai_agent.sh database_agent "implement multi-database integration"
    ./ai_agent.sh security_agent "implement authentication and authorization"

    # Start services phase (after database & security complete)
    ./ai_agent.sh backend_agent "implement core microservices and API gateway"

    # Start client phase (after backend completes)
    ./ai_agent.sh frontend_agent "implement React/Tauri client application"
    ./ai_agent.sh integration_agent "implement external API integrations"

    # Start QA phase (after all development completes)
    ./ai_agent.sh qa_agent "implement comprehensive testing framework"

The AI will automatically:
- Read the agent-specific prompt from dev-works/dev-agentsprompts/
- Follow BUILD/RUN/TEST/FIX cycle requirements
- Coordinate with other agents through shared workspace
- Ensure 100% quality before proceeding to next task

EOF
}

# Main logic
main() {
    local command=${1:-"help"}
    local task_or_agent=${2:-""}
    local task=${3:-""}

    case "$command" in
        "list")
            list_agents
            ;;
        "info")
            if [[ -z "$task_or_agent" ]]; then
                print_status $RED "❌ Agent name required for info command"
                exit 1
            fi
            show_agent_info "$task_or_agent"
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            # If first arg is an agent name, treat as start command
            if [[ " ${AGENTS[@]} " =~ " ${command} " ]]; then
                if [[ -z "$task_or_agent" ]]; then
                    print_status $RED "❌ Task description required"
                    print_status $YELLOW "Usage: ./ai_agent.sh ${command} 'task description'"
                    exit 1
                fi
                start_agent "$command" "$task_or_agent"
            else
                print_status $RED "❌ Unknown command or agent: ${command}"
                show_help
                exit 1
            fi
            ;;
    esac
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
