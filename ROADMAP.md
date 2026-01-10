# Ferrite Roadmap

## Known Issues 🐛

### Blocked by egui TextEdit
These issues cannot be fixed without replacing egui's built-in text editor:
- [ ] **Multi-cursor incomplete** - Basic cursor rendering works, but text operations not implemented
- [ ] **Code folding incomplete** - Detection works, but text hiding not possible
- [ ] **Scroll sync imperfect** - Limited access to egui's internal scroll state

---

## Planned Features 🚀

### v0.2.1 (In Progress) - Mermaid Diagram Improvements

> **Status:** Active development  
> **Focus:** Polish and complete native Mermaid diagram rendering

This patch release focuses on improving the native Mermaid diagram renderer added in v0.2.0:

#### Text & Layout Fixes
- [x] **Accurate text measurement** - Replace character-count estimation with egui font metrics
- [x] **Dynamic node sizing** - Nodes resize to fit their labels without clipping
- [x] **Text overflow handling** - Edge labels truncate with ellipsis when too long
- [x] **User Journey icons** - Fixed unsupported emoji rendering

#### Sequence Diagram Enhancements
- [x] **Control-flow blocks** - Support for `loop`, `alt`, `opt`, `par` blocks with nesting
- [ ] **Activation boxes** - `activate`/`deactivate` markers on lifelines
- [ ] **Notes** - `Note left/right/over` syntax support

#### Flowchart Improvements
- [ ] **Proper branching layout** - Fix single-column rendering, implement multi-path layout
- [ ] **Subgraph support** - Nested subgraphs with direction overrides

#### State Diagram Enhancements
- [ ] **Composite states** - Nested state machines
- [ ] **Advanced transitions** - Fork/join, choice pseudostates

---

### v0.3.0 (Planned) - Custom Editor + Modular Architecture

> **Status:** Collecting v0.2.0 feedback before implementation  
> **Docs:** [Custom Editor Plan](docs/technical/custom-editor-widget-plan.md) | [Modular Refactor Plan](docs/refactor.md)

v0.3.0 is a foundational release with two major architectural changes:

#### 1. Custom Editor Widget
Replace egui's `TextEdit` with a custom `FerriteEditor` widget to unblock advanced editing features.

- [ ] **FerriteEditor widget** - Custom text editor using egui drawing primitives
- [ ] **Rope-based buffer** - Efficient text storage via `ropey` crate
- [ ] **Full input handling** - Direct keyboard/mouse event processing
- [ ] **Full multi-cursor editing** - Text operations at all cursor positions
- [ ] **Code folding with text hiding** - Actually collapse regions visually
- [ ] **Perfect scroll sync** - Direct line-to-pixel mapping access

#### 2. Modular Architecture Refactor
Transform Ferrite from monolithic to "Core + Features" using Rust's compile-time feature flags.

- [ ] **Feature-gated dependencies** - `markdown`, `json`, `yaml`, `syntax_highlighting`, `git` as optional
- [ ] **DocumentView enum** - Type-safe file handling with `#[cfg(feature)]` variants
- [ ] **Feature switchboard** - Central file-type detection with graceful fallback to plain text
- [ ] **Directory restructure** - Move specialized code to `src/features/` module

**Benefits:**
- Compile minimal builds (`cargo build --no-default-features`)
- Faster dev cycles (disable unused features)
- Smaller binaries for users who don't need all formats
- Future WASM compatibility

#### Additional v0.3.0 Goals
- [ ] **Split view preview editing** - Make edits in preview persist

### Future (v0.4.0+)
- [ ] Spell checking
- [ ] Custom themes (import/export)
- [ ] Virtual/ghost text (AI completions, etc.)
- [ ] Column/box selection

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
