import { ReactNode } from 'react';
import { QueryProvider } from './QueryProvider';
import { useConfigQuery } from '../hooks/useServerQueries';
import { useConfigManager } from '../hooks/useConfigManager';
import { EventManager } from '../components/EventManager';

interface AppProviderProps {
  children: ReactNode;
}

// Internal component that uses React Query hooks
const AppInitializer = ({ children }: { children: ReactNode }) => {
  // Load initial configuration using React Query
  const { isLoading, error } = useConfigQuery();
  
  // Set up debounced config saving using React patterns
  useConfigManager();

  // Show loading state while config is loading
  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center bg-bg-main text-text-main">
        <div className="text-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary mx-auto mb-4"></div>
          <p>Loading configuration...</p>
        </div>
      </div>
    );
  }

  // Show error state if config loading failed
  if (error) {
    return (
      <div className="flex h-screen items-center justify-center bg-bg-main text-text-main">
        <div className="text-center">
          <p className="text-red-500 mb-4">Failed to load configuration</p>
          <p className="text-sm text-text-muted">{String(error)}</p>
        </div>
      </div>
    );
  }

  return (
    <>
      <EventManager />
      {children}
    </>
  );
};

export const AppProvider = ({ children }: AppProviderProps) => {
  return (
    <QueryProvider>
      <AppInitializer>
        {children}
      </AppInitializer>
    </QueryProvider>
  );
};