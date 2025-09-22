import React from 'react';
import { motion } from 'framer-motion';
import {
  RefreshCw,
  CheckCircle,
  AlertCircle,
  Clock,
  Zap,
  Activity,
  Globe,
} from 'lucide-react';
import { ServiceStatusProps } from '../types';

const ServiceStatus: React.FC<ServiceStatusProps> = ({ services, onRefresh }) => {
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'active':
        return 'text-green-600 bg-green-100';
      case 'error':
        return 'text-red-600 bg-red-100';
      case 'starting':
        return 'text-yellow-600 bg-yellow-100';
      default:
        return 'text-gray-600 bg-gray-100';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'active':
        return <CheckCircle className="w-4 h-4" />;
      case 'error':
        return <AlertCircle className="w-4 h-4" />;
      case 'starting':
        return <Clock className="w-4 h-4" />;
      default:
        return <Activity className="w-4 h-4" />;
    }
  };

  const formatServiceName = (name: string) => {
    return name
      .split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  };

  const activeServices = services.filter(s => s.status === 'active').length;
  const totalServices = services.length;

  return (
    <div className="bg-white rounded-lg p-6 shadow-sm border">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-semibold text-gray-900">MCP Services</h2>
        <div className="flex items-center space-x-3">
          <span className="text-sm text-gray-500">
            {activeServices}/{totalServices} online
          </span>
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={onRefresh}
            className="text-gray-500 hover:text-blue-600 transition-colors"
            title="Refresh Services"
          >
            <RefreshCw className="w-5 h-5" />
          </motion.button>
        </div>
      </div>

      <div className="space-y-3">
        {services.map((service, index) => (
          <motion.div
            key={service.name}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: index * 0.1 }}
            className="border rounded-lg p-4 hover:shadow-sm transition-shadow"
          >
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center space-x-3">
                <div className={`p-2 rounded-lg ${getStatusColor(service.status)}`}>
                  {getStatusIcon(service.status)}
                </div>
                <div>
                  <h3 className="font-medium text-gray-900">
                    {formatServiceName(service.name)}
                  </h3>
                  <p className="text-sm text-gray-500">Port {service.port}</p>
                </div>
              </div>

              <div className="text-right">
                <div className={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(service.status)}`}>
                  {service.status.toUpperCase()}
                </div>
                {service.responseTime && (
                  <p className="text-xs text-gray-500 mt-1">
                    {service.responseTime}ms
                  </p>
                )}
              </div>
            </div>

            <div className="flex items-center justify-between text-sm">
              <div className="flex items-center space-x-4">
                <div className="flex items-center text-gray-500">
                  <Globe className="w-4 h-4 mr-1" />
                  <span>{service.url}</span>
                </div>
                <div className="flex items-center text-gray-500">
                  <Zap className="w-4 h-4 mr-1" />
                  <span>{service.capabilities.length} capabilities</span>
                </div>
              </div>

              <div className="text-xs text-gray-400">
                Last check: {new Date(service.lastHealthCheck).toLocaleTimeString()}
              </div>
            </div>

            {service.capabilities.length > 0 && (
              <div className="mt-3 pt-3 border-t border-gray-100">
                <p className="text-xs text-gray-500 mb-2">Capabilities</p>
                <div className="flex flex-wrap gap-1">
                  {service.capabilities.slice(0, 4).map((capability) => (
                    <span
                      key={capability}
                      className="inline-block bg-blue-100 text-blue-700 px-2 py-1 rounded text-xs"
                    >
                      {capability.replace('_', ' ')}
                    </span>
                  ))}
                  {service.capabilities.length > 4 && (
                    <span className="inline-block bg-gray-100 text-gray-500 px-2 py-1 rounded text-xs">
                      +{service.capabilities.length - 4} more
                    </span>
                  )}
                </div>
              </div>
            )}

            {service.status === 'active' && (
              <div className="mt-2">
                <div className="flex items-center text-green-600 text-xs">
                  <div className="w-2 h-2 bg-green-500 rounded-full mr-2 animate-pulse" />
                  <span>Ready for requests</span>
                </div>
              </div>
            )}

            {service.status === 'error' && (
              <div className="mt-2">
                <div className="flex items-center text-red-600 text-xs">
                  <AlertCircle className="w-3 h-3 mr-2" />
                  <span>Service unavailable</span>
                </div>
              </div>
            )}
          </motion.div>
        ))}
      </div>

      {services.length === 0 && (
        <div className="text-center py-8 text-gray-500">
          <Activity className="w-12 h-12 mx-auto mb-3 text-gray-300" />
          <p>No services registered</p>
        </div>
      )}
    </div>
  );
};

export default ServiceStatus;
