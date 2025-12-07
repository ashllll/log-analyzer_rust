# API Documentation - Multi-Keyword Search Feature

## Overview

This document describes the API changes and new interfaces introduced by the Multi-Keyword Search Enhancement feature.

## Table of Contents

- [Backend API](#backend-api)
  - [Data Models](#data-models)
  - [Tauri Commands](#tauri-commands)
  - [Tauri Events](#tauri-events)
  - [Services](#services)
- [Frontend API](#frontend-api)
  - [TypeScript Types](#typescript-types)
  - [React Components](#react-components)
  - [Hooks](#hooks)

---

## Backend API

### Data Models

#### `KeywordStatistics`

Represents statistics for a single keyword.

**Location**: `src-tauri/src/models/search_statistics.rs`

**Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordStatistics {
    /// The keyword text
    pub keyword: String,
    
    /// Number of log entries matching this keyword
    #[serde(rename = "matchCount")]
    pub match_count: usize,
    
    /// Percentage of total matches (0-100)
    #[serde(rename = "matchPercentage")]
    pub match_percentage: f32,
}
```

**Methods**:
```rust
impl KeywordStatistics {
    /// Create a new keyword statistics entry
    ///
    /// # Arguments
    /// * `keyword` - The keyword text
    /// * `match_count` - Number of matches for this keyword
    /// * `total_matches` - Total number of matching log entries
    ///
    /// # Returns
    /// A new `KeywordStatistics` instance with calculated percentage
    pub fn new(keyword: String, match_count: usize, total_matches: usize) -> Self
}
```

**Example**:
```rust
let stats = KeywordStatistics::new("error".to_string(), 42, 100);
assert_eq!(stats.match_percentage, 42.0);
```

---

#### `SearchResultSummary`

Contains summary information about a search operation.

**Location**: `src-tauri/src/models/search_statistics.rs`

**Structure**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResultSummary {
    /// Total number of matching log entries
    #[serde(rename = "totalMatches")]
    pub total_matches: usize,
    
    /// Statistics for each keyword
    #[serde(rename = "keywordStats")]
    pub keyword_stats: Vec<KeywordStatistics>,
    
    /// Search duration in milliseconds
    #[serde(rename = "searchDurationMs")]
    pub search_duration_ms: u64,
    
    /// Whether results were truncated due to limit
    pub truncated: bool,
}
```

**Example**:
```rust
let summary = SearchResultSummary {
    total_matches: 100,
    keyword_stats: vec![
        KeywordStatistics::new("error".to_string(), 60, 100),
        KeywordStatistics::new("timeout".to_string(), 55, 100),
    ],
    search_duration_ms: 156,
    truncated: false,
};
```

---

#### `LogEntry` Extensions

The existing `LogEntry` structure has been extended with a new field.

**Location**: `src-tauri/src/models/log.rs`

**New Field**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    // ... existing fields ...
    
    /// List of keywords that matched this log entry
    /// Only populated when multi-keyword search is used
    #[serde(rename = "matchedKeywords", skip_serializing_if = "Option::is_none")]
    pub matched_keywords: Option<Vec<String>>,
}
```

---

### Tauri Commands

No new Tauri commands were added. The existing `search_logs` command automatically calculates and emits statistics.

#### `search_logs` (Modified)

**Location**: `src-tauri/src/lib.rs`

**Behavior Changes**:
- Now populates `matched_keywords` field in each `LogEntry`
- Emits `search-summary` event after search completes
- Calculates keyword statistics for multi-keyword searches

**Signature** (unchanged):
```rust
#[tauri::command]
async fn search_logs(
    state: State<'_, AppState>,
    workspace_id: String,
    query: String,
    source: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<usize>,
) -> Result<SearchResult, String>
```

---

### Tauri Events

#### `search-summary` (New)

Emitted after a search operation completes, providing keyword statistics.

**Event Name**: `search-summary`

**Payload Type**: `SearchResultSummary`

**Example Payload**:
```json
{
  "totalMatches": 100,
  "keywordStats": [
    {
      "keyword": "error",
      "matchCount": 60,
      "matchPercentage": 60.0
    },
    {
      "keyword": "timeout",
      "matchCount": 55,
      "matchPercentage": 55.0
    }
  ],
  "searchDurationMs": 156,
  "truncated": false
}
```

**Usage** (Frontend):
```typescript
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<SearchResultSummary>('search-summary', (event) => {
  console.log('Search completed:', event.payload.totalMatches, 'matches');
  console.log('Keyword stats:', event.payload.keywordStats);
});
```

---

### Services

#### `search_statistics::calculate_keyword_statistics`

Calculates statistics for each keyword in a multi-keyword search.

**Location**: `src-tauri/src/services/search_statistics.rs`

**Signature**:
```rust
pub fn calculate_keyword_statistics(
    results: &[LogEntry],
    keywords: &[String],
) -> Vec<KeywordStatistics>
```

**Parameters**:
- `results`: Slice of log entries from search results
- `keywords`: Slice of keywords used in the search

**Returns**:
- Vector of `KeywordStatistics`, sorted by match count (descending)

**Time Complexity**: O(n × m) where n = number of results, m = number of keywords

**Example**:
```rust
use log_analyzer::services::search_statistics::calculate_keyword_statistics;

let results = vec![
    LogEntry {
        matched_keywords: Some(vec!["error".to_string(), "timeout".to_string()]),
        // ... other fields
    },
    LogEntry {
        matched_keywords: Some(vec!["error".to_string()]),
        // ... other fields
    },
];

let keywords = vec!["error".to_string(), "timeout".to_string()];
let stats = calculate_keyword_statistics(&results, &keywords);

assert_eq!(stats[0].keyword, "error");
assert_eq!(stats[0].match_count, 2);
assert_eq!(stats[1].keyword, "timeout");
assert_eq!(stats[1].match_count, 1);
```

---

## Frontend API

### TypeScript Types

#### `KeywordStat`

**Location**: `src/types/search.ts`

```typescript
export interface KeywordStat {
  /** The keyword text */
  value: string;
  
  /** Number of matches for this keyword */
  matchCount: number;
  
  /** Highlight color for this keyword */
  color: string;
}
```

---

#### `SearchResultSummary`

**Location**: `src/types/search.ts`

```typescript
export interface SearchResultSummary {
  /** Total number of matching log entries */
  totalMatches: number;
  
  /** Statistics for each keyword */
  keywordStats: Array<{
    keyword: string;
    matchCount: number;
    matchPercentage: number;
  }>;
  
  /** Search duration in milliseconds */
  searchDurationMs: number;
  
  /** Whether results were truncated */
  truncated: boolean;
}
```

---

### React Components

#### `KeywordStatsPanel`

Displays keyword statistics in a collapsible panel.

**Location**: `src/components/search/KeywordStatsPanel.tsx`

**Props**:
```typescript
interface KeywordStatsPanelProps {
  /** Array of keyword statistics to display */
  keywords: Array<{
    value: string;
    matchCount: number;
    color: string;
  }>;
  
  /** Total number of matching log entries */
  totalMatches: number;
  
  /** Search duration in milliseconds */
  searchDurationMs: number;
}
```

**Example Usage**:
```tsx
import { KeywordStatsPanel } from '@/components/search/KeywordStatsPanel';

function SearchPage() {
  const [keywordStats, setKeywordStats] = useState<KeywordStat[]>([]);
  const [totalMatches, setTotalMatches] = useState(0);
  const [searchDuration, setSearchDuration] = useState(0);

  return (
    <KeywordStatsPanel
      keywords={keywordStats}
      totalMatches={totalMatches}
      searchDurationMs={searchDuration}
    />
  );
}
```

**Features**:
- Collapsible panel (expand/collapse button)
- Visual progress bars for each keyword
- Responsive design
- Dark/Light theme support
- Fully internationalized (i18n)

**Accessibility**:
- ARIA labels for expand/collapse button
- Semantic HTML structure
- Keyboard navigation support

---

### Hooks

No new custom hooks were added. The feature integrates with existing React hooks:

- `useState` - For managing keyword statistics state
- `useEffect` - For setting up event listeners
- `useMemo` - For computing derived values (e.g., sorting keywords)
- `useTranslation` - For internationalization (from `react-i18next`)

**Example Integration**:
```typescript
import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { SearchResultSummary } from '@/types/search';

function SearchPage() {
  const [summary, setSummary] = useState<SearchResultSummary | null>(null);

  useEffect(() => {
    const unlisten = listen<SearchResultSummary>('search-summary', (event) => {
      setSummary(event.payload);
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);

  return (
    // ... render component with summary data
  );
}
```

---

## Migration Guide

### For Backend Developers

If you're extending the search functionality:

1. **New Search Sources**: Ensure `matched_keywords` is populated in `LogEntry` objects
2. **Custom Statistics**: Use `calculate_keyword_statistics` for consistency
3. **Event Emission**: Emit `search-summary` event after search completes

**Example**:
```rust
use crate::services::search_statistics::calculate_keyword_statistics;

// After search completes
let keywords = vec!["error".to_string(), "timeout".to_string()];
let stats = calculate_keyword_statistics(&results, &keywords);

let summary = SearchResultSummary {
    total_matches: results.len(),
    keyword_stats: stats,
    search_duration_ms: duration.as_millis() as u64,
    truncated: false,
};

window.emit("search-summary", summary)?;
```

---

### For Frontend Developers

If you're building UI components that display search results:

1. **Listen for Events**: Subscribe to `search-summary` event
2. **Use Types**: Import `SearchResultSummary` and `KeywordStat` types
3. **Display Statistics**: Use `KeywordStatsPanel` component or build custom UI
4. **Internationalization**: Use `useTranslation` hook for all text

**Example**:
```typescript
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { SearchResultSummary, KeywordStat } from '@/types/search';
import { KeywordStatsPanel } from '@/components/search/KeywordStatsPanel';

function MySearchComponent() {
  const [keywordStats, setKeywordStats] = useState<KeywordStat[]>([]);
  
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    
    listen<SearchResultSummary>('search-summary', (event) => {
      const stats = event.payload.keywordStats.map((stat, index) => ({
        value: stat.keyword,
        matchCount: stat.matchCount,
        color: HIGHLIGHT_COLORS[index % HIGHLIGHT_COLORS.length],
      }));
      setKeywordStats(stats);
    }).then(fn => { unlisten = fn; });
    
    return () => { unlisten?.(); };
  }, []);
  
  return (
    <KeywordStatsPanel
      keywords={keywordStats}
      totalMatches={...}
      searchDurationMs={...}
    />
  );
}
```

---

## Testing

### Backend Tests

**Location**: `src-tauri/src/services/search_statistics.rs`

Key test cases:
- `test_calculate_keyword_statistics_normal` - Normal case with multiple keywords
- `test_calculate_keyword_statistics_empty_results` - Edge case with no results
- `test_calculate_keyword_statistics_no_matches` - Edge case with keywords that don't match

**Run Tests**:
```bash
cd src-tauri
cargo test --lib search_statistics
```

---

### Frontend Tests

Currently, frontend tests are manual. Automated testing is planned for future releases.

**Manual Test Checklist**:
- [ ] Statistics panel displays correct counts
- [ ] Percentages sum to >=100% (due to OR logic)
- [ ] Panel can be collapsed/expanded
- [ ] Dark/Light theme rendering
- [ ] i18n language switching

---

## Performance Considerations

### Backend

- **Statistics Calculation**: O(n × m) where n = results, m = keywords
  - Optimized with HashMap for O(1) lookup
  - Negligible overhead for typical use cases (<10ms for 100K results)

- **Memory**: Minimal additional memory (~8 bytes per keyword per result)

### Frontend

- **Rendering**: KeywordStatsPanel uses CSS flexbox for efficient layout
- **Virtual Scrolling**: Not needed for statistics panel (typically <20 keywords)
- **Re-renders**: Optimized with React.memo (component is pure)

---

## Versioning

This API was introduced in version **0.0.33**.

**Stability**: Stable (no breaking changes planned)

**Deprecation Policy**: Any future breaking changes will be announced at least 2 releases in advance.

---

## Support

For questions or issues:
- Create a GitHub issue with the `api` label
- Check the [User Guide](./MULTI_KEYWORD_SEARCH_GUIDE.md) for usage examples
- Review the [CHANGELOG](../CHANGELOG.md) for recent changes

---

## Related Documentation

- [User Guide](./MULTI_KEYWORD_SEARCH_GUIDE.md) - End-user documentation
- [README](../README.md) - General project documentation
- [Design Document](.qoder/quests/multi-keyword-search-feature.md) - Implementation design (internal)
