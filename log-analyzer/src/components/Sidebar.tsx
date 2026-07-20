import { useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useLocation } from "react-router-dom";
import { Search, LayoutGrid, ListTodo, Layers, Cog, Zap } from "lucide-react";
import { NavItem } from "./ui";

interface NavConfig {
  icon: React.ComponentType<{ size?: number }>;
  label: string;
  page: string;
  testId: string;
}

/**
 * 侧边栏导航组件
 * 包含 Logo、主导航菜单和设置入口
 */
export const Sidebar: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const currentPage = location.pathname.slice(1) || "workspaces";

  const setPage = useCallback(
    (page: string) => {
      navigate(`/${page}`);
    },
    [navigate]
  );

  const handleNavClick = useCallback(
    (page: string) => {
      return () => setPage(page);
    },
    [setPage]
  );

  const navItems: NavConfig[] = useMemo(
    () => [
      {
        icon: LayoutGrid,
        label: t("nav.workspaces"),
        page: "workspaces",
        testId: "nav-workspaces",
      },
      {
        icon: Search,
        label: t("nav.search"),
        page: "search",
        testId: "nav-search",
      },
      {
        icon: ListTodo,
        label: t("nav.keywords"),
        page: "keywords",
        testId: "nav-keywords",
      },
      {
        icon: Layers,
        label: t("nav.tasks"),
        page: "tasks",
        testId: "nav-tasks",
      },
    ],
    [t]
  );

  return (
    <aside
      className="apple-material w-[224px] border-r border-border-subtle flex flex-col shrink-0 z-50"
      style={{ background: "var(--material-sidebar)" }}
    >
      {/* Logo 区域 */}
      <div className="h-14 flex items-center px-4 border-b border-border-subtle select-none">
        <div className="h-7 w-7 bg-primary rounded-lg flex items-center justify-center text-white mr-2.5 shadow-sm">
          <Zap size={18} fill="currentColor" />
        </div>
        <span className="font-bold text-[15px] tracking-[-0.01em] text-text-main">
          LogAnalyzer
        </span>
      </div>

      <div className="flex-1 px-2.5 py-3 space-y-1">
        {navItems.map(({ icon, label, page, testId }) => (
          <NavItem
            key={page}
            icon={icon}
            label={label}
            active={currentPage === page}
            onClick={handleNavClick(page)}
            data-testid={testId}
          />
        ))}
      </div>
      <div className="p-3 border-t border-border-subtle">
        <NavItem
          icon={Cog}
          label={t("nav.settings")}
          active={currentPage === "settings"}
          onClick={() => setPage("settings")}
          data-testid="nav-settings"
        />
      </div>
    </aside>
  );
};
