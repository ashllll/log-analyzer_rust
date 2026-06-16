import type { KeywordGroup, Workspace } from "../types/common";

export interface ConfigSyncSnapshot {
  keyword_groups: KeywordGroup[];
  workspaces: Workspace[];
}

/**
 * Compute a fingerprint that captures the persisted configuration fields.
 * Must include keyword and workspace content, not just ids, so edits trigger persistence.
 */
export const computeConfigFingerprint = (
  keywordGroups: KeywordGroup[],
  workspaces: Pick<
    Workspace,
    | "id"
    | "name"
    | "path"
    | "status"
    | "size"
    | "files"
    | "watching"
    | "ready_files"
  >[]
): string => {
  return JSON.stringify({
    keywords: keywordGroups.map((group) => ({
      id: group.id,
      name: group.name,
      color: group.color,
      enabled: group.enabled,
      patterns: group.patterns.map((pattern) => pattern.regex),
    })),
    workspaces: workspaces.map((workspace) => ({
      id: workspace.id,
      name: workspace.name,
      path: workspace.path,
      status: workspace.status,
      size: workspace.size,
      files: workspace.files,
      watching: workspace.watching,
      ready_files: workspace.ready_files,
    })),
  });
};

export const hasPersistableConfig = ({
  keyword_groups,
  workspaces,
}: ConfigSyncSnapshot) => {
  return keyword_groups.length > 0 || workspaces.length > 0;
};
