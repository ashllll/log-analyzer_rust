# Multi-Keyword Search User Guide

## Overview

The Multi-Keyword Search feature allows you to search log files using multiple keywords simultaneously with OR logic, fully aligned with Notepad++'s `|` separator behavior.

## Quick Start

### Basic Multi-Keyword Search

1. Enter multiple keywords separated by `|` in the search box:
   ```
   error|timeout|exception
   ```

2. Press Enter or click the Search button

3. All log entries containing **any** of the keywords will be displayed

### Search Logic

- **OR Logic**: Matches lines containing **at least one** keyword
- **Case-Insensitive**: By default, searches are case-insensitive
- **Regex Support**: Each keyword can be a regular expression pattern

## Features

### 1. Keyword Statistics Panel

After searching, a statistics panel appears showing:

- **Total Matches**: Total number of matching log entries
- **Search Duration**: Time taken to complete the search
- **Keyword Breakdown**: For each keyword:
  - Match count (number of entries containing this keyword)
  - Percentage of total matches
  - Visual progress bar

**Example**:
```
Search Statistics (OR Logic)
Total: 1,234 matches in 156ms

error      │████████████░░░░░░░░│ 45%  556 matches
timeout    │████████░░░░░░░░░░░░│ 35%  432 matches  
exception  │████░░░░░░░░░░░░░░░░│ 20%  246 matches
```

### 2. Multi-Keyword Highlighting

- All matched keywords are highlighted in each log entry
- Each keyword uses a distinct color for easy identification
- Highlighting works even in very long log lines (>1000 characters)

### 3. Smart Text Truncation

For log entries longer than 1000 characters:

- Text is intelligently truncated to show keyword context
- Matched keywords and surrounding text (±100 characters) are preserved
- Overlapping snippets are automatically merged
- An "Expand" button allows viewing the full text
- Performance protection: rendering degrades for entries with >20 keyword matches

### 4. International Support

- Available in English and Chinese
- Language can be switched in the application settings

## Usage Examples

### Example 1: Error Log Analysis

Search for different types of errors:
```
fatal|error|critical|panic
```

The statistics panel will show you which error type is most common.

### Example 2: Network Issues

Search for network-related problems:
```
timeout|connection.*refused|network.*error|dns.*fail
```

Note: You can use regex patterns like `connection.*refused` for more flexible matching.

### Example 3: User Activity

Track specific user actions:
```
user.*login|user.*logout|session.*created|session.*expired
```

## Tips and Best Practices

### 1. Ordering Keywords

- The statistics panel automatically sorts keywords by match count (highest first)
- You don't need to order keywords in your search query

### 2. Combining with Other Features

- Multi-keyword search works with:
  - Time range filters
  - Log level filters
  - File selection
  - Virtual scrolling for large result sets

### 3. Performance Considerations

- **Optimal**: 2-5 keywords
- **Good**: 6-10 keywords
- **Acceptable**: 11-20 keywords
- For >20 keywords, consider refining your search criteria

### 4. Regex Patterns

You can use regular expressions in any keyword:

```
error|warn(ing)?|fail(ed|ure)?
```

This matches: `error`, `warn`, `warning`, `fail`, `failed`, `failure`

### 5. Case Sensitivity

- Default: case-insensitive search
- To enable case-sensitive: use the case-sensitive toggle in the search options

## Understanding the Results

### Statistics Accuracy

- **Match Count**: Number of log entries containing this specific keyword
- **Total Matches**: Total unique log entries matching any keyword
- **Percentage**: (Keyword Match Count ÷ Total Matches) × 100%

**Important**: Since multiple keywords can match the same log entry, the sum of all keyword percentages may exceed 100%.

### Example Scenario

Search: `error|timeout`

Results:
- Total: 100 entries
- "error": 60 matches (60%)
- "timeout": 55 matches (55%)

This means:
- 60 entries contain "error"
- 55 entries contain "timeout"
- 15 entries contain both "error" AND "timeout" (60 + 55 - 100 = 15)

## Keyboard Shortcuts

- `Ctrl+F` or `Cmd+F`: Focus search box
- `Enter`: Execute search
- `Esc`: Clear search
- `Ctrl+K` or `Cmd+K`: Toggle statistics panel

## Troubleshooting

### No Results Found

- Check spelling of keywords
- Verify case-sensitivity setting
- Try removing one keyword at a time to isolate the issue
- Ensure regex patterns are valid

### Search is Slow

- Reduce the number of keywords
- Use more specific keywords (avoid very common words)
- Apply time range or file filters to reduce search scope

### Keywords Not Highlighted

- Check if "Enable Highlighting" option is enabled
- For very long lines (>5000 chars), highlighting may be disabled for performance
- Try the "Expand" button to view full text with highlighting

### Statistics Panel Not Showing

- Ensure the search has completed
- Click the expand button (▼) if the panel is collapsed
- Check if panel is scrolled off-screen

## Comparison with Notepad++

### What's the Same

✓ `|` separator for multiple keywords  
✓ OR logic matching  
✓ All keywords highlighted in results  
✓ Regex pattern support  

### What's Better

⭐ **Keyword Statistics**: See match count for each keyword (Notepad++ only shows total)  
⭐ **Async Search**: Non-blocking search with progress feedback (Notepad++ blocks UI)  
⭐ **Virtual Scrolling**: Handle millions of results smoothly (Notepad++ struggles with >100K)  
⭐ **Smart Truncation**: Keyword context preserved in long lines (Notepad++ may truncate)  
⭐ **Dark Mode**: Full theme support  

## FAQ

**Q: Can I use AND logic instead of OR?**  
A: Currently, multi-keyword search uses OR logic only. For AND logic, use regex: `(?=.*error)(?=.*timeout)`

**Q: How many keywords can I search at once?**  
A: No hard limit, but performance is optimal with 2-10 keywords.

**Q: Can I save my favorite searches?**  
A: This feature is planned for a future release.

**Q: Does it work offline?**  
A: Yes, all log analysis is performed locally on your machine.

## Related Documentation

- [README.md](../README.md) - General application documentation
- [API Documentation](./API.md) - Developer API reference
- [CHANGELOG.md](../CHANGELOG.md) - Version history and changes

## Support

For issues or feature requests, please:
- Check the [GitHub Issues](https://github.com/yourusername/log-analyzer/issues)
- Create a new issue with the "multi-keyword-search" label
