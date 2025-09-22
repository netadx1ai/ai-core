interface WorkflowStatsProps {
  stats: {
    total: number;
    running: number;
    completed: number;
    failed: number;
    draft: number;
  };
}

export default function WorkflowStats({ stats }: WorkflowStatsProps) {
  const statItems = [
    {
      label: 'Total Workflows',
      value: stats.total,
      color: 'text-gray-600 dark:text-gray-300',
      bgColor: 'bg-gray-50 dark:bg-dark-700'
    },
    {
      label: 'Running',
      value: stats.running,
      color: 'text-green-600 dark:text-green-400',
      bgColor: 'bg-green-50 dark:bg-green-900/20'
    },
    {
      label: 'Completed',
      value: stats.completed,
      color: 'text-blue-600 dark:text-blue-400',
      bgColor: 'bg-blue-50 dark:bg-blue-900/20'
    },
    {
      label: 'Failed',
      value: stats.failed,
      color: 'text-red-600 dark:text-red-400',
      bgColor: 'bg-red-50 dark:bg-red-900/20'
    },
    {
      label: 'Draft',
      value: stats.draft,
      color: 'text-yellow-600 dark:text-yellow-400',
      bgColor: 'bg-yellow-50 dark:bg-yellow-900/20'
    }
  ];

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4 mb-6">
      {statItems.map((item) => (
        <div
          key={item.label}
          className={`${item.bgColor} rounded-lg p-4 border border-gray-200 dark:border-dark-600`}
        >
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-gray-600 dark:text-gray-400">
                {item.label}
              </p>
              <p className={`text-2xl font-semibold ${item.color}`}>
                {item.value}
              </p>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}