import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from '@tauri-apps/plugin-dialog';
import { useVirtualizer } from "@tanstack/react-virtual";
import { 
  Search, LayoutGrid, ListTodo, Settings, Layers, 
  CheckCircle2, AlertCircle, X, Plus, Terminal, 
  RefreshCw, Trash2, FolderOpen, Moon, Zap, Play, StopCircle, 
  MoreHorizontal, FileText, ChevronRight, Edit2, Save, Filter,
  ChevronDown, Tag, Hash, PauseCircle, Copy, Info, Loader2
} from "lucide-react";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)); }

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

// Types
type Page = 'search' | 'keywords' | 'workspaces' | 'tasks' | 'settings';
type ColorKey = 'blue' | 'green' | 'red' | 'orange' | 'purple';
type ToastType = 'success' | 'error' | 'info';

interface LogEntry { id: number; timestamp: string; level: string; file: string; line: number; content: string; tags: any[]; real_path?: string; }
interface KeywordPattern { regex: string; comment: string; }
interface KeywordGroup { id: string; name: string; color: ColorKey; patterns: KeywordPattern[]; enabled: boolean; }
interface Workspace { id: string; name: string; path: string; status: 'READY' | 'SCANNING' | 'OFFLINE' | 'PROCESSING'; size: string; files: number; }
// Enhanced Task Interface
interface Task { 
    id: string; 
    type: string; 
    target: string; 
    progress: number; 
    message: string; // Real-time message from backend
    status: 'RUNNING' | 'COMPLETED' | 'FAILED' | 'STOPPED'; 
}
interface Toast { id: number; type: ToastType; message: string; }
interface TaskUpdateEvent { task_id: string; status: string; message: string; progress: number; }

// Color System
const COLOR_STYLES: Record<ColorKey, any> = {
  blue: { dot: "bg-blue-500", badge: "bg-blue-500/15 text-blue-400 border-blue-500/20", border: "border-blue-500", text: "text-blue-400", activeBtn: "bg-blue-500 text-white border-blue-400 shadow-[0_0_10px_rgba(59,130,246,0.4)]", hoverBorder: "hover:border-blue-500/50", highlight: "bg-blue-500/20 text-blue-300 border-blue-500/30" },
  green: { dot: "bg-emerald-500", badge: "bg-emerald-500/15 text-emerald-400 border-emerald-500/20", border: "border-emerald-500", text: "text-emerald-400", activeBtn: "bg-emerald-500 text-white border-emerald-400 shadow-[0_0_10px_rgba(16,185,129,0.4)]", hoverBorder: "hover:border-emerald-500/50", highlight: "bg-emerald-500/20 text-emerald-300 border-emerald-500/30" },
  red: { dot: "bg-red-500", badge: "bg-red-500/15 text-red-400 border-red-500/20", border: "border-red-500", text: "text-red-400", activeBtn: "bg-red-500 text-white border-red-400 shadow-[0_0_10px_rgba(239,68,68,0.4)]", hoverBorder: "hover:border-red-500/50", highlight: "bg-red-500/20 text-red-300 border-red-500/30" },
  orange: { dot: "bg-amber-500", badge: "bg-amber-500/15 text-amber-400 border-amber-500/20", border: "border-amber-500", text: "text-amber-400", activeBtn: "bg-amber-500 text-white border-amber-400 shadow-[0_0_10px_rgba(245,158,11,0.4)]", hoverBorder: "hover:border-amber-500/50", highlight: "bg-amber-500/20 text-amber-300 border-amber-500/30" },
  purple: { dot: "bg-purple-500", badge: "bg-purple-500/15 text-purple-400 border-purple-500/20", border: "border-purple-500", text: "text-purple-400", activeBtn: "bg-purple-500 text-white border-purple-400 shadow-[0_0_10px_rgba(168,85,247,0.4)]", hoverBorder: "hover:border-purple-500/50", highlight: "bg-purple-500/20 text-purple-300 border-purple-500/30" }
};

// UI Components
const NavItem = ({ icon: Icon, label, active, onClick }: { icon: any, label: string, active: boolean, onClick: () => void }) => (
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

const Button = ({ children, variant = 'primary', className, icon: Icon, onClick, ...props }: any) => {
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
const Input = ({ className, ref, ...props }: any) => (<input ref={ref} className={cn("h-9 w-full bg-bg-main border border-border-base rounded-md px-3 text-sm text-text-main placeholder:text-text-dim focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/50 transition-all", className)} {...props} />);
const Card = ({ children, className, ...props }: any) => (<div className={cn("bg-bg-card border border-border-base rounded-lg overflow-hidden", className)} {...props}>{children}</div>);
const ToastContainer = ({ toasts, removeToast }: { toasts: Toast[], removeToast: (id: number) => void }) => (<div className="fixed bottom-6 right-6 z-[100] flex flex-col gap-3 pointer-events-none">{toasts.map(toast => (<div key={toast.id} className={cn("pointer-events-auto min-w-[300px] p-4 rounded-lg shadow-2xl border flex items-center gap-3 animate-in slide-in-from-right-full duration-300", toast.type === 'success' ? "bg-bg-card border-emerald-500/30 text-emerald-400" : toast.type === 'error' ? "bg-bg-card border-red-500/30 text-red-400" : "bg-bg-card border-blue-500/30 text-blue-400")}>{toast.type === 'success' ? <CheckCircle2 size={20}/> : toast.type === 'error' ? <AlertCircle size={20}/> : <Info size={20}/>}<span className="text-sm font-medium text-text-main">{toast.message}</span><button onClick={() => removeToast(toast.id)} className="ml-auto text-text-dim hover:text-text-main"><X size={16}/></button></div>))}</div>);

// Components: KeywordModal, HybridLogRenderer, FilterPalette (Assuming they are defined as in previous steps)
// --- Keyword Modal ---
const KeywordModal = ({ isOpen, onClose, onSave, initialData }: any) => {
  if (!isOpen) return null;
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
            <div className="flex justify-between items-center mb-2"><label className="text-xs text-text-dim uppercase font-bold">Patterns & Comments</label><Button variant="ghost" size="sm" className="h-6 text-xs" icon={Plus} onClick={() => setPatterns([...patterns, { regex: "", comment: "" }])}>Add</Button></div>
            <div className="space-y-2">{patterns.map((p, i) => (<div key={i} className="flex gap-2 items-center group"><div className="flex-1"><Input value={p.regex} onChange={(e:any) => { const n = [...patterns]; n[i].regex = e.target.value; setPatterns(n); }} placeholder="RegEx" className="font-mono text-xs"/></div><div className="flex-1"><Input value={p.comment} onChange={(e:any) => { const n = [...patterns]; n[i].comment = e.target.value; setPatterns(n); }} placeholder="Comment" className="text-xs"/></div><Button variant="icon" icon={Trash2} className="text-red-400 opacity-0 group-hover:opacity-100 transition-opacity" onClick={() => setPatterns(patterns.filter((_, idx) => idx !== i))} /></div>))}</div>
          </div>
        </div>
        <div className="px-6 py-4 border-t border-border-base bg-bg-sidebar flex justify-end gap-3"><Button variant="secondary" onClick={onClose}>Cancel</Button><Button onClick={handleSave}>Save Configuration</Button></div>
      </div>
    </div>
  );
};
const HybridLogRenderer = ({ text, query, keywordGroups }: any) => {
  const { patternMap, regexPattern } = useMemo(() => {
    const map = new Map(); const patterns = new Set();
    keywordGroups.filter((g:any) => g.enabled).forEach((group:any) => { group.patterns.forEach((p:any) => { if (p.regex?.trim()) { map.set(p.regex.toLowerCase(), { color: group.color, comment: p.comment }); patterns.add(p.regex); } }); });
    if (query) { query.split('|').map((t:string) => t.trim()).filter((t:string) => t.length > 0).forEach((term:string, index:number) => { if (!map.has(term.toLowerCase())) { map.set(term.toLowerCase(), { color: ['blue', 'purple', 'green', 'orange'][index % 4], comment: "" }); } patterns.add(term); }); }
    const sorted = Array.from(patterns).sort((a:any, b:any) => b.length - a.length);
    return { regexPattern: sorted.length > 0 ? new RegExp(`(${sorted.map((p:any) => p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})`, 'gi') : null, patternMap: map };
  }, [keywordGroups, query]);
  if (!regexPattern) return <span>{text}</span>;
  return <span>{text.split(regexPattern).map((part:string, i:number) => { const info = patternMap.get(part.toLowerCase()); if (info) { const style = COLOR_STYLES[info.color as ColorKey]?.highlight || COLOR_STYLES['blue'].highlight; return <span key={i} className="inline-flex items-baseline mx-[1px]"><span className={cn("rounded-[2px] px-1 border font-bold break-all", style)}>{part}</span>{info.comment && <span className={cn("ml-1 px-1.5 rounded-[2px] text-[10px] font-normal border select-none whitespace-nowrap transform -translate-y-[1px]", style.replace("bg-", "bg-opacity-10 bg-"))}>{info.comment}</span>}</span>; } return <span key={i}>{part}</span>; })}</span>;
};
const FilterPalette = ({ isOpen, onClose, groups, currentQuery, onToggleRule }: any) => {
    if (!isOpen) return null;
    const isPatternActive = (regex: string) => currentQuery.split('|').map((t:string) => t.trim().toLowerCase()).includes(regex.toLowerCase());
    const colorOrder: ColorKey[] = ['red', 'orange', 'blue', 'purple', 'green'];
    return (<><div className="fixed inset-0 z-[45] bg-transparent" onClick={onClose}></div><div className="absolute top-full right-0 mt-2 w-[600px] max-h-[60vh] overflow-y-auto bg-[#18181b] border border-border-base rounded-lg shadow-2xl z-50 p-4 grid gap-6 animate-in fade-in zoom-in-95 duration-100 origin-top-right ring-1 ring-white/10"><div className="flex justify-between items-center pb-2 border-b border-white/10"><h3 className="text-sm font-bold text-text-main flex items-center gap-2"><Filter size={14} className="text-primary"/> Filter Command Center</h3></div>{colorOrder.map(color => { const colorGroups = groups.filter((g: KeywordGroup) => g.color === color); if (colorGroups.length === 0) return null; return (<div key={color}><div className={cn("text-[10px] font-bold uppercase mb-2 flex items-center gap-2", COLOR_STYLES[color].text)}><div className={cn("w-2 h-2 rounded-full", COLOR_STYLES[color].dot)}></div>{color} Priority Level</div><div className="grid grid-cols-2 gap-3">{colorGroups.map((group: KeywordGroup) => (<div key={group.id} className="bg-bg-card/50 border border-white/5 rounded p-2"><div className="text-xs font-semibold text-text-muted mb-2 px-1">{group.name}</div><div className="flex flex-wrap gap-2">{group.patterns.map((p, idx) => { const active = isPatternActive(p.regex); return <button key={idx} onClick={() => onToggleRule(p.regex)} className={cn("text-[11px] px-2 py-1 rounded border transition-all duration-150 flex items-center gap-1.5 cursor-pointer", active ? COLOR_STYLES[color].activeBtn : `bg-bg-main text-text-dim border-border-base hover:bg-bg-hover ${COLOR_STYLES[color].hoverBorder}`)}>{active && <CheckCircle2 size={10} />}<span className="font-mono">{p.regex}</span></button> })}</div></div>))}</div></div>) })}</div></>);
};

// --- Search Page ---
const SearchPage = ({ keywordGroups, addToast, searchInputRef, activeWorkspace }: any) => {
  const [query, setQuery] = useState("");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [isFilterPaletteOpen, setIsFilterPaletteOpen] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const parentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlistenResults = listen<LogEntry[]>('search-results', (e) => setLogs(prev => [...prev, ...e.payload]));
    const unlistenComplete = listen('search-complete', (e) => { setIsSearching(false); addToast('success', `Found ${e.payload} logs.`); });
    const unlistenError = listen('search-error', (e) => { setIsSearching(false); addToast('error', `${e.payload}`); });
    return () => { unlistenResults.then(f => f()); unlistenComplete.then(f => f()); unlistenError.then(f => f()); }
  }, []);

  const handleSearch = async () => {
    if (!activeWorkspace) return addToast('error', 'Select a workspace first.');
    setLogs([]); setIsSearching(true);
    try { await invoke("search_logs", { query, searchPath: activeWorkspace.path }); } catch(e) { setIsSearching(false); }
  };

  const toggleRuleInQuery = (ruleRegex: string) => {
    const terms = query.split('|').map((t:string) => t.trim()).filter((t:string) => t.length > 0);
    const idx = terms.findIndex((t:string) => t.toLowerCase() === ruleRegex.toLowerCase());
    setQuery(idx !== -1 ? terms.filter((_:any, i:number) => i !== idx).join('|') : [...terms, ruleRegex].join('|'));
  };

  const copyToClipboard = (text: string) => { navigator.clipboard.writeText(text).then(() => addToast('success', 'Copied')); };
  const tryFormatJSON = (content: string) => { try { const start = content.indexOf('{'); if (start === -1) return content; const jsonPart = content.substring(start); const obj = JSON.parse(jsonPart); return JSON.stringify(obj, null, 2); } catch (e) { return content; } };
  
  // 优化：动态高度估算
  const rowVirtualizer = useVirtualizer({ 
    count: logs.length, 
    getScrollElement: () => parentRef.current, 
    estimateSize: useCallback((index: number) => {
      const log = logs[index];
      if (!log) return 46;
      // 根据内容长度估算高度
      const lines = Math.ceil(log.content.length / 120);
      return Math.max(46, Math.min(lines * 22, 200));  // 最小 46px，最大 200px
    }, [logs]),
    overscan: 20,  // 增加 overscan
    measureElement: (element) => element?.getBoundingClientRect().height || 46  // 精确测量
  });
  
  const activeLog = logs.find(l => l.id === selectedId);

  return (
    <div className="flex flex-col h-full relative">
      <div className="p-4 border-b border-border-base bg-bg-sidebar space-y-3 shrink-0 relative z-40">
        <div className="flex gap-2">
          <div className="relative flex-1"><Search className="absolute left-3 top-2.5 text-text-dim" size={16} /><Input ref={searchInputRef} value={query} onChange={(e: any) => setQuery(e.target.value)} className="pl-9 font-mono bg-bg-main" placeholder="Search regex (Cmd+K)..." onKeyDown={(e:any) => e.key === 'Enter' && handleSearch()} /></div>
          <div className="relative"><Button variant={isFilterPaletteOpen ? "active" : "secondary"} icon={Filter} onClick={() => setIsFilterPaletteOpen(!isFilterPaletteOpen)} className="w-[140px] justify-between">Filters <ChevronDown size={14} className={cn("transition-transform", isFilterPaletteOpen ? "rotate-180" : "")}/></Button><FilterPalette isOpen={isFilterPaletteOpen} onClose={() => setIsFilterPaletteOpen(false)} groups={keywordGroups} currentQuery={query} onToggleRule={toggleRuleInQuery} /></div>
          <Button icon={isSearching ? Loader2 : Search} onClick={handleSearch} disabled={isSearching} className={isSearching ? "animate-pulse" : ""}>{isSearching ? '...' : 'Search'}</Button>
        </div>
        <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none h-6 min-h-[24px]"><span className="text-[10px] font-bold text-text-dim uppercase">Active:</span>{query ? query.split('|').map((term:string, i:number) => <span key={i} className="flex items-center text-[10px] bg-bg-card border border-border-base px-1.5 rounded text-text-main whitespace-nowrap"><Hash size={8} className="mr-1 opacity-50"/> {term}</span>) : <span className="text-[10px] text-text-dim italic">None</span>}</div>
      </div>
      <div className="flex-1 flex overflow-hidden">
        <div ref={parentRef} className="flex-1 overflow-auto bg-bg-main scrollbar-thin">
          <div className="sticky top-0 z-10 grid grid-cols-[60px_190px_220px_1fr] px-4 py-2 bg-bg-main border-b border-border-base text-xs font-bold text-text-dim uppercase tracking-wider"><div>Level</div> <div>Time</div> <div>File</div> <div>Content</div></div>
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const log = logs[virtualRow.index];
              const isActive = log.id === selectedId;
              return (
                <div key={virtualRow.key} data-index={virtualRow.index} ref={rowVirtualizer.measureElement} onClick={() => setSelectedId(log.id)} style={{ transform: `translateY(${virtualRow.start}px)` }} className={cn("absolute top-0 left-0 w-full grid grid-cols-[60px_190px_220px_1fr] px-4 py-2 border-b border-border-base/40 cursor-pointer text-[13px] font-mono hover:bg-bg-hover transition-colors items-start", isActive && "bg-blue-500/10 border-l-2 border-l-primary")}>
                  <div className={cn("font-bold pt-0.5", log.level === 'ERROR' ? 'text-red-400' : log.level === 'WARN' ? 'text-yellow-400' : 'text-blue-400')}>{log.level.substring(0,1)}</div>
                  <div className="text-text-muted whitespace-nowrap pt-0.5">{log.timestamp}</div>
                  <div className="text-text-muted break-all pr-4 pt-0.5 leading-relaxed">{log.file}:{log.line}</div>
                  <div className="text-text-main whitespace-pre-wrap break-all leading-relaxed pr-4"><HybridLogRenderer text={log.content} query={query} keywordGroups={keywordGroups} /></div>
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

const KeywordsPage = ({ keywordGroups, setKeywordGroups, addToast }: any) => {
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingGroup, setEditingGroup] = useState<KeywordGroup | null>(null);
  const handleSave = (group: KeywordGroup) => {
    if (editingGroup) { setKeywordGroups((prev: KeywordGroup[]) => prev.map(g => g.id === group.id ? group : g)); addToast('success', 'Updated'); } 
    else { setKeywordGroups((prev: KeywordGroup[]) => [...prev, group]); addToast('success', 'Created'); }
  };
  const handleDelete = (id: string) => { setKeywordGroups((prev: KeywordGroup[]) => prev.filter(g => g.id !== id)); addToast('info', 'Deleted'); };

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

const WorkspacesPage = ({ workspaces, setWorkspaces, addToast, setActiveWorkspaceId, setImportStatus, activeWorkspaceId, setTasks }: any) => {
  const handleDelete = (id: string) => { setWorkspaces((prev: Workspace[]) => prev.filter(w => w.id !== id)); addToast('info', 'Deleted'); };
  
  const handleImportFile = async () => {
    logger.debug('handleImportFile called');
    try {
        // 选择单个文件或压缩包
        const selected = await open({ 
          directory: false,
          multiple: false,
          filters: [{
            name: 'Log Files & Archives',
            extensions: ['log', 'txt', 'gz', 'zip', 'tar', 'tgz', 'rar', '*']
          }]
        });
        logger.debug('Selected file:', selected);
        if (selected) {
            await importPath(selected as string);
        }
    } catch (e) { 
      logger.error('handleImportFile error:', e);
      addToast('error', `${e}`); 
    }
  };
  
  const handleImportFolder = async () => {
    logger.debug('handleImportFolder called');
    try {
        // 选择文件夹
        const selected = await open({ 
          directory: true,
          multiple: false
        });
        logger.debug('Selected folder:', selected);
        if (selected) {
            await importPath(selected as string);
        }
    } catch (e) { 
      logger.error('handleImportFolder error:', e);
      addToast('error', `${e}`); 
    }
  };
  
  const importPath = async (pathStr: string) => {
    logger.debug('importPath called with:', pathStr);
    try {
      const fileName = pathStr.split(/[/\\]/).pop() || "New";
      const newWs: Workspace = { 
        id: Date.now().toString(), 
        name: fileName, 
        path: pathStr, 
        status: 'PROCESSING', 
        size: '-', 
        files: 0 
      };
      logger.debug('Creating workspace:', newWs);
      setWorkspaces((prev: Workspace[]) => [...prev, newWs]);
      setActiveWorkspaceId(newWs.id);
      
      logger.debug('Invoking import_folder with:', { path: pathStr, workspaceId: newWs.id });
      const taskId = await invoke<string>("import_folder", { 
        path: pathStr, 
        workspaceId: newWs.id
      });
      logger.debug('import_folder returned taskId:', taskId);
      
      // 注意：任务会通过 task-update 事件自动添加，这里仅作为后备
      setTasks((prev: Task[]) => {
        // 如果已经通过事件添加，则不重复
        if (prev.find(t => t.id === taskId)) {
          logger.debug('Task already exists, skipping');
          return prev;
        }
        logger.debug('Adding task to list:', taskId);
        return [...prev, { 
          id: taskId, 
          type: 'Import', 
          target: pathStr, 
          progress: 0, 
          status: 'RUNNING', 
          message: 'Initializing...' 
        }];
      });
      
      addToast('info', 'Import started');
    } catch (e) {
      logger.error('importPath error:', e);
      addToast('error', `Failed to start import: ${e}`);
      // 删除刚创建的工作区
      setWorkspaces((prev: Workspace[]) => prev.slice(0, -1));
    }
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
           <Card key={ws.id} className={cn("h-full flex flex-col hover:border-primary/50 transition-colors group cursor-pointer", activeWorkspaceId === ws.id ? "border-primary ring-1 ring-primary" : "border-border-base")} onClick={() => setActiveWorkspaceId(ws.id)}>
              <div className="px-4 py-3 border-b border-border-base bg-bg-sidebar/50 font-bold text-sm flex justify-between items-center">
                  {ws.name}<Button variant="ghost" icon={Trash2} className="h-6 w-6 p-0 text-text-dim hover:text-red-400" onClick={() => handleDelete(ws.id)}/>
              </div>
              <div className="p-4 space-y-4">
                 <code className="text-xs bg-bg-main px-2 py-1.5 rounded border border-border-base block truncate font-mono text-text-muted">{ws.path}</code>
                 <div className="flex items-center gap-2 text-xs font-bold">{ws.status === 'READY' ? <><CheckCircle2 size={14} className="text-emerald-500"/> <span className="text-emerald-500">READY</span></> : <><RefreshCw size={14} className="text-blue-500 animate-spin"/> <span className="text-blue-500">PROCESSING</span></>}</div>
              </div>
           </Card>
         ))}
      </div>
    </div>
  );
};

// --- Updated Tasks Page (Real Events) ---
const TasksPage = ({ tasks, setTasks, addToast }: { tasks: Task[], setTasks: any, addToast: any }) => {
  const handleDelete = (id: string) => { setTasks((prev: Task[]) => prev.filter(t => t.id !== id)); addToast('info', 'Task removed'); };

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

// --- Main App ---
export default function App() {
  const [page, setPage] = useState<Page>('workspaces');
  const [toasts, setToasts] = useState<Toast[]>([]);
  const searchInputRef = useRef<HTMLInputElement>(null);
  
  const [keywordGroups, setKeywordGroups] = useState<KeywordGroup[]>([]);
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(null);
  const [importStatus, setImportStatus] = useState("");

  useEffect(() => { invoke<any>("load_config").then(c => { if(c.keyword_groups) setKeywordGroups(c.keyword_groups); if(c.workspaces) setWorkspaces(c.workspaces); }); }, []);
  useEffect(() => { if(keywordGroups.length>0 || workspaces.length>0) invoke("save_config", { config: { keyword_groups: keywordGroups, workspaces } }); }, [keywordGroups, workspaces]);

  // 新增：工作区切换时自动加载索引
  useEffect(() => {
    if (activeWorkspaceId) {
      const workspace = workspaces.find(w => w.id === activeWorkspaceId);
      if (workspace && workspace.status === 'READY') {
        invoke('load_workspace', { workspaceId: activeWorkspaceId })  // 修复：使用 camelCase
          .then(() => {
            addToast('success', `Loaded workspace: ${workspace.name}`);
          })
          .catch((e) => {
            addToast('error', `Failed to load index: ${e}`);
          });
      }
    }
  }, [activeWorkspaceId]);

  useEffect(() => {
      // Listen for task updates from Rust
      const u1 = listen<TaskUpdateEvent>('task-update', e => {
          logger.debug('task-update event received:', e.payload);
          const update = e.payload;
          setTasks((prev: Task[]) => {
            const existingTask = prev.find(t => t.id === update.task_id);
            if (existingTask) {
              logger.debug('Updating existing task:', update.task_id);
              // 更新已存在的任务
              return prev.map(t => t.id === update.task_id ? { ...t, status: update.status as any, message: update.message, progress: update.progress } : t);
            } else {
              logger.debug('Creating new task from event:', update.task_id);
              // 创建新任务（如果前端没有创建）
              return [...prev, {
                id: update.task_id,
                type: 'Import',
                target: 'Unknown',
                progress: update.progress,
                status: update.status as any,
                message: update.message
              }];
            }
          });
          // Optional: show global spinner text
          if (update.status === 'RUNNING') setImportStatus(update.message);
      });
      
      const u2 = listen('import-complete', (e: any) => { 
          setImportStatus(""); 
          setWorkspaces((prev: Workspace[]) => prev.map(w => w.status === 'PROCESSING' ? { ...w, status: 'READY' } : w));
          // 更新对应的任务状态
          const taskId = e.payload;
          if (taskId) {
            setTasks((prev: Task[]) => prev.map(t => t.id === taskId ? { ...t, status: 'COMPLETED', progress: 100, message: 'Done' } : t));
          }
          addToast('success', 'Process complete'); 
      });
      
      const u3 = listen('import-error', (e) => { setImportStatus(""); addToast('error', `Error: ${e.payload}`); });
      
      return () => { u1.then(f=>f()); u2.then(f=>f()); u3.then(f=>f()); }
  }, []);

  const addToast = (type: ToastType, message: string) => { const id = Date.now(); setToasts(p => [...p, { id, type, message }]); setTimeout(() => setToasts(p => p.filter(t => t.id !== id)), 3000); };
  const activeWorkspace = workspaces.find(w => w.id === activeWorkspaceId) || null;

  return (
    <div className="flex h-screen bg-bg-main text-text-main font-sans selection:bg-primary/30">
      <div className="w-[240px] bg-bg-sidebar border-r border-border-base flex flex-col shrink-0 z-50">
        <div className="h-14 flex items-center px-5 border-b border-border-base mb-2 select-none"><div className="h-8 w-8 bg-primary rounded-lg flex items-center justify-center text-white mr-3 shadow-lg shadow-primary/20"><Zap size={18} fill="currentColor" /></div><span className="font-bold text-lg tracking-tight">LogAnalyzer</span></div>
        <div className="flex-1 px-3 py-4 space-y-1">
            <NavItem icon={LayoutGrid} label="Workspaces" active={page === 'workspaces'} onClick={() => setPage('workspaces')} />
            <NavItem icon={Search} label="Search Logs" active={page === 'search'} onClick={() => setPage('search')} />
            <NavItem icon={ListTodo} label="Keywords" active={page === 'keywords'} onClick={() => setPage('keywords')} />
            <NavItem icon={Layers} label="Tasks" active={page === 'tasks'} onClick={() => setPage('tasks')} />
        </div>
        {importStatus && <div className="p-3 m-3 bg-bg-card border border-primary/20 rounded text-xs text-primary animate-pulse"><div className="font-bold mb-1 flex items-center gap-2"><Loader2 size={12} className="animate-spin"/> Processing</div><div className="truncate opacity-80">{importStatus}</div></div>}
        <div className="p-3 border-t border-border-base">
          <NavItem icon={Settings} label="Settings" active={page === 'settings'} onClick={() => setPage('settings')} />
        </div>
      </div>
      <div className="flex-1 flex flex-col min-w-0 bg-bg-main">
        <div className="h-14 border-b border-border-base bg-bg-main flex items-center justify-between px-6 shrink-0 z-40"><div className="flex items-center text-sm text-text-muted select-none"><span className="opacity-50">Workspace / </span><span className="font-medium text-text-main ml-2 flex items-center gap-2"><FileText size={14} className="text-primary"/> {activeWorkspace ? activeWorkspace.name : "Select Workspace"}</span></div></div>
        <div className="flex-1 overflow-hidden relative">
           {page === 'search' && <SearchPage keywordGroups={keywordGroups} addToast={addToast} searchInputRef={searchInputRef} activeWorkspace={activeWorkspace} />}
           {page === 'keywords' && <KeywordsPage keywordGroups={keywordGroups} setKeywordGroups={setKeywordGroups} addToast={addToast} />}
           {page === 'workspaces' && <WorkspacesPage workspaces={workspaces} setWorkspaces={setWorkspaces} addToast={addToast} setActiveWorkspaceId={setActiveWorkspaceId} setImportStatus={setImportStatus} activeWorkspaceId={activeWorkspaceId} setTasks={setTasks} />}
           {/* FIX: Pass addToast correctly */}
           {page === 'tasks' && <TasksPage tasks={tasks} setTasks={setTasks} addToast={addToast} />}
           {page === 'settings' && <div className="p-10 text-center text-text-dim">Settings</div>}
        </div>
      </div>
      <ToastContainer toasts={toasts} removeToast={(id) => setToasts(prev => prev.filter(t => t.id !== id))} />
    </div>
  );
}