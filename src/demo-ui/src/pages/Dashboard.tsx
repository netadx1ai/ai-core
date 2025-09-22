import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Play,
  Square,
  Activity,
  Zap,
  Clock,
  CheckCircle,
  AlertCircle,
  Workflow,
  FileText,
  Image,
  Brain,
  BarChart3,
  RefreshCw,
  Plus,
} from 'lucide-react';
import { useAppStore, useWorkflowActions, useServiceActions } from '../hooks/useAppStore';
import WorkflowCard from '../components/WorkflowCard';
import ServiceStatus from '../components/ServiceStatus';
import MetricsDisplay from '../components/MetricsDisplay';
import WorkflowBuilder from '../components/WorkflowBuilder';

const Dashboard: React.FC = () => {
  const {
    services,
    workflows,
    activeWorkflow,
    currentSession,
    notifications,
    startSession,
    endSession,
    addNotification,
  } = useAppStore();

  const { createWorkflow, cancelWorkflow } = useWorkflowActions();
  const { refreshServices } = useServiceActions();

  const [showWorkflowBuilder, setShowWorkflowBuilder] = useState(false);
  const [liveMetrics, setLiveMetrics] = useState({
    activeWorkflows: 0,
    completedWorkflows: 0,
    totalProcessingTime: 0,
    averageResponseTime: 0,
    successRate: 95.2,
    servicesOnline: 4,
  });

  // Simulate live metrics updates
  useEffect(() => {
    const interval = setInterval(() => {
      const activeCount = workflows.filter(w => w.status === 'running' || w.status === 'queued').length;
      const completedCount = workflows.filter(w => w.status === 'completed').length;
      const totalTime = workflows.reduce((sum, w) => sum + (w.processingTimeMs || 0), 0);
      const avgResponseTime = services.reduce((sum, s) => sum + (s.responseTime || 0), 0) / services.length;
      const onlineServices = services.filter(s => s.status === 'active').length;

      setLiveMetrics({
        activeWorkflows: activeCount,
        completedWorkflows: completedCount,
        totalProcessingTime: totalTime,
        averageResponseTime: Math.round(avgResponseTime),
        successRate: completedCount > 0 ? (completedCount / workflows.length) * 100 : 95.2,
        servicesOnline: onlineServices,
      });
    }, 2000);

    return () => clearInterval(interval);
  }, [workflows, services]);

  const handleStartDemo = () => {
    if (!currentSession) {
      startSession();
      addNotification({
        id: `demo-${Date.now()}`,
        type: 'success',
        title: 'AI-CORE Demo Started',
        message: 'Welcome to the AI-CORE Multi-MCP Demonstration Platform',
        timestamp: new Date().toISOString(),
        autoHide: true,
        duration: 5000,
      });
    }
  };

  const handleQuickWorkflow = (type: string, params: Record<string, any>) => {
    if (!currentSession) {
      handleStartDemo();
    }
    createWorkflow(type, params);
  };

  const quickWorkflowTemplates = [
    {
      id: 'blog-campaign',
      title: 'Blog Post Campaign',
      description: 'Generate blog post + image + social media content',
      icon: FileText,
      color: 'bg-blue-500',
      params: {
        topic: 'AI Innovation in Healthcare',
        target_audience: 'healthcare professionals',
        tone: 'professional',
        image_style: 'modern',
      },
    },
    {
      id: 'content-analysis',
      title: 'Content Analysis',
      description: 'Comprehensive text analysis with AI insights',
      icon: Brain,
      color: 'bg-purple-500',
      params: {
        text: 'Artificial intelligence is revolutionizing healthcare by enabling faster diagnoses, personalized treatments, and improved patient outcomes. Machine learning algorithms can analyze medical images with unprecedented accuracy.',
      },
    },
    {
      id: 'creative-pipeline',
      title: 'Creative Pipeline',
      description: 'AI-powered creative content generation',
      icon: Image,
      color: 'bg-green-500',
      params: {
        concept: 'Future of Work with AI',
        style: 'futuristic',
        iterations: 3,
      },
    },
  ];

  return (
    <div className="space-y-6">
      {/* Header Section */}
      <div className="bg-gradient-to-r from-blue-600 to-purple-600 rounded-lg p-6 text-white">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold mb-2">AI-CORE Multi-MCP Platform</h1>
            <p className="text-blue-100 text-lg">
              Real-time workflow orchestration with Google Gemini AI integration
            </p>
          </div>
          <div className="flex items-center space-x-4">
            {currentSession ? (
              <div className="text-right">
                <p className="text-sm text-blue-100">Session Active</p>
                <p className="text-lg font-semibold">
                  {new Date(currentSession.startedAt).toLocaleTimeString()}
                </p>
              </div>
            ) : (
              <motion.button
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={handleStartDemo}
                className="bg-white text-blue-600 px-6 py-3 rounded-lg font-semibold hover:bg-blue-50 transition-colors"
              >
                <Play className="w-5 h-5 inline-block mr-2" />
                Start Demo Session
              </motion.button>
            )}
          </div>
        </div>
      </div>

      {/* Live Metrics Dashboard */}
      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-white rounded-lg p-4 shadow-sm border"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Active Workflows</p>
              <p className="text-2xl font-bold text-blue-600">{liveMetrics.activeWorkflows}</p>
            </div>
            <Activity className="w-8 h-8 text-blue-500" />
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-white rounded-lg p-4 shadow-sm border"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Completed</p>
              <p className="text-2xl font-bold text-green-600">{liveMetrics.completedWorkflows}</p>
            </div>
            <CheckCircle className="w-8 h-8 text-green-500" />
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.2 }}
          className="bg-white rounded-lg p-4 shadow-sm border"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Services Online</p>
              <p className="text-2xl font-bold text-purple-600">{liveMetrics.servicesOnline}/4</p>
            </div>
            <Zap className="w-8 h-8 text-purple-500" />
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-white rounded-lg p-4 shadow-sm border"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Avg Response</p>
              <p className="text-2xl font-bold text-orange-600">{liveMetrics.averageResponseTime}ms</p>
            </div>
            <Clock className="w-8 h-8 text-orange-500" />
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="bg-white rounded-lg p-4 shadow-sm border"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Success Rate</p>
              <p className="text-2xl font-bold text-teal-600">{liveMetrics.successRate.toFixed(1)}%</p>
            </div>
            <BarChart3 className="w-8 h-8 text-teal-500" />
          </div>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="bg-white rounded-lg p-4 shadow-sm border"
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600">Total Processing</p>
              <p className="text-2xl font-bold text-indigo-600">
                {(liveMetrics.totalProcessingTime / 1000).toFixed(1)}s
              </p>
            </div>
            <RefreshCw className="w-8 h-8 text-indigo-500" />
          </div>
        </motion.div>
      </div>

      {/* Quick Actions */}
      <div className="bg-white rounded-lg p-6 shadow-sm border">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold text-gray-900">Quick Workflow Templates</h2>
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={() => setShowWorkflowBuilder(true)}
            className="bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-blue-700 transition-colors"
          >
            <Plus className="w-4 h-4 inline-block mr-2" />
            Custom Workflow
          </motion.button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {quickWorkflowTemplates.map((template, index) => (
            <motion.div
              key={template.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.1 }}
              className="border rounded-lg p-4 hover:shadow-md transition-shadow cursor-pointer"
              onClick={() => handleQuickWorkflow(template.id.replace('-', '_'), template.params)}
            >
              <div className="flex items-center mb-3">
                <div className={`${template.color} p-2 rounded-lg mr-3`}>
                  <template.icon className="w-5 h-5 text-white" />
                </div>
                <h3 className="font-semibold text-gray-900">{template.title}</h3>
              </div>
              <p className="text-sm text-gray-600 mb-3">{template.description}</p>
              <motion.button
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                className="w-full bg-gray-100 text-gray-700 py-2 rounded text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                <Workflow className="w-4 h-4 inline-block mr-2" />
                Start Workflow
              </motion.button>
            </motion.div>
          ))}
        </div>
      </div>

      {/* Services Status */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <ServiceStatus
          services={services}
          onRefresh={refreshServices}
        />

        {/* Recent Workflows */}
        <div className="bg-white rounded-lg p-6 shadow-sm border">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold text-gray-900">Recent Workflows</h2>
            <span className="text-sm text-gray-500">{workflows.length} total</span>
          </div>

          <div className="space-y-3 max-h-96 overflow-y-auto">
            <AnimatePresence>
              {workflows.slice(0, 5).map((workflow) => (
                <motion.div
                  key={workflow.workflowId}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: 20 }}
                  className="border rounded-lg p-3 hover:shadow-sm transition-shadow"
                >
                  <WorkflowCard
                    workflow={workflow}
                    onViewDetails={(id) => console.log('View details:', id)}
                    onCancel={cancelWorkflow}
                  />
                </motion.div>
              ))}
            </AnimatePresence>

            {workflows.length === 0 && (
              <div className="text-center py-8 text-gray-500">
                <Workflow className="w-12 h-12 mx-auto mb-3 text-gray-300" />
                <p>No workflows yet. Start your first workflow above!</p>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Active Workflow Details */}
      {activeWorkflow && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-white rounded-lg p-6 shadow-sm border"
        >
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-semibold text-gray-900">Active Workflow Details</h2>
            <div className="flex items-center space-x-2">
              <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                activeWorkflow.status === 'running' ? 'bg-blue-100 text-blue-800' :
                activeWorkflow.status === 'completed' ? 'bg-green-100 text-green-800' :
                activeWorkflow.status === 'failed' ? 'bg-red-100 text-red-800' :
                'bg-yellow-100 text-yellow-800'
              }`}>
                {activeWorkflow.status.toUpperCase()}
              </span>
              {activeWorkflow.status === 'running' && (
                <motion.button
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                  onClick={() => cancelWorkflow(activeWorkflow.workflowId)}
                  className="bg-red-100 text-red-700 px-3 py-1 rounded text-sm hover:bg-red-200 transition-colors"
                >
                  <Square className="w-3 h-3 inline-block mr-1" />
                  Cancel
                </motion.button>
              )}
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <h3 className="font-medium text-gray-700 mb-2">Workflow Information</h3>
              <dl className="space-y-1 text-sm">
                <div className="flex justify-between">
                  <dt className="text-gray-500">Type:</dt>
                  <dd className="font-medium">{activeWorkflow.workflowType}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-gray-500">Created:</dt>
                  <dd>{new Date(activeWorkflow.createdAt).toLocaleString()}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-gray-500">Steps:</dt>
                  <dd>{activeWorkflow.steps.length}</dd>
                </div>
              </dl>
            </div>

            <div>
              <h3 className="font-medium text-gray-700 mb-2">Progress</h3>
              <div className="space-y-2">
                {activeWorkflow.steps.map((step, index) => (
                  <div key={step.stepId} className="flex items-center text-sm">
                    <div className={`w-4 h-4 rounded-full mr-3 flex items-center justify-center ${
                      step.status === 'completed' ? 'bg-green-500' :
                      step.status === 'running' ? 'bg-blue-500' :
                      step.status === 'failed' ? 'bg-red-500' :
                      'bg-gray-300'
                    }`}>
                      {step.status === 'completed' && <CheckCircle className="w-3 h-3 text-white" />}
                      {step.status === 'failed' && <AlertCircle className="w-3 h-3 text-white" />}
                    </div>
                    <span className={step.status === 'completed' ? 'text-green-700' : 'text-gray-700'}>
                      {step.stepName}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </motion.div>
      )}

      {/* Workflow Builder Modal */}
      <AnimatePresence>
        {showWorkflowBuilder && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
            onClick={() => setShowWorkflowBuilder(false)}
          >
            <motion.div
              initial={{ scale: 0.9, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.9, opacity: 0 }}
              className="bg-white rounded-lg p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto"
              onClick={(e) => e.stopPropagation()}
            >
              <WorkflowBuilder
                onClose={() => setShowWorkflowBuilder(false)}
                onCreateWorkflow={(type, params) => {
                  createWorkflow(type, params);
                  setShowWorkflowBuilder(false);
                }}
              />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Notifications */}
      <div className="fixed top-4 right-4 space-y-2 z-40">
        <AnimatePresence>
          {notifications.slice(0, 3).map((notification) => (
            <motion.div
              key={notification.id}
              initial={{ opacity: 0, x: 300 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 300 }}
              className={`rounded-lg p-4 shadow-lg max-w-sm ${
                notification.type === 'success' ? 'bg-green-100 border-green-500 text-green-800' :
                notification.type === 'error' ? 'bg-red-100 border-red-500 text-red-800' :
                notification.type === 'warning' ? 'bg-yellow-100 border-yellow-500 text-yellow-800' :
                'bg-blue-100 border-blue-500 text-blue-800'
              } border-l-4`}
            >
              <h4 className="font-medium">{notification.title}</h4>
              <p className="text-sm mt-1">{notification.message}</p>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
};

export default Dashboard;
