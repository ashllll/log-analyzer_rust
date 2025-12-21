import { useAppStore } from '../stores/appStore';

// Hook that provides the same interface as the original useApp hook
export const useApp = () => {
  const page = useAppStore((state) => state.page);
  const toasts = useAppStore((state) => state.toasts);
  const activeWorkspaceId = useAppStore((state) => state.activeWorkspaceId);
  
  const setPage = useAppStore((state) => state.setPage);
  const addToast = useAppStore((state) => state.addToast);
  const removeToast = useAppStore((state) => state.removeToast);
  const setActiveWorkspace = useAppStore((state) => state.setActiveWorkspace);

  return {
    state: {
      page,
      toasts,
      activeWorkspaceId,
    },
    setPage,
    addToast,
    removeToast,
    setActiveWorkspace,
  };
};

// Hook that provides the same interface as the original useWorkspaceState hook
export const useWorkspaceState = () => {
  const workspaces = useAppStore((state) => state.workspaces);
  const loading = useAppStore((state) => state.workspacesLoading);
  const error = useAppStore((state) => state.workspacesError);
  
  const setWorkspaces = useAppStore((state) => state.setWorkspaces);
  const addWorkspace = useAppStore((state) => state.addWorkspace);
  const updateWorkspace = useAppStore((state) => state.updateWorkspace);
  const deleteWorkspace = useAppStore((state) => state.deleteWorkspace);
  const setLoading = useAppStore((state) => state.setWorkspacesLoading);
  const setError = useAppStore((state) => state.setWorkspacesError);

  // Create a dispatch function that mimics the old reducer pattern
  const dispatch = (action: any) => {
    switch (action.type) {
      case 'SET_WORKSPACES':
        setWorkspaces(action.payload);
        break;
      case 'ADD_WORKSPACE':
        addWorkspace(action.payload);
        break;
      case 'UPDATE_WORKSPACE':
        updateWorkspace(action.payload.id, action.payload.updates);
        break;
      case 'DELETE_WORKSPACE':
        deleteWorkspace(action.payload);
        break;
      case 'SET_LOADING':
        setLoading(action.payload);
        break;
      case 'SET_ERROR':
        setError(action.payload);
        break;
      default:
        console.warn('Unknown workspace action:', action.type);
    }
  };

  return {
    state: {
      workspaces,
      loading,
      error,
    },
    dispatch,
  };
};

// Hook that provides the same interface as the original useKeywordState hook
export const useKeywordState = () => {
  const keywordGroups = useAppStore((state) => state.keywordGroups);
  const loading = useAppStore((state) => state.keywordsLoading);
  const error = useAppStore((state) => state.keywordsError);
  
  const setKeywordGroups = useAppStore((state) => state.setKeywordGroups);
  const addKeywordGroup = useAppStore((state) => state.addKeywordGroup);
  const updateKeywordGroup = useAppStore((state) => state.updateKeywordGroup);
  const deleteKeywordGroup = useAppStore((state) => state.deleteKeywordGroup);
  const toggleKeywordGroup = useAppStore((state) => state.toggleKeywordGroup);
  const setLoading = useAppStore((state) => state.setKeywordsLoading);
  const setError = useAppStore((state) => state.setKeywordsError);

  // Create a dispatch function that mimics the old reducer pattern
  const dispatch = (action: any) => {
    switch (action.type) {
      case 'SET_KEYWORD_GROUPS':
        setKeywordGroups(action.payload);
        break;
      case 'ADD_KEYWORD_GROUP':
        addKeywordGroup(action.payload);
        break;
      case 'UPDATE_KEYWORD_GROUP':
        updateKeywordGroup(action.payload);
        break;
      case 'DELETE_KEYWORD_GROUP':
        deleteKeywordGroup(action.payload);
        break;
      case 'TOGGLE_KEYWORD_GROUP':
        toggleKeywordGroup(action.payload);
        break;
      case 'SET_LOADING':
        setLoading(action.payload);
        break;
      case 'SET_ERROR':
        setError(action.payload);
        break;
      default:
        console.warn('Unknown keyword action:', action.type);
    }
  };

  return {
    state: {
      keywordGroups,
      loading,
      error,
    },
    dispatch,
  };
};

// Hook that provides the same interface as the original useTaskState hook
export const useTaskState = () => {
  const tasks = useAppStore((state) => state.tasks);
  const loading = useAppStore((state) => state.tasksLoading);
  const error = useAppStore((state) => state.tasksError);
  
  const setTasks = useAppStore((state) => state.setTasks);
  const addTaskIfNotExists = useAppStore((state) => state.addTaskIfNotExists);
  const updateTask = useAppStore((state) => state.updateTask);
  const deleteTask = useAppStore((state) => state.deleteTask);
  const setLoading = useAppStore((state) => state.setTasksLoading);
  const setError = useAppStore((state) => state.setTasksError);

  // Create a dispatch function that mimics the old reducer pattern
  const dispatch = (action: any) => {
    switch (action.type) {
      case 'SET_TASKS':
        setTasks(action.payload);
        break;
      case 'ADD_TASK':
        addTaskIfNotExists(action.payload);
        break;
      case 'UPDATE_TASK':
        updateTask(action.payload.id, action.payload.updates);
        break;
      case 'DELETE_TASK':
        deleteTask(action.payload);
        break;
      case 'SET_LOADING':
        setLoading(action.payload);
        break;
      case 'SET_ERROR':
        setError(action.payload);
        break;
      default:
        console.warn('Unknown task action:', action.type);
    }
  };

  return {
    state: {
      tasks,
      loading,
      error,
    },
    dispatch,
  };
};