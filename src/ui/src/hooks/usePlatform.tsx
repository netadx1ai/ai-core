import { createContext, useContext } from 'react';
import type { ReactNode } from 'react';
import { detectPlatform } from '../utils/platform';
import type { PlatformInfo } from '../utils/platform';

const PlatformContext = createContext<PlatformInfo | null>(null);

interface PlatformProviderProps {
  children: ReactNode;
}

export function PlatformProvider({ children }: PlatformProviderProps) {
  const platformInfo = detectPlatform();
  
  return (
    <PlatformContext.Provider value={platformInfo}>
      {children}
    </PlatformContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function usePlatform(): PlatformInfo {
  const context = useContext(PlatformContext);
  if (!context) {
    throw new Error('usePlatform must be used within a PlatformProvider');
  }
  return context;
}