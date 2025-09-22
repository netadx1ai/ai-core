import type { DemoScenario } from "../types";

export const demoScenarios: DemoScenario[] = [
    {
        id: "blog-post-creation",
        title: "📝 Blog Post Creation & Publishing",
        icon: "📝",
        description: "Create professional blog posts with SEO optimization and multi-platform publishing",
        example_prompt:
            "Create a blog post about AI automation trends and schedule it on our WordPress site and LinkedIn",
        expected_outcome: "High-quality blog post generated, optimized for SEO, and published to multiple platforms",
        workflow_type: "blog-post-social",
    },
    {
        id: "social-media-campaign",
        title: "📱 Social Media Campaign",
        icon: "📱",
        description: "Generate engaging social media content optimized for different platforms",
        example_prompt:
            "Create a social media campaign about our new product launch for Twitter, LinkedIn, and Facebook",
        expected_outcome: "Platform-optimized social media posts with appropriate hashtags and scheduling",
        workflow_type: "social-media-campaign",
    },
    {
        id: "email-newsletter",
        title: "📧 Email Newsletter",
        icon: "📧",
        description: "Generate professional newsletters with curated content and personalization",
        example_prompt:
            "Generate our weekly tech newsletter with the latest AI developments and send it to subscribers",
        expected_outcome: "Professional newsletter with curated content and personalized sections",
        workflow_type: "email-newsletter",
    },
    {
        id: "multi-client-federation",
        title: "🔗 Multi-Client Federation Demo",
        icon: "🔗",
        description: "Demonstrate cross-client collaboration and resource sharing",
        example_prompt:
            "Create marketing content for Client A and publish it using Client B's premium publishing service",
        expected_outcome: "Content created by one client's system and published through another's infrastructure",
        workflow_type: "federation-demo",
    },
    {
        id: "cost-optimization",
        title: "💰 Cost Optimization Demo",
        icon: "💰",
        description: "Showcase intelligent provider selection for maximum cost efficiency",
        example_prompt: "Generate a comprehensive market analysis report using the most cost-effective AI providers",
        expected_outcome: "High-quality report generated using optimal provider selection for cost efficiency",
        workflow_type: "cost-optimized",
    },
    {
        id: "custom-workflow",
        title: "🎯 Custom Workflow Demo",
        icon: "🎯",
        description: "Create complex multi-step workflows with custom automation",
        example_prompt: "Create a technical documentation page, generate API examples, and set up automated testing",
        expected_outcome: "Complete technical documentation with working code examples and test suite",
        workflow_type: "custom-workflow",
    },
];

export const defaultPrompts = [
    "Create a blog post about sustainable AI practices in modern business",
    "Generate a comprehensive guide on remote work productivity tools",
    "Write a technical article about microservices architecture best practices",
    "Create content for a product launch campaign across social media platforms",
    "Generate a newsletter about the latest trends in artificial intelligence",
    "Write a case study about successful digital transformation initiatives",
];

export const workflowSteps = [
    {
        id: "intent-parsing",
        name: "Intent Parsing",
        description: "Analyzing user input and extracting intent",
    },
    {
        id: "workflow-creation",
        name: "Workflow Creation",
        description: "Creating optimized workflow based on parsed intent",
    },
    {
        id: "content-generation",
        name: "Content Generation",
        description: "Generating high-quality content using AI models",
    },
    {
        id: "image-creation",
        name: "Image Creation",
        description: "Creating featured images and visual content",
    },
    {
        id: "quality-validation",
        name: "Quality Validation",
        description: "Validating content quality and SEO optimization",
    },
    {
        id: "federation-orchestration",
        name: "MCP Federation",
        description: "Orchestrating multiple service providers",
    },
    {
        id: "final-processing",
        name: "Final Processing",
        description: "Final formatting and delivery preparation",
    },
];

export const federationNodes = [
    {
        id: "intent-parser",
        name: "Intent Parser",
        status: "idle" as const,
        last_active: new Date().toISOString(),
    },
    {
        id: "workflow-engine",
        name: "Workflow Engine",
        status: "idle" as const,
        last_active: new Date().toISOString(),
    },
    {
        id: "content-mcp",
        name: "Content MCP",
        status: "idle" as const,
        last_active: new Date().toISOString(),
    },
    {
        id: "image-mcp",
        name: "Image MCP",
        status: "idle" as const,
        last_active: new Date().toISOString(),
    },
    {
        id: "publishing-mcp",
        name: "Publishing MCP",
        status: "idle" as const,
        last_active: new Date().toISOString(),
    },
];

export const sampleMetrics = {
    total_requests: 0,
    successful_requests: 0,
    failed_requests: 0,
    average_execution_time_ms: 0,
    average_quality_score: 0,
    total_cost_usd: 0,
    cost_savings_usd: 0,
    uptime_percentage: 100,
    last_updated: new Date().toISOString(),
};
