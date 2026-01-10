# Ferrite Roadmap

## Known Issues 🐛

### Blocked by egui TextEdit
These issues cannot be fixed without replacing egui's built-in text editor:
- [ ] **Multi-cursor incomplete** - Basic cursor rendering works, but text operations not implemented
- [ ] **Code folding incomplete** - Detection works, but text hiding not possible
- [ ] **Scroll sync imperfect** - Limited access to egui's internal scroll state

---

## Planned Features 🚀

### v0.3.0 (Planned) - Mermaid Crate + Editor Improvements

> **Status:** Planning  
> **Docs:** [Mermaid Crate Plan](docs/mermaid-crate-plan.md) | [Custom Editor Plan](docs/technical/custom-editor-widget-plan.md) | [Modular Refactor Plan](docs/refactor.md)

v0.3.0 focuses on extracting the Mermaid renderer as a standalone crate and continuing diagram improvements.

#### 1. Mermaid Crate Extraction
Extract Ferrite's native Mermaid renderer (~6000 lines) into a standalone pure-Rust crate.

- [ ] **Standalone crate** - Backend-agnostic architecture with SVG, PNG, and egui outputs
- [ ] **Public API** - `parse()`, `layout()`, `render()` pipeline
- [ ] **SVG export** - Generate valid SVG files from diagrams
- [ ] **PNG export** - Rasterize via resvg
- [ ] **WASM compatible** - SVG backend works in browsers

#### 2. Mermaid Diagram Improvements
Continue improving diagram rendering quality:

##### Git Graph (Major Rewrite)
- [ ] **Horizontal timeline layout** - Left-to-right commit flow like Mermaid
- [ ] **Branch lanes** - Distinct horizontal lanes per branch with colored labels
- [ ] **Merge visualization** - Curved paths connecting branches
- [ ] **Tags and highlights** - Visual markers on commits

##### Flowchart
- [ ] **More node shapes** - Parallelogram, trapezoid, double-circle, etc.
- [ ] **Styling syntax** - `style` and `classDef` directives

##### State Diagram
- [ ] **Fork/join pseudostates** - Parallel regions
- [ ] **History states** - Shallow (H) and deep (H*) history

#### 3. Custom Editor Widget (Stretch Goal)
Replace egui's `TextEdit` with a custom `FerriteEditor` widget to unblock advanced editing features.

- [ ] **FerriteEditor widget** - Custom text editor using egui drawing primitives
- [ ] **Rope-based buffer** - Efficient text storage via `ropey` crate
- [ ] **Full multi-cursor editing** - Text operations at all cursor positions
- [ ] **Code folding with text hiding** - Actually collapse regions visually

#### Platform & Distribution
- [ ] **macOS app signing & notarization** - Create proper `.app` bundle, sign with Developer ID, notarize with Apple

### Future (v0.4.0+)
- [ ] Spell checking
- [ ] Custom themes (import/export)
- [ ] Virtual/ghost text (AI completions, etc.)
- [ ] Column/box selection

### Long-Term Vision

#### Headless Editor Library
Extract `FerriteEditor` as a standalone, framework-agnostic text editing library for the Rust ecosystem.

> **Context:** There's currently no general-purpose "headless" code editor library in Rust. Existing implementations (egui's TextEdit, Lapce/Floem, Zed/gpui) are tightly coupled to their UI frameworks. The v0.3.0 custom editor and modular architecture lay the groundwork for potential extraction.

**Prerequisites (from v0.3.0):**
- Custom `FerriteEditor` widget with rope-based buffer
- Modular architecture with clean separation of concerns
- Framework-agnostic core logic

**Extraction would involve:**
- [ ] Abstract rendering backend (trait-based: egui, wgpu, vello, SVG, etc.)
- [ ] Framework-agnostic input handling
- [ ] Standalone crate with minimal dependencies
- [ ] Integration with [Parley](https://github.com/linebender/parley) for advanced text layout/shaping (optional)

---

## Completed ✅

### v0.2.1 (Current Release)

#### Mermaid Diagram Enhancements
- [x] **Accurate text measurement** - Replace character-count estimation with egui font metrics
- [x] **Dynamic node sizing** - Nodes resize to fit their labels without clipping
- [x] **Text overflow handling** - Edge labels truncate with ellipsis when too long
- [x] **User Journey icons** - Fixed unsupported emoji rendering with text fallbacks
- [x] **Sequence control-flow blocks** - Support for `loop`, `alt`, `opt`, `par`, `critical`, `break` blocks with nesting
- [x] **Sequence activation boxes** - `activate`/`deactivate` markers and `+`/`-` shorthand on lifelines
- [x] **Sequence notes** - `Note left/right/over` syntax support with dog-ear rendering
- [x] **Flowchart branching layout** - Sugiyama-style layered graph with side-by-side branches
- [x] **Flowchart subgraphs** - Nested `subgraph`/`end` blocks with direction overrides
- [x] **Back-edge routing** - Cycle edges rendered with smooth bezier curves
- [x] **Smart edge exit points** - Decision node edges exit from different points to prevent crossing
- [x] **Composite/nested states** - `state Parent { ... }` syntax with recursive nesting
- [x] **Advanced state transitions** - Color-coded transitions and smart anchor points

### v0.2.0

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
