# AI-PLATFORM Source Code

This directory contains the main source code for the AI-PLATFORM Intelligent Automation Platform, organized by functional areas with clear agent ownership.

## 📁 Structure

- **api-gateway/** - Main API gateway implementation (backend-agent)
- **services/** - Core microservices (backend-agent, security-agent)
- **database/** - Database integration layers (database-agent)
- **security/** - Security modules and middleware (security-agent)
- **ui/** - Frontend React application (frontend-agent)
- **integrations/** - Third-party integration modules (integration-agent)
- **shared/** - Common utilities and types (all agents)

## 🤝 Collaboration Rules

- Each agent has primary ownership of their designated folders
- Shared utilities go in `shared/` with proper coordination
- Cross-folder changes require file locking and coordination
- All agents contribute tests for their components

See `/dev-works/dev-agentsshared-workspace/WORKSPACE_STRUCTURE.md` for detailed coordination protocols.
