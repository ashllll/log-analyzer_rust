import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save } from '@tauri-apps/plugin-dialog';
import { useVirtualizer } from "@tanstack/react-virtual";
import { 
  Search, LayoutGrid, ListTodo, Settings, Layers, 
  CheckCircle2, AlertCircle, X, Plus, 
  RefreshCw, Trash2, Zap, Filter,
  ChevronDown, Hash, Copy, Info, Loader2, FileText, Edit2, Download, Eye, EyeOff, RotateCcw
} from "lucide-react";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

// 导入全局Context和Hooks
import { AppProvider, useApp, Toast, ToastType, Workspace, Task, KeywordGroup, KeywordPattern, ColorKey } from './contexts/AppContext';
import { useWorkspaceOperations } from './hooks/useWorkspaceOperations';
import { useTaskManager } from './hooks/useTaskManager';
import { useKeywordManager } from './hooks/useKeywordManager';

// 导入结构化查询模块
import { SearchQueryBuilder } from './services/SearchQueryBuilder';
// import { queryApi } from './services/queryApi';  // 保留以备将来使用
import { SearchQuery } from './types/search';
import { saveQuery, loadQuery } from './services/queryStorage';

function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)); }

// 错误处理工具 - 供内部使用
// @ts-ignore - used internally
class ErrorHandler {
  private static errorMap: Record<string, { message: string; suggestion: string }> = {
    'Path canonicalization failed': { message: '路径无效或不存在', suggestion: '检查路径是否正确' },
    'Failed to lock': { message: '资源正在使用中', suggestion: '稍后重试' },
    'unrar command not found': { message: 'RAR支持未安装', suggestion: '查看安装指南' },
    'Invalid Regex': { message: '搜索表达式语法错误', suggestion: '检查正则表达式格式' },
    'Disk space': { message: '磁盘空间不足', suggestion: '清理磁盘空间后重试' },
    'Path does not exist': { message: '路径不存在', suggestion: '选择有效的文件或目录' },
    'Workspace ID cannot be empty': { message: '工作区 ID 不能为空', suggestion: '请选择一个工作区' },
    'Search query cannot be empty': { message: '搜索查询不能为空', suggestion: '输入搜索关键词' },
  };

  static handle(error: any): string {
    const errorStr = String(error);
    logger.error('Error occurred:', errorStr);
    
    // 匹配错误模式
    for (const [pattern, info] of Object.entries(this.errorMap)) {
      if (errorStr.includes(pattern)) {
        return `${info.message} - ${info.suggestion}`;
      }
    }
    
    // 默认错误消息
    if (errorStr.length > 100) {
      return '操作失败，请查看控制台详情';
    }
    return errorStr;
  }
  
  static isRetryable(error: any): boolean {
    const errorStr = String(error);
    return errorStr.includes('Failed to lock') || 
           errorStr.includes('Resource busy') ||
           errorStr.includes('timeout');
  }
}

// 统一日志工具
const logger = {
  debug: (message: string, ...args: any[]) => {
    if (import.meta.env.DEV) {
      console.log(`[DEBUG] ${message}`, ...args);
    }
  },
  info: (message: string, ...args: any[]) => {
    console.log(`[INFO] ${message}`, ...args);
  },
  error: (message: string, ...args: any[]) => {
    console.error(`[ERROR] ${message}`, ...args);
  }
};

// 本地类型定义
type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'active' | 'icon';
type LucideIcon = React.ComponentType<{ size?: number; className?: string }>;

// 高级过滤器类型
interface FilterOptions {
  timeRange: { start: string | null; end: string | null };
  levels: string[];
  filePattern: string;
}

// 性能指标类型
interface PerformanceStats {
  memoryUsed: number;
  pathMapSize: number;
  cacheSize: number;
  lastSearchDuration: number;
  cacheHitRate: number;
  indexedFilesCount: number;
  indexFileSizeMb: number;
}

interface LogEntry { id: number; timestamp: string; level: string; file: string; line: number; content: string; tags: any[]; real_path?: string; }

// Component Props Types
interface NavItemProps { icon: LucideIcon; label: string; active: boolean; onClick: () => void; }
interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> { 
    children?: React.ReactNode; 
    variant?: ButtonVariant; 
    icon?: LucideIcon;
}
interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
    ref?: React.Ref<HTMLInputElement | null>;
}
interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
    children: React.ReactNode;
}
interface KeywordModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSave: (group: KeywordGroup) => void;
    initialData?: KeywordGroup | null;
}
interface HybridLogRendererProps {
    text: string;
    query: string;
    keywordGroups: KeywordGroup[];
}
interface FilterPaletteProps {
    isOpen: boolean;
    onClose: () => void;
    groups: KeywordGroup[];
    currentQuery: string;
    onToggleRule: (regex: string) => void;
}
interface SearchPageProps {
    keywordGroups: KeywordGroup[];
    addToast: (type: ToastType, message: string) => void;
    searchInputRef: React.RefObject<HTMLInputElement | null>;
    activeWorkspace: Workspace | null;
}
// Component Props Types (legacy - kept for future use)
// @ts-ignore
interface KeywordsPageProps {
    keywordGroups: KeywordGroup[];
    setKeywordGroups: React.Dispatch<React.SetStateAction<KeywordGroup[]>>;
    addToast: (type: ToastType, message: string) => void;
}
// @ts-ignore
interface WorkspacesPageProps {
    workspaces: Workspace[];
    setWorkspaces: React.Dispatch<React.SetStateAction<Workspace[]>>;
    addToast: (type: ToastType, message: string) => void;
    setActiveWorkspaceId: (id: string | null) => void;
    activeWorkspaceId: string | null;
    setTasks: React.Dispatch<React.SetStateAction<Task[]>>;
}

// Color System
const COLOR_STYLES: Record<ColorKey, any> = {
  blue: { dot: "bg-blue-500", badge: "bg-blue-500/15 text-blue-400 border-blue-500/20", border: "border-blue-500", text: "text-blue-400", activeBtn: "bg-blue-500 text-white border-blue-400 shadow-[0_0_10px_rgba(59,130,246,0.4)]", hoverBorder: "hover:border-blue-500/50", highlight: "bg-blue-500/20 text-blue-300 border-blue-500/30" },
  green: { dot: "bg-emerald-500", badge: "bg-emerald-500/15 text-emerald-400 border-emerald-500/20", border: "border-emerald-500", text: "text-emerald-400", activeBtn: "bg-emerald-500 text-white border-emerald-400 shadow-[0_0_10px_rgba(16,185,129,0.4)]", hoverBorder: "hover:border-emerald-500/50", highlight: "bg-emerald-500/20 text-emerald-300 border-emerald-500/30" },
  red: { dot: "bg-red-500", badge: "bg-red-500/15 text-red-400 border-red-500/20", border: "border-red-500", text: "text-red-400", activeBtn: "bg-red-500 text-white border-red-400 shadow-[0_0_10px_rgba(239,68,68,0.4)]", hoverBorder: "hover:border-red-500/50", highlight: "bg-red-500/20 text-red-300 border-red-500/30" },
  orange: { dot: "bg-amber-500", badge: "bg-amber-500/15 text-amber-400 border-amber-500/20", border: "border-amber-500", text: "text-amber-400", activeBtn: "bg-amber-500 text-white border-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.4)]", hoverBorder: "hover:border-amber-500/50", highlight: "bg-amber-500/20 text-amber-300 border-amber-500/30" },
  purple: { dot: "bg-purple-500", badge: "bg-purple-500/15 text-purple-400 border-purple-500/20", border: "border-purple-500", text: "text-purple-400", activeBtn: "bg-purple-500 text-white border-purple-400 shadow-[0_0_10px_rgba(168,85,247,0.4)]", hoverBorder: "hover:border-purple-500/50", highlight: "bg-purple-500/20 text-purple-300 border-purple-500/30" }
};

// UI Components
const NavItem = ({ icon: Icon, label, active, onClick }: NavItemProps) => (
  <button 
    onClick={onClick}
    className={cn(
      "w-full flex items-center gap-3 px-3 py-2 rounded-md transition-all",
      active ? "bg-primary text-white" : "text-text-muted hover:bg-bg-hover"
    )}
  >
    <Icon size={18}/> {label}
  </button>
);

const Button = ({ children, variant = 'primary', className, icon: Icon, onClick, ...props }: ButtonProps) => {
  const variants = {
    primary: "bg-primary hover:bg-primary-hover text-white shadow-sm active:scale-95",
    secondary: "bg-bg-card hover:bg-bg-hover text-text-main border border-border-base active:scale-95",
    ghost: "hover:bg-bg-hover text-text-muted hover:text-text-main active:bg-bg-hover/80",
    danger: "bg-red-500/10 text-red-400 hover:bg-red-500/20 border border-red-500/20 hover:text-red-300 active:scale-95",
    active: "bg-primary/20 text-primary border border-primary/50", 
    icon: "h-8 w-8 p-0 bg-transparent hover:bg-bg-hover text-text-dim hover:text-text-main rounded-full"
  };
  return <button type="button" className={cn("h-9 px-4 rounded-md text-sm font-medium transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed shrink-0 select-none cursor-pointer", variants[variant as keyof typeof variants], className)} onClick={(e) => { e.stopPropagation(); onClick && onClick(e); }} {...props}>{Icon && <Icon size={16} />}{children}</button>;
};
const Input = ({ className, ref, ...props }: InputProps) => (<input ref={ref} className={cn("h-9 w-full bg-bg-main border border-border-base rounded-md px-3 text-sm text-text-main placeholder:text-text-dim focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/50 transition-all", className)} {...props} />);
const Card = ({ children, className, ...props }: CardProps) => (<div className={cn("bg-bg-card border border-border-base rounded-lg overflow-hidden", className)} {...props}>{children}</div>);
const ToastContainer = ({ toasts, removeToast }: { toasts: Toast[], removeToast: (id: number) => void }) => (<div className="fixed bottom-6 right-6 z-[100] flex flex-col gap-3 pointer-events-none">{toasts.map(toast => (<div key={toast.id} className={cn("pointer-events-auto min-w-[300px] p-4 rounded-lg shadow-2xl border flex items-center gap-3 animate-in slide-in-from-right-full duration-300", toast.type === 'success' ? "bg-bg-card border-emerald-500/30 text-emerald-400" : toast.type === 'error' ? "bg-bg-card border-red-500/30 text-red-400" : "bg-bg-card border-blue-500/30 text-blue-400")}>{toast.type === 'success' ? <CheckCircle2 size={20}/> : toast.type === 'error' ? <AlertCircle size={20}/> : <Info size={20}/>}<span className="text-sm font-medium text-text-main">{toast.message}</span><button onClick={() => removeToast(toast.id)} className="ml-auto text-text-dim hover:text-text-main"><X size={16}/></button></div>))}</div>);

// Components: KeywordModal, HybridLogRenderer, FilterPalette (Assuming they are defined as in previous steps)
// --- Keyword Modal ---
const KeywordModal = ({ isOpen, onClose, onSave, initialData }: KeywordModalProps) => {
  const [name, setName] = useState(initialData?.name || "");
  const [color, setColor] = useState<ColorKey>(initialData?.color || "blue");
  const [patterns, setPatterns] = useState<KeywordPattern[]>(initialData?.patterns || [{ regex: "", comment: "" }]);
  useEffect(() => { if (isOpen) { setName(initialData?.name || ""); setColor(initialData?.color || "blue"); setPatterns(initialData?.patterns || [{ regex: "", comment: "" }]); } }, [isOpen, initialData]);
  const handleSave = () => {
    const validPatterns = patterns.filter(p => p.regex.trim() !== "");
    if (!name || validPatterns.length === 0) return;
    onSave({ id: initialData?.id || Date.now().toString(), name, color, patterns: validPatterns, enabled: true });
    onClose();
  };
  if (!isOpen) return null;
  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div className="w-[600px] bg-bg-card border border-border-base rounded-lg shadow-2xl flex flex-col max-h-[85vh] animate-in fade-in zoom-in-95 duration-200" onClick={e => e.stopPropagation()}>
        <div className="px-6 py-4 border-b border-border-base flex justify-between items-center bg-bg-sidebar"><h2 className="text-lg font-bold text-text-main">{initialData ? 'Edit Keyword Group' : 'New Keyword Group'}</h2><Button variant="icon" icon={X} onClick={onClose} /></div>
        <div className="p-6 overflow-y-auto flex-1 space-y-6">
          <div className="grid grid-cols-2 gap-4">
            <div><label className="text-xs text-text-dim uppercase font-bold mb-1.5 block">Group Name</label><Input value={name} onChange={(e:any) => setName(e.target.value)} placeholder="Name" /></div>
            <div><label className="text-xs text-text-dim uppercase font-bold mb-1.5 block">Highlight Color</label><div className="flex gap-2 h-9 items-center">{(['blue', 'green', 'orange', 'red', 'purple'] as ColorKey[]).map((c) => (<button key={c} onClick={() => setColor(c)} className={cn("w-6 h-6 rounded-full border-2 transition-all cursor-pointer", COLOR_STYLES[c].dot, color === c ? "border-white scale-110 shadow-lg" : "border-transparent opacity-40 hover:opacity-100")} />))}</div></div>
          </div>
          <div>
            <div className="flex justify-between items-center mb-2"><label className="text-xs text-text-dim uppercase font-bold">Patterns & Comments</label><Button variant="ghost" className="h-6 text-xs" icon={Plus} onClick={() => setPatterns([...patterns, { regex: "", comment: "" }])}>Add</Button></div>
            <div className="space-y-2">{patterns.map((p, i) => (<div key={i} className="flex gap-2 items-center group"><div className="flex-1"><Input value={p.regex} onChange={(e:any) => { const n = [...patterns]; n[i].regex = e.target.value; setPatterns(n); }} placeholder="RegEx" className="font-mono text-xs"/></div><div className="flex-1"><Input value={p.comment} onChange={(e:any) => { const n = [...patterns]; n[i].comment = e.target.value; setPatterns(n); }} placeholder="Comment" className="text-xs"/></div><Button variant="icon" icon={Trash2} className="text-red-400 opacity-0 group-hover:opacity-100 transition-opacity" onClick={() => setPatterns(patterns.filter((_, idx) => idx !== i))} /></div>))}</div>
          </div>
        </div>
        <div className="px-6 py-4 border-t border-border-base bg-bg-sidebar flex justify-end gap-3"><Button variant="secondary" onClick={onClose}>Cancel</Button><Button onClick={handleSave}>Save Configuration</Button></div>
      </div>
    </div>
  );
};
const HybridLogRenderer = ({ text, query, keywordGroups }: HybridLogRendererProps) => {
  const { patternMap, regexPattern } = useMemo(() => {
    const map = new Map(); const patterns = new Set();
    keywordGroups.filter((g:any) => g.enabled).forEach((group:any) => { group.patterns.forEach((p:any) => { if (p.regex?.trim()) { map.set(p.regex.toLowerCase(), { color: group.color, comment: p.comment }); patterns.add(p.regex); } }); });
    if (query) { query.split('|').map((t:string) => t.trim()).filter((t:string) => t.length > 0).forEach((term:string, index:number) => { if (!map.has(term.toLowerCase())) { map.set(term.toLowerCase(), { color: ['blue', 'purple', 'green', 'orange'][index % 4], comment: "" }); } patterns.add(term); }); }
    const sorted = Array.from(patterns).sort((a:any, b:any) => b.length - a.length);
    return { regexPattern: sorted.length > 0 ? new RegExp(`(${sorted.map((p:any) => p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})`, 'gi') : null, patternMap: map };
  }, [keywordGroups, query]);
  
  // 性能优化：文本长度检查
  if (text.length > 500) {
    // 超长文本截断显示
    const truncated = text.substring(0, 500) + '...';
    if (!regexPattern) return <span>{truncated}</span>;
    const parts = truncated.split(regexPattern);
    // 仅高亮可见部分
    return <span>{parts.map((part:string, i:number) => { 
      const info = patternMap.get(part.toLowerCase()); 
      if (info) { 
        const style = COLOR_STYLES[info.color as ColorKey]?.highlight || COLOR_STYLES['blue'].highlight; 
        return <span key={i} className={cn("rounded-[2px] px-1 border font-bold break-all", style)}>{part}</span>;
      } 
      return <span key={i}>{part}</span>; 
    })}<span className="text-text-dim ml-1">[truncated]</span></span>;
  }
  
  if (!regexPattern) return <span>{text}</span>;
  
  const parts = text.split(regexPattern);
  
  // 性能优化：匹配数量检查
  const matchCount = parts.filter(part => patternMap.has(part.toLowerCase())).length;
  if (matchCount > 20) {
    // 降级为纯文本+提示
    return <span className="text-text-dim">{text} <span className="text-[10px] bg-red-500/20 text-red-400 px-1 py-0.5 rounded border border-red-500/30 ml-1">[{matchCount} matches - rendering disabled for performance]</span></span>;
  }
  
  return <span>{parts.map((part:string, i:number) => { 
    const info = patternMap.get(part.toLowerCase()); 
    if (info) { 
      const style = COLOR_STYLES[info.color as ColorKey]?.highlight || COLOR_STYLES['blue'].highlight; 
      return <span key={i} className="inline-flex items-baseline mx-[1px]"><span className={cn("rounded-[2px] px-1 border font-bold break-all", style)}>{part}</span>{info.comment && <span className={cn("ml-1 px-1.5 rounded-[2px] text-[10px] font-normal border select-none whitespace-nowrap transform -translate-y-[1px]", style.replace("bg-", "bg-opacity-10 bg-"))}>{info.comment}</span>}</span>; 
    } 
    return <span key={i}>{part}</span>; 
  })}</span>;
};
const FilterPalette = ({ isOpen, onClose, groups, currentQuery, onToggleRule }: FilterPaletteProps) => {
    if (!isOpen) return null;
    const isPatternActive = (regex: string) => currentQuery.split('|').map((t:string) => t.trim().toLowerCase()).includes(regex.toLowerCase());
    const colorOrder: ColorKey[] = ['red', 'orange', 'blue', 'purple', 'green'];
    return (<><div className="fixed inset-0 z-[45] bg-transparent" onClick={onClose}></div><div className="absolute top-full right-0 mt-2 w-[600px] max-h-[60vh] overflow-y-auto bg-[#18181b] border border-border-base rounded-lg shadow-2xl z-50 p-4 grid gap-6 animate-in fade-in zoom-in-95 duration-100 origin-top-right ring-1 ring-white/10"><div className="flex justify-between items-center pb-2 border-b border-white/10"><h3 className="text-sm font-bold text-text-main flex items-center gap-2"><Filter size={14} className="text-primary"/> Filter Command Center</h3></div>{colorOrder.map(color => { const colorGroups = groups.filter((g: KeywordGroup) => g.color === color); if (colorGroups.length === 0) return null; return (<div key={color}><div className={cn("text-[10px] font-bold uppercase mb-2 flex items-center gap-2", COLOR_STYLES[color].text)}><div className={cn("w-2 h-2 rounded-full", COLOR_STYLES[color].dot)}></div>{color} Priority Level</div><div className="grid grid-cols-2 gap-3">{colorGroups.map((group: KeywordGroup) => (<div key={group.id} className="bg-bg-card/50 border border-white/5 rounded p-2"><div className="text-xs font-semibold text-text-muted mb-2 px-1">{group.name}</div><div className="flex flex-wrap gap-2">{group.patterns.map((p, idx) => { const active = isPatternActive(p.regex); return <button key={idx} onClick={() => onToggleRule(p.regex)} className={cn("text-[11px] px-2 py-1 rounded border transition-all duration-150 flex items-center gap-1.5 cursor-pointer", active ? COLOR_STYLES[color].activeBtn : `bg-bg-main text-text-dim border-border-base hover:bg-bg-hover ${COLOR_STYLES[color].hoverBorder}`)}>{active && <CheckCircle2 size={10} />}<span className="font-mono">{p.regex}</span></button> })}</div></div>))}</div></div>) })}</div></>);
};

// --- Search Page ---
const SearchPage = ({ keywordGroups, addToast, searchInputRef, activeWorkspace }: SearchPageProps) => {
  const [query, setQuery] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isFilterPaletteOpen, setIsFilterPaletteOpen] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  
  // 结构化查询状态
  const [currentQuery, setCurrentQuery] = useState<SearchQuery | null>(null);
  
  // 高级过滤状态
  const [filterOptions, setFilterOptions] = useState<FilterOptions>({
    timeRange: { start: null, end: null },
    levels: [],
    filePattern: ""
  });
  
  const parentRef = useRef<HTMLDivElement>(null);

  // ResizeObserver优化：监听容器尺寸变化，即时更新虚拟滚动
  useEffect(() => {
    if (!parentRef.current) return;
    
    const resizeObserver = new ResizeObserver(() => {
      // 当容器尺寸变化时，虚拟滚动会自动重新计算
      // 这里无需额外操作，useVirtualizer会自动响应
    });
    
    resizeObserver.observe(parentRef.current);
    
    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  useEffect(() => {
    const unlistenResults = listen<LogEntry[]>('search-results', (e) => setLogs(prev => [...prev, ...e.payload]));
    const unlistenComplete = listen('search-complete', (e) => { setIsSearching(false); addToast('success', `Found ${e.payload} logs.`); });
    const unlistenError = listen('search-error', (e) => { setIsSearching(false); addToast('error', `${e.payload}`); });
    return () => { unlistenResults.then(f => f()); unlistenComplete.then(f => f()); unlistenError.then(f => f()); }
  }, [addToast]);

  // 加载保存的查询
  useEffect(() => {
    const saved = loadQuery();
    if (saved) {
      setCurrentQuery(saved);
      const builder = SearchQueryBuilder.import(JSON.stringify(saved));
      setQuery(builder.toQueryString());
    }
  }, []);

  // 自动保存查询变化
  useEffect(() => {
    if (currentQuery) {
      saveQuery(currentQuery);
    }
  }, [currentQuery]);

  const handleSearch = async () => {
    if (!activeWorkspace) return addToast('error', 'Select a workspace first.');
    setLogs([]); setIsSearching(true);
    try { 
      // 构建过滤器对象
      const filters = {
        time_start: filterOptions.timeRange.start,
        time_end: filterOptions.timeRange.end,
        levels: filterOptions.levels,
        file_pattern: filterOptions.filePattern || null
      };
      
      await invoke("search_logs", { 
        query, 
        searchPath: activeWorkspace.path,
        filters: filters
      }); 
      
      // 如果使用了结构化查询，更新执行次数
      if (currentQuery) {
        currentQuery.metadata.executionCount += 1;
        setCurrentQuery({...currentQuery});
      }
    } catch(_e) { setIsSearching(false); }
  };
  
  // 重置过滤器
  const handleResetFilters = () => {
    setFilterOptions({
      timeRange: { start: null, end: null },
      levels: [],
      filePattern: ""
    });
    addToast('info', '过滤器已重置');
  };

  // 删除单个关键词
  const removeTermFromQuery = useCallback((termToRemove: string) => {
    const terms = query.split('|').map(t => t.trim()).filter(t => t.length > 0);
    const newTerms = terms.filter(t => t.toLowerCase() !== termToRemove.toLowerCase());
    setQuery(newTerms.join('|'));
    
    // 同时更新结构化查询
    if (currentQuery) {
      const builder = SearchQueryBuilder.import(JSON.stringify(currentQuery));
      const existing = builder.findTermByValue(termToRemove);
      if (existing) {
        builder.removeTerm(existing.id);
        setCurrentQuery(builder.getQuery());
      }
    }
  }, [query, currentQuery]);

  // | 仅作为分隔符，多个关键词用 OR 逻辑组合（匹配任意一个）
  const toggleRuleInQuery = useCallback((ruleRegex: string) => {
    // 创建或更新查询构建器
    const builder = currentQuery 
      ? SearchQueryBuilder.import(JSON.stringify(currentQuery))
      : SearchQueryBuilder.fromString(query, keywordGroups);

    // 检查是否已存在
    const existing = builder.findTermByValue(ruleRegex);

    if (existing) {
      // 已存在：切换启用状态
      builder.toggleTerm(existing.id);
    } else {
      // 不存在：添加新项
      builder.addTerm(ruleRegex, {
        source: 'preset',
        isRegex: true,
        operator: 'AND'
      });
    }

    // 验证查询
    const validation = builder.validate();
    if (!validation.isValid) {
      const errors = validation.issues
        .filter(i => i.severity === 'error')
        .map(i => i.message)
        .join(', ');
      console.error('Query validation failed:', errors);
      addToast('error', `查询验证失败: ${errors}`);
      return;
    }

    // 更新状态
    const newQuery = builder.getQuery();
    setCurrentQuery(newQuery);
    
    // 更新查询字符串（用于显示）
    const queryString = builder.toQueryString();
    setQuery(queryString);
  }, [query, keywordGroups, currentQuery, addToast]);

  const copyToClipboard = (text: string) => { navigator.clipboard.writeText(text).then(() => addToast('success', 'Copied')); };
  const tryFormatJSON = (content: string) => { try { const start = content.indexOf('{'); if (start === -1) return content; const jsonPart = content.substring(start); const obj = JSON.parse(jsonPart); return JSON.stringify(obj, null, 2); } catch (_e) { return content; } };
  
  // 导出功能
  const handleExport = async (format: 'csv' | 'json') => {
    if (logs.length === 0) {
      addToast('error', '没有可导出的数据');
      return;
    }

    try {
      const defaultPath = `log-export-${Date.now()}.${format}`;
      const savePath = await save({
        defaultPath,
        filters: [{
          name: format.toUpperCase(),
          extensions: [format]
        }]
      });

      if (!savePath) {
        // 用户取消
        return;
      }

      logger.debug('Exporting to:', savePath);
      await invoke('export_results', {
        results: logs,
        format,
        savePath
      });

      addToast('success', `已导出 ${logs.length} 条日志到 ${format.toUpperCase()}`);
    } catch (e) {
      logger.error('Export error:', e);
      addToast('error', `导出失败: ${e}`);
    }
  };
  
  // 优化：动态高度估算 - 提高显示密度，添加ResizeObserver支持
  const rowVirtualizer = useVirtualizer({ 
    count: logs.length, 
    getScrollElement: () => parentRef.current, 
    estimateSize: useCallback((index: number) => {
      const log = logs[index];
      if (!log) return 28;  // 减小最小高度从 46px 到 28px
      // 根据内容长度估算高度，使用更紧凑的行高
      const lines = Math.ceil(log.content.length / 140);
      return Math.max(28, Math.min(lines * 16, 120));  // 最小 28px，最大 120px，行高从 22px 减到 16px
    }, [logs]),
    overscan: 25,  // 增加 overscan 以保证流畅滚动
    measureElement: (element) => element?.getBoundingClientRect().height || 28,
    // 启用动态测量以响应尺寸变化
    enabled: true
  });
  
  const activeLog = logs.find(l => l.id === selectedId);

  return (
    <div className="flex flex-col h-full relative">
      <div className="p-4 border-b border-border-base bg-bg-sidebar space-y-3 shrink-0 relative z-40">
        <div className="flex gap-2">
          <div className="relative flex-1"><Search className="absolute left-3 top-2.5 text-text-dim" size={16} /><Input ref={searchInputRef} value={query} onChange={(e: any) => {
            // 规范化输入：移除 | 前后的空格
            const normalized = e.target.value.replace(/\s*\|\s*/g, '|');
            setQuery(normalized);
          }} className="pl-9 font-mono bg-bg-main" placeholder="Search keywords separated by | ..." onKeyDown={(e:any) => e.key === 'Enter' && handleSearch()} /></div>
          <div className="relative"><Button variant={isFilterPaletteOpen ? "active" : "secondary"} icon={Filter} onClick={() => setIsFilterPaletteOpen(!isFilterPaletteOpen)} className="w-[140px] justify-between">Filters <ChevronDown size={14} className={cn("transition-transform", isFilterPaletteOpen ? "rotate-180" : "")}/></Button><FilterPalette isOpen={isFilterPaletteOpen} onClose={() => setIsFilterPaletteOpen(false)} groups={keywordGroups} currentQuery={query} onToggleRule={toggleRuleInQuery} /></div>
          <Button icon={Download} onClick={() => handleExport('csv')} disabled={logs.length === 0} variant="secondary" title="Export to CSV">CSV</Button>
          <Button icon={Download} onClick={() => handleExport('json')} disabled={logs.length === 0} variant="secondary" title="Export to JSON">JSON</Button>
          <Button icon={isSearching ? Loader2 : Search} onClick={handleSearch} disabled={isSearching} className={isSearching ? "animate-pulse" : ""}>{isSearching ? '...' : 'Search'}</Button>
        </div>
        
        {/* 高级过滤器 UI */}
        <div className="flex items-center gap-2 mb-2">
          <span className="text-[10px] font-bold text-text-dim uppercase">Advanced Filters</span>
          {(filterOptions.levels.length > 0 || filterOptions.timeRange.start || filterOptions.timeRange.end || filterOptions.filePattern) && (
            <>
              <span className="text-[10px] bg-primary/20 text-primary px-1.5 py-0.5 rounded border border-primary/30">
                {[
                  filterOptions.levels.length > 0 && `${filterOptions.levels.length} levels`,
                  filterOptions.timeRange.start && 'time range',
                  filterOptions.filePattern && 'file pattern'
                ].filter(Boolean).join(', ')}
              </span>
              <Button variant="ghost" onClick={handleResetFilters} className="h-5 text-[10px] px-2" icon={RotateCcw}>Reset</Button>
            </>
          )}
        </div>
        <div className="grid grid-cols-4 gap-2">
          {/* 日志级别过滤 */}
          <div className="col-span-1">
            <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">Level</label>
            <div className="flex gap-1">
              {['ERROR', 'WARN', 'INFO', 'DEBUG'].map((level) => (
                <button
                  key={level}
                  onClick={() => {
                    setFilterOptions(prev => ({
                      ...prev,
                      levels: prev.levels.includes(level) 
                        ? prev.levels.filter(l => l !== level)
                        : [...prev.levels, level]
                    }));
                  }}
                  className={cn(
                    "text-[10px] px-2 py-1 rounded border transition-all",
                    filterOptions.levels.includes(level)
                      ? "bg-primary text-white border-primary"
                      : "bg-bg-main text-text-dim border-border-base hover:border-primary/50"
                  )}
                  title={level}
                >
                  {level.substring(0, 1)}
                </button>
              ))}
            </div>
          </div>
          
          {/* 时间范围过滤 */}
          <div className="col-span-2">
            <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">Time Range</label>
            <div className="flex gap-1">
              <input
                type="datetime-local"
                value={filterOptions.timeRange.start || ""}
                onChange={(e) => setFilterOptions(prev => ({
                  ...prev,
                  timeRange: { ...prev.timeRange, start: e.target.value || null }
                }))}
                className="h-7 text-[11px] flex-1 bg-bg-main border border-border-base rounded px-2 text-text-main focus:outline-none focus:border-primary/50"
                placeholder="Start"
              />
              <span className="text-text-dim self-center">~</span>
              <input
                type="datetime-local"
                value={filterOptions.timeRange.end || ""}
                onChange={(e) => setFilterOptions(prev => ({
                  ...prev,
                  timeRange: { ...prev.timeRange, end: e.target.value || null }
                }))}
                className="h-7 text-[11px] flex-1 bg-bg-main border border-border-base rounded px-2 text-text-main focus:outline-none focus:border-primary/50"
                placeholder="End"
              />
            </div>
          </div>
          
          {/* 文件来源过滤 */}
          <div className="col-span-1">
            <label className="text-[10px] text-text-dim uppercase font-bold mb-1 block">File Pattern</label>
            <Input
              value={filterOptions.filePattern}
              onChange={(e: any) => setFilterOptions(prev => ({ ...prev, filePattern: e.target.value }))}
              className="h-7 text-[11px]"
              placeholder="e.g. error.log"
            />
          </div>
        </div>
        
        <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none h-6 min-h-[24px]"><span className="text-[10px] font-bold text-text-dim uppercase">Active:</span>{query ? query.split('|').map((term:string, i:number) => {
          const trimmedTerm = term.trim();
          return (
            <span key={i} className="flex items-center text-[10px] bg-bg-card border border-border-base px-1.5 py-0.5 rounded text-text-main whitespace-nowrap group gap-1">
              <Hash size={8} className="mr-0.5 opacity-50"/> 
              {trimmedTerm}
              <button 
                onClick={() => removeTermFromQuery(trimmedTerm)} 
                className="opacity-0 group-hover:opacity-100 hover:text-red-400 transition-all ml-0.5"
                title="Remove keyword"
              >
                <X size={10} />
              </button>
            </span>
          );
        }) : <span className="text-[10px] text-text-dim italic">None</span>}</div>
      </div>
      <div className="flex-1 flex overflow-hidden">
        <div ref={parentRef} className="flex-1 overflow-auto bg-bg-main scrollbar-thin">
          <div className="sticky top-0 z-10 grid grid-cols-[50px_160px_200px_1fr] px-3 py-1.5 bg-bg-main border-b border-border-base text-[10px] font-bold text-text-dim uppercase tracking-wider"><div>Lvl</div> <div>Time</div> <div>File</div> <div>Content</div></div>
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const log = logs[virtualRow.index];
              const isActive = log.id === selectedId;
              return (
                <div key={virtualRow.key} data-index={virtualRow.index} ref={rowVirtualizer.measureElement} onClick={() => setSelectedId(log.id)} style={{ transform: `translateY(${virtualRow.start}px)` }} className={cn("absolute top-0 left-0 w-full grid grid-cols-[50px_160px_200px_1fr] px-3 py-0.5 border-b border-border-base/40 cursor-pointer text-[11px] font-mono hover:bg-bg-hover transition-colors items-start", isActive && "bg-blue-500/10 border-l-2 border-l-primary")}>
                  <div className={cn("font-bold", log.level === 'ERROR' ? 'text-red-400' : log.level === 'WARN' ? 'text-yellow-400' : 'text-blue-400')}>{log.level.substring(0,1)}</div>
                  <div className="text-text-muted whitespace-nowrap text-[10px]">{log.timestamp}</div>
                  <div className="text-text-muted break-all pr-2 text-[10px] leading-tight">{log.file}:{log.line}</div>
                  <div className="text-text-main whitespace-pre-wrap break-all leading-tight pr-2"><HybridLogRenderer text={log.content} query={query} keywordGroups={keywordGroups} /></div>
                </div>
              );
            })}
          </div>
          {logs.length === 0 && !isSearching && <div className="flex items-center justify-center h-full text-text-dim">No logs found. Select workspace & search.</div>}
        </div>
        {activeLog && (
          <div className="w-[450px] bg-bg-sidebar border-l border-border-base flex flex-col shrink-0 shadow-xl z-20 animate-in slide-in-from-right duration-200">
            <div className="h-10 border-b border-border-base flex items-center justify-between px-4 bg-bg-card/50">
              <span className="text-xs font-semibold text-text-muted uppercase">Log Inspector</span>
              <div className="flex gap-1"><Button variant="ghost" className="h-6 w-6 p-0" onClick={() => copyToClipboard(activeLog.content)}><Copy size={14}/></Button><Button variant="ghost" className="h-6 w-6 p-0" onClick={() => setSelectedId(null)}><X size={14}/></Button></div>
            </div>
            <div className="flex-1 overflow-auto p-4 font-mono text-xs">
              <div className="bg-bg-main p-3 rounded border border-border-base mb-4"><div className="text-text-dim text-[10px] uppercase mb-1">Message Body</div><div className="text-text-main whitespace-pre-wrap break-all leading-relaxed"><HybridLogRenderer text={tryFormatJSON(activeLog.content)} query={query} keywordGroups={keywordGroups} /></div></div>
              <div className="p-2 bg-bg-card border border-border-base rounded mb-2"><div className="text-[10px] text-text-dim uppercase">File</div><div className="break-all text-text-main">{activeLog.real_path}</div></div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

const KeywordsPage = () => {
  const { saveKeywordGroup, deleteKeywordGroup, keywordGroups } = useKeywordManager();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingGroup, setEditingGroup] = useState<KeywordGroup | null>(null);
  
  const handleSave = (group: KeywordGroup) => {
    const isEditing = !!editingGroup;
    saveKeywordGroup(group, isEditing);
  };
  
  const handleDelete = (id: string) => { 
    deleteKeywordGroup(id);
  };

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      <div className="flex justify-between items-center mb-6"><h1 className="text-2xl font-bold text-text-main">Keyword Configuration</h1><Button icon={Plus} onClick={() => { setEditingGroup(null); setIsModalOpen(true); }}>New Group</Button></div>
      <div className="space-y-4">
        {keywordGroups.map((group: KeywordGroup) => (
          <Card key={group.id} className="overflow-hidden hover:border-primary/50 transition-colors">
            <div className="px-6 py-4 flex items-center justify-between bg-bg-sidebar/30 border-b border-border-base/50">
              <div className="flex items-center gap-4"><div className={cn("w-3 h-3 rounded-full shadow-[0_0_8px_currentColor]", COLOR_STYLES[group.color].dot)}></div><div><h3 className="text-sm font-bold text-text-main">{group.name}</h3></div></div>
              <div className="flex items-center gap-2"><Button variant="ghost" icon={Edit2} onClick={() => { setEditingGroup(group); setIsModalOpen(true); }}>Edit</Button><Button variant="danger" icon={Trash2} onClick={() => handleDelete(group.id)}>Delete</Button></div>
            </div>
            <div className="px-6 py-3 bg-bg-card flex flex-wrap gap-2">{group.patterns.map((p, i) => (<div key={i} className="flex items-center bg-bg-main border border-border-base rounded px-2 py-1 text-xs"><span className="font-mono text-text-main mr-2">{p.regex}</span>{p.comment && <span className={cn("text-[10px] px-1.5 rounded", COLOR_STYLES[group.color].badge)}>{p.comment}</span>}</div>))}</div>
          </Card>
        ))}
      </div>
      <KeywordModal isOpen={isModalOpen} onClose={() => setIsModalOpen(false)} initialData={editingGroup} onSave={handleSave} />
    </div>
  );
};

const WorkspacesPage = () => {
  const { workspaces, importFile, importFolder, refreshWorkspace, deleteWorkspace, toggleWatch } = useWorkspaceOperations();
  const { state: appState, setActiveWorkspace } = useApp();
  
  const handleDelete = (id: string) => { 
    deleteWorkspace(id); 
  };
  
  const handleToggleWatch = async (ws: Workspace) => {
    await toggleWatch(ws);
  };
  
  const handleRefresh = async (ws: Workspace) => {
    await refreshWorkspace(ws);
  };
  
  const handleImportFile = async () => {
    await importFile();
  };
  
  const handleImportFolder = async () => {
    await importFolder();
  };

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-text-main">Workspaces</h1>
        <div className="flex gap-2">
          <Button icon={FileText} onClick={handleImportFile}>Import File</Button>
          <Button icon={Plus} onClick={handleImportFolder}>Import Folder</Button>
        </div>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
         {workspaces.map((ws: Workspace) => (
           <Card key={ws.id} className={cn("h-full flex flex-col hover:border-primary/50 transition-colors group cursor-pointer", appState.activeWorkspaceId === ws.id ? "border-primary ring-1 ring-primary" : "border-border-base")} onClick={() => setActiveWorkspace(ws.id)}>
              <div className="px-4 py-3 border-b border-border-base bg-bg-sidebar/50 font-bold text-sm flex justify-between items-center">
                  {ws.name}
                  <div className="flex gap-1">
                    <Button 
                      variant="ghost" 
                      icon={ws.watching ? EyeOff : Eye} 
                      className="h-6 w-6 p-0 text-text-dim hover:text-emerald-400" 
                      onClick={(e) => { e.stopPropagation(); handleToggleWatch(ws); }} 
                      title={ws.watching ? "Stop watching" : "Start watching"}
                    />
                    <Button variant="ghost" icon={RefreshCw} className="h-6 w-6 p-0 text-text-dim hover:text-blue-400" onClick={(e) => { e.stopPropagation(); handleRefresh(ws); }} title="Refresh workspace"/>
                    <Button variant="ghost" icon={Trash2} className="h-6 w-6 p-0 text-text-dim hover:text-red-400" onClick={(e) => { e.stopPropagation(); handleDelete(ws.id); }}/>
                  </div>
              </div>
              <div className="p-4 space-y-4">
                 <code className="text-xs bg-bg-main px-2 py-1.5 rounded border border-border-base block truncate font-mono text-text-muted">{ws.path}</code>
                 <div className="flex items-center gap-2 text-xs font-bold">
                   {ws.status === 'READY' ? <><CheckCircle2 size={14} className="text-emerald-500"/> <span className="text-emerald-500">READY</span></> : <><RefreshCw size={14} className="text-blue-500 animate-spin"/> <span className="text-blue-500">PROCESSING</span></>}
                   {ws.watching && <><Eye size={14} className="text-amber-500 ml-2"/> <span className="text-amber-500">WATCHING</span></>}
                 </div>
              </div>
           </Card>
         ))}
      </div>
    </div>
  );
};

const TasksPage = () => {
  const { tasks, deleteTask } = useTaskManager();
  
  const handleDelete = (id: string) => { 
    deleteTask(id);
  };

  return (
    <div className="p-8 max-w-4xl mx-auto h-full overflow-auto">
      <h1 className="text-2xl font-bold mb-6 text-text-main">Background Tasks</h1>
      <div className="space-y-4">
        {tasks.length === 0 && <div className="text-text-dim text-center py-10">No active tasks</div>}
        {tasks.map((t: Task) => (
          <div key={t.id} className="p-4 bg-bg-card border border-border-base rounded-lg flex items-center gap-4 animate-in fade-in slide-in-from-bottom-2">
            <div className={cn("p-2 rounded-full bg-bg-hover", t.status === 'RUNNING' ? "text-blue-500" : t.status === 'FAILED' ? "text-red-500" : "text-emerald-500")}>
              {t.status === 'RUNNING' ? <RefreshCw size={20} className="animate-spin"/> : t.status === 'FAILED' ? <AlertCircle size={20}/> : <CheckCircle2 size={20}/>}
            </div>
            <div className="flex-1 min-w-0">
               <div className="flex justify-between mb-1"><h3 className="font-semibold text-sm text-text-main truncate">{t.type}: {t.target}</h3><span className="text-xs font-mono text-text-dim font-bold">{t.status}</span></div>
               <div className="w-full bg-bg-main h-2 rounded-full overflow-hidden relative">
                  <div className={cn("h-full transition-all duration-500", t.status==='FAILED'?'bg-red-500':t.status==='COMPLETED'?'bg-emerald-500':'bg-blue-500')} style={{width: `${t.progress || 5}%`}}></div>
               </div>
               <div className="flex justify-between mt-1 text-xs text-text-dim">
                  <span className="truncate max-w-[300px]">{t.message}</span>
                  <span>{t.progress}%</span>
               </div>
            </div>
            <div className="flex gap-2">
               <Button variant="ghost" className="h-8 w-8 p-0 text-red-400 hover:text-red-300" onClick={() => handleDelete(t.id)}><Trash2 size={16}/></Button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

// --- Performance Page ---
const PerformancePage = ({ addToast }: { addToast: any }) => {
  const [stats, setStats] = useState<PerformanceStats | null>(null);
  const [loading, setLoading] = useState(false);

  const loadStats = async () => {
    setLoading(true);
    try {
      const metrics = await invoke<any>('get_performance_metrics');
      setStats({
        memoryUsed: metrics.memory_used_mb,
        pathMapSize: metrics.path_map_size,
        cacheSize: metrics.cache_size,
        lastSearchDuration: metrics.last_search_duration_ms,
        cacheHitRate: metrics.cache_hit_rate,
        indexedFilesCount: metrics.indexed_files_count,
        indexFileSizeMb: metrics.index_file_size_mb
      });
    } catch (e) {
      addToast('error', `Failed to load stats: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStats();
    const interval = setInterval(loadStats, 5000); // 每5秒刷新
    return () => clearInterval(interval);
  }, []);

  if (!stats) {
    return <div className="p-10 text-center text-text-dim">Loading performance stats...</div>;
  }

  return (
    <div className="p-8 max-w-6xl mx-auto h-full overflow-auto">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold text-text-main">Performance Monitor</h1>
        <Button icon={RefreshCw} onClick={loadStats} disabled={loading}>
          {loading ? 'Refreshing...' : 'Refresh'}
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {/* 内存使用 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Memory Usage</div>
          <div className="text-3xl font-bold text-primary">
            {stats.memoryUsed > 0 ? `${stats.memoryUsed.toFixed(1)} MB` : 'N/A'}
          </div>
          <div className="text-xs text-text-muted mt-1">进程内存占用</div>
        </Card>

        {/* 索引文件数 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Indexed Files</div>
          <div className="text-3xl font-bold text-emerald-400">
            {stats.indexedFilesCount.toLocaleString()}
          </div>
          <div className="text-xs text-text-muted mt-1">已索引文件数量</div>
        </Card>

        {/* 缓存大小 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Cache Size</div>
          <div className="text-3xl font-bold text-blue-400">
            {stats.cacheSize}
          </div>
          <div className="text-xs text-text-muted mt-1">搜索缓存条目数</div>
        </Card>

        {/* 搜索耗时 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Last Search</div>
          <div className="text-3xl font-bold text-amber-400">
            {stats.lastSearchDuration > 0 ? `${stats.lastSearchDuration} ms` : 'N/A'}
          </div>
          <div className="text-xs text-text-muted mt-1">最近搜索耗时</div>
        </Card>

        {/* 缓存命中率 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Cache Hit Rate</div>
          <div className="text-3xl font-bold text-purple-400">
            {stats.cacheHitRate.toFixed(1)}%
          </div>
          <div className="text-xs text-text-muted mt-1">缓存命中率</div>
        </Card>

        {/* 索引文件大小 */}
        <Card className="p-6">
          <div className="text-xs text-text-dim uppercase font-bold mb-2">Index Size</div>
          <div className="text-3xl font-bold text-red-400">
            {stats.indexFileSizeMb.toFixed(2)} MB
          </div>
          <div className="text-xs text-text-muted mt-1">索引文件磁盘占用</div>
        </Card>
      </div>
    </div>
  );
};

// --- Main App Component (Internal) ---
function AppContent() {
  const { state: appState, setPage, addToast, removeToast } = useApp();
  const { keywordGroups } = useKeywordManager();
  const { workspaces } = useWorkspaceOperations();
  
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [importStatus] = useState("");  // 保留以兼容旧代码，但实际不再使用
  
  const activeWorkspace = workspaces.find(w => w.id === appState.activeWorkspaceId) || null;

  return (
    <div className="flex h-screen bg-bg-main text-text-main font-sans selection:bg-primary/30">
      <div className="w-[240px] bg-bg-sidebar border-r border-border-base flex flex-col shrink-0 z-50">
        <div className="h-14 flex items-center px-5 border-b border-border-base mb-2 select-none"><div className="h-8 w-8 bg-primary rounded-lg flex items-center justify-center text-white mr-3 shadow-lg shadow-primary/20"><Zap size={18} fill="currentColor" /></div><span className="font-bold text-lg tracking-tight">LogAnalyzer</span></div>
        <div className="flex-1 px-3 py-4 space-y-1">
            <NavItem icon={LayoutGrid} label="Workspaces" active={appState.page === 'workspaces'} onClick={() => setPage('workspaces')} />
            <NavItem icon={Search} label="Search Logs" active={appState.page === 'search'} onClick={() => setPage('search')} />
            <NavItem icon={ListTodo} label="Keywords" active={appState.page === 'keywords'} onClick={() => setPage('keywords')} />
            <NavItem icon={Layers} label="Tasks" active={appState.page === 'tasks'} onClick={() => setPage('tasks')} />
        </div>
        {importStatus && <div className="p-3 m-3 bg-bg-card border border-primary/20 rounded text-xs text-primary animate-pulse"><div className="font-bold mb-1 flex items-center gap-2"><Loader2 size={12} className="animate-spin"/> Processing</div><div className="truncate opacity-80">{importStatus}</div></div>}
        <div className="p-3 border-t border-border-base">
          <NavItem icon={Settings} label="Settings" active={appState.page === 'settings'} onClick={() => setPage('settings')} />
        </div>
      </div>
      <div className="flex-1 flex flex-col min-w-0 bg-bg-main">
        <div className="h-14 border-b border-border-base bg-bg-main flex items-center justify-between px-6 shrink-0 z-40"><div className="flex items-center text-sm text-text-muted select-none"><span className="opacity-50">Workspace / </span><span className="font-medium text-text-main ml-2 flex items-center gap-2"><FileText size={14} className="text-primary"/> {activeWorkspace ? activeWorkspace.name : "Select Workspace"}</span></div></div>
        <div className="flex-1 overflow-hidden relative">
           {appState.page === 'search' && <SearchPage keywordGroups={keywordGroups} addToast={addToast} searchInputRef={searchInputRef} activeWorkspace={activeWorkspace} />}
           {appState.page === 'keywords' && <KeywordsPage />}
           {appState.page === 'workspaces' && <WorkspacesPage />}
           {appState.page === 'tasks' && <TasksPage />}
           {appState.page === 'settings' && <PerformancePage addToast={addToast} />}
        </div>
      </div>
      <ToastContainer toasts={appState.toasts} removeToast={removeToast} />
    </div>
  );
}

// --- Main App (Wrapped with Provider) ---
export default function App() {
  return (
    <AppProvider>
      <AppContent />
    </AppProvider>
  );
}