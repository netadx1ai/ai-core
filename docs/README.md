# AI-CORE Documentation

> Documentation for the first open source AI agents platform powered by Temporal.io, OpenAI Agents SDK, MCP standard, and A2A standard.

Welcome to AI-CORE, the pioneering open source platform for building, orchestrating, and scaling AI agent workflows. This documentation will guide you through everything from basic setup to advanced multi-agent orchestration.

## 🚀 Quick Navigation

- **[Getting Started](#getting-started)** - Set up your first AI agent in 10 minutes
- **[Architecture](#architecture)** - Understanding the AI agents platform
- **[API Reference](#api-reference)** - Complete endpoint documentation
- **[AI Agents Guide](#ai-agents-guide)** - Building intelligent agents
- **[Workflow Orchestration](#workflow-orchestration)** - Temporal.io integration
- **[Contributing](#contributing)** - Join the development

## 🎯 Getting Started

### Prerequisites

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Node.js 18+** - [Install Node.js](https://nodejs.org/)
- **Docker** - [Install Docker](https://docs.docker.com/get-docker/)
- **OpenAI API Key** - [Get API Key](https://platform.openai.com/api-keys)
- **Temporal Server** - [Install Temporal](https://docs.temporal.io/cli#install)

### Quick Setup

```bash
# Clone the repository
git clone https://github.com/netadx1ai/ai-core.git
cd ai-core

# Install dependencies
cargo build --release
cd src/ui && npm install && cd ../..

# Configure environment
cp .env.example .env
# Edit .env with your OpenAI API key and settings

# Start Temporal server (in separate terminal)
temporal server start-dev

# Start Redis for MCP caching
redis-server

# Launch AI-CORE platform
./tools/deployment/start-ai-core.sh
```

### First AI Agent

Create your first AI agent in minutes:

```rust
use ai_core::agents::{Agent, AgentConfig};
use ai_core::temporal::WorkflowContext;

#[derive(Agent)]
struct CodeReviewAgent {
    openai_client: OpenAIClient,
}

impl CodeReviewAgent {
    async fn review_code(&self, code: &str) -> Result<CodeReview, Error> {
        // AI agent logic using OpenAI SDK
        let review = self.openai_client
            .analyze_code(code)
            .await?;

        Ok(review)
    }
}
```

## 🏗️ Architecture

AI-CORE is built on four foundational technologies:

### Core Technologies

```
┌─────────────────────────────────────────────────────────┐
│                   AI AGENTS LAYER                      │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐    │
│  │   OpenAI    │ │ Temporal.io │ │ A2A Communication│    │
│  │ Agents SDK  │ │  Workflows  │ │   & MCP Protocol│    │
│  └─────────────┘ └─────────────┘ └─────────────────┘    │
└─────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────┐
│                 PLATFORM SERVICES                      │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐    │
│  │API Gateway  │ │MCP Manager  │ │ Intent Parser   │    │
│  │   (Rust)    │ │   (Rust)    │ │     (Rust)      │    │
│  └─────────────┘ └─────────────┘ └─────────────────┘    │
│                                                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐    │
│  │   Agents    │ │   Shared    │ │       UI        │    │
│  │   (Rust)    │ │   (Rust)    │ │  (TypeScript)   │    │
│  └─────────────┘ └─────────────┘ └─────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

### Key Components

- **🔄 Temporal.io**: Durable workflow execution and agent orchestration
- **🤖 OpenAI Agents SDK**: Advanced AI reasoning and decision-making
- **🔗 MCP (Model Context Protocol)**: Standardized AI model communication
- **🤝 A2A (Agent-to-Agent)**: Inter-agent collaboration and coordination
- **⚡ Rust Backend**: High-performance microservices architecture
- **🎯 TypeScript Frontend**: Modern React-based agent dashboard

## 📚 Documentation Sections

### Core Concepts

- **[AI Agents Fundamentals](concepts/agents.md)** - Understanding AI agents
- **[Workflow Orchestration](concepts/workflows.md)** - Temporal.io workflows
- **[MCP Protocol](concepts/mcp.md)** - Model Context Protocol
- **[A2A Communication](concepts/a2a.md)** - Agent-to-agent messaging

### Development Guides

- **[Building Agents](guides/building-agents.md)** - Create custom AI agents
- **[Workflow Development](guides/workflows.md)** - Design agent workflows
- **[MCP Integration](guides/mcp-integration.md)** - Implement MCP services
- **[Testing Agents](guides/testing.md)** - Test agent behaviors

### API Reference

- **[REST API](api/rest.md)** - HTTP endpoints for agent management
- **[WebSocket API](api/websocket.md)** - Real-time agent communication
- **[Temporal API](api/temporal.md)** - Workflow management
- **[Agent SDK](api/sdk.md)** - Rust agent development kit

### Deployment

- **[Local Development](deployment/local.md)** - Development environment setup
- **[Docker Deployment](deployment/docker.md)** - Containerized deployment
- **[Kubernetes](deployment/kubernetes.md)** - Production orchestration
- **[Cloud Deployment](deployment/cloud.md)** - AWS, GCP, Azure deployment

### Examples

- **[Basic Agent](examples/basic-agent.md)** - Simple AI agent example
- **[Multi-Agent Workflow](examples/multi-agent.md)** - Coordinated agents
- **[Custom MCP Service](examples/mcp-service.md)** - Build MCP integration
- **[Production Deployment](examples/production.md)** - Real-world setup

## 🔌 API Reference

### Agent Management

```http
POST /api/v1/agents
GET /api/v1/agents
GET /api/v1/agents/{id}
PUT /api/v1/agents/{id}
DELETE /api/v1/agents/{id}
```

### Workflow Execution

```http
POST /api/v1/workflows
GET /api/v1/workflows
GET /api/v1/workflows/{id}
POST /api/v1/workflows/{id}/start
POST /api/v1/workflows/{id}/stop
```

### Real-time Communication

```javascript
// WebSocket connection for real-time agent monitoring
const ws = new WebSocket('ws://localhost:8080/ws/agents');
ws.onmessage = (event) => {
  const update = JSON.parse(event.data);
  console.log('Agent update:', update);
};
```

## 🤖 AI Agents Guide

### Agent Types

**Decision Agents**: Make autonomous decisions based on context
```rust
#[derive(Agent)]
struct DecisionAgent {
    decision_engine: OpenAIClient,
}
```

**Workflow Agents**: Orchestrate complex multi-step processes
```rust
#[derive(Agent)]
struct WorkflowAgent {
    temporal_client: TemporalClient,
}
```

**Communication Agents**: Handle A2A messaging and coordination
```rust
#[derive(Agent)]
struct CommunicationAgent {
    mcp_client: MCPClient,
}
```

### Agent Lifecycle

1. **Registration**: Register agent with the platform
2. **Initialization**: Set up agent context and dependencies
3. **Execution**: Process tasks and make decisions
4. **Communication**: Coordinate with other agents
5. **Monitoring**: Track performance and health
6. **Termination**: Graceful shutdown and cleanup

## ⚡ Workflow Orchestration

### Temporal.io Integration

AI-CORE uses Temporal.io for reliable workflow execution:

```rust
use temporal_sdk::{Workflow, Activity};

#[derive(Workflow)]
struct AgentWorkflow {
    agents: Vec<Box<dyn Agent>>,
}

impl AgentWorkflow {
    async fn execute_multi_agent_task(&self, task: Task) -> Result<TaskResult, Error> {
        // Coordinate multiple agents
        let results = futures::try_join!(
            self.agents[0].process(task.clone()),
            self.agents[1].analyze(task.clone()),
            self.agents[2].validate(task.clone()),
        )?;

        Ok(TaskResult::combine(results))
    }
}
```

### Workflow Patterns

- **Sequential**: Execute agents one after another
- **Parallel**: Run multiple agents simultaneously
- **Conditional**: Branch based on agent decisions
- **Loop**: Iterate until conditions are met
- **Saga**: Handle distributed transactions

## 🔗 MCP & A2A Standards

### Model Context Protocol (MCP)

Standardized communication between AI models:

```rust
use ai_core::mcp::{MCPMessage, MCPClient};

let message = MCPMessage::new()
    .with_context("code_review")
    .with_model("gpt-4")
    .with_prompt("Review this code for security issues");

let response = mcp_client.send(message).await?;
```

### Agent-to-Agent (A2A) Communication

Direct agent collaboration:

```rust
use ai_core::a2a::{A2AMessage, AgentAddress};

let message = A2AMessage::new()
    .to(AgentAddress::new("code-reviewer"))
    .with_payload(code_to_review);

agent.send_to_agent(message).await?;
```

## 🚀 Getting Help

### Community Resources

- **[GitHub Discussions](https://github.com/netadx1ai/ai-core/discussions)** - Ask questions and share ideas
- **[Discord Community](https://discord.gg/netadx)** - Real-time community support
- **[GitHub Issues](https://github.com/netadx1ai/ai-core/issues)** - Report bugs and request features

### Professional Support

- **Email**: [support@netadx.ai](mailto:support@netadx.ai)
- **Enterprise**: [enterprise@netadx.ai](mailto:enterprise@netadx.ai)
- **Security**: [security@netadx.ai](mailto:security@netadx.ai)

### Learning Resources

- **[Examples Repository](https://github.com/netadx1ai/ai-core-examples)** - Code samples and tutorials
- **[Video Tutorials](https://youtube.com/@netadxai)** - Visual learning content
- **[Blog Posts](https://blog.netadx.ai)** - In-depth articles and guides

## 📈 Roadmap

### Current Version: 1.0.0-alpha

- ✅ Core AI agents platform
- ✅ Temporal.io integration
- ✅ OpenAI Agents SDK support
- ✅ MCP protocol implementation
- ✅ A2A communication framework

### Coming Soon

- 🚧 Enhanced multi-agent orchestration
- 🚧 Visual workflow designer
- 🚧 Agent marketplace
- 🚧 Enterprise governance features
- 🚧 Multi-cloud deployment

### Future Vision

- 🔮 Advanced AI reasoning capabilities
- 🔮 Self-improving agent systems
- 🔮 Cross-platform agent federation
- 🔮 Industry-specific agent templates

---

**Welcome to the future of AI agent development!** 🤖

Start building intelligent, collaborative AI agents today with the first open source platform designed specifically for agent orchestration.

*Built with ❤️ by the NetADX.ai team*
