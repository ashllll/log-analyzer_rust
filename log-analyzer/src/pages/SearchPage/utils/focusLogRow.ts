export function restoreLogRowFocus(logId: number) {
  requestAnimationFrame(() => {
    document.querySelector<HTMLElement>(`[data-log-id="${logId}"]`)?.focus();
  });
}
