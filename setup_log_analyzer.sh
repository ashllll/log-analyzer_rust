#!/bin/bash

# 设置错误时停止
set -e

# 定义颜色
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}   Log Analyzer (Tauri+React) Setup      ${NC}"
echo -e "${BLUE}=========================================${NC}"

# 1. 环境检查
echo -e "${GREEN}[1/6] Checking prerequisites...${NC}"
if ! command -v node &> /dev/null; then
    echo -e "${RED}Error: Node.js is not installed.${NC}"
    exit 1
fi
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust (cargo) is not installed.${NC}"
    echo "Please install Rust via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

PROJECT_NAME="log-analyzer"

if [ -d "$PROJECT_NAME" ]; then
    echo -e "${RED}Error: Directory '$PROJECT_NAME' already exists. Please remove it first.${NC}"
    exit 1
fi

# 2. 创建 Tauri 项目 (React + TS)
echo -e "${GREEN}[2/6] Scaffolding Tauri app ($PROJECT_NAME)...${NC}"
npm create tauri-app@latest "$PROJECT_NAME" -- --template react-ts --manager npm

cd "$PROJECT_NAME"

# 3. 安装前端依赖
echo -e "${GREEN}[3/6] Installing frontend dependencies...${NC}"
# 核心 UI 库
npm install lucide-react clsx tailwind-merge @tanstack/react-virtual framer-motion

# --- [修复点] 强制安装 Tailwind CSS v3，避免 v4 导致 init 命令失败 ---
echo -e "${GREEN}Installing Tailwind CSS v3 (stable)...${NC}"
npm install -D tailwindcss@3.4.17 postcss autoprefixer

# 初始化 Tailwind (v3 命令)
npx tailwindcss init -p

# 4. 写入配置文件 (Tailwind & CSS)
echo -e "${GREEN}[4/6] Configuring UI styles...${NC}"

# 写入 tailwind.config.js
cat > tailwind.config.js <<EOF
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "#1E1E1E", 
        sidebar: "#252526",
        border: "#3E3E42",
        primary: "#007ACC",
        accent: "#4EC9B0",
        "accent-dim": "rgba(78, 201, 176, 0.15)",
        tag: "#3C3C3C",
        muted: "#CCCCCC",
        "muted-dim": "#858585",
        selection: "#264F78",
      },
      fontFamily: {
        mono: ['"JetBrains Mono"', '"Fira Code"', 'monospace'],
        sans: ['"Segoe UI"', 'Inter', 'sans-serif'],
      }
    },
  },
  plugins: [],
}
EOF

# 写入 src/index.css
cat > src/index.css <<EOF
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  color-scheme: dark;
}

body {
  background-color: theme('colors.background');
  color: theme('colors.muted');
  overflow: hidden;
}

::-webkit-scrollbar {
  width: 10px;
  height: 10px;
}
::-webkit-scrollbar-track {
  background: #1e1e1e;
}
::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 5px;
}
::-webkit-scrollbar-thumb:hover {
  background: #4f4f4f;
}
EOF

# 5. 写入核心代码
echo -e "${GREEN}[5/6] Injecting source code...${NC}"

# 写入 src/App.tsx
cat > src/App.tsx <<EOF
import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useVirtualizer } from "@tanstack/react-virtual";
import { 
  Search, Layers, Folder, 
  Activity, X, Settings
} from "lucide-react";
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

interface Tag {
  text: string;
  tooltip: string;
  color: string;
}

interface LogEntry {
  id: number;
  timestamp: string;
  level: string;
  file: string;
  line: number;
  content: string;
  tags: Tag[];
}

function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

const Sidebar = () => (
  <div className="w-12 bg-sidebar flex flex-col items-center py-4 border-r border-border shrink-0 select-none">
    <div className="mb-6 text-primary"><Activity size={24} /></div>
    <div className="flex flex-col gap-6 w-full">
      <NavItem icon={<Search size={22} />} active />
      <NavItem icon={<Layers size={22} />} />
      <NavItem icon={<Folder size={22} />} />
    </div>
    <div className="mt-auto mb-2">
      <NavItem icon={<Settings size={22} />} />
    </div>
  </div>
);

const NavItem = ({ icon, active }: { icon: React.ReactNode; active?: boolean }) => (
  <div className={cn(
    "relative h-10 w-full flex items-center justify-center cursor-pointer hover:text-white transition-colors",
    active ? "text-white" : "text-muted-dim"
  )}>
    {active && <div className="absolute left-0 top-0 bottom-0 w-0.5 bg-primary" />}
    {icon}
  </div>
);

const LogContent = ({ content, keyword, tags }: { content: string, keyword: string, tags: Tag[] }) => {
  if (!keyword) return <span>{content}</span>;
  const parts = content.split(new RegExp(\`(\${keyword})\`, 'gi'));
  return (
    <span className="break-all whitespace-pre-wrap">
      {parts.map((part, i) => 
        part.toLowerCase() === keyword.toLowerCase() ? (
          <span key={i} className="bg-accent-dim text-accent font-bold px-0.5 rounded-sm">{part}</span>
        ) : <span key={i}>{part}</span>
      )}
      {tags.map((tag, idx) => (
        <span key={\`tag-\${idx}\`} className="group relative inline-flex items-center ml-3 cursor-help align-middle -mt-0.5">
          <span className="px-1.5 py-0.5 rounded text-[10px] bg-tag text-white hover:brightness-110 border border-white/10">{tag.text}</span>
          <span className="hidden group-hover:block absolute bottom-full left-0 mb-2 p-2 bg-zinc-900 border border-border text-xs rounded shadow-xl whitespace-nowrap z-50 pointer-events-none">
            {tag.tooltip}
          </span>
        </span>
      ))}
    </span>
  );
};

function App() {
  const [query, setQuery] = useState("timeout");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedLogId, setSelectedLogId] = useState<number | null>(null);
  const parentRef = useRef<HTMLDivElement>(null);

  const handleSearch = async () => {
    setLoading(true);
    try {
      const result = await invoke<LogEntry[]>("search_logs", { pattern: query });
      setLogs(result);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { handleSearch(); }, []);

  const rowVirtualizer = useVirtualizer({
    count: logs.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 28,
    overscan: 20,
  });

  const activeLog = logs.find(l => l.id === selectedLogId);

  return (
    <div className="flex h-screen bg-background font-sans text-sm selection:bg-selection selection:text-white">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <header className="h-14 border-b border-border bg-[#2d2d2d] flex items-center px-4 gap-3 shadow-md z-10 select-none">
          <div className="font-semibold text-white mr-4 flex items-center gap-2">LOG ANALYZER</div>
          <div className="flex-1 max-w-2xl flex items-center bg-[#3C3C3C] rounded-md border border-transparent focus-within:border-primary transition-all">
            <div className="px-2 text-muted-dim"><Search size={16} /></div>
            <input 
              className="flex-1 bg-transparent border-none text-white h-8 focus:outline-none placeholder:text-muted-dim/50 font-mono"
              placeholder="Search logs (RegEx supported)..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
            />
            {query && (
              <button onClick={() => setQuery('')} className="px-2 text-muted-dim hover:text-white"><X size={14} /></button>
            )}
          </div>
          <button onClick={handleSearch} className="bg-primary hover:bg-blue-600 text-white px-4 py-1.5 rounded text-sm font-medium transition-colors">
            {loading ? 'Scanning...' : 'Search'}
          </button>
        </header>

        <div className="h-10 border-b border-border bg-background flex items-center px-4 gap-2 select-none">
          <span className="text-xs text-muted-dim uppercase font-bold tracking-wider mr-2">Filters:</span>
          {['Errors Only', 'Last 1 Hour', 'Network Group'].map(label => (
            <button key={label} className="px-2 py-0.5 rounded-full border border-border bg-[#2a2a2a] text-xs hover:border-muted-dim hover:text-white transition-colors">{label}</button>
          ))}
          <div className="ml-auto text-xs text-muted-dim font-mono">{logs.length.toLocaleString()} hits found</div>
        </div>

        <div className="flex-1 flex overflow-hidden">
          <div ref={parentRef} className="flex-1 overflow-auto">
            <div style={{ height: \`\${rowVirtualizer.getTotalSize()}px\`, w