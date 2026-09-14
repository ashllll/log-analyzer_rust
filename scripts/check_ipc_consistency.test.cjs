const test = require('node:test');
const assert = require('node:assert/strict');

const {
  extractBackendCommands,
  extractFrontendInvokes,
} = require('./check_ipc_consistency.cjs');

test('extracts commands routed through the API error wrapper', () => {
  const calls = extractFrontendInvokes(`
    return this.invokeWithErrorHandling(
      'export_results',
      { searchId, format, savePath },
      (raw) => raw as string
    );
  `);

  assert.equal(calls.length, 1);
  assert.equal(calls[0].name, 'export_results');
  assert.deepEqual(calls[0].args, ['searchId', 'format', 'savePath']);
});

test('extracts direct generic invoke calls', () => {
  const calls = extractFrontendInvokes(
    `invoke<string>('load_workspace', { workspaceId: id })`
  );

  assert.equal(calls[0].name, 'load_workspace');
  assert.deepEqual(calls[0].args, ['workspaceId']);
});

test('extracts tauri command parameters', () => {
  const commands = extractBackendCommands(`
    #[tauri::command]
    pub async fn export_results(
      state: State<'_, AppState>,
      #[allow(non_snake_case)] searchId: String,
      format: String
    ) {}
  `);

  assert.deepEqual(commands, [
    { name: 'export_results', params: ['state', 'searchId', 'format'] },
  ]);
});
