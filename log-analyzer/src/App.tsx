import { useState, useRef, useEffect, ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useVirtualizer } from "@tanstack/react-virtual";
import { 
  Search, LayoutGrid, ListTodo, Settings, Layers, 
  ChevronRight, Filter, Download, Play, CheckCircle2, 
  AlertCircle, X, Plus, Terminal, RefreshCw, Trash2, FolderOpen,
  MoreVertical, Moon, Sun, Laptop, Zap, Copy
} from "lucide-react";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

// --- Utility ---
function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)); }

// --- Types ---
type Page = 'search' | 'keywords' | 'workspaces' | 'tasks' | 'settings';
interface LogEntry { id: number; timestamp: string; level: string; file: string; line: number; content: string; tags: any[]; }

// --- UI Components (Design System) ---

const Button = ({ children, variant = 'primary', className, icon: Icon, ...props }: any) => {
  const base = "h-9 px-4 rounded-md text-sm font-medium transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed";
  const variants = {
    primary: "bg-primary hover:bg-primary-hover text-white shadow-sm",
    secondary: "bg-bg-card hover:bg-bg-hover text-text-main border border-border-base",
    ghost: "hover:bg-bg-hover text-text-muted hover:text-text-main",
    danger: "bg-red-500/10 text-red-500 hover:bg-red-500/20 border border-red-500/20"
  };
  return (
    <button className={cn(base, variants[variant as keyof typeof variants], className)} {...props}>
      {Icon && <Icon size={16} />}
      {children}
    </button>
  );
};

const Input = ({ className, ...props }: any) => (
  <input 
    className={cn(
      "h-9 w-full bg-bg-main border border-border-base rounded-md px-3 text-sm text-text-main placeholder:text-text-dim focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/50 transition-all", 
      className
    )} 
    {...props} 
  />
);

const Card = ({ children, className, title, actions }: any) => (
  <div className={cn("bg-bg-card border border-border-base rounded-lg overflow-hidden", className)}>
    {(title || actions) && (
      <div className="px-4 py-3 border-b border-border-base flex items-center justify-between bg-bg-sidebar/50">
        {title && <h3 className="text-sm font-semibold text-text-main">{title}</h3>}
        {actions && <div className="flex gap-2">{actions}</div>}
      </div>
    )}
    <div className="p-4">{children}</div>
  </div>
);

const Badge = ({ children, variant = 'default' }: any) => {
  const styles = {
    default: "bg-bg-hover text-text-muted",
    blue: "bg-primary/10 text-primary-text border-primary/20",
    green: "bg-emerald-500/10 text-emerald-500 border-emerald-500/20",
    red: "bg-red-500/10 text-red-500 border-red-500/20",
    orange: "bg-amber-500/10 text-amber-500 border-amber-500/20",
  };
  return (
    <span className={cn("px-2 py-0.5 rounded text-[11px] font-medium border border-transparent", styles[variant as keyof typeof styles])}>
      {children}
    </span>
  );
};

// --- Page: Search Logs (The Core) ---
const SearchPage = () => {
  const [query, setQuery] = useState("timeout|error");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const parentRef = useRef<HTMLDivElement>(null);

  // Load Initial Data
  useEffect(() => { invoke<LogEntry[]>("search_logs", { pattern: query }).then(setLogs); }, []);
  const handleSearch = () => invoke<LogEntry[]>("search_logs", { pattern: query }).then(setLogs);

  const rowVirtualizer = useVirtualizer({
    count: logs.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 34,
    overscan: 20,
  });

  const activeLog = logs.find(l => l.id === selectedId);

  return (
    <div className="flex flex-col h-full">
      {/* Search Header */}
      <div className="p-4 border-b border-border-base bg-bg-sidebar space-y-3 shrink-0">
        <div className="flex gap-2">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-2.5 text-text-dim" size={16} />
            <Input 
              value={query} onChange={(e: any) => setQuery(e.target.value)} 
              className="pl-9 font-mono" placeholder="Search logs (RegEx supported)..." 
            />
          </div>
          <select className="h-9 bg-bg-main border border-border-base rounded-md px-3 text-sm text-text-main focus:outline-none focus:border-primary">
            <option>All Levels</option>
            <option>ERROR</option>
            <option>WARN</option>
          </select>
          <Button icon={Search} onClick={handleSearch}>Search</Button>
        </div>
        <div className="flex items-center gap-2">
           <span className="text-xs font-semibold text-text-muted uppercase">File Filter:</span>
           <Input className="h-7 text-xs w-[300px]" placeholder="*.log, app-*.txt" />
           <div className="ml-auto text-xs text-text-dim flex gap-4">
             <span>Result: {logs.length} matched</span>
             <span>Time: 12ms</span>
           </div>
        </div>
      </div>

      {/* Main Content Area (Split View) */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left: Log List */}
        <div ref={parentRef} className="flex-1 overflow-auto bg-bg-main">
          {/* Table Header */}
          <div className="sticky top-0 z-10 grid grid-cols-[60px_150px_200px_1fr] px-4 py-2 bg-bg-main border-b border-border-base text-xs font-semibold text-text-dim uppercase tracking-wider">
            <div>Level</div>
            <div>Time</div>
            <div>File</div>
            <div>Content</div>
          </div>
          
          <div style={{ height: `${rowVirtualizer.getTotalSize()}px`, width: '100%', position: 'relative' }}>
            {rowVirtualizer.getVirtualItems().map((virtualRow) => {
              const log = logs[virtualRow.index];
              const isActive = log.id === selectedId;
              const levelColor = log.level === 'ERROR' ? 'text-status-error' : log.level === 'WARN' ? 'text-status-warn' : 'text-status-info';
              
              return (
                <div 
                  key={virtualRow.key}
                  onClick={() => setSelectedId(log.id)}
                  style={{ height: `${virtualRow.size}px`, transform: `translateY(${virtualRow.start}px)` }}
                  className={cn(
                    "absolute top-0 left-0 w-full grid grid-cols-[60px_150px_200px_1fr] items-center px-4 border-b border-border-base/50 cursor-pointer text-[13px] font-mono hover:bg-bg-hover transition-colors",
                    isActive && "bg-blue-900/20 border-l-2 border-l-primary"
                  )}
                >
                  <div className={cn("font-bold", levelColor)}>{log.level.substring(0,1)}</div>
                  <div className="text-text-muted">{log.timestamp}</div>
                  <div className="text-text-muted truncate pr-2" title={log.file}>{log.file}:{log.line}</div>
                  <div className="text-text-main truncate opacity-90">{log.content}</div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right: Context Panel */}
        {activeLog && (
          <div className="w-[450px] bg-bg-sidebar border-l border-border-base flex flex-col shrink-0">
            <div className="h-10 border-b border-border-base flex items-center justify-between px-4 bg-bg-card/50">
              <span className="text-xs font-semibold text-text-muted uppercase">Log Context</span>
              <div className="flex gap-2">
                <Button variant="ghost" className="h-6 w-6 p-0"><Copy size={14}/></Button>
                <Button variant="ghost" className="h-6 w-6 p-0" onClick={() => setSelectedId(null)}><X size={14}/></Button>
              </div>
            </div>
            
            <div className="flex-1 overflow-auto p-4 font-mono text-xs">
              <div className="mb-6">
                <div className="text-text-dim mb-1">Source File</div>
                <div className="text-primary break-all">/var/www/production/{activeLog.file}</div>
              </div>
              
              <div className="bg-bg-main border border-border-base rounded-md overflow-hidden">
                <div className="flex border-b border-border-base bg-bg-card/30">
                   <div className="w-10 py-1 text-center text-text-dim border-r border-border-base bg-bg-card">Line</div>
                   <div className="px-2 py-1 text-text-dim">Content</div>
                </div>
                {[-2, -1, 0, 1, 2].map(offset => (
                  <div key={offset} className={cn("flex", offset === 0 ? "bg-blue-500/10" : "")}>
                    <div className="w-10 py-1 text-center text-text-dim border-r border-border-base opacity-50 select-none">
                      {activeLog.line + offset}
                    </div>
                    <div className={cn("px-2 py-1 whitespace-pre-wrap break-all", offset === 0 ? "text-text-main" : "text-text-dim")}>
                      {offset === 0 ? activeLog.content : `Context line placeholder content...`}
                    </div>
                  </div>
                ))}
              </div>

              <div className="mt-6">
                <div className="text-text-dim mb-2">Metadata Tags</div>
                <div className="flex flex-wrap gap-2">
                   {activeLog.level === 'ERROR' && <Badge variant="red">Critical Error</Badge>}
                   <Badge variant="blue">Network</Badge>
                   <Badge>Microservice: Auth</Badge>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

// --- Page: Workspaces (Mock) ---
const WorkspacesPage = () => {
  const workspaces = [
    { name: "Production Logs - US", path: "/var/logs/prod-us", status: "READY", size: "48.7 GB", files: 1247 },
    { name: "Staging Environment", path: "/var/logs/staging", status: "SCANNING", size: "12.1 GB", files: 456 },
    { name: "Archive Q4 2024", path: "/mnt/archive/2024", status: "ERROR", size: "0 GB", files: 0 },
  ];

  return (
    <div className="p-8 max-w-6xl mx-auto space-y-6">
      <div className="flex justify-between items-center">
         <div>
            <h1 className="text-2xl font-semibold text-text-main">Workspace Management</h1>
            <p className="text-text-muted mt-1">Manage your log analysis targets and indexes.</p>
         </div>
         <Button icon={Plus}>New Workspace</Button>
      </div>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
         {workspaces.map((ws, i) => (
           <Card key={i} title={ws.name} className="h-full flex flex-col">
              <div className="space-y-4 mb-6">
                 <div>
                    <div className="text-xs text-text-dim uppercase font-bold mb-1">Path</div>
                    <code className="text-xs bg-bg-main px-2 py-1 rounded border border-border-base block truncate">{ws.path}</code>
                 </div>
                 <div className="grid grid-cols-3 gap-2 text-center">
                    <div className="bg-bg-hover rounded p-2">
                       <div className="text-xs text-text-dim">Files</div>
                       <div className="font-mono text-sm font-bold text-text-main">{ws.files}</div>
                    </div>
                    <div className="bg-bg-hover rounded p-2">
                       <div className="text-xs text-text-dim">Size</div>
                       <div className="font-mono text-sm font-bold text-text-main">{ws.size}</div>
                    </div>
                    <div className="bg-bg-hover rounded p-2 flex flex-col items-center justify-center">
                       {ws.status === 'READY' ? <CheckCircle2 size={16} className="text-status-success mb-1"/> : 
                        ws.status === 'ERROR' ? <AlertCircle size={16} className="text-status-error mb-1"/> : 
                        <RefreshCw size={16} className="text-primary animate-spin mb-1"/>}
                       <div className="text-[10px] font-bold">{ws.status}</div>
                    </div>
                 </div>
              </div>
              <div className="mt-auto grid grid-cols-2 gap-2">
                 <Button variant="secondary" icon={FolderOpen} className="text-xs">Browse</Button>
                 <Button variant="secondary" icon={RefreshCw} className="text-xs">Rescan</Button>
              </div>
           </Card>
         ))}
      </div>
    </div>
  );
};

// --- Main App Shell ---
export default function App() {
  const [page, setPage] = useState<Page>('search');

  const NavItem = ({ id, icon: Icon, label }: any) => (
    <button 
      onClick={() => setPage(id)}
      className={cn(
        "w-full flex items-center gap-3 px-3 py-2 rounded-md transition-all duration-200 group",
        page === id ? "bg-primary text-white shadow-md" : "text-text-muted hover:bg-bg-hover hover:text-text-main"
      )}
    >
      <Icon size={18} />
      <span className="text-sm font-medium">{label}</span>
      {page === id && <ChevronRight size={14} className="ml-auto opacity-50" />}
    </button>
  );

  return (
    <div className="flex h-screen bg-bg-main text-text-main font-sans selection:bg-primary/30">
      {/* Sidebar - Wide Professional Style */}
      <div className="w-[240px] bg-bg-sidebar border-r border-border-base flex flex-col shrink-0">
        <div className="h-14 flex items-center px-5 border-b border-border-base mb-2">
           <div className="h-8 w-8 bg-primary rounded-lg flex items-center justify-center text-white mr-3">
             <Zap size={18} fill="currentColor" />
           </div>
           <span className="font-bold text-lg tracking-tight">LogAnalyzer</span>
        </div>

        <div className="flex-1 px-3 py-4 space-y-1 overflow-y-auto">
          <div className="text-xs font-semibold text-text-dim px-3 mb-2 uppercase tracking-wider">Analyze</div>
          <NavItem id="search" icon={Search} label="Search Logs" />
          <NavItem id="keywords" icon={ListTodo} label="Keywords" />
          
          <div className="text-xs font-semibold text-text-dim px-3 mb-2 mt-6 uppercase tracking-wider">Manage</div>
          <NavItem id="workspaces" icon={LayoutGrid} label="Workspaces" />
          <NavItem id="tasks" icon={Layers} label="Tasks & Jobs" />
        </div>

        <div className="p-3 border-t border-border-base">
          <NavItem id="settings" icon={Settings} label="Settings" />
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 flex flex-col min-w-0 bg-bg-main">
        {/* Top Header */}
        <div className="h-14 border-b border-border-base bg-bg-main flex items-center justify-between px-6 shrink-0">
           <div className="flex items-center text-sm text-text-muted">
              <span className="opacity-50">Workspace / </span>
              <span className="ml-2 font-medium text-text-main flex items-center gap-2">
                Production Logs <span className="w-1.5 h-1.5 rounded-full bg-status-success"></span>
              </span>
           </div>
           <div className="flex items-center gap-3">
              <Button variant="ghost" className="h-8 w-8 p-0 rounded-full"><Moon size={16}/></Button>
              <Button variant="ghost" className="h-8 w-8 p-0 rounded-full"><LayoutGrid size={16}/></Button>
              <div className="w-8 h-8 rounded-full bg-primary/20 border border-primary/30 flex items-center justify-center text-xs font-bold text-primary">
                JS
              </div>
           </div>
        </div>

        {/* Page Content */}
        <div className="flex-1 overflow-hidden relative">
           {page === 'search' && <SearchPage />}
           {page === 'workspaces' && <WorkspacesPage />}
           
           {/* Mock pages for demo */}
           {page === 'keywords' && (
             <div className="p-8 max-w-5xl mx-auto">
                <h1 className="text-2xl font-bold mb-6">Keyword Configuration</h1>
                <Card className="mb-6">
                   <div className="grid grid-cols-[1fr_200px_100px_100px] gap-4 p-3 border-b border-border-base bg-bg-hover/30 text-xs font-bold text-text-dim uppercase">
                      <div>Pattern</div> <div>Group</div> <div>Color</div> <div>Actions</div>
                   </div>
                   {[
                     { p: "timeout|超时", g: "Network", c: "orange" },
                     { p: "auth.*failed", g: "Security", c: "red" },
                     { p: "heap space", g: "Performance", c: "purple" }
                   ].map((k, i) => (
                     <div key={i} className="grid grid-cols-[1fr_200px_100px_100px] gap-4 p-4 border-b border-border-base items-center text-sm">
                        <code className="bg-bg-main px-2 py-1 rounded border border-border-base font-mono text-xs">{k.p}</code>
                        <div><Badge variant="default">{k.g}</Badge></div>
                        <div className={`w-4 h-4 rounded-full bg-${k.c}-500`}></div>
                        <div className="flex gap-2 text-text-dim"><Terminal size={14}/><Trash2 size={14}/></div>
                     </div>
                   ))}
                </Card>
             </div>
           )}

           {page === 'tasks' && (
             <div className="p-8 max-w-4xl mx-auto">
                <h1 className="text-2xl font-bold mb-6">Background Tasks</h1>
                <div className="space-y-4">
                  <div className="p-4 bg-bg-card border border-border-base rounded-lg">
                     <div className="flex justify-between mb-2">
                        <span className="font-semibold text-sm flex items-center gap-2"><RefreshCw size={14} className="animate-spin text-primary"/> Indexing: Production Logs</span>
                        <span className="text-xs text-text-dim">45%</span>
                     </div>
                     <div className="h-1.5 w-full bg-bg-main rounded-full overflow-hidden">
                        <div className="h-full bg-primary w-[45%]"></div>
                     </div>
                  </div>
                  <div className="p-4 bg-bg-card border border-border-base rounded-lg opacity-70">
                     <div className="flex justify-between mb-2">
                        <span className="font-semibold text-sm flex items-center gap-2"><CheckCircle2 size={14} className="text-status-success"/> Export: Weekly Report</span>
                        <span className="text-xs text-text-dim">Completed</span>
                     </div>
                     <div className="h-1.5 w-full bg-bg-main rounded-full overflow-hidden">
                        <div className="h-full bg-status-success w-full"></div>
                     </div>
                  </div>
                </div>
             </div>
           )}
           
           {page === 'settings' && (
             <div className="p-8 max-w-2xl mx-auto">
                <h1 className="text-2xl font-bold mb-6">Settings</h1>
                <Card title="General" className="mb-6">
                   <div className="space-y-4">
                      <div>
                         <label className="text-sm font-medium mb-1 block">API Endpoint</label>
                         <Input defaultValue="https://api.log-analyzer.internal/v1" />
                      </div>
                      <div className="flex items-center justify-between">
                         <span className="text-sm">Auto-update index</span>
                         <div className="w-10 h-5 bg-primary rounded-full relative cursor-pointer"><div className="absolute right-1 top-1 w-3 h-3 bg-white rounded-full"></div></div>
                      </div>
                   </div>
                </Card>
             </div>
           )}
        </div>
      </div>
    </div>
  );
}