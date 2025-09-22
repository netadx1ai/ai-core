import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { usePlatform } from "../hooks/usePlatform";
import { useAuth } from "../hooks/useAuth";
import { apiService, type ServiceStatus } from "../services/api";
import LoadingButton from "../components/LoadingButton";
import ThemeToggle from "../components/ThemeToggle";

interface ServiceStatusMap {
    federation: ServiceStatus;
    intentParser: ServiceStatus;
    mcpManager: ServiceStatus;
    mcpProxy: ServiceStatus;
}

export default function Dashboard() {
    const [isLoggingOut, setIsLoggingOut] = useState(false);
    const [serviceStatuses, setServiceStatuses] = useState<ServiceStatusMap>({
        federation: { healthy: false, type: "UNKNOWN" },
        intentParser: { healthy: false, type: "UNKNOWN" },
        mcpManager: { healthy: false, type: "UNKNOWN" },
        mcpProxy: { healthy: false, type: "UNKNOWN" },
    });
    const [isLoadingServices, setIsLoadingServices] = useState(true);
    const platform = usePlatform();
    const { user, logout } = useAuth();

    const handleLogout = async () => {
        setIsLoggingOut(true);
        try {
            await logout();
        } finally {
            setIsLoggingOut(false);
        }
    };

    const checkServicesHealth = async () => {
        setIsLoadingServices(true);
        try {
            const statuses = await apiService.checkAllServicesHealth();
            setServiceStatuses(statuses as unknown as ServiceStatusMap);
        } catch (error) {
            console.error("Error checking service health:", error);
        } finally {
            setIsLoadingServices(false);
        }
    };

    useEffect(() => {
        checkServicesHealth();

        // Check service health every 30 seconds
        const interval = setInterval(checkServicesHealth, 30000);

        return () => clearInterval(interval);
    }, []);

    const getStatusColor = (status: ServiceStatus) => {
        if (!status.healthy) return "text-red-500";
        return status.type === "REAL" ? "text-green-500" : "text-yellow-500";
    };

    const getStatusDot = (status: ServiceStatus) => {
        if (!status.healthy) return "bg-red-500";
        return status.type === "REAL" ? "bg-green-500" : "bg-yellow-500";
    };

    const getStatusText = (status: ServiceStatus) => {
        if (!status.healthy) return `OFFLINE${status.error ? ` (${status.error})` : ""}`;
        return status.type;
    };

    const realServicesCount = Object.values(serviceStatuses).filter((s) => s.type === "REAL").length;
    const healthyServicesCount = Object.values(serviceStatuses).filter((s) => s.healthy).length;
    const totalServices = Object.keys(serviceStatuses).length;

    return (
        <div className="min-h-screen bg-gray-50 dark:bg-dark-900">
            <header className="bg-white dark:bg-dark-800 shadow">
                <div className="max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
                    <div className="flex justify-between items-center">
                        <div>
                            <h1 className="text-3xl font-bold text-gray-900 dark:text-white">AI-CORE Dashboard</h1>
                            <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">
                                Welcome back, {user?.username || user?.email}! | Platform: {platform.platform} | OS:{" "}
                                {platform.os || "unknown"}
                            </p>
                        </div>
                        <div className="flex items-center space-x-4">
                            <Link
                                to="/workflows"
                                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition-colors"
                            >
                                Workflows
                            </Link>
                            <ThemeToggle />
                            <LoadingButton onClick={handleLogout} loading={isLoggingOut} variant="secondary" size="sm">
                                Logout
                            </LoadingButton>
                        </div>
                    </div>
                </div>
            </header>

            <main className="max-w-7xl mx-auto py-6 sm:px-6 lg:px-8">
                <div className="px-4 py-6 sm:px-0">
                    {/* Service Status Alert */}
                    <div className="mb-6">
                        {realServicesCount === totalServices ? (
                            <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-700 rounded-md p-4">
                                <div className="flex">
                                    <div className="flex-shrink-0">
                                        <svg className="h-5 w-5 text-green-400" viewBox="0 0 20 20" fill="currentColor">
                                            <path
                                                fillRule="evenodd"
                                                d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                                                clipRule="evenodd"
                                            />
                                        </svg>
                                    </div>
                                    <div className="ml-3">
                                        <h3 className="text-sm font-medium text-green-800 dark:text-green-200">
                                            All Real Services Active
                                        </h3>
                                        <p className="mt-1 text-sm text-green-700 dark:text-green-300">
                                            All {totalServices} AI-CORE services are running in real mode.
                                        </p>
                                    </div>
                                </div>
                            </div>
                        ) : healthyServicesCount < totalServices ? (
                            <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-700 rounded-md p-4">
                                <div className="flex">
                                    <div className="flex-shrink-0">
                                        <svg className="h-5 w-5 text-red-400" viewBox="0 0 20 20" fill="currentColor">
                                            <path
                                                fillRule="evenodd"
                                                d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
                                                clipRule="evenodd"
                                            />
                                        </svg>
                                    </div>
                                    <div className="ml-3">
                                        <h3 className="text-sm font-medium text-red-800 dark:text-red-200">
                                            Service Issues Detected
                                        </h3>
                                        <p className="mt-1 text-sm text-red-700 dark:text-red-300">
                                            {totalServices - healthyServicesCount} of {totalServices} services are
                                            offline. Some features may not work properly.
                                        </p>
                                    </div>
                                </div>
                            </div>
                        ) : (
                            <div className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-700 rounded-md p-4">
                                <div className="flex">
                                    <div className="flex-shrink-0">
                                        <svg
                                            className="h-5 w-5 text-yellow-400"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fillRule="evenodd"
                                                d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                                                clipRule="evenodd"
                                            />
                                        </svg>
                                    </div>
                                    <div className="ml-3">
                                        <h3 className="text-sm font-medium text-yellow-800 dark:text-yellow-200">
                                            Mixed Service Mode
                                        </h3>
                                        <p className="mt-1 text-sm text-yellow-700 dark:text-yellow-300">
                                            {realServicesCount} of {totalServices} services are in real mode. Consider
                                            starting all real services for full functionality.
                                        </p>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>

                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                        {/* Quick Actions */}
                        <div className="bg-white dark:bg-dark-800 overflow-hidden shadow rounded-lg">
                            <div className="p-5">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Quick Actions</h3>
                                <div className="mt-4 space-y-3">
                                    <Link
                                        to="/workflows"
                                        className="w-full block text-center px-4 py-2 bg-primary-500 text-white rounded-md hover:bg-primary-600 transition-colors"
                                    >
                                        Create New Workflow
                                    </Link>
                                    <button className="w-full text-left px-4 py-2 bg-gray-100 dark:bg-dark-700 text-gray-700 dark:text-gray-300 rounded-md hover:bg-gray-200 dark:hover:bg-dark-600 transition-colors">
                                        View Analytics
                                    </button>
                                    <button
                                        onClick={checkServicesHealth}
                                        disabled={isLoadingServices}
                                        className="w-full text-left px-4 py-2 bg-blue-100 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 rounded-md hover:bg-blue-200 dark:hover:bg-blue-900/40 transition-colors disabled:opacity-50"
                                    >
                                        {isLoadingServices ? "Checking..." : "Refresh Services"}
                                    </button>
                                </div>
                            </div>
                        </div>

                        {/* Recent Workflows */}
                        <div className="bg-white dark:bg-dark-800 overflow-hidden shadow rounded-lg">
                            <div className="p-5">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Recent Workflows</h3>
                                <div className="mt-4 space-y-3">
                                    <p className="text-gray-500 dark:text-gray-400">No workflows created yet</p>
                                    <Link
                                        to="/workflows"
                                        className="text-sm text-primary-600 hover:text-primary-500 dark:text-primary-400 dark:hover:text-primary-300"
                                    >
                                        Create your first workflow →
                                    </Link>
                                </div>
                            </div>
                        </div>

                        {/* Service Status */}
                        <div className="bg-white dark:bg-dark-800 overflow-hidden shadow rounded-lg">
                            <div className="p-5">
                                <div className="flex items-center justify-between">
                                    <h3 className="text-lg font-medium text-gray-900 dark:text-white">
                                        Service Status
                                    </h3>
                                    {isLoadingServices && (
                                        <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary-500"></div>
                                    )}
                                </div>
                                <div className="mt-4 space-y-3">
                                    <div className="flex items-center justify-between">
                                        <div className="flex items-center">
                                            <div
                                                className={`h-3 w-3 rounded-full mr-2 ${getStatusDot(serviceStatuses.federation)}`}
                                            ></div>
                                            <span className="text-sm text-gray-600 dark:text-gray-300">Federation</span>
                                        </div>
                                        <span
                                            className={`text-xs font-medium ${getStatusColor(serviceStatuses.federation)}`}
                                        >
                                            {getStatusText(serviceStatuses.federation)}
                                        </span>
                                    </div>

                                    <div className="flex items-center justify-between">
                                        <div className="flex items-center">
                                            <div
                                                className={`h-3 w-3 rounded-full mr-2 ${getStatusDot(serviceStatuses.intentParser)}`}
                                            ></div>
                                            <span className="text-sm text-gray-600 dark:text-gray-300">
                                                Intent Parser
                                            </span>
                                        </div>
                                        <span
                                            className={`text-xs font-medium ${getStatusColor(serviceStatuses.intentParser)}`}
                                        >
                                            {getStatusText(serviceStatuses.intentParser)}
                                        </span>
                                    </div>

                                    <div className="flex items-center justify-between">
                                        <div className="flex items-center">
                                            <div
                                                className={`h-3 w-3 rounded-full mr-2 ${getStatusDot(serviceStatuses.mcpManager)}`}
                                            ></div>
                                            <span className="text-sm text-gray-600 dark:text-gray-300">
                                                MCP Manager
                                            </span>
                                        </div>
                                        <span
                                            className={`text-xs font-medium ${getStatusColor(serviceStatuses.mcpManager)}`}
                                        >
                                            {getStatusText(serviceStatuses.mcpManager)}
                                        </span>
                                    </div>

                                    <div className="flex items-center justify-between">
                                        <div className="flex items-center">
                                            <div
                                                className={`h-3 w-3 rounded-full mr-2 ${getStatusDot(serviceStatuses.mcpProxy)}`}
                                            ></div>
                                            <span className="text-sm text-gray-600 dark:text-gray-300">MCP Proxy</span>
                                        </div>
                                        <span
                                            className={`text-xs font-medium ${getStatusColor(serviceStatuses.mcpProxy)}`}
                                        >
                                            {getStatusText(serviceStatuses.mcpProxy)}
                                        </span>
                                    </div>

                                    <div className="pt-3 border-t border-gray-200 dark:border-dark-600">
                                        <div className="flex items-center justify-between">
                                            <span className="text-sm font-medium text-gray-900 dark:text-white">
                                                Summary
                                            </span>
                                            <span className="text-sm text-gray-600 dark:text-gray-300">
                                                {healthyServicesCount}/{totalServices} healthy
                                            </span>
                                        </div>
                                        <div className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                            {realServicesCount} real • {healthyServicesCount - realServicesCount}{" "}
                                            mock/other
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </main>
        </div>
    );
}
