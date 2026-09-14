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
      className="glass-sidebar z-50 flex w-[64px] shrink-0 flex-col border-r border-border-subtle xl:w-[208px]"
      aria-label={t("nav.main_navigation", "Main navigation")}
    >
      {/* Logo 区域 */}
      <div className="flex h-12 items-center justify-center border-b border-border-subtle px-2 select-none xl:justify-start xl:px-3">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-primary text-white shadow-sm">
          <Zap size={17} fill="currentColor" />
        </div>
        <span className="ml-2.5 hidden truncate text-[14px] font-semibold tracking-[-0.01em] text-text-main xl:block">
          LogAnalyzer
        </span>
      </div>

      <nav className="flex-1 space-y-1 px-2 py-2.5">
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
      </nav>
      <div className="border-t border-border-subtle p-2">
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
