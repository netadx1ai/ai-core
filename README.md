# AI-CORE by NetADX.ai

<div align="center">
  <img src="https://netadx.ai/assets/NetADX.AI_logo_white.png" alt="NetADX AI Logo" width="300" />
</div>

> The first open source AI agents platform powered by Temporal.io, OpenAI Agents SDK, MCP standard, and A2A standard. A comprehensive AI-assisted software development environment with intelligent agent orchestration, multi-service architecture, and advanced development workflows.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![TypeScript](https://img.shields.io/badge/typescript-%23007ACC.svg?style=for-the-badge&logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=for-the-badge&logo=docker&logoColor=white)](https://www.docker.com/)

## Overview

AI-CORE is the **first open source AI agents platform** that revolutionizes software development through intelligent agent orchestration. Built on industry-leading standards and frameworks:

- **Temporal.io**: Reliable workflow orchestration for complex AI agent interactions
- **OpenAI Agents SDK**: Advanced AI agent capabilities and reasoning
- **MCP (Model Context Protocol)**: Standardized AI model communication
- **A2A (Agent-to-Agent)**: Seamless inter-agent collaboration standard

The platform combines Rust's performance with TypeScript's flexibility, creating a next-generation development environment where AI agents assist in development, testing, and deployment workflows. AI-CORE provides a comprehensive suite of tools for building, testing, and deploying modern applications with fully automated AI-driven workflows.

### Key Features

#### AI Agents Platform (First of its Kind)

- **Temporal.io Integration**: Durable workflow execution for AI agent orchestration
- **OpenAI Agents SDK**: Advanced reasoning and decision-making capabilities
- **MCP Standard Compliance**: Standardized model context protocol implementation
- **A2A Communication**: Native agent-to-agent collaboration and coordination
- **Multi-Agent Systems**: Coordinated AI agents working together on complex tasks

#### Platform Infrastructure

- **High-Performance Backend**: Rust-based microservices with async/await support
- **Modern Frontend**: TypeScript/React-based user interface
- **Development Tools**: Comprehensive tooling for testing, deployment, and monitoring
- **Real-time Analytics**: Built-in performance monitoring and metrics collection
- **Security-First**: Comprehensive security scanning and vulnerability assessment
- **Container-Ready**: Full Docker and Kubernetes support
- **Rich Documentation**: Extensive guides, examples, and API documentation

## Architecture

### AI Agents Platform Stack

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
└─────────────────────────────────────────────────────────┘
```

### Directory Structure

```
AI-CORE/
├── src/                    # Source code
│   ├── api-gateway/        # API gateway service (Rust)
│   ├── mcp-manager/        # MCP protocol implementation (Rust)
│   ├── intent-parser/      # AI intent parsing service (Rust)
│   ├── agents/             # AI agents implementation
│   │   ├── temporal/       # Temporal.io workflow agents
│   │   ├── openai/         # OpenAI SDK integration
│   │   └── coordination/   # A2A communication layer
│   ├── shared/             # Shared libraries and utilities
│   └── ui/                 # Frontend TypeScript/React application
├── tests/                  # Test suites
│   ├── integration/        # Integration tests
│   ├── unit/               # Unit tests
│   ├── e2e/                # End-to-end tests
│   └── agents/             # AI agents testing
├── docs/                   # Documentation
├── tools/                  # Development tools and scripts
├── config/                 # Configuration files
├── infrastructure/         # Infrastructure as code
└── examples/               # Usage examples and tutorials
    ├── basic/              # Basic AI agent examples
    ├── advanced/           # Multi-agent workflows
    └── tutorials/          # Step-by-step guides
```

## Quick Start

### Prerequisites

#### Core Requirements

- **Rust** 1.70+ ([Install Rust](https://rustup.rs/))
- **Node.js** 18+ ([Install Node.js](https://nodejs.org/))
- **Docker** 20+ ([Install Docker](https://docs.docker.com/get-docker/))
- **Git** ([Install Git](https://git-scm.com/downloads))

#### AI Platform Requirements

- **OpenAI API Key** ([Get API Key](https://platform.openai.com/api-keys))
- **Temporal Server** ([Install Temporal](https://docs.temporal.io/cli#install))
- **Redis** (for MCP protocol caching)

### Installation

1. **Clone the repository**

    ```bash
    git clone https://github.com/netadx1ai/ai-core.git
    cd AI-CORE
    ```

2. **Set up the development environment**

    ```bash
    # Install Rust dependencies
    cargo build --release

    # Install Node.js dependencies (if using UI)
    cd src/ui && npm install && cd ../..

    # Set up AI platform dependencies
    # Start Temporal server (in separate terminal)
    temporal server start-dev

    # Start Redis for MCP caching
    redis-server

    # Configure environment variables
    cp .env.example .env
    # Edit .env with your OpenAI API key and other settings

    # Set up development tools
    ./tools/deployment/setup-dev-environment.sh
    ```

3. **Run the AI agents platform**

    ```bash
    # Start all services (includes AI agents orchestration)
    ./tools/deployment/start-ai-core.sh

    # Or run individual services
    cargo run --bin api-gateway      # API Gateway
    cargo run --bin mcp-manager      # MCP Protocol Manager
    cargo run --bin intent-parser    # AI Intent Parser
    cargo run --bin agent-orchestrator # Temporal.io Agent Orchestrator

    # Start AI agents workflows
    ./tools/deployment/start-agents.sh
    ```

4. **Access the AI agents platform**
    - API Gateway: `http://localhost:8080`
    - Frontend UI: `http://localhost:3000`
    - API Documentation: `http://localhost:8080/docs`
    - Temporal Web UI: `http://localhost:8233`
    - AI Agents Dashboard: `http://localhost:3000/agents`
    - MCP Protocol Monitor: `http://localhost:8080/mcp`

## Documentation

- **[Quick Start Guide](docs/quick-start/)** - Get up and running in 5 minutes
- **[Architecture Overview](docs/architecture/)** - System design and components
- **[API Documentation](docs/api/)** - Complete API reference
- **[Development Guide](docs/developer-guides/)** - Contributing and development workflow
- **[Deployment Guide](docs/deployment/)** - Production deployment instructions
- **[Examples](examples/)** - Code examples and tutorials

## Development

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run integration tests
./tools/testing/run-integration-tests.sh

# Run frontend tests
cd src/ui && npm test
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run security audit
cargo audit

# Validate environment
./tools/testing/validate-environment.sh
```

### Development Workflow

1. Create a feature branch: `git checkout -b feature/your-feature`
2. Make your changes
3. Run tests: `cargo test --workspace`
4. Commit your changes: `git commit -m "feat: your feature description"`
5. Push to the branch: `git push origin feature/your-feature`
6. Create a Pull Request

## Docker Deployment

### Using Docker Compose

```bash
# Development environment
docker-compose -f config/docker/docker-compose.mvp.yml up

# Production environment
docker-compose -f infrastructure/docker/docker-compose.prod.yml up
```

### Building Images

```bash
# Build all services
docker build -t ai-core/api-gateway src/api-gateway/
docker build -t ai-core/mcp-manager src/mcp-manager/
docker build -t ai-core/intent-parser src/intent-parser/
```

## Contributing

We welcome contributions from the community! Please read our [Contributing Guide](docs/contributing/CONTRIBUTING.md) for details on:

- Code of Conduct
- Development workflow
- Pull request process
- Issue reporting guidelines

### Development Setup

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/ai-core.git`
3. Create a feature branch: `git checkout -b feature/amazing-feature`
4. Make your changes and add tests
5. Ensure all tests pass: `cargo test --workspace`
6. Commit your changes: `git commit -m "feat: add amazing feature"`
7. Push to your branch: `git push origin feature/amazing-feature`
8. Open a Pull Request

## Project Status

### Current Version: 1.0.0-alpha (First Open Source AI Agents Platform)

#### Platform Foundation

- ✓ Core API Gateway implementation
- ✓ MCP Protocol Manager service
- ✓ AI Intent Parser service
- ✓ Basic frontend UI with agents dashboard
- ✓ Docker containerization
- ✓ Integration test suite

#### AI Agents Core Features

- ✓ Temporal.io workflow integration
- ✓ OpenAI Agents SDK integration
- ✓ MCP (Model Context Protocol) implementation
- ✓ A2A (Agent-to-Agent) communication framework
- ✓ Multi-agent coordination system

#### Advanced Features (In Development)

- [ ] Advanced AI agent orchestration workflows
- [ ] Enhanced A2A communication protocols
- [ ] Performance optimization for agent interactions
- [ ] Production deployment guides
- [ ] Kubernetes manifests for agent scaling
- [ ] Advanced monitoring and observability for AI workflows

### Performance Metrics

- **API Response Time**: < 100ms (average)
- **Throughput**: > 1000 requests/second
- **Memory Usage**: < 512MB per service
- **Container Start Time**: < 30 seconds

## Security

Security is a top priority for AI-CORE. We follow industry best practices:

- Regular security audits with `cargo audit`
- Dependency vulnerability scanning
- Container security scanning
- OWASP compliance
- Encrypted communication between services

To report security vulnerabilities, please email security@netadx.ai (do not create public issues).

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Rust Community](https://www.rust-lang.org/community) for the amazing ecosystem
- [TypeScript Team](https://www.typescriptlang.org/) for the excellent language
- [Docker](https://www.docker.com/) for containerization technology
- All contributors who have helped shape this project

## Links

- **Documentation**: [https://docs.netadx.ai](https://docs.netadx.ai)
- **API Reference**: [https://docs.netadx.ai/api](https://docs.netadx.ai/api)
- **Issues**: [GitHub Issues](https://github.com/netadx1ai/ai-core/issues)
- **Discussions**: [GitHub Discussions](https://github.com/netadx1ai/ai-core/discussions)
- **Discord**: [NetADX.ai Community](https://discord.gg/netadx)

## Roadmap

### Q4 2024 - AI Agents Platform Evolution

- [ ] Advanced multi-agent workflow orchestration
- [ ] Enhanced Temporal.io integration with custom activities
- [ ] Extended OpenAI Agents SDK capabilities
- [ ] Advanced MCP protocol features
- [ ] Real-time A2A collaboration dashboard
- [ ] Mobile application support for agent monitoring

### Q1 2025 - Enterprise AI Platform

- [ ] Machine learning model integration beyond OpenAI
- [ ] Advanced AI agents analytics dashboard
- [ ] Multi-tenant agent isolation architecture
- [ ] Enterprise AI governance features
- [ ] Agent marketplace and plugin ecosystem
- [ ] Advanced security for AI agent communications

---

<div align="center">
  <strong>Built with passion by the NetADX.ai team</strong>
  <br>
  <a href="https://github.com/netadx1ai/ai-core/stargazers">Star us on GitHub</a>
</div>
