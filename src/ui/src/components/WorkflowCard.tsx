import { 
  PlayIcon, 
  PauseIcon, 
  StopIcon, 
  ClockIcon,
  CheckCircleIcon,
  ExclamationCircleIcon 
} from '@heroicons/react/24/outline';

interface WorkflowCardProps {
  workflow: {
    id: string;
    name: string;
    description: string;
    status: 'draft' | 'running' | 'paused' | 'completed' | 'failed';
    progress: number;
    createdAt: string;
    lastRun?: string;
    nextRun?: string;
  };
  onStart: (id: string) => void;
  onPause: (id: string) => void;
  onStop: (id: string) => void;
  onEdit: (id: string) => void;
}

export default function WorkflowCard({ workflow, onStart, onPause, onStop, onEdit }: WorkflowCardProps) {
  const getStatusIcon = () => {
    switch (workflow.status) {
      case 'running':
        return <PlayIcon className="h-5 w-5 text-green-500" />;
      case 'paused':
        return <PauseIcon className="h-5 w-5 text-yellow-500" />;
      case 'completed':
        return <CheckCircleIcon className="h-5 w-5 text-green-500" />;
      case 'failed':
        return <ExclamationCircleIcon className="h-5 w-5 text-red-500" />;
      default:
        return <ClockIcon className="h-5 w-5 text-gray-500" />;
    }
  };

  const getStatusColor = () => {
    switch (workflow.status) {
      case 'running':
        return 'bg-green-100 text-green-800';
      case 'paused':
        return 'bg-yellow-100 text-yellow-800';
      case 'completed':
        return 'bg-green-100 text-green-800';
      case 'failed':
        return 'bg-red-100 text-red-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="bg-white dark:bg-dark-800 rounded-lg shadow-sm border border-gray-200 dark:border-dark-600 p-6 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center space-x-3">
            {getStatusIcon()}
            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
              {workflow.name}
            </h3>
            <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getStatusColor()}`}>
              {workflow.status}
            </span>
          </div>
          <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
            {workflow.description}
          </p>
          
          {workflow.status === 'running' && (
            <div className="mt-4">
              <div className="flex items-center justify-between text-sm text-gray-600 dark:text-gray-400 mb-1">
                <span>Progress</span>
                <span>{workflow.progress}%</span>
              </div>
              <div className="w-full bg-gray-200 dark:bg-dark-600 rounded-full h-2">
                <div 
                  className="bg-primary-600 h-2 rounded-full transition-all duration-300" 
                  style={{ width: `${workflow.progress}%` }}
                />
              </div>
            </div>
          )}
          
          <div className="mt-4 text-xs text-gray-500 dark:text-gray-400 space-y-1">
            <div>Created: {new Date(workflow.createdAt).toLocaleDateString()}</div>
            {workflow.lastRun && (
              <div>Last run: {new Date(workflow.lastRun).toLocaleDateString()}</div>
            )}
            {workflow.nextRun && (
              <div>Next run: {new Date(workflow.nextRun).toLocaleDateString()}</div>
            )}
          </div>
        </div>
        
        <div className="flex items-center space-x-2 ml-4">
          {workflow.status === 'draft' || workflow.status === 'paused' ? (
            <button
              onClick={() => onStart(workflow.id)}
              className="p-2 text-green-600 hover:text-green-700 hover:bg-green-50 rounded-full transition-colors"
              title="Start workflow"
            >
              <PlayIcon className="h-4 w-4" />
            </button>
          ) : null}
          
          {workflow.status === 'running' ? (
            <>
              <button
                onClick={() => onPause(workflow.id)}
                className="p-2 text-yellow-600 hover:text-yellow-700 hover:bg-yellow-50 rounded-full transition-colors"
                title="Pause workflow"
              >
                <PauseIcon className="h-4 w-4" />
              </button>
              <button
                onClick={() => onStop(workflow.id)}
                className="p-2 text-red-600 hover:text-red-700 hover:bg-red-50 rounded-full transition-colors"
                title="Stop workflow"
              >
                <StopIcon className="h-4 w-4" />
              </button>
            </>
          ) : null}
          
          <button
            onClick={() => onEdit(workflow.id)}
            className="px-3 py-1 text-sm text-primary-600 hover:text-primary-700 hover:bg-primary-50 rounded-md transition-colors"
          >
            Edit
          </button>
        </div>
      </div>
    </div>
  );
}