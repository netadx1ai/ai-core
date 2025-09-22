/**
 * AI-PLATFORM Test Data Schema for MongoDB
 * FAANG-Enhanced Testing Infrastructure - Database Agent Implementation T4.1
 *
 * Flexible document schema for:
 * - Unstructured test configurations and metadata
 * - Dynamic test data generation templates
 * - Test artifacts and media storage
 * - AI model training data and results
 * - Complex test scenarios and workflows
 * - Cross-platform compatibility matrices
 * - Performance baselines and benchmarks
 */

// ============================================================================
// Database and Collection Setup
// ============================================================================

// Connect to test database
use aicore_test;

// Enable sharding for large collections (if using MongoDB sharding)
// sh.enableSharding("aicore_test");

// ============================================================================
// Test Configuration Documents
// ============================================================================

// Flexible test configurations with dynamic schemas
db.createCollection("test_configurations", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["configId", "name", "type", "createdAt"],
      properties: {
        configId: {
          bsonType: "string",
          description: "Unique configuration identifier"
        },
        name: {
          bsonType: "string",
          description: "Configuration name"
        },
        type: {
          bsonType: "string",
          enum: ["browser", "environment", "test-suite", "user-profile", "security", "performance"],
          description: "Configuration type"
        },
        version: {
          bsonType: "string",
          description: "Configuration version"
        },
        isActive: {
          bsonType: "bool",
          description: "Whether configuration is active"
        },
        environment: {
          bsonType: "string",
          description: "Target environment"
        },
        configuration: {
          bsonType: "object",
          description: "Flexible configuration data"
        },
        metadata: {
          bsonType: "object",
          description: "Additional metadata"
        },
        tags: {
          bsonType: "array",
          items: {
            bsonType: "string"
          },
          description: "Configuration tags"
        },
        createdBy: {
          bsonType: "string",
          description: "Creator identifier"
        },
        createdAt: {
          bsonType: "date",
          description: "Creation timestamp"
        },
        updatedAt: {
          bsonType: "date",
          description: "Last update timestamp"
        }
      }
    }
  }
});

// Create indexes for test configurations
db.test_configurations.createIndex({ "configId": 1 }, { unique: true });
db.test_configurations.createIndex({ "name": 1, "type": 1 });
db.test_configurations.createIndex({ "type": 1, "environment": 1 });
db.test_configurations.createIndex({ "isActive": 1 });
db.test_configurations.createIndex({ "tags": 1 });
db.test_configurations.createIndex({ "createdAt": -1 });

// Sample test configurations
db.test_configurations.insertMany([
  {
    configId: "browser-chrome-desktop",
    name: "Chrome Desktop Configuration",
    type: "browser",
    version: "1.0.0",
    isActive: true,
    environment: "testing",
    configuration: {
      browserName: "chromium",
      browserVersion: "latest",
      platform: "desktop",
      viewport: { width: 1920, height: 1080 },
      userAgent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
      capabilities: {
        javascript: true,
        cookies: true,
        localStorage: true,
        webWorkers: true,
        serviceWorkers: true
      },
      flags: [
        "--no-sandbox",
        "--disable-web-security",
        "--disable-features=TranslateUI"
      ],
      performance: {
        slowMo: 0,
        timeout: 30000,
        waitForTimeout: 5000
      }
    },
    metadata: {
      supportedFeatures: ["webgl", "canvas", "audio", "video"],
      performanceProfile: "high",
      stabilityRating: 9.5
    },
    tags: ["browser", "chromium", "desktop", "stable"],
    createdBy: "system",
    createdAt: new Date(),
    updatedAt: new Date()
  },
  {
    configId: "security-policy-strict",
    name: "Strict Security Policy",
    type: "security",
    version: "2.1.0",
    isActive: true,
    environment: "production",
    configuration: {
      authentication: {
        mfaRequired: ["admin", "manager"],
        sessionTimeout: 3600,
        maxLoginAttempts: 3,
        lockoutDuration: 1800,
        passwordPolicy: {
          minLength: 14,
          requireUppercase: true,
          requireLowercase: true,
          requireNumbers: true,
          requireSpecialChars: true,
          preventCommonPasswords: true,
          maxAge: 90
        }
      },
      authorization: {
        enforceRBAC: true,
        auditLogRetention: 365,
        privilegeEscalationPrevention: true
      },
      network: {
        enforceHTTPS: true,
        allowedOrigins: ["https://aicore.dev", "https://staging.aicore.dev"],
        csrfProtection: true,
        rateLimiting: {
          enabled: true,
          requests: 100,
          window: 3600
        }
      }
    },
    metadata: {
      complianceStandards: ["SOC2", "GDPR", "HIPAA"],
      lastSecurityReview: new Date("2025-01-01"),
      approvedBy: "security-team"
    },
    tags: ["security", "production", "compliance", "strict"],
    createdBy: "security-agent",
    createdAt: new Date(),
    updatedAt: new Date()
  }
]);

// ============================================================================
// Test Data Templates and Generation
// ============================================================================

// Dynamic test data generation templates
db.createCollection("test_data_templates", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["templateId", "name", "dataType", "template"],
      properties: {
        templateId: {
          bsonType: "string",
          description: "Unique template identifier"
        },
        name: {
          bsonType: "string",
          description: "Template name"
        },
        dataType: {
          bsonType: "string",
          enum: ["user", "organization", "workflow", "transaction", "product", "custom"],
          description: "Type of data this template generates"
        },
        template: {
          bsonType: "object",
          description: "Template definition with generation rules"
        },
        aiEnhanced: {
          bsonType: "bool",
          description: "Whether template uses AI for generation"
        },
        aiModel: {
          bsonType: "string",
          description: "AI model used for generation"
        },
        generationRules: {
          bsonType: "object",
          description: "Rules for data generation"
        },
        validationSchema: {
          bsonType: "object",
          description: "Schema for validating generated data"
        }
      }
    }
  }
});

// Indexes for test data templates
db.test_data_templates.createIndex({ "templateId": 1 }, { unique: true });
db.test_data_templates.createIndex({ "dataType": 1 });
db.test_data_templates.createIndex({ "aiEnhanced": 1 });

// Sample test data templates
db.test_data_templates.insertMany([
  {
    templateId: "user-profile-comprehensive",
    name: "Comprehensive User Profile Template",
    dataType: "user",
    template: {
      personalInfo: {
        firstName: { type: "faker", method: "name.firstName" },
        lastName: { type: "faker", method: "name.lastName" },
        email: { type: "faker", method: "internet.email" },
        phone: { type: "faker", method: "phone.phoneNumber" },
        dateOfBirth: {
          type: "faker",
          method: "date.between",
          args: ["1950-01-01", "2005-12-31"]
        }
      },
      address: {
        street: { type: "faker", method: "address.streetAddress" },
        city: { type: "faker", method: "address.city" },
        state: { type: "faker", method: "address.state" },
        zipCode: { type: "faker", method: "address.zipCode" },
        country: { type: "faker", method: "address.country" }
      },
      preferences: {
        language: { type: "random", values: ["en", "es", "fr", "de", "it"] },
        timezone: { type: "faker", method: "address.timeZone" },
        theme: { type: "random", values: ["light", "dark", "auto"] },
        notifications: {
          email: { type: "boolean", probability: 0.8 },
          sms: { type: "boolean", probability: 0.3 },
          push: { type: "boolean", probability: 0.9 }
        }
      },
      account: {
        username: { type: "faker", method: "internet.userName" },
        role: {
          type: "weighted",
          values: [
            { value: "user", weight: 60 },
            { value: "manager", weight: 20 },
            { value: "admin", weight: 5 },
            { value: "developer", weight: 10 },
            { value: "tester", weight: 5 }
          ]
        },
        subscriptionTier: {
          type: "weighted",
          values: [
            { value: "free", weight: 70 },
            { value: "pro", weight: 25 },
            { value: "enterprise", weight: 5 }
          ]
        },
        isActive: { type: "boolean", probability: 0.95 }
      }
    },
    aiEnhanced: false,
    generationRules: {
      constraints: {
        emailDomainWhitelist: ["example.com", "test.aicore.dev", "qa.local"],
        phoneNumberFormat: "US",
        ageRange: { min: 18, max: 75 }
      },
      relationships: {
        usernameFromEmail: true,
        rolePermissionMapping: true
      },
      localization: {
        addressCountryConsistency: true,
        phoneNumberCountryMatch: true
      }
    },
    validationSchema: {
      personalInfo: {
        email: { pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$" },
        phone: { pattern: "^[+]?[1-9]?[0-9]{7,15}$" }
      },
      account: {
        username: { minLength: 3, maxLength: 30, pattern: "^[a-zA-Z0-9_-]+$" }
      }
    },
    tags: ["user", "comprehensive", "testing"],
    createdBy: "database-agent",
    createdAt: new Date(),
    updatedAt: new Date()
  },
  {
    templateId: "ai-workflow-scenario",
    name: "AI-Generated Workflow Scenario Template",
    dataType: "workflow",
    template: {
      scenario: {
        name: { type: "ai", prompt: "Generate a realistic business workflow name" },
        description: {
          type: "ai",
          prompt: "Create a detailed description for a business workflow involving user interactions, data processing, and decision points"
        },
        complexity: { type: "random", values: ["simple", "moderate", "complex", "advanced"] },
        estimatedDuration: { type: "range", min: 30, max: 1800 }, // seconds
        category: {
          type: "random",
          values: ["authentication", "data-entry", "approval", "reporting", "integration"]
        }
      },
      steps: {
        type: "ai",
        prompt: "Generate 3-10 detailed workflow steps with specific user actions, expected outcomes, and validation points"
      },
      testData: {
        type: "ai",
        prompt: "Create realistic test data that would be used in this workflow"
      },
      expectedOutcomes: {
        type: "ai",
        prompt: "Define the expected successful outcomes and potential failure scenarios"
      },
      validationPoints: {
        type: "ai",
        prompt: "Identify key validation points where the workflow should be verified"
      }
    },
    aiEnhanced: true,
    aiModel: "gemini-pro",
    generationRules: {
      aiParameters: {
        temperature: 0.7,
        maxTokens: 2048,
        topP: 0.9
      },
      postProcessing: {
        validateJSON: true,
        checkRealism: true,
        ensureTestability: true
      }
    },
    validationSchema: {
      scenario: {
        name: { minLength: 5, maxLength: 100 },
        description: { minLength: 50, maxLength: 1000 }
      },
      steps: {
        minItems: 3,
        maxItems: 10
      }
    },
    tags: ["workflow", "ai-generated", "scenario", "dynamic"],
    createdBy: "ai-enhancement-agent",
    createdAt: new Date(),
    updatedAt: new Date()
  }
]);

// ============================================================================
// Test Artifacts and Media Storage
// ============================================================================

// Test execution artifacts (screenshots, videos, logs, reports)
db.createCollection("test_artifacts", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["artifactId", "testExecutionId", "artifactType", "createdAt"],
      properties: {
        artifactId: {
          bsonType: "string",
          description: "Unique artifact identifier"
        },
        testExecutionId: {
          bsonType: "string",
          description: "Associated test execution ID"
        },
        testName: {
          bsonType: "string",
          description: "Test name for easier querying"
        },
        artifactType: {
          bsonType: "string",
          enum: ["screenshot", "video", "log", "report", "trace", "coverage", "performance"],
          description: "Type of artifact"
        },
        fileName: {
          bsonType: "string",
          description: "Original file name"
        },
        filePath: {
          bsonType: "string",
          description: "Storage path or URL"
        },
        fileSize: {
          bsonType: "long",
          description: "File size in bytes"
        },
        mimeType: {
          bsonType: "string",
          description: "MIME type of the file"
        },
        metadata: {
          bsonType: "object",
          description: "Artifact-specific metadata"
        },
        thumbnailPath: {
          bsonType: "string",
          description: "Path to thumbnail (for images/videos)"
        },
        processingStatus: {
          bsonType: "string",
          enum: ["pending", "processing", "completed", "failed"],
          description: "Processing status"
        },
        expiresAt: {
          bsonType: "date",
          description: "When artifact expires and should be cleaned up"
        },
        tags: {
          bsonType: "array",
          items: {
            bsonType: "string"
          }
        }
      }
    }
  }
});

// Indexes for test artifacts
db.test_artifacts.createIndex({ "artifactId": 1 }, { unique: true });
db.test_artifacts.createIndex({ "testExecutionId": 1 });
db.test_artifacts.createIndex({ "testName": 1, "artifactType": 1 });
db.test_artifacts.createIndex({ "artifactType": 1, "createdAt": -1 });
db.test_artifacts.createIndex({ "expiresAt": 1 }, { expireAfterSeconds: 0 });
db.test_artifacts.createIndex({ "processingStatus": 1 });

// ============================================================================
// Complex Test Scenarios and Workflows
// ============================================================================

// Multi-step, complex test scenarios with branching logic
db.createCollection("test_scenarios", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["scenarioId", "name", "steps", "createdAt"],
      properties: {
        scenarioId: {
          bsonType: "string",
          description: "Unique scenario identifier"
        },
        name: {
          bsonType: "string",
          description: "Scenario name"
        },
        description: {
          bsonType: "string",
          description: "Detailed scenario description"
        },
        category: {
          bsonType: "string",
          description: "Scenario category"
        },
        complexity: {
          bsonType: "string",
          enum: ["simple", "moderate", "complex", "advanced"],
          description: "Scenario complexity level"
        },
        steps: {
          bsonType: "array",
          description: "Array of test steps with branching logic"
        },
        preconditions: {
          bsonType: "array",
          description: "Prerequisites for scenario execution"
        },
        postconditions: {
          bsonType: "array",
          description: "Expected state after scenario completion"
        },
        testData: {
          bsonType: "object",
          description: "Test data requirements and generation rules"
        },
        environmentRequirements: {
          bsonType: "object",
          description: "Environment and platform requirements"
        },
        expectedDuration: {
          bsonType: "object",
          description: "Expected execution duration ranges"
        },
        successCriteria: {
          bsonType: "array",
          description: "Criteria for successful scenario completion"
        },
        riskFactors: {
          bsonType: "array",
          description: "Known risk factors and mitigation strategies"
        },
        aiGenerated: {
          bsonType: "bool",
          description: "Whether scenario was AI-generated"
        },
        aiMetadata: {
          bsonType: "object",
          description: "AI generation metadata"
        }
      }
    }
  }
});

// Indexes for test scenarios
db.test_scenarios.createIndex({ "scenarioId": 1 }, { unique: true });
db.test_scenarios.createIndex({ "name": 1 });
db.test_scenarios.createIndex({ "category": 1, "complexity": 1 });
db.test_scenarios.createIndex({ "aiGenerated": 1 });

// Sample complex test scenario
db.test_scenarios.insertOne({
  scenarioId: "auth-workflow-comprehensive",
  name: "Comprehensive Authentication Workflow",
  description: "End-to-end authentication scenario covering login, MFA, role-based access, session management, and logout with error handling and recovery paths",
  category: "authentication",
  complexity: "complex",
  steps: [
    {
      stepId: "auth-001",
      name: "Navigate to Login Page",
      type: "navigation",
      action: "navigate",
      target: "/auth/login",
      expectedOutcome: "Login form is displayed",
      timeout: 10000,
      retryAttempts: 3,
      validations: [
        { type: "element", selector: "[data-testid='login-form']", visible: true },
        { type: "url", pattern: ".*/auth/login.*" },
        { type: "title", contains: "Login" }
      ],
      onFailure: "terminate",
      metadata: {
        screenshot: true,
        performanceMetrics: true
      }
    },
    {
      stepId: "auth-002",
      name: "Enter Valid Credentials",
      type: "interaction",
      action: "fillForm",
      target: "[data-testid='login-form']",
      data: {
        username: "{{testUser.username}}",
        password: "{{testUser.password}}"
      },
      validations: [
        { type: "element", selector: "[data-testid='username-input']", hasValue: true },
        { type: "element", selector: "[data-testid='password-input']", hasValue: true }
      ],
      onFailure: "retry"
    },
    {
      stepId: "auth-003",
      name: "Submit Login Form",
      type: "interaction",
      action: "click",
      target: "[data-testid='login-button']",
      expectedOutcome: "Login request submitted",
      waitFor: {
        type: "networkResponse",
        url: "/api/auth/login",
        timeout: 15000
      },
      branches: [
        {
          condition: "response.status === 200 && !response.body.mfaRequired",
          nextStep: "auth-010" // Skip MFA, go to dashboard
        },
        {
          condition: "response.status === 200 && response.body.mfaRequired",
          nextStep: "auth-004" // Continue to MFA
        },
        {
          condition: "response.status >= 400",
          nextStep: "auth-error-001" // Handle login error
        }
      ]
    },
    {
      stepId: "auth-004",
      name: "Handle MFA Challenge",
      type: "conditional",
      condition: "mfaRequired === true",
      action: "waitForElement",
      target: "[data-testid='mfa-form']",
      timeout: 10000,
      validations: [
        { type: "element", selector: "[data-testid='mfa-code-input']", visible: true },
        { type: "element", selector: "[data-testid='mfa-submit-button']", enabled: true }
      ],
      nextStep: "auth-005"
    },
    {
      stepId: "auth-005",
      name: "Enter MFA Code",
      type: "interaction",
      action: "fillInput",
      target: "[data-testid='mfa-code-input']",
      data: "{{mfaCode}}", // Generated or mocked MFA code
      validations: [
        { type: "element", selector: "[data-testid='mfa-code-input']", hasValue: true }
      ]
    },
    {
      stepId: "auth-006",
      name: "Submit MFA Code",
      type: "interaction",
      action: "click",
      target: "[data-testid='mfa-submit-button']",
      waitFor: {
        type: "networkResponse",
        url: "/api/auth/mfa/verify",
        timeout: 10000
      },
      branches: [
        {
          condition: "response.status === 200",
          nextStep: "auth-010" // Continue to dashboard
        },
        {
          condition: "response.status === 400",
          nextStep: "auth-007" // Invalid MFA code
        }
      ]
    },
    {
      stepId: "auth-007",
      name: "Handle MFA Error",
      type: "errorHandling",
      action: "validateError",
      expectedElements: [
        { selector: "[data-testid='error-message']", visible: true },
        { selector: "[data-testid='mfa-resend-button']", enabled: true }
      ],
      recoveryAction: "auth-008"
    },
    {
      stepId: "auth-008",
      name: "Resend MFA Code",
      type: "interaction",
      action: "click",
      target: "[data-testid='mfa-resend-button']",
      waitFor: {
        type: "networkResponse",
        url: "/api/auth/mfa/resend"
      },
      nextStep: "auth-005" // Return to MFA code entry
    },
    {
      stepId: "auth-010",
      name: "Verify Successful Login",
      type: "validation",
      action: "validateState",
      validations: [
        { type: "url", pattern: ".*/dashboard.*" },
        { type: "element", selector: "[data-testid='user-menu']", visible: true },
        { type: "element", selector: "[data-testid='logout-button']", visible: true },
        { type: "localStorage", key: "authToken", exists: true }
      ],
      nextStep: "auth-011"
    },
    {
      stepId: "auth-011",
      name: "Validate Role-Based Access",
      type: "conditional",
      branches: [
        {
          condition: "{{testUser.role}} === 'admin'",
          action: "validateElement",
          target: "[data-testid='admin-panel']",
          expected: { visible: true }
        },
        {
          condition: "{{testUser.role}} === 'manager'",
          action: "validateElement",
          target: "[data-testid='management-dashboard']",
          expected: { visible: true }
        },
        {
          condition: "{{testUser.role}} === 'user'",
          action: "validateElement",
          target: "[data-testid='user-workspace']",
          expected: { visible: true }
        }
      ],
      nextStep: "auth-012"
    },
    {
      stepId: "auth-012",
      name: "Perform Logout",
      type: "interaction",
      action: "click",
      target: "[data-testid='user-menu']",
      waitFor: {
        type: "element",
        selector: "[data-testid='logout-button']",
        state: "visible"
      },
      nextStep: "auth-013"
    },
    {
      stepId: "auth-013",
      name: "Complete Logout",
      type: "interaction",
      action: "click",
      target: "[data-testid='logout-button']",
      waitFor: {
        type: "networkResponse",
        url: "/api/auth/logout"
      },
      expectedOutcome: "User logged out successfully",
      finalValidations: [
        { type: "url", pattern: ".*/auth/login.*" },
        { type: "localStorage", key: "authToken", exists: false },
        { type: "element", selector: "[data-testid='login-form']", visible: true }
      ]
    }
  ],
  preconditions: [
    "Test user account exists and is active",
    "Application is running and accessible",
    "Database is available and populated with test data"
  ],
  postconditions: [
    "User is logged out",
    "Session is terminated",
    "Authentication state is cleared"
  ],
  testData: {
    userTemplate: "user-profile-comprehensive",
    generateMFA: true,
    userRoles: ["admin", "manager", "user"],
    environmentVariables: {
      "BASE_URL": "{{environment.baseUrl}}",
      "API_URL": "{{environment.apiUrl}}"
    }
  },
  environmentRequirements: {
    browsers: ["chromium", "firefox", "webkit"],
    platforms: ["desktop", "mobile"],
    environments: ["testing", "staging"],
    prerequisites: {
      services: ["auth-service", "mfa-service", "user-service"],
      databases: ["postgresql", "redis"]
    }
  },
  expectedDuration: {
    minimum: 30000, // 30 seconds
    typical: 45000, // 45 seconds
    maximum: 120000, // 2 minutes
    timeout: 300000 // 5 minutes
  },
  successCriteria: [
    "All authentication steps complete successfully",
    "Role-based access control is validated",
    "MFA flow works correctly (if required)",
    "Session management functions properly",
    "Logout completes successfully"
  ],
  riskFactors: [
    {
      risk: "MFA code timing out",
      probability: "low",
      mitigation: "Implement code resend functionality test"
    },
    {
      risk: "Network latency affecting timeouts",
      probability: "medium",
      mitigation: "Adjust timeouts based on environment"
    },
    {
      risk: "Browser-specific authentication behavior",
      probability: "medium",
      mitigation: "Test across multiple browsers"
    }
  ],
  aiGenerated: false,
  tags: ["authentication", "complex", "e2e", "mfa", "rbac"],
  createdBy: "qa-agent",
  createdAt: new Date(),
  updatedAt: new Date()
});

// ============================================================================
// Cross-Platform Compatibility Matrix
// ============================================================================

// Browser and platform compatibility testing matrix
db.createCollection("compatibility_matrix", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["matrixId", "name", "combinations"],
      properties: {
        matrixId: {
          bsonType: "string",
          description: "Unique matrix identifier"
        },
        name: {
          bsonType: "string",
          description: "Matrix name"
        },
        description: {
          bsonType: "string",
          description: "Matrix description"
        },
        combinations: {
          bsonType: "array",
          description: "Array of platform/browser combinations"
        },
        priority: {
          bsonType: "string",
          enum: ["P0", "P1", "P2", "P3"],
          description: "Testing priority"
        },
        coverage: {
          bsonType: "object",
          description: "Coverage requirements and targets"
        },
        excludedCombinations: {
          bsonType: "array",
          description: "Combinations to exclude from testing"
        },
        testingStrategy: {
          bsonType: "object",
          description: "Strategy for cross-platform testing"
        }
      }
    }
  }
});

// Indexes for compatibility matrix
db.compatibility_matrix.createIndex({ "matrixId": 1 }, { unique: true });
db.compatibility_matrix.createIndex({ "priority": 1 });

// Sample compatibility matrix
db.compatibility_matrix.insertOne({
  matrixId: "auth-compatibility-full",
  name: "Full Authentication Compatibility Matrix",
  description: "Comprehensive browser and platform compatibility matrix for authentication flows",
  combinations: [
    {
      id: "chrome-desktop-windows",
      browser: {
        name: "chromium",
        version: "latest",
        channel: "stable"
      },
      platform: {
        os: "windows",
        version: "10",
        architecture: "x64"
      },
      viewport: { width: 1920, height: 1080 },
      deviceType: "desktop",
      priority: "P0",
      expectedSupport: "full",
      knownIssues: []
    },
    {
      id: "firefox-desktop-macos",
      browser: {
        name: "firefox",
        version: "latest",
        channel: "stable"
      },
      platform: {
        os: "macos",
        version: "12",
        architecture: "arm64"
      },
      viewport: { width: 1680, height: 1050 },
      deviceType: "desktop",
      priority: "P0",
      expectedSupport: "full",
      knownIssues: []
    },
    {
      id: "safari-mobile-ios",
      browser: {
        name: "webkit",
        version: "latest",
        channel: "stable"
      },
      platform: {
        os: "ios",
        version: "15",
        device: "iPhone 13"
      },
      viewport: { width: 390, height: 844 },
      deviceType: "mobile",
      priority: "P1",
      expectedSupport: "partial",
      knownIssues: [
        "Touch ID integration limited",
        "Autofill behavior differences"
      ]
    },
    {
      id: "chrome-tablet-android",
      browser: {
        name: "chromium",
        version: "latest",
        channel: "stable"
      },
      platform: {
        os: "android",
        version: "11",
        device: "Galaxy Tab S7"
      },
      viewport: { width: 800, height: 1280 },
      deviceType: "tablet",
      priority: "P2",
      expectedSupport: "full",
      knownIssues: []
    }
  ],
  priority: "P0",
  coverage: {
    minimumBrowsers: ["chromium", "firefox", "webkit"],
    minimumPlatforms: ["windows", "macos", "ios", "android"],
    targetCoverage: 95,
    criticalFlows: ["login", "logout", "mfa", "password-reset"]
  },
  excludedCombinations: [
    {
      reason: "Browser no longer supported",
      criteria: {
        browser: "internet-explorer",
        version: "*"
      }
    },
    {
      reason: "OS version end of life",
      criteria: {
        platform: { os: "windows", version: "7" }
      }
    }
  ],
  testingStrategy: {
    parallelExecution: true,
    maxConcurrency: 5,
    retryFailedCombinations: true,
    screenshotComparison: true,
    performanceBenchmarking: true,
    accessibilityTesting: true
  },
  tags: ["compatibility", "cross-platform", "authentication"],
  createdBy: "qa-agent",
  createdAt: new Date(),
  updatedAt: new Date()
});

// ============================================================================
// Performance Baselines and Benchmarks
// ============================================================================

// Performance baseline storage for comparison
db.createCollection("performance_baselines", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["baselineId", "name", "metrics", "createdAt"],
      properties: {
        baselineId: {
          bsonType: "string",
          description: "Unique baseline identifier"
        },
        name: {
          bsonType: "string",
          description: "Baseline name"
        },
        version: {
          bsonType: "string",
          description: "Application version"
        },
        environment: {
          bsonType: "string",
          description: "Environment where baseline was captured"
        },
        testSuite: {
          bsonType: "string",
          description: "Test suite name"
        },
        metrics: {
          bsonType: "object",
          description: "Performance metrics and thresholds"
        },
        platformConfig: {
          bsonType: "object",
          description: "Platform and browser configuration"
        },
        isActive: {
          bsonType: "bool",
          description: "Whether this baseline is currently active"
        },
        validUntil: {
          bsonType: "date",
          description: "When baseline expires"
        }
      }
    }
  }
});

// Indexes for performance baselines
db.performance_baselines.createIndex({ "baselineId": 1 }, { unique: true });
db.performance_baselines.createIndex({ "environment": 1, "testSuite": 1 });
db.performance_baselines.createIndex({ "isActive": 1 });
db.performance_baselines.createIndex({ "validUntil": 1 });

// Sample performance baseline
db.performance_baselines.insertOne({
  baselineId: "auth-perf-baseline-v1.0",
  name: "Authentication Performance Baseline v1.0",
  version: "1.0.0",
  environment: "staging",
  testSuite: "authentication-suite",
  metrics: {
    loginFlow: {
      averageDuration: 2500, // milliseconds
      p95Duration: 4000,
      p99Duration: 6000,
      successRate: 99.5, // percentage
      thresholds: {
        maxAverageDuration: 3000,
        maxP95Duration: 5000,
        minSuccessRate: 98.0
      }
    },
    mfaFlow: {
      averageDuration: 1800,
      p95Duration: 3200,
      p99Duration: 5000,
      successRate: 98.8,
      thresholds: {
        maxAverageDuration: 2500,
        maxP95Duration: 4000,
        minSuccessRate: 97.0
      }
    },
    pageLoad: {
      loginPage: {
        averageLoadTime: 1200,
        p95LoadTime: 2000,
        p99LoadTime: 3500,
        firstContentfulPaint: 800,
        largestContentfulPaint: 1500,
        cumulativeLayoutShift: 0.05,
        thresholds: {
          maxLoadTime: 1500,
          maxP95LoadTime: 2500,
          maxFirstContentfulPaint: 1000,
          maxLargestContentfulPaint: 2000,
          maxCumulativeLayoutShift: 0.1
        }
      },
      dashboard: {
        averageLoadTime: 1800,
        p95LoadTime: 3000,
        p99LoadTime: 4500,
        firstContentfulPaint: 1200,
        largestContentfulPaint: 2200,
        cumulativeLayoutShift: 0.08,
        thresholds: {
          maxLoadTime: 2500,
          maxP95LoadTime: 4000,
          maxFirstContentfulPaint: 1500,
          maxLargestContentfulPaint: 3000,
          maxCumulativeLayoutShift: 0.15
        }
      }
    },
    apiPerformance: {
      authLogin: {
        averageResponseTime: 150,
        p95ResponseTime: 300,
        p99ResponseTime: 500,
        successRate: 99.9,
        thresholds: {
          maxAverageResponseTime: 200,
          maxP95ResponseTime: 400,
          minSuccessRate: 99.5
        }
      },
      mfaVerify: {
        averageResponseTime: 100,
        p95ResponseTime: 200,
        p99ResponseTime: 350,
        successRate: 99.7,
        thresholds: {
          maxAverageResponseTime: 150,
          maxP95ResponseTime: 250,
          minSuccessRate: 99.0
        }
      }
    }
  },
  platformConfig: {
    browser: {
      name: "chromium",
      version: "110.0.0.0"
    },
    platform: {
      os: "linux",
      version: "ubuntu-20.04",
      cpu: "4-core",
      memory: "8GB"
    },
    network: {
      type: "broadband",
      latency: 20, // milliseconds
      bandwidth: "100Mbps"
    }
  },
  isActive: true,
  validUntil: new Date(Date.now() + 90 * 24 * 60 * 60 * 1000), // 90 days from now
  tags: ["performance", "baseline", "authentication"],
  createdBy: "performance-agent",
  createdAt: new Date(),
  updatedAt: new Date()
});

// ============================================================================
// Utility Functions and Aggregation Pipelines
// ============================================================================

// Create a function to clean up expired artifacts
function cleanupExpiredArtifacts() {
  const result = db.test_artifacts.deleteMany({
    expiresAt: { $lt: new Date() }
  });

  print(`Cleaned up ${result.deletedCount} expired artifacts`);
  return result.deletedCount;
}

// Create a function to get active test configurations by type
function getActiveConfigsByType(configType) {
  return db.test_configurations.find({
    type: configType,
    isActive: true
  }).toArray();
}

// Aggregation pipeline to get test execution summary by browser
function getTestExecutionSummaryByBrowser(days = 7) {
  const startDate = new Date(Date.now() - days * 24 * 60 * 60 * 1000);

  return db.test_artifacts.aggregate([
    {
      $match: {
        createdAt: { $gte: startDate },
        artifactType: "report"
      }
    },
    {
      $lookup: {
        from: "test_scenarios",
        localField: "testName",
        foreignField: "name",
        as: "scenario"
      }
    },
    {
      $group: {
        _id: "$metadata.browser",
        totalTests: { $sum: 1 },
        avgDuration: { $avg: "$metadata.duration" },
        successCount: {
          $sum: { $cond: [{ $eq: ["$metadata.status", "passed"] }, 1, 0] }
        }
      }
    },
    {
      $project: {
        browser: "$_id",
        totalTests: 1,
        avgDuration: { $round: ["$avgDuration", 2] },
        successRate: {
          $round: [
            { $multiply: [{ $divide: ["$successCount", "$totalTests"] }, 100] },
            2
          ]
        }
      }
    },
    { $sort: { successRate: -1, avgDuration: 1 } }
  ]).toArray();
}

// ============================================================================
// Initial Data Setup and Indexes
// ============================================================================

print("MongoDB Test Schema Setup Complete!");
print("Collections created:");
print("- test_configurations");
print("- test_data_templates");
print("- test_artifacts");
print("- test_scenarios");
print("- compatibility_matrix");
print("- performance_baselines");
print("");
print("Sample data inserted and indexes created.");
print("Use the utility functions to manage test data:");
print("- cleanupExpiredArtifacts()");
print("- getActiveConfigsByType('browser')");
print("- getTestExecutionSummaryByBrowser(7)");
