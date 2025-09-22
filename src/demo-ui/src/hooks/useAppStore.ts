import React from "react";
import { v4 as uuidv4 } from "uuid";
import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { AppStore, DemoSession, McpService, NotificationMessage, Workflow } from "../types";

export const useAppStore = create<AppStore>()(
    devtools(
        (set, get) => ({
            // Services State
            services: [
                {
                    name: "demo-content-mcp",
                    url: "http://localhost:8804",
                    port: 8804,
                    status: "active",
                    capabilities: ["content_generation", "blog_posts", "social_media"],
                    lastHealthCheck: new Date().toISOString(),
                    responseTime: 120,
                },
                {
                    name: "text-processing-mcp",
                    url: "http://localhost:8805",
                    port: 8805,
                    status: "active",
                    capabilities: ["text_analysis", "keywords", "sentiment", "readability"],
                    lastHealthCheck: new Date().toISOString(),
                    responseTime: 95,
                },
                {
                    name: "image-generation-mcp",
                    url: "http://localhost:8806",
                    port: 8806,
                    status: "active",
                    capabilities: ["image_generation", "style_control", "batch_generation"],
                    lastHealthCheck: new Date().toISOString(),
                    responseTime: 1800,
                },
                {
                    name: "mcp-orchestrator",
                    url: "http://localhost:8807",
                    port: 8807,
                    status: "active",
                    capabilities: ["workflow_orchestration", "multi_mcp_coordination"],
                    lastHealthCheck: new Date().toISOString(),
                    responseTime: 150,
                },
            ],
            servicesLoading: false,

            // Workflows State
            workflows: [],
            activeWorkflow: null,
            workflowsLoading: false,

            // Session State
            currentSession: null,

            // UI State
            notifications: [],
            sidebarOpen: true,

            // Actions
            setServices: (services: McpService[]) => set({ services }, false, "setServices"),

            addWorkflow: (workflow: Workflow) =>
                set(
                    (state) => ({
                        workflows: [workflow, ...state.workflows],
                    }),
                    false,
                    "addWorkflow",
                ),

            updateWorkflow: (workflowId: string, updates: Partial<Workflow>) =>
                set(
                    (state) => ({
                        workflows: state.workflows.map((w) => (w.workflowId === workflowId ? { ...w, ...updates } : w)),
                        activeWorkflow:
                            state.activeWorkflow?.workflowId === workflowId
                                ? { ...state.activeWorkflow, ...updates }
                                : state.activeWorkflow,
                    }),
                    false,
                    "updateWorkflow",
                ),

            setActiveWorkflow: (workflow: Workflow | null) =>
                set({ activeWorkflow: workflow }, false, "setActiveWorkflow"),

            addNotification: (notification: NotificationMessage) =>
                set(
                    (state) => {
                        const newNotification = {
                            ...notification,
                            id: notification.id || uuidv4(),
                        };

                        // Keep only the last 5 notifications
                        const notifications = [newNotification, ...state.notifications].slice(0, 5);

                        return { notifications };
                    },
                    false,
                    "addNotification",
                ),

            removeNotification: (id: string) =>
                set(
                    (state) => ({
                        notifications: state.notifications.filter((n) => n.id !== id),
                    }),
                    false,
                    "removeNotification",
                ),

            toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen }), false, "toggleSidebar"),

            startSession: () => {
                const session: DemoSession = {
                    sessionId: uuidv4(),
                    startedAt: new Date().toISOString(),
                    workflows: [],
                    totalProcessingTime: 0,
                    successfulWorkflows: 0,
                    failedWorkflows: 0,
                    generatedContent: [],
                    analysisResults: [],
                    generatedImages: [],
                };

                set({ currentSession: session }, false, "startSession");

                // Add notification
                get().addNotification({
                    id: uuidv4(),
                    type: "success",
                    title: "Demo Session Started",
                    message: "New AI-CORE demonstration session has begun",
                    timestamp: new Date().toISOString(),
                    autoHide: true,
                    duration: 3000,
                });
            },

            endSession: () => {
                const session = get().currentSession;
                if (session) {
                    // Add final notification
                    get().addNotification({
                        id: uuidv4(),
                        type: "info",
                        title: "Demo Session Ended",
                        message: `Session completed with ${session.successfulWorkflows} successful workflows`,
                        timestamp: new Date().toISOString(),
                        autoHide: true,
                        duration: 5000,
                    });
                }

                set({ currentSession: null }, false, "endSession");
            },
        }),
        {
            name: "ai-core-demo-store",
            partialize: (state) => ({
                sidebarOpen: state.sidebarOpen,
                // Persist only UI preferences, not dynamic data
            }),
        },
    ),
);

// Utility functions for common operations
export const useServiceActions = () => {
    const store = useAppStore();

    const refreshServices = async () => {
        store.setState({ servicesLoading: true });

        try {
            // In a real app, this would fetch from the API
            const services = store.services.map((service) => ({
                ...service,
                lastHealthCheck: new Date().toISOString(),
                responseTime: Math.floor(Math.random() * 1000) + 50,
                status: Math.random() > 0.1 ? "active" : ("error" as const),
            }));

            store.setServices(services);
        } catch (error) {
            store.addNotification({
                id: uuidv4(),
                type: "error",
                title: "Service Refresh Failed",
                message: "Failed to refresh service status",
                timestamp: new Date().toISOString(),
            });
        } finally {
            store.setState({ servicesLoading: false });
        }
    };

    const checkServiceHealth = async (serviceName: string) => {
        const service = store.services.find((s) => s.name === serviceName);
        if (!service) return false;

        try {
            // Simulate health check
            const isHealthy = Math.random() > 0.2;

            const updatedService = {
                ...service,
                status: isHealthy ? "active" : ("error" as const),
                lastHealthCheck: new Date().toISOString(),
                responseTime: isHealthy ? Math.floor(Math.random() * 500) + 50 : undefined,
            };

            store.setServices(store.services.map((s) => (s.name === serviceName ? updatedService : s)));

            return isHealthy;
        } catch (error) {
            return false;
        }
    };

    return {
        refreshServices,
        checkServiceHealth,
    };
};

export const useWorkflowActions = () => {
    const store = useAppStore();

    const createWorkflow = (type: string, parameters: Record<string, any>) => {
        const workflow: Workflow = {
            workflowId: uuidv4(),
            workflowType: type,
            status: "queued",
            steps: [],
            results: {},
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            parameters,
        };

        store.addWorkflow(workflow);
        store.setActiveWorkflow(workflow);

        store.addNotification({
            id: uuidv4(),
            type: "info",
            title: "Workflow Created",
            message: `${type} workflow has been queued for execution`,
            timestamp: new Date().toISOString(),
            autoHide: true,
            duration: 3000,
        });

        return workflow;
    };

    const cancelWorkflow = (workflowId: string) => {
        store.updateWorkflow(workflowId, {
            status: "cancelled",
            updatedAt: new Date().toISOString(),
        });

        store.addNotification({
            id: uuidv4(),
            type: "warning",
            title: "Workflow Cancelled",
            message: "Workflow execution has been cancelled",
            timestamp: new Date().toISOString(),
            autoHide: true,
            duration: 3000,
        });
    };

    return {
        createWorkflow,
        cancelWorkflow,
    };
};

// Helper to clear old notifications
export const useNotificationCleanup = () => {
    const { notifications, removeNotification } = useAppStore();

    React.useEffect(() => {
        const interval = setInterval(() => {
            const now = Date.now();
            notifications.forEach((notification) => {
                if (
                    notification.autoHide &&
                    notification.duration &&
                    now - new Date(notification.timestamp).getTime() > notification.duration
                ) {
                    removeNotification(notification.id);
                }
            });
        }, 1000);

        return () => clearInterval(interval);
    }, [notifications, removeNotification]);
};
