import type { LogEntry } from "../../../types/api-responses";

export function findSelectedLog(
  selectedId: number | null,
  entries: ReadonlyMap<number, LogEntry>
): LogEntry | undefined {
  return selectedId === null ? undefined : entries.get(selectedId);
}
