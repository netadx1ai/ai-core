import { useState, useEffect } from "react";
import { PlusIcon, MagnifyingGlassIcon, SparklesIcon } from "@heroicons/react/24/outline";
import { apiService } from "../services/api";
import WorkflowCard from "../components/WorkflowCard";
import WorkflowStats from "../components/WorkflowStats";

interface Workflow {
    id: string;
    name: string;
    description: string;
    status: "draft" | "running" | "paused" | "completed" | "failed";
    progress: number;
    createdAt: string;
    lastRun?: string;
    nextRun?: string;
    intent?: string;
    workflowType?: string;
}

export default function WorkflowManager() {
    const [workflows, setWorkflows] = useState<Workflow[]>([]);
    const [searchTerm, setSearchTerm] = useState("");
    const [statusFilter, setStatusFilter] = useState<string>("all");
    const [loading, setLoading] = useState(true);
    const [showCreateModal, setShowCreateModal] = useState(false);
    const [createIntent, setCreateIntent] = useState("");
    const [workflowType, setWorkflowType] = useState("blog-post-social");
    const [isCreating, setIsCreating] = useState(false);

    // Mock data for demonstration
    useEffect(() => {
        setTimeout(() => {
            setWorkflows([
                {
                    id: "1",
                    name: "Daily Sales Report",
                    description: "Automatically generate and send daily sales reports to stakeholders",
                    status: "running",
                    progress: 75,
                    createdAt: "2024-01-15T10:00:00Z",
                    lastRun: "2024-01-20T08:00:00Z",
                    nextRun: "2024-01-21T08:00:00Z",
                },
                {
                    id: "2",
                    name: "Customer Onboarding",
                    description: "Send welcome emails and setup accounts for new customers",
                    status: "completed",
                    progress: 100,
                    createdAt: "2024-01-10T14:30:00Z",
                    lastRun: "2024-01-20T09:15:00Z",
                },
                {
                    id: "3",
                    name: "Data Backup Process",
                    description: "Weekly backup of all critical business data",
                    status: "paused",
                    progress: 0,
                    createdAt: "2024-01-05T16:00:00Z",
                    lastRun: "2024-01-14T02:00:00Z",
                },
                {
                    id: "4",
                    name: "Social Media Posting",
                    description: "Automatically post content across social media platforms",
                    status: "failed",
                    progress: 25,
                    createdAt: "2024-01-18T11:00:00Z",
                    lastRun: "2024-01-20T12:00:00Z",
                },
                {
                    id: "5",
                    name: "Invoice Processing",
                    description: "Process and categorize incoming invoices",
                    status: "draft",
                    progress: 0,
                    createdAt: "2024-01-19T13:45:00Z",
                },
            ]);
            setLoading(false);
        }, 1000);
    }, []);

    const filteredWorkflows = workflows.filter((workflow) => {
        const matchesSearch =
            workflow.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
            workflow.description.toLowerCase().includes(searchTerm.toLowerCase());
        const matchesStatus = statusFilter === "all" || workflow.status === statusFilter;
        return matchesSearch && matchesStatus;
    });

    const stats = {
        total: workflows.length,
        running: workflows.filter((w) => w.status === "running").length,
        completed: workflows.filter((w) => w.status === "completed").length,
        failed: workflows.filter((w) => w.status === "failed").length,
        draft: workflows.filter((w) => w.status === "draft").length,
    };

    const handleStartWorkflow = (id: string) => {
        setWorkflows((prev) => prev.map((w) => (w.id === id ? { ...w, status: "running" as const, progress: 0 } : w)));
    };

    const handlePauseWorkflow = (id: string) => {
        setWorkflows((prev) => prev.map((w) => (w.id === id ? { ...w, status: "paused" as const } : w)));
    };

    const handleStopWorkflow = (id: string) => {
        setWorkflows((prev) => prev.map((w) => (w.id === id ? { ...w, status: "draft" as const, progress: 0 } : w)));
    };

    const handleEditWorkflow = (id: string) => {
        console.log("Edit workflow:", id);
    };

    const handleCreateAIWorkflow = async () => {
        if (!createIntent.trim()) return;

        setIsCreating(true);
        try {
            const response = await apiService.createWorkflowFromIntent(createIntent, workflowType);

            if (response.workflow_id) {
                const newWorkflow: Workflow = {
                    id: response.workflow_id,
                    name: `AI Workflow: ${createIntent.substring(0, 50)}...`,
                    description: createIntent,
                    status: "running",
                    progress: 0,
                    createdAt: new Date().toISOString(),
                    intent: createIntent,
                    workflowType: workflowType,
                };

                setWorkflows((prev) => [newWorkflow, ...prev]);
                setShowCreateModal(false);
                setCreateIntent("");

                // Start polling for workflow status
                pollWorkflowStatus(response.workflow_id);
            }
        } catch (error) {
            console.error("Failed to create AI workflow:", error);
            alert("Failed to create workflow. Please ensure AI-CORE services are running.");
        } finally {
            setIsCreating(false);
        }
    };

    const pollWorkflowStatus = async (workflowId: string) => {
        const pollInterval = setInterval(async () => {
            try {
                const status = await apiService.getWorkflowStatus(workflowId);

                setWorkflows((prev) =>
                    prev.map((w) =>
                        w.id === workflowId
                            ? {
                                  ...w,
                                  status:
                                      status.status === "completed"
                                          ? "completed"
                                          : status.status === "failed"
                                            ? "failed"
                                            : "running",
                                  progress: status.progress || 0,
                              }
                            : w,
                    ),
                );

                if (status.status === "completed" || status.status === "failed") {
                    clearInterval(pollInterval);
                }
            } catch (error) {
                console.error("Error polling workflow status:", error);
                clearInterval(pollInterval);
            }
        }, 5000);

        // Stop polling after 10 minutes
        setTimeout(() => clearInterval(pollInterval), 600000);
    };

    if (loading) {
        return (
            <div className="min-h-screen bg-gray-50 dark:bg-dark-900 flex items-center justify-center">
                <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-primary-600"></div>
            </div>
        );
    }

    return (
        <div className="min-h-screen bg-gray-50 dark:bg-dark-900">
            <header className="bg-white dark:bg-dark-800 shadow">
                <div className="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                    <div className="flex justify-between items-center">
                        <div>
                            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Workflow Manager</h1>
                            <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
                                Manage and monitor your automation workflows
                            </p>
                        </div>
                        <div className="flex space-x-3">
                            <button
                                onClick={() => setShowCreateModal(true)}
                                className="inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500"
                            >
                                <SparklesIcon className="-ml-1 mr-2 h-5 w-5" />
                                Create AI Workflow
                            </button>
                            <button className="inline-flex items-center px-4 py-2 border border-gray-300 dark:border-dark-600 rounded-md shadow-sm text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-dark-700 hover:bg-gray-50 dark:hover:bg-dark-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500">
                                <PlusIcon className="-ml-1 mr-2 h-5 w-5" />
                                Manual Workflow
                            </button>
                        </div>
                    </div>
                </div>
            </header>

            <main className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
                <div className="px-4 py-6 sm:px-0">
                    <WorkflowStats stats={stats} />

                    <div className="bg-white dark:bg-dark-800 shadow rounded-lg">
                        <div className="px-4 py-5 sm:p-6">
                            <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center mb-6">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4 sm:mb-0">
                                    Your Workflows ({filteredWorkflows.length})
                                </h3>

                                <div className="flex flex-col sm:flex-row space-y-2 sm:space-y-0 sm:space-x-4 w-full sm:w-auto">
                                    <div className="relative">
                                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                                            <MagnifyingGlassIcon className="h-5 w-5 text-gray-400" />
                                        </div>
                                        <input
                                            type="text"
                                            placeholder="Search workflows..."
                                            value={searchTerm}
                                            onChange={(e) => setSearchTerm(e.target.value)}
                                            className="block w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-dark-600 rounded-md leading-5 bg-white dark:bg-dark-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 sm:text-sm"
                                        />
                                    </div>

                                    <select
                                        value={statusFilter}
                                        onChange={(e) => setStatusFilter(e.target.value)}
                                        className="block w-full px-3 py-2 border border-gray-300 dark:border-dark-600 rounded-md bg-white dark:bg-dark-700 text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-primary-500 sm:text-sm"
                                    >
                                        <option value="all">All Status</option>
                                        <option value="draft">Draft</option>
                                        <option value="running">Running</option>
                                        <option value="paused">Paused</option>
                                        <option value="completed">Completed</option>
                                        <option value="failed">Failed</option>
                                    </select>
                                </div>
                            </div>

                            {filteredWorkflows.length === 0 ? (
                                <div className="text-center py-12">
                                    <svg
                                        className="mx-auto h-12 w-12 text-gray-400"
                                        fill="none"
                                        viewBox="0 0 24 24"
                                        stroke="currentColor"
                                        aria-hidden="true"
                                    >
                                        <path
                                            vectorEffect="non-scaling-stroke"
                                            strokeLinecap="round"
                                            strokeLinejoin="round"
                                            strokeWidth={2}
                                            d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"
                                        />
                                    </svg>
                                    <h3 className="mt-2 text-sm font-medium text-gray-900 dark:text-white">
                                        {searchTerm || statusFilter !== "all"
                                            ? "No matching workflows"
                                            : "No workflows"}
                                    </h3>
                                    <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                                        {searchTerm || statusFilter !== "all"
                                            ? "Try adjusting your search or filter criteria."
                                            : "Get started by creating your first automation workflow."}
                                    </p>
                                    {!searchTerm && statusFilter === "all" && (
                                        <div className="mt-6">
                                            <button
                                                type="button"
                                                onClick={() => setShowCreateModal(true)}
                                                className="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-primary-600 hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500"
                                            >
                                                <SparklesIcon className="-ml-1 mr-2 h-5 w-5" />
                                                Create AI Workflow
                                            </button>
                                        </div>
                                    )}
                                </div>
                            ) : (
                                <div className="space-y-4">
                                    {filteredWorkflows.map((workflow) => (
                                        <WorkflowCard
                                            key={workflow.id}
                                            workflow={workflow}
                                            onStart={handleStartWorkflow}
                                            onPause={handlePauseWorkflow}
                                            onStop={handleStopWorkflow}
                                            onEdit={handleEditWorkflow}
                                        />
                                    ))}
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </main>

            {/* Create AI Workflow Modal */}
            {showCreateModal && (
                <div className="fixed inset-0 bg-gray-600 bg-opacity-50 dark:bg-black dark:bg-opacity-50 overflow-y-auto h-full w-full z-50">
                    <div className="relative top-20 mx-auto p-5 border w-full max-w-2xl shadow-lg rounded-md bg-white dark:bg-dark-800">
                        <div className="mt-3">
                            <div className="flex items-center justify-between mb-4">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white">
                                    Create AI-Powered Workflow
                                </h3>
                                <button
                                    onClick={() => setShowCreateModal(false)}
                                    className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                                >
                                    <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path
                                            strokeLinecap="round"
                                            strokeLinejoin="round"
                                            strokeWidth={2}
                                            d="M6 18L18 6M6 6l12 12"
                                        />
                                    </svg>
                                </button>
                            </div>

                            <div className="space-y-4">
                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                        Describe what you want to create
                                    </label>
                                    <textarea
                                        value={createIntent}
                                        onChange={(e) => setCreateIntent(e.target.value)}
                                        placeholder="Example: Write a blog post about AI automation trends in healthcare for business professionals"
                                        rows={4}
                                        className="block w-full px-3 py-2 border border-gray-300 dark:border-dark-600 rounded-md shadow-sm placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-primary-500 focus:border-primary-500 bg-white dark:bg-dark-700 text-gray-900 dark:text-white"
                                    />
                                    <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                        Be specific about your requirements, target audience, tone, and any other
                                        details.
                                    </p>
                                </div>

                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                        Workflow Type
                                    </label>
                                    <select
                                        value={workflowType}
                                        onChange={(e) => setWorkflowType(e.target.value)}
                                        className="block w-full px-3 py-2 border border-gray-300 dark:border-dark-600 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 bg-white dark:bg-dark-700 text-gray-900 dark:text-white"
                                    >
                                        <option value="blog-post-social">Blog Post + Social Media</option>
                                        <option value="blog-post">Blog Post Only</option>
                                        <option value="social-media">Social Media Content</option>
                                        <option value="marketing-copy">Marketing Copy</option>
                                        <option value="technical-docs">Technical Documentation</option>
                                    </select>
                                </div>

                                <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700 rounded-md p-4">
                                    <div className="flex">
                                        <SparklesIcon className="h-5 w-5 text-blue-400 mr-2 flex-shrink-0" />
                                        <div>
                                            <h4 className="text-sm font-medium text-blue-800 dark:text-blue-200">
                                                AI-Powered Content Generation
                                            </h4>
                                            <p className="mt-1 text-sm text-blue-700 dark:text-blue-300">
                                                This workflow will use AI to parse your intent, generate high-quality
                                                content, and create supporting materials automatically.
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            </div>

                            <div className="flex justify-end space-x-3 mt-6">
                                <button
                                    onClick={() => setShowCreateModal(false)}
                                    className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-dark-700 border border-gray-300 dark:border-dark-600 rounded-md hover:bg-gray-50 dark:hover:bg-dark-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500"
                                >
                                    Cancel
                                </button>
                                <button
                                    onClick={handleCreateAIWorkflow}
                                    disabled={!createIntent.trim() || isCreating}
                                    className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 disabled:bg-primary-400 disabled:cursor-not-allowed border border-transparent rounded-md focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500 flex items-center"
                                >
                                    {isCreating ? (
                                        <>
                                            <div className="animate-spin -ml-1 mr-2 h-4 w-4 border-2 border-white border-t-transparent rounded-full"></div>
                                            Creating...
                                        </>
                                    ) : (
                                        <>
                                            <SparklesIcon className="-ml-1 mr-2 h-4 w-4" />
                                            Create Workflow
                                        </>
                                    )}
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
