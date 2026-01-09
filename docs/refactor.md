Based on our discussion and your repository's goals, I have designed a comprehensive refactoring plan.

This plan focuses on **Compile-Time Modularity**. This means using Rust's `[features]` flags so that if a user (or you) compiles the app without a specific feature, the dependencies are never downloaded, and the code is never compiled into the binary.

Here is the strategy document.

***

# MODULAR_REFACTOR_PLAN.md

## 1. Goal & Philosophy
The goal is to transform **Ferrite** from a monolithic editor into a "Core + Plugins" architecture (where plugins are compile-time features).

*   **Core:** Filesystem I/O, Basic Text Buffer, Window Management, Theme System.
*   **Modules:** Markdown Preview, JSON Tree, Syntax Highlighting, Git Integration.

**The Golden Rule:** If a feature flag is disabled, the application treats those specific files as plain text. The binary size shrinks, and dependencies (like `syntect` or `serde`) are dropped.

---

## 2. Cargo.toml Restructuring
We need to define the boundaries in your manifest file. This separates the dependencies.

**Current (Conceptual):**
```toml
[dependencies]
eframe = "..."
serde_json = "..."
comrak = "..."
syntect = "..."
```

**Refactored `Cargo.toml`:**
```toml
[package]
name = "ferrite"
# ...

[features]
default = ["syntax_highlighting", "markdown", "json", "languages"]

# Core Features
syntax_highlighting = ["dep:syntect"]

# Language/Format Features
markdown = ["dep:comrak"]
json = ["dep:serde_json"]
yaml = ["dep:serde_yaml"] 
# "languages" aggregates all format support
languages = ["markdown", "json", "yaml"]

[dependencies]
# Core dependencies (Always required)
eframe = "0.29"
anyhow = "1.0"

# Optional Dependencies (Gated)
serde_json = { version = "1.0", optional = true }
comrak = { version = "0.18", optional = true }
syntect = { version = "5.0", optional = true }
serde_yaml = { version = "0.9", optional = true }
```

---

## 3. Directory Structure Refactor
Move code away from the root `src/` into a `features` folder. This keeps the `main.rs` clean and makes it easy to delete/disable modules.

```text
src/
├── main.rs
├── app.rs            <-- Main loop, generic UI
├── editor.rs         <-- The Plain Text Editor (The Core)
└── features/
    ├── mod.rs        <-- The "Switchboard"
    ├── markdown.rs   <-- #[cfg(feature = "markdown")]
    ├── json.rs       <-- #[cfg(feature = "json")]
    └── syntax.rs     <-- #[cfg(feature = "syntax_highlighting")]
```

---

## 4. The Code Architecture

### A. The Document State Enum
Instead of having `string_content`, `json_tree`, and `markdown_preview` living side-by-side in your struct, we make them **mutually exclusive** using an Enum.

**`src/app.rs`**
```rust
use crate::editor::TextEditor;

// This enum holds the specific state for the current tab
pub enum DocumentView {
    // Plain text is the fallback and always exists
    Plain(TextEditor),
    
    // These variants only exist if the feature is enabled
    #[cfg(feature = "markdown")]
    Markdown(crate::features::markdown::MarkdownViewer),

    #[cfg(feature = "json")]
    Json(crate::features::json::JsonEditor),
}

pub struct OpenedFile {
    pub path: std::path::PathBuf,
    pub view: DocumentView, // The modular part
    pub is_dirty: bool,
}
```

### B. The Feature Switchboard (File Detection)
We need a central function that decides how to open a file. This is where the logic "If JSON is disabled, open as Text" lives.

**`src/features/mod.rs`**
```rust
use std::path::Path;
use crate::app::DocumentView;
use crate::editor::TextEditor;

pub fn determine_view_type(path: &Path, content: &str) -> DocumentView {
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    // 1. Try JSON
    #[cfg(feature = "json")]
    if extension == "json" {
        // Attempt to parse. If valid, return Json view. 
        // If invalid, fall through to Plain text.
        if let Ok(json_state) = crate::features::json::JsonEditor::new(content) {
            return DocumentView::Json(json_state);
        }
    }

    // 2. Try Markdown
    #[cfg(feature = "markdown")]
    if extension == "md" {
        return DocumentView::Markdown(
            crate::features::markdown::MarkdownViewer::new(content)
        );
    }

    // 3. Fallback to Plain Text (Always compiles)
    DocumentView::Plain(TextEditor::new(content))
}
```

### C. The UI Update Loop
In your main `update` function, you simply match on the enum.

**`src/app.rs`**
```rust
impl eframe::App for FerriteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... window setup ...

        match &mut self.active_document.view {
            DocumentView::Plain(editor) => {
                editor.ui(ui);
            }
            
            #[cfg(feature = "markdown")]
            DocumentView::Markdown(md_view) => {
                md_view.ui(ui);
            }

            #[cfg(feature = "json")]
            DocumentView::Json(json_view) => {
                json_view.ui(ui);
            }
        }
    }
}
```

---

## 5. Handling "Sync"
You mentioned you sync the views constantly.
Since we are using an Enum (`DocumentView`), you usually can't be in `Plain` mode and `Json` mode at the exact same time *in the same variable*.

**Strategy:**
1.  **Shared Buffer:** The `DocumentView` variants should probably own the specialized state (like the Tree expansion state), but they should perhaps share or clone the raw text.
2.  **Toggle Mode:** Add a button in the UI to switch views.
    *   *Action:* User clicks "View Source" while in JSON mode.
    *   *Code:* Serialize the JSON tree back to string, drop the `DocumentView::Json` variant, and create a `DocumentView::Plain` variant with that string.

---

## 6. Implementation Steps

1.  **Backup:** Create a `refactor-modularity` branch.
2.  **Manifest:** Edit `Cargo.toml` to add the features and set dependencies to `optional = true`.
3.  **Breakage:** The code will immediately break because usages of `serde_json` or `comrak` are now undefined if the feature isn't explicitly enabled.
4.  **Isolation:** Move the JSON logic to `src/features/json.rs`. Wrap the whole file (or module) in `#![cfg(feature = "json")]`.
5.  **Integration:** Implement the `DocumentView` enum in `app.rs`.
6.  **Fix UI:** Update the main `update` loop to use the `match` statement with `#[cfg]` guards.
7.  **Test:**
    *   Run `cargo run` (All features default).
    *   Run `cargo run --no-default-features` (Should be a lightweight notepad).
    *   Run `cargo run --no-default-features --features json` (Only JSON tools enabled).

## 7. Future Proofing (The Roadmap)
*   **Git:** Add a `git` feature using `git2` (optional dependency).
*   **Wasm:** Since generic dependencies (like `std::fs`) don't work in WebAssembly, this modular approach allows you to disable file-system heavy features easily when compiling for `target_arch = "wasm32"`.