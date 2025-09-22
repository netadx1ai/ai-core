# Contributing to AI-CORE

Thank you for your interest in contributing to AI-CORE! This document provides guidelines and information for contributors.

## 🌟 Welcome

AI-CORE is an open-source project that thrives on community contributions. Whether you're fixing bugs, adding features, improving documentation, or helping with testing, your contributions are valuable and appreciated.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Contributing Workflow](#contributing-workflow)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)
- [Community](#community)

## 🤝 Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:

### Our Pledge

- Be respectful and inclusive to all contributors
- Foster a welcoming and harassment-free environment
- Focus on constructive feedback and collaboration
- Respect differing viewpoints and experiences

### Unacceptable Behavior

- Harassment, discrimination, or offensive language
- Personal attacks or trolling
- Public or private harassment
- Publishing others' private information without permission

## 🚀 Getting Started

### Prerequisites

Before contributing, ensure you have:

- **Rust 1.70+** - [Install Rust](https://rustup.rs/)
- **Node.js 18+** - [Install Node.js](https://nodejs.org/)
- **Docker 20+** - [Install Docker](https://docs.docker.com/get-docker/)
- **Git** - [Install Git](https://git-scm.com/downloads)

### Areas for Contribution

We welcome contributions in these areas:

- **🐛 Bug Fixes** - Fix reported issues
- **✨ New Features** - Add new functionality
- **📚 Documentation** - Improve guides and API docs
- **🧪 Testing** - Add or improve test coverage
- **🔧 Tooling** - Enhance development tools
- **🎨 UI/UX** - Improve frontend experience
- **⚡ Performance** - Optimize code performance
- **🛡️ Security** - Enhance security measures

## 🛠️ Development Setup

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/ai-core.git
cd ai-core

# Add upstream remote
git remote add upstream https://github.com/netadx1ai/ai-core.git
```

### 2. Environment Setup

```bash
# Install Rust dependencies
cargo build --release

# Install frontend dependencies
cd src/ui && npm install && cd ../..

# Set up development environment
./tools/deployment/setup-dev-environment.sh

# Verify installation
cargo test --workspace
```

### 3. Development Tools

```bash
# Install additional development tools
cargo install cargo-watch cargo-audit cargo-tarpaulin
rustup component add clippy rustfmt

# For frontend development
cd src/ui && npm install -g typescript @types/node
```

## 🔄 Contributing Workflow

### 1. Create a Feature Branch

```bash
# Update your main branch
git checkout main
git pull upstream main

# Create a feature branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

### 2. Make Changes

- Write clean, well-documented code
- Follow our coding standards
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all tests
cargo test --workspace

# Run frontend tests
cd src/ui && npm test

# Run integration tests
./tools/testing/run-integration-tests.sh

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check
```

### 4. Commit Changes

```bash
# Stage your changes
git add .

# Commit with descriptive message
git commit -m "feat: add new feature description"
```

#### Commit Message Format

Use conventional commits format:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**

```
feat(api): add user authentication endpoint
fix(parser): resolve intent parsing edge case
docs(readme): update installation instructions
test(integration): add API gateway test suite
```

### 5. Push and Create PR

```bash
# Push to your fork
git push origin feature/your-feature-name

# Create a Pull Request on GitHub
```

## 📝 Coding Standards

### Rust Code Style

```rust
// Use descriptive names
fn parse_user_intent(input: &str) -> Result<Intent, ParseError> {
    // Implementation
}

// Document public APIs
/// Parses user input into structured intent
///
/// # Arguments
/// * `input` - Raw user input string
///
/// # Returns
/// * `Ok(Intent)` - Successfully parsed intent
/// * `Err(ParseError)` - Parsing failed
pub fn parse_intent(input: &str) -> Result<Intent, ParseError> {
    // Implementation
}

// Use Result for error handling
fn risky_operation() -> Result<Success, Error> {
    // Never use unwrap() in production code
    // Prefer ? operator for error propagation
}
```

### TypeScript Code Style

```typescript
// Use descriptive interfaces
interface UserPreferences {
    theme: "light" | "dark";
    language: string;
    notifications: boolean;
}

// Use async/await for promises
async function fetchUserData(userId: string): Promise<User> {
    try {
        const response = await api.get(`/users/${userId}`);
        return response.data;
    } catch (error) {
        throw new Error(`Failed to fetch user: ${error.message}`);
    }
}

// Use proper error handling
function processData(data: unknown): ProcessedData {
    if (!isValidData(data)) {
        throw new Error("Invalid data format");
    }
    // Process data
}
```

### General Guidelines

- **Naming**: Use clear, descriptive names
- **Functions**: Keep functions small and focused
- **Comments**: Explain why, not what
- **Error Handling**: Always handle errors gracefully
- **Security**: Never expose sensitive information
- **Performance**: Consider performance implications

## 🧪 Testing Guidelines

### Test Structure

```
tests/
├── unit/           # Unit tests
├── integration/    # Integration tests
└── e2e/           # End-to-end tests
```

### Writing Tests

#### Rust Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_intent() {
        let input = "create user john";
        let result = parse_intent(input).unwrap();

        assert_eq!(result.action, Action::Create);
        assert_eq!(result.entity, Entity::User);
        assert_eq!(result.target, "john");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_operation().await;
        assert!(result.is_ok());
    }
}
```

#### TypeScript Tests

```typescript
import { describe, it, expect } from "@jest/globals";
import { parseUserInput } from "../src/parser";

describe("User Input Parser", () => {
    it("should parse simple commands", () => {
        const input = "create user john";
        const result = parseUserInput(input);

        expect(result.action).toBe("create");
        expect(result.entity).toBe("user");
        expect(result.target).toBe("john");
    });
});
```

### Test Requirements

- **Coverage**: Aim for >80% test coverage
- **Unit Tests**: Test individual functions/methods
- **Integration Tests**: Test component interactions
- **E2E Tests**: Test complete user workflows
- **Edge Cases**: Test error conditions and edge cases

## 📚 Documentation

### Documentation Types

1. **Code Documentation**: Inline comments and docstrings
2. **API Documentation**: REST API endpoints and schemas
3. **User Guides**: How-to guides and tutorials
4. **Architecture Documentation**: System design and decisions

### Writing Guidelines

- **Clear and Concise**: Use simple, direct language
- **Examples**: Provide practical examples
- **Up-to-date**: Keep documentation current with code
- **Accessible**: Consider different skill levels

### Documentation Format

````markdown
# Component Name

Brief description of what the component does.

## Usage

```rust
// Example usage
let component = Component::new();
let result = component.process(input)?;
```
````

## API Reference

### Methods

#### `process(input: &str) -> Result<Output, Error>`

Processes the given input and returns the result.

**Parameters:**

- `input`: The input string to process

**Returns:**

- `Ok(Output)`: Successfully processed output
- `Err(Error)`: Processing error

**Example:**

```rust
let output = component.process("example input")?;
```

````

## 🔄 Pull Request Process

### Before Submitting

- [ ] Code follows style guidelines
- [ ] Tests pass locally
- [ ] Documentation is updated
- [ ] Commit messages follow convention
- [ ] Branch is up-to-date with main

### PR Template

When creating a PR, include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Other (please describe)

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
````

### Review Process

1. **Automated Checks**: CI/CD pipeline runs tests
2. **Code Review**: Maintainers review code
3. **Feedback**: Address review comments
4. **Approval**: Two maintainer approvals required
5. **Merge**: Squash and merge to main

## 🐛 Issue Reporting

### Bug Reports

Use the bug report template:

```markdown
**Bug Description**
Clear description of the bug

**Steps to Reproduce**

1. Step one
2. Step two
3. See error

**Expected Behavior**
What should happen

**Actual Behavior**
What actually happens

**Environment**

- OS: [e.g., macOS 13.0]
- Rust version: [e.g., 1.70.0]
- AI-CORE version: [e.g., 1.0.0]

**Additional Context**
Any other relevant information
```

### Feature Requests

```markdown
**Feature Description**
Clear description of the requested feature

**Use Case**
Why is this feature needed?

**Proposed Solution**
How should this feature work?

**Alternatives Considered**
Other solutions you've considered

**Additional Context**
Any other relevant information
```

## 🏷️ Labels and Categories

### Issue Labels

- `bug`: Something isn't working
- `enhancement`: New feature or request
- `documentation`: Improvements to docs
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention needed
- `priority/high`: High priority issue
- `status/blocked`: Progress is blocked

### PR Labels

- `ready for review`: PR is ready
- `work in progress`: PR is not ready
- `needs tests`: PR needs test coverage
- `needs documentation`: PR needs docs

## 🎯 Development Focus Areas

### Current Priorities

1. **Performance Optimization**: Improve response times
2. **Test Coverage**: Increase test coverage to >90%
3. **Documentation**: Complete API documentation
4. **Security**: Enhance security measures
5. **DevOps**: Improve CI/CD pipeline

### Future Roadmap

- Advanced AI agent features
- Real-time collaboration
- Mobile application support
- Machine learning integration

## 💬 Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General questions and ideas
- **Discord**: Real-time community chat
- **Email**: security@netadx.ai (security issues only)

### Getting Help

1. **Documentation**: Check existing docs first
2. **Search Issues**: Look for similar problems
3. **Ask Questions**: Use GitHub Discussions
4. **Community Chat**: Join our Discord server

### Recognition

Contributors are recognized through:

- **Contributors list**: Listed in README
- **Release notes**: Mentioned in changelog
- **Hall of Fame**: Special recognition page
- **Swag**: Stickers and merchandise for significant contributions

## 📜 License

By contributing to AI-CORE, you agree that your contributions will be licensed under the MIT License.

---

**Thank you for contributing to AI-CORE by NetADX.ai!** 🎉

Your contributions help make AI-CORE better for everyone. If you have questions about contributing, don't hesitate to reach out to the community.
