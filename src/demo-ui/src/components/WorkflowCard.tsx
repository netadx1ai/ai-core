import React from 'react';
import { motion } from 'framer-motion';
import {
  Clock,
  CheckCircle,
  AlertCircle,
  Play,
  Square,
  Eye,
  Activity,
  Zap,
} from 'lucide-react';
import { WorkflowCardProps } from '../types';

const WorkflowCard: React.FC<WorkflowCardProps> = ({
  workflow,
  onViewDetails,
  onCancel,
}) => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed':
        return 'bg-green-100 text-green-800 border-green-200';
      case 'running':
        return 'bg-blue-100 text-blue-800 border-blue-200';
      case 'failed':
        return 'bg-red-100 text-red-800 border-red-200';
      case 'cancelled':
        return 'bg-gray-100 text-gray-800 border-gray-200';
      case 'queued':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-200';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircle className="w-4 h-4" />;
      case 'running':
        return <Activity className="w-4 h-4 animate-pulse" />;
      case 'failed':
        return <AlertCircle className="w-4 h-4" />;
      case 'queued':
        return <Clock className="w-4 h-4" />;
      default:
        return <Clock className="w-4 h-4" />;
    }
  };

  const completedSteps = workflow.steps.filter(step => step.status === 'completed').length;
  const totalSteps = workflow.steps.length;
  const progressPercentage = totalSteps > 0 ? (completedSteps / totalSteps) * 100 : 0;

  const formatDuration = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  const formatWorkflowType = (type: string) => {
    return type
      .split('_')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  };

  return (
    <motion.div
      layout
      className="bg-white border rounded-lg p-4 hover:shadow-md transition-shadow"
    >
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center space-x-3">
          <div className={`px-2 py-1 rounded-full text-xs font-medium border ${getStatusColor(workflow.status)}`}>
            <div className="flex items-center space-x-1">
              {getStatusIcon(workflow.status)}
              <span>{workflow.status.toUpperCase()}</span>
            </div>
          </div>
          <h3 className="font-semibold text-gray-900">
            {formatWorkflowType(workflow.workflowType)}
          </h3>
        </div>

        <div className="flex items-center space-x-2">
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={() => onViewDetails(workflow.workflowId)}
            className="text-gray-500 hover:text-blue-600 transition-colors"
            title="View Details"
          >
            <Eye className="w-4 h-4" />
          </motion.button>

          {(workflow.status === 'running' || workflow.status === 'queued') && onCancel && (
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              onClick={() => onCancel(workflow.workflowId)}
              className="text-gray-500 hover:text-red-600 transition-colors"
              title="Cancel Workflow"
            >
              <Square className="w-4 h-4" />
            </motion.button>
          )}
        </div>
      </div>

      {/* Progress Bar */}
      {totalSteps > 0 && (
        <div className="mb-3">
          <div className="flex items-center justify-between text-xs text-gray-500 mb-1">
            <span>Progress: {completedSteps}/{totalSteps} steps</span>
            <span>{Math.round(progressPercentage)}%</span>
          </div>
          <div className="w-full bg-gray-200 rounded-full h-2">
            <motion.div
              className={`h-2 rounded-full transition-all duration-500 ${
                workflow.status === 'completed' ? 'bg-green-500' :
                workflow.status === 'failed' ? 'bg-red-500' :
                workflow.status === 'running' ? 'bg-blue-500' :
                'bg-yellow-500'
              }`}
              initial={{ width: 0 }}
              animate={{ width: `${progressPercentage}%` }}
            />
          </div>
        </div>
      )}

      {/* Workflow Info */}
      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <p className="text-gray-500">Created</p>
          <p className="font-medium">
            {new Date(workflow.createdAt).toLocaleDateString()}
          </p>
          <p className="text-xs text-gray-400">
            {new Date(workflow.createdAt).toLocaleTimeString()}
          </p>
        </div>

        <div>
          <p className="text-gray-500">Duration</p>
          <p className="font-medium">
            {formatDuration(workflow.processingTimeMs)}
          </p>
          {workflow.status === 'running' && (
            <p className="text-xs text-blue-600 flex items-center">
              <Activity className="w-3 h-3 mr-1 animate-pulse" />
              In progress...
            </p>
          )}
        </div>
      </div>

      {/* Key Parameters */}
      {workflow.parameters && Object.keys(workflow.parameters).length > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-100">
          <p className="text-xs text-gray-500 mb-2">Key Parameters</p>
          <div className="flex flex-wrap gap-1">
            {Object.entries(workflow.parameters)
              .slice(0, 3)
              .map(([key, value]) => (
                <span
                  key={key}
                  className="inline-block bg-gray-100 text-gray-700 px-2 py-1 rounded text-xs"
                  title={`${key}: ${value}`}
                >
                  {key}: {String(value).substring(0, 15)}
                  {String(value).length > 15 ? '...' : ''}
                </span>
              ))}
            {Object.keys(workflow.parameters).length > 3 && (
              <span className="inline-block bg-gray-100 text-gray-500 px-2 py-1 rounded text-xs">
                +{Object.keys(workflow.parameters).length - 3} more
              </span>
            )}
          </div>
        </div>
      )}

      {/* Error Message */}
      {workflow.status === 'failed' && (
        <div className="mt-3 pt-3 border-t border-gray-100">
          <div className="flex items-center text-red-600 text-sm">
            <AlertCircle className="w-4 h-4 mr-2" />
            <span className="font-medium">Workflow Failed</span>
          </div>
          <p className="text-xs text-red-500 mt-1">
            Check workflow details for error information
          </p>
        </div>
      )}

      {/* Success Summary */}
      {workflow.status === 'completed' && workflow.results && Object.keys(workflow.results).length > 0 && (
        <div className="mt-3 pt-3 border-t border-gray-100">
          <div className="flex items-center text-green-600 text-sm">
            <CheckCircle className="w-4 h-4 mr-2" />
            <span className="font-medium">Completed Successfully</span>
          </div>
          <p className="text-xs text-gray-500 mt-1">
            {Object.keys(workflow.results).length} results generated
          </p>
        </div>
      )}

      {/* Live Indicators */}
      {workflow.status === 'running' && (
        <div className="mt-3 pt-3 border-t border-gray-100">
          <div className="flex items-center justify-between">
            <div className="flex items-center text-blue-600 text-sm">
              <Zap className="w-4 h-4 mr-2 animate-pulse" />
              <span>Processing...</span>
            </div>

            {/* Animated dots for running status */}
            <div className="flex space-x-1">
              {[0, 1, 2].map((i) => (
                <motion.div
                  key={i}
                  className="w-2 h-2 bg-blue-500 rounded-full"
                  animate={{
                    scale: [1, 1.2, 1],
                    opacity: [0.5, 1, 0.5]
                  }}
                  transition={{
                    duration: 1.5,
                    repeat: Infinity,
                    delay: i * 0.2
                  }}
                />
              ))}
            </div>
          </div>
        </div>
      )}
    </motion.div>
  );
};

export default WorkflowCard;
