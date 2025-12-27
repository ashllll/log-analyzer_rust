# Legacy Format Detection API

## Overview

The legacy format detection system helps users transition from the old path_map-based workspace format to the new Content-Addressable Storage (CAS) architecture.

## Backend Commands

### 1. `scan_legacy_formats`

Scans for all legacy workspace formats in the indices directory.

**Usage:**

```typescript
import { invoke } from '@tauri-apps/api/tauri';

interface LegacyDetectionResponse {
  has_legacy_workspaces: boolean;
  count: number;
  message: string;
  workspace_ids: string[];
}

const response: LegacyDetectionResponse = await invoke('scan_legacy_formats');
```

**Response:**

```typescript
{
  has_legacy_workspaces: true,
  count: 2,
  message: "⚠️  Legacy Workspace Format Detected\n\nWe found 2 workspace(s)...",
  workspace_ids: ["production-logs", "test-workspace"]
}
```

**When to use:**
- On application startup
- In settings/workspace management page
- Before displaying workspace list

### 2. `get_legacy_workspace_info`

Checks if a specific workspace uses legacy format.

**Usage:**

```typescript
import { invoke } from '@tauri-apps/api/tauri';

interface LegacyWorkspaceInfo {
  workspace_id: string;
  index_path: string;
  format_type: 'CompressedIndex' | 'UncompressedIndex';
}

const info: LegacyWorkspaceInfo | null = await invoke('get_legacy_workspace_info', {
  workspaceId: 'my-workspace'
});
```

**Response:**

```typescript
// Legacy workspace
{
  workspace_id: "production-logs",
  index_path: "/path/to/indices/production-logs.idx.gz",
  format_type: "CompressedIndex"
}

// CAS workspace (no legacy format)
null
```

**When to use:**
- Before opening a workspace
- In workspace validation logic
- When displaying workspace details

## Frontend Integration Examples

### Example 1: Startup Check

```typescript
// In App.tsx or main component
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

function App() {
  const [legacyWarning, setLegacyWarning] = useState<string | null>(null);

  useEffect(() => {
    checkLegacyWorkspaces();
  }, []);

  async function checkLegacyWorkspaces() {
    try {
      const response = await invoke('scan_legacy_formats');
      if (response.has_legacy_workspaces) {
        setLegacyWarning(response.message);
      }
    } catch (error) {
      console.error('Failed to scan legacy formats:', error);
    }
  }

  return (
    <div>
      {legacyWarning && (
        <LegacyWarningModal 
          message={legacyWarning}
          onClose={() => setLegacyWarning(null)}
        />
      )}
      {/* Rest of app */}
    </div>
  );
}
```

### Example 2: Workspace Validation

```typescript
// In workspace opening logic
async function openWorkspace(workspaceId: string) {
  try {
    // Check if workspace uses legacy format
    const legacyInfo = await invoke('get_legacy_workspace_info', {
      workspaceId
    });

    if (legacyInfo) {
      // Show error - cannot open legacy workspace
      showError(
        'Legacy Format Not Supported',
        `Workspace "${workspaceId}" uses an old format that is no longer supported. ` +
        'Please create a new workspace and re-import your data.'
      );
      return;
    }

    // Proceed with opening workspace
    await invoke('load_workspace', { workspaceId });
    
  } catch (error) {
    console.error('Failed to open workspace:', error);
  }
}
```

### Example 3: Settings Page

```typescript
// In settings or workspace management page
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

function WorkspaceSettings() {
  const [legacyWorkspaces, setLegacyWorkspaces] = useState<string[]>([]);

  useEffect(() => {
    scanForLegacy();
  }, []);

  async function scanForLegacy() {
    const response = await invoke('scan_legacy_formats');
    if (response.has_legacy_workspaces) {
      setLegacyWorkspaces(response.workspace_ids);
    }
  }

  return (
    <div>
      <h2>Workspace Management</h2>
      
      {legacyWorkspaces.length > 0 && (
        <div className="warning-banner">
          <h3>⚠️ Legacy Workspaces Detected</h3>
          <p>
            The following workspaces use an old format and cannot be opened:
          </p>
          <ul>
            {legacyWorkspaces.map(id => (
              <li key={id}>{id}</li>
            ))}
          </ul>
          <button onClick={showMigrationGuide}>
            View Migration Guide
          </button>
        </div>
      )}
      
      {/* Rest of settings */}
    </div>
  );
}
```

### Example 4: Migration Guide Modal

```typescript
interface LegacyWarningModalProps {
  message: string;
  onClose: () => void;
}

function LegacyWarningModal({ message, onClose }: LegacyWarningModalProps) {
  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <h2>⚠️ Legacy Workspace Format Detected</h2>
        
        <div className="message-content">
          <pre>{message}</pre>
        </div>

        <div className="action-buttons">
          <button onClick={onClose} className="primary">
            I Understand
          </button>
          <button onClick={openDocumentation} className="secondary">
            Learn More
          </button>
        </div>
      </div>
    </div>
  );
}
```

## User Experience Recommendations

### 1. Startup Notification

- Show a non-blocking notification if legacy workspaces are detected
- Don't block the user from using the app
- Provide a "Don't show again" option (store in local settings)

### 2. Workspace List

- Mark legacy workspaces with a warning icon
- Disable the "Open" button for legacy workspaces
- Show a tooltip explaining the issue

### 3. Migration Guide

- Provide clear step-by-step instructions
- Highlight benefits of the new format
- Offer to create a new workspace

### 4. Error Messages

When a user tries to open a legacy workspace:

```
❌ Cannot Open Workspace

This workspace uses an old format that is no longer supported.

What you need to do:
1. Create a new workspace
2. Re-import your log files or archives
3. The old data will be automatically cleaned up

Benefits of the new format:
• Automatic deduplication saves storage
• Faster search performance
• Better reliability

[Create New Workspace] [Cancel]
```

## Automatic Cleanup

The system automatically detects and logs legacy workspaces on startup. The legacy index files (`.idx.gz` and `.idx`) will be removed when:

1. The workspace is deleted using `delete_workspace` command
2. Manual cleanup is triggered (future feature)

## Testing

### Manual Testing

1. Create test legacy files:
```bash
# In app data directory
mkdir -p indices
echo "dummy" > indices/test-workspace.idx.gz
echo "dummy" > indices/another-workspace.idx
```

2. Start the application and check logs for detection messages

3. Call the commands from frontend and verify responses

### Automated Testing

Backend tests are located in:
- `src/utils/legacy_detection.rs` - Unit tests
- `src/commands/legacy.rs` - Command tests

Run tests:
```bash
cargo test legacy
```

## Logging

The system logs legacy workspace detection:

```
[WARN] Legacy workspace formats detected:
⚠️  Legacy Workspace Format Detected
...

[WARN] Workspace production-logs uses legacy format
```

Check application logs for these messages during development and troubleshooting.

## Future Enhancements

Potential improvements:

1. **Automatic Migration Tool**: One-click migration from legacy to CAS format
2. **Batch Cleanup**: Remove all legacy files at once
3. **Migration Progress**: Show progress during re-import
4. **Backup Creation**: Automatically backup legacy data before cleanup
5. **Telemetry**: Track how many users have legacy workspaces

## Support

For issues or questions:
- Check application logs for detailed error messages
- Verify file permissions in the indices directory
- Ensure the application has write access to clean up files
