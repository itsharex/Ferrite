# Sync Scrolling Implementation

## Overview

Sync scrolling maintains the user's document position when switching between Raw and Rendered view modes. When toggling view modes (Ctrl+E), the scroll position is synchronized using a **hybrid approach**:
- **Boundaries (top/bottom)**: Percentage-based to ensure you stay at document edges
- **Middle content**: Line-based with interpolation to show the same content

## Architecture

### Key Components

1. **Tab State** (`src/state.rs`):
   - `scroll_offset: f32` - Current scroll position in pixels
   - `content_height: f32` - Total content height of scroll area
   - `viewport_height: f32` - Visible viewport height
   - `pending_scroll_offset: Option<f32>` - Target scroll offset to apply
   - `pending_scroll_ratio: Option<f32>` - Target scroll ratio (0.0 to 1.0)
   - `pending_scroll_to_line: Option<usize>` - Target line for line-based sync
   - `raw_line_height: f32` - Actual line height in Raw mode
   - `rendered_line_mappings: Vec<(usize, usize, f32)>` - Line-to-Y position mappings

2. **Mode Toggle** (`src/app.rs`):
   - `handle_toggle_view_mode()` - Determines sync strategy and stores target
   - `find_rendered_y_for_line_interpolated()` - Precise line-to-Y lookup
   - `find_source_line_for_rendered_y_interpolated()` - Y-to-line reverse lookup

3. **Editors**:
   - `EditorWidget` (`src/editor/widget.rs`) - Raw text editor, tracks `raw_line_height`
   - `MarkdownEditor` (`src/markdown/editor.rs`) - Rendered WYSIWYG editor, builds line mappings

### Settings

- `sync_scroll_enabled: bool` in `Settings` - User preference for sync scrolling
- Toggled via ribbon button or settings panel

## Hybrid Scroll Sync Algorithm

### Decision Logic

```rust
if at_top {
    // Within 5px of top - snap to top
    pending_scroll_offset = Some(0.0);
} else if at_bottom {
    // Within 5px of max scroll - use ratio=1.0 to stay at bottom
    pending_scroll_ratio = Some(1.0);
} else {
    // In the middle - use line-based mapping for content preservation
    // Raw→Rendered: Store target line
    // Rendered→Raw: Look up line from Y position, calculate raw offset
}
```

### Line-Based with Interpolation

The key innovation is **interpolating within elements** for sub-element precision:

```rust
fn find_rendered_y_for_line_interpolated(mappings, target_line, content_height) {
    // Find element containing target_line
    for (i, (start, end, y)) in mappings {
        if target_line in start..=end {
            // Calculate element height
            let element_height = next_mapping.y - y;
            
            // Interpolate within element
            let progress = (target_line - start) / (end - start + 1);
            return y + progress * element_height;
        }
    }
}
```

This ensures that if you're looking at line 150 (which is 50% through a code block spanning lines 100-200), you'll see the same line 150 in the other mode, not just the start of the code block.

### Two-Frame Application

```
Frame 1: Render, build mappings, convert pending_scroll_to_line → pending_scroll_offset
Frame 2: Apply pending_scroll_offset via ScrollArea
```

## Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                     User Presses Ctrl+E                          │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ handle_toggle_view_mode()                                        │
│   if at_top → pending_scroll_offset = 0                          │
│   if at_bottom → pending_scroll_ratio = 1.0                      │
│   else (middle):                                                 │
│     Raw→Rendered: pending_scroll_to_line = topmost_line          │
│     Rendered→Raw: pending_scroll_offset = line × line_height     │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Frame 1: Render New Mode                                         │
│   1. Render content, build fresh line mappings                   │
│   2. If pending_scroll_to_line:                                  │
│      - Use interpolation to find exact Y position                │
│      - Store as pending_scroll_offset                            │
│   3. If pending_scroll_ratio:                                    │
│      - Convert to offset using actual content_height             │
│   4. Request repaint                                             │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Frame 2: Apply Scroll Position                                   │
│   Apply pending_scroll_offset via vertical_scroll_offset()       │
└─────────────────────────────────────────────────────────────────┘
```

## Why This Hybrid Approach?

| Scenario | Pure Percentage | Pure Line-Based | Hybrid |
|----------|----------------|-----------------|--------|
| At top | ✓ Stays at top | ✓ | ✓ |
| At bottom | ✓ Stays at bottom | ✗ May be 75% down | ✓ |
| In middle | ✗ Different content | ✓ Same content | ✓ |

The hybrid approach gives the best of both worlds:
- **Boundary preservation**: Top/bottom stay at edges
- **Content preservation**: Middle content stays visible

## Edge Cases

### No Scrollable Content
If `content_height <= viewport_height`, there's nothing to scroll, so sync is skipped.

### Boundary Detection
- **Top boundary**: Within 5px of scroll=0 → snap to top
- **Bottom boundary**: Within 5px of max_scroll → use ratio=1.0 to stay at bottom

### Empty or Missing Line Mappings
On first toggle to Rendered mode, mappings may be empty. The system falls back to:
1. Estimate line ratio: `target_line / total_lines`
2. Convert to scroll offset: `line_ratio × max_scroll`

### Large Elements
The interpolation handles large elements (like code blocks spanning 100+ lines) by calculating progress within the element, ensuring sub-element precision.

## Future Enhancements

The `SyncScrollState` infrastructure in `src/preview/sync_scroll.rs` is designed for future split-view support:

1. **Split View**: Show Raw and Rendered side-by-side with real-time sync
2. **Bidirectional Sync**: Scrolling either pane updates the other
3. **Debouncing**: Prevent feedback loops in split-view

## Testing

### Manual Test Scenarios

1. **Basic Toggle**:
   - Open a markdown file with 100+ lines
   - Scroll to middle (50%)
   - Toggle view mode (Ctrl+E)
   - Verify position is approximately maintained

2. **Edge Positions**:
   - Test at top (0%), middle (50%), bottom (100%)
   - Toggle back and forth

3. **Different Content Types**:
   - Documents with many headings
   - Documents with code blocks
   - Documents with nested lists

4. **Sync Scrolling Disabled**:
   - Disable sync scrolling in settings
   - Toggle view mode
   - Verify scroll position resets to 0

## Related Files

- `src/state.rs` - Tab struct with scroll state
- `src/app.rs` - View mode toggle logic
- `src/editor/widget.rs` - Raw editor scroll handling
- `src/markdown/editor.rs` - Rendered editor scroll handling
- `src/preview/sync_scroll.rs` - Sync scroll infrastructure (future)
- `src/config/settings.rs` - sync_scroll_enabled setting
