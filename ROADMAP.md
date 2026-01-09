# Ferrite Roadmap

## Known Bugs 🐛

### Low Priority
- [ ] **Multi-cursor incomplete** - Basic cursor rendering works, but text operations not implemented (deferred to v0.3.0)

---

## Planned Features 🚀

### v0.3.0 (Planned)
- [ ] **Code folding text hiding** - Actually hide collapsed regions (requires custom text editor integration)
- [ ] **Full multi-cursor editing** - Complete multi-cursor support with text operations (requires custom text editor)
- [ ] **Scroll sync perfection** - Further improve scroll synchronization accuracy between Raw and Rendered modes
- [ ] **Split view scroll sync** - Synchronized scrolling between raw editor and preview panes in split view
- [ ] **Split view preview editing** - Make edits in split view preview persist back to original content
- [ ] **Split view read-only indicator** - Visual indicator in preview pane that edits don't persist
- [ ] **Mermaid diagram optimization** - Performance improvements, subgraph support, advanced syntax features
- [ ] **Mermaid error recovery** - Show partial diagram on parse errors instead of failing completely
- [ ] **Search multi-line match tests** - Add test coverage for multi-line search result navigation

### Future
- [ ] Spell checking
- [ ] Custom themes (import/export)

---

## Completed ✅

### v0.2.0 (Current Release)

#### Major Features
- [x] **Side-by-side split view** - Raw editor on left, rendered preview on right with resizable divider
- [x] **MermaidJS native rendering** - 11 diagram types rendered natively in Rust/egui (flowchart, sequence, pie, state, mindmap, class, ER, git graph, gantt, timeline, user journey)
- [x] **Editor minimap** - VS Code-style scaled preview with click-to-navigate and viewport indicator
- [x] **Code folding indicators** - Fold detection for headings, code blocks, lists; gutter indicators (▶/▼)
- [x] **Live Pipeline panel** - Pipe JSON/YAML content through shell commands with real-time output
- [x] **Zen Mode** - Distraction-free writing with centered text column
- [x] **Git integration** - Visual status indicators in file tree (modified, added, untracked, ignored)
- [x] **Auto-save** - Configurable delay, per-tab toggle, temp-file based safety
- [x] **Session persistence** - Restore open tabs, cursor position, scroll offset, view mode on restart
- [x] **Bracket matching** - Highlight matching brackets `()[]{}<>` and markdown emphasis `**` `__`
- [x] **Syntax highlighting** - Full-file syntax highlighting for source code files (40+ languages including Rust, Python, JavaScript, Go, C/C++, etc.)

#### Bug Fixes
- [x] **Rendered mode list editing** - Fixed item index mapping, structural key hashing, edit state consistency
- [x] **Light mode contrast** - Improved text/border visibility, WCAG AA compliant, added tab/editor separator
- [x] **Scroll synchronization** - Bidirectional sync between Raw/Rendered, mode switch preservation
- [x] **Search-in-Files navigation** - Click result scrolls to match with transient highlight
- [x] **Search panel viewport** - Fixed top/bottom clipping issues

#### UX Improvements
- [x] **Tab context menu** - Reorganized icons with logical grouping

### v0.1.0

#### Core Features
- [x] WYSIWYG Markdown editing
- [x] Multi-format support (Markdown, JSON, YAML, TOML)
- [x] Tree viewer for structured data
- [x] Workspace mode with file tree
- [x] Quick switcher (Ctrl+P)
- [x] Search in files (Ctrl+Shift+F)
- [x] Light and dark themes
- [x] Document outline panel
- [x] HTML export
- [x] Formatting toolbar
- [x] Custom borderless window
- [x] Multi-tab editing
- [x] Find and replace
- [x] Undo/redo per tab

---

## Contributing

Found a bug or have a feature request? Please [open an issue](https://github.com/OlaProeis/Ferrite/issues/new/choose) on GitHub!
