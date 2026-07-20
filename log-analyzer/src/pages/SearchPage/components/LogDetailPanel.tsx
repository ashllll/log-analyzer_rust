/**
 * 日志详情面板组件
 * 展示选中日志的详细信息
 */
import { memo, useRef } from "react";
import { Copy, X } from "lucide-react";
import { Button } from "../../../components/ui";
import { HybridLogRenderer } from "../../../components/renderers";
import { useTranslation } from "react-i18next";
import type { LogEntry, KeywordGroup } from "../../../types/common";
import type { SearchTerm } from "../../../types/search";
import { useResizableInspector } from "../hooks/useResizableInspector";

export interface LogDetailPanelProps {
  log: LogEntry;
  query: string;
  queryTerms: SearchTerm[] | null;
  keywordGroups: KeywordGroup[];
  onClose: () => void;
  onCopy: (text: string) => void;
}

function tryFormatJSON(content: string): string {
  try {
    const start = content.indexOf("{");
    if (start === -1) return content;
    const jsonPart = content.substring(start);
    const obj = JSON.parse(jsonPart);
    return JSON.stringify(obj, null, 2);
  } catch {
    return content;
  }
}

export const LogDetailPanel = memo<LogDetailPanelProps>(
  ({ log, query, queryTerms, keywordGroups, onClose, onCopy }) => {
    const { t } = useTranslation();
    const { width, handleProps } = useResizableInspector();
    const panelRef = useRef<HTMLDivElement>(null);

    const handleClose = () => {
      const panel = panelRef.current;
      if (
        !panel?.animate ||
        window.matchMedia("(prefers-reduced-motion: reduce)").matches
      ) {
        onClose();
        return;
      }
      panel
        .animate(
          [
            { opacity: 1, transform: "translateX(0)" },
            { opacity: 0, transform: "translateX(10%)" },
          ],
          {
            duration: 180,
            easing: "cubic-bezier(.23, 1, .32, 1)",
            fill: "forwards",
          }
        )
        .finished.then(onClose, onClose);
    };

    return (
      <aside
        ref={panelRef}
        className="inspector-surface apple-material motion-spatial relative border-l border-border-subtle flex flex-col shrink-0 z-20"
        style={{ width, background: "var(--material-sidebar)" }}
      >
        <div {...handleProps} className="inspector-resize-handle" />
        <div className="h-10 border-b border-border-subtle flex items-center justify-between px-4 bg-bg-elevated">
          <span className="text-xs font-bold text-text-muted uppercase tracking-wide">
            {t("search.inspector.title", "日志详情")}
          </span>
          <div className="flex gap-1">
            <Button
              variant="ghost"
              className="h-11 w-11 p-0"
              onClick={() => onCopy(log.content)}
              aria-label={t("search.inspector.copy", "复制内容")}
            >
              <Copy size={14} />
            </Button>
            <Button
              variant="ghost"
              className="h-11 w-11 p-0"
              onClick={handleClose}
              aria-label={t("search.inspector.close", "关闭面板")}
            >
              <X size={14} />
            </Button>
          </div>
        </div>
        <div className="flex-1 overflow-auto p-4 font-mono text-xs">
          <div className="bg-bg-main p-3 rounded border border-border-base mb-4">
            <div className="text-text-dim text-xs uppercase mb-1">
              {t("search.inspector.message", "消息内容")}
            </div>
            <div className="text-text-main whitespace-pre-wrap break-all leading-relaxed">
              <HybridLogRenderer
                text={tryFormatJSON(log.content)}
                query={query}
                queryTerms={queryTerms}
                keywordGroups={keywordGroups}
              />
            </div>
          </div>
          <div className="p-2 bg-bg-card border border-border-base rounded mb-2">
            <div className="text-xs text-text-dim uppercase">
              {t("search.inspector.file", "文件")}
            </div>
            <div className="break-all text-text-main">
              {log.real_path || t("search.inspector.not_available", "无")}
            </div>
          </div>
        </div>
      </aside>
    );
  }
);

LogDetailPanel.displayName = "LogDetailPanel";
