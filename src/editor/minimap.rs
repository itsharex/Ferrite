//! Minimap Navigation Panel for Ferrite
//!
//! This module provides a VS Code-style minimap that shows a zoomed-out view
//! of the entire document on the right side of the editor. The minimap supports:
//!
//! - Scaled-down document preview with character density visualization
//! - Click-to-navigate functionality
//! - Draggable viewport indicator
//! - Search result highlight visualization
//! - Full theme integration (light/dark mode)
//!
//! # Usage
//!
//! ```ignore
//! use crate::editor::Minimap;
//!
//! let minimap = Minimap::new(content)
//!     .width(80.0)
//!     .scroll_offset(current_scroll)
//!     .viewport_height(visible_height)
//!     .content_height(total_height)
//!     .search_highlights(&matches)
//!     .theme_colors(colors);
//!
//! let output = minimap.show(ui);
//! if let Some(scroll_to) = output.scroll_to_offset {
//!     // Navigate to the requested scroll position
//! }
//! ```

// MinimapSettings is available for future settings UI integration
#![allow(dead_code)]

use crate::theme::ThemeColors;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Default width of the minimap in pixels
const DEFAULT_MINIMAP_WIDTH: f32 = 80.0;

/// Minimum width for the minimap
const MIN_MINIMAP_WIDTH: f32 = 40.0;

/// Maximum width for the minimap
const MAX_MINIMAP_WIDTH: f32 = 150.0;

/// Scale factor for text in the minimap (pixels per character)
/// Higher value = wider lines filling more horizontal space
const MINIMAP_CHAR_SCALE: f32 = 0.8;

/// Line height in the minimap (pixels)
const MINIMAP_LINE_HEIGHT: f32 = 2.0;

/// Horizontal padding inside the minimap
const MINIMAP_PADDING: f32 = 4.0;

/// Maximum number of lines to render for performance
const MAX_LINES_TO_RENDER: usize = 10000;

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Output
// ─────────────────────────────────────────────────────────────────────────────

/// Output from the minimap widget
#[derive(Debug, Clone, Default)]
pub struct MinimapOutput {
    /// If set, the editor should scroll to this offset
    pub scroll_to_offset: Option<f32>,
    /// Whether the minimap was clicked
    pub clicked: bool,
    /// Whether the minimap is being dragged
    pub dragging: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Widget
// ─────────────────────────────────────────────────────────────────────────────

/// A VS Code-style minimap widget showing a zoomed-out document preview.
///
/// The minimap displays the entire document in a compressed format, allowing
/// users to quickly navigate large documents by clicking or dragging on the
/// minimap surface.
pub struct Minimap<'a> {
    /// The document content to display
    content: &'a str,
    /// Width of the minimap in pixels
    width: f32,
    /// Current vertical scroll offset in the editor
    scroll_offset: f32,
    /// Height of the visible viewport in the editor
    viewport_height: f32,
    /// Total height of the document content in the editor
    content_height: f32,
    /// Line height in the editor (for scroll calculations)
    line_height: f32,
    /// Theme colors for styling
    theme_colors: Option<ThemeColors>,
    /// Search matches to highlight (start, end byte positions)
    search_highlights: Option<&'a [(usize, usize)]>,
    /// Current search match index (for distinct highlighting)
    current_match: usize,
}

impl<'a> Minimap<'a> {
    /// Create a new minimap widget for the given content.
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            width: DEFAULT_MINIMAP_WIDTH,
            scroll_offset: 0.0,
            viewport_height: 100.0,
            content_height: 100.0,
            line_height: 16.0,
            theme_colors: None,
            search_highlights: None,
            current_match: 0,
        }
    }

    /// Set the width of the minimap.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_MINIMAP_WIDTH, MAX_MINIMAP_WIDTH);
        self
    }

    /// Set the current scroll offset.
    #[must_use]
    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the viewport height.
    #[must_use]
    pub fn viewport_height(mut self, height: f32) -> Self {
        self.viewport_height = height;
        self
    }

    /// Set the total content height.
    #[must_use]
    pub fn content_height(mut self, height: f32) -> Self {
        self.content_height = height;
        self
    }

    /// Set the line height for scroll calculations.
    #[must_use]
    pub fn line_height(mut self, height: f32) -> Self {
        self.line_height = height;
        self
    }

    /// Set the theme colors for styling.
    #[must_use]
    pub fn theme_colors(mut self, colors: ThemeColors) -> Self {
        self.theme_colors = Some(colors);
        self
    }

    /// Set search highlights to display.
    #[must_use]
    pub fn search_highlights(mut self, matches: &'a [(usize, usize)]) -> Self {
        self.search_highlights = Some(matches);
        self
    }

    /// Set the current match index for distinct highlighting.
    #[must_use]
    pub fn current_match(mut self, index: usize) -> Self {
        self.current_match = index;
        self
    }

    /// Show the minimap widget and return the output.
    pub fn show(self, ui: &mut Ui) -> MinimapOutput {
        let mut output = MinimapOutput::default();

        // Determine colors based on theme
        let is_dark = self.theme_colors.as_ref().map(|c| c.is_dark()).unwrap_or(false);
        let colors = MinimapColors::new(is_dark);

        // Calculate minimap dimensions
        let line_count = self.content.lines().count().max(1);
        let minimap_content_height = (line_count as f32 * MINIMAP_LINE_HEIGHT).max(1.0);
        let available_height = ui.available_height();

        // Calculate scale factor to fit content in available height
        let scale = if minimap_content_height > available_height {
            available_height / minimap_content_height
        } else {
            1.0
        };

        let minimap_height = (minimap_content_height * scale).min(available_height);

        // Allocate space for the minimap
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(self.width, available_height),
            Sense::click_and_drag(),
        );

        // Draw background
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, colors.background);

        // Draw left border
        painter.line_segment(
            [rect.left_top(), rect.left_bottom()],
            Stroke::new(1.0, colors.border),
        );

        // Build line information for rendering
        let lines: Vec<&str> = self.content.lines().take(MAX_LINES_TO_RENDER).collect();
        let content_rect = Rect::from_min_size(
            rect.min + Vec2::new(MINIMAP_PADDING, 0.0),
            Vec2::new(self.width - MINIMAP_PADDING * 2.0, minimap_height),
        );

        // Draw document lines
        self.render_lines(&painter, &lines, content_rect, scale, &colors);

        // Draw search highlights
        if let Some(highlights) = self.search_highlights {
            self.render_search_highlights(
                &painter,
                highlights,
                content_rect,
                scale,
                &colors,
            );
        }

        // Draw viewport indicator
        let viewport_rect = self.calculate_viewport_rect(content_rect, scale, line_count);
        if let Some(vp_rect) = viewport_rect {
            painter.rect_filled(vp_rect, 2.0, colors.viewport_fill);
            painter.rect_stroke(vp_rect, 2.0, Stroke::new(1.0, colors.viewport_border));
        }

        // Handle interaction
        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                output.scroll_to_offset = Some(self.calculate_scroll_offset(
                    pos,
                    content_rect,
                    scale,
                    line_count,
                ));
                output.clicked = response.clicked();
                output.dragging = response.dragged();
            }
        }

        // Show hover cursor
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        output
    }

    /// Render document lines in the minimap.
    fn render_lines(
        &self,
        painter: &egui::Painter,
        lines: &[&str],
        content_rect: Rect,
        scale: f32,
        colors: &MinimapColors,
    ) {
        let usable_width = content_rect.width();
        let char_width = MINIMAP_CHAR_SCALE * scale;
        let line_height = MINIMAP_LINE_HEIGHT * scale;

        for (line_idx, line) in lines.iter().enumerate() {
            let y = content_rect.min.y + line_idx as f32 * line_height;

            if y > content_rect.max.y {
                break;
            }

            // Determine line color based on content
            let color = self.get_line_color(line, colors);

            // Calculate line width based on character count
            let char_count = line.chars().count();
            let line_width = (char_count as f32 * char_width).min(usable_width);

            if line_width > 0.5 {
                let line_rect = Rect::from_min_size(
                    Pos2::new(content_rect.min.x, y),
                    Vec2::new(line_width, line_height.max(1.0)),
                );
                painter.rect_filled(line_rect, 0.0, color);
            }
        }
    }

    /// Get the color for a line based on its content (simplified syntax highlighting).
    fn get_line_color(&self, line: &str, colors: &MinimapColors) -> Color32 {
        let trimmed = line.trim();

        // Heading detection
        if trimmed.starts_with('#') {
            return colors.heading;
        }

        // Code block markers
        if trimmed.starts_with("```") {
            return colors.code_marker;
        }

        // List items (check before comments since `* item` is a list, not a comment)
        if (trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+'))
            && trimmed.chars().nth(1).map(|c| c.is_whitespace()).unwrap_or(false)
        {
            return colors.list;
        }

        // Numbered list items
        if trimmed.len() > 1
            && trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && trimmed.contains('.')
        {
            return colors.list;
        }

        // Comments (common patterns in code)
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with(" *") {
            return colors.comment;
        }

        // Blockquotes
        if trimmed.starts_with('>') {
            return colors.blockquote;
        }

        // Links or images
        if trimmed.contains("](") || trimmed.starts_with('[') {
            return colors.link;
        }

        // Empty lines get a lighter color
        if trimmed.is_empty() {
            return colors.empty_line;
        }

        // Default text color
        colors.text
    }

    /// Render search highlight indicators in the minimap.
    fn render_search_highlights(
        &self,
        painter: &egui::Painter,
        highlights: &[(usize, usize)],
        content_rect: Rect,
        scale: f32,
        colors: &MinimapColors,
    ) {
        let line_height = MINIMAP_LINE_HEIGHT * scale;

        // Build a map of byte offset to line number
        let mut line_offsets: Vec<usize> = vec![0];
        let mut current_offset = 0;
        for line in self.content.lines() {
            current_offset += line.len() + 1; // +1 for newline
            line_offsets.push(current_offset);
        }

        for (idx, &(start, _end)) in highlights.iter().enumerate() {
            // Find which line this match is on
            let line_idx = line_offsets
                .iter()
                .position(|&offset| offset > start)
                .map(|pos| pos.saturating_sub(1))
                .unwrap_or(0);

            let y = content_rect.min.y + line_idx as f32 * line_height;

            if y > content_rect.max.y {
                continue;
            }

            // Highlight indicator on the right edge
            let is_current = idx == self.current_match;
            let color = if is_current {
                colors.current_match
            } else {
                colors.other_match
            };

            let indicator_width = if is_current { 4.0 } else { 3.0 };
            let indicator_rect = Rect::from_min_size(
                Pos2::new(
                    content_rect.max.x - indicator_width,
                    y - line_height * 0.5,
                ),
                Vec2::new(indicator_width, line_height * 2.0),
            );
            painter.rect_filled(indicator_rect, 1.0, color);
        }
    }

    /// Calculate the viewport indicator rectangle.
    fn calculate_viewport_rect(
        &self,
        content_rect: Rect,
        scale: f32,
        line_count: usize,
    ) -> Option<Rect> {
        if self.content_height <= 0.0 {
            return None;
        }

        let minimap_content_height = line_count as f32 * MINIMAP_LINE_HEIGHT * scale;
        let scroll_ratio = self.scroll_offset / self.content_height.max(1.0);
        let viewport_ratio = self.viewport_height / self.content_height.max(1.0);

        let viewport_y = content_rect.min.y + scroll_ratio * minimap_content_height;
        let viewport_height = (viewport_ratio * minimap_content_height).max(10.0);

        Some(Rect::from_min_size(
            Pos2::new(content_rect.min.x - 2.0, viewport_y),
            Vec2::new(content_rect.width() + 4.0, viewport_height),
        ))
    }

    /// Calculate scroll offset from a click/drag position.
    fn calculate_scroll_offset(
        &self,
        pos: Pos2,
        content_rect: Rect,
        scale: f32,
        line_count: usize,
    ) -> f32 {
        let minimap_content_height = line_count as f32 * MINIMAP_LINE_HEIGHT * scale;
        let click_ratio = (pos.y - content_rect.min.y) / minimap_content_height.max(1.0);

        // Center the viewport on the clicked position
        let target_ratio = click_ratio - (self.viewport_height / self.content_height / 2.0);
        let target_offset = target_ratio * self.content_height;

        // Clamp to valid range
        target_offset.clamp(0.0, (self.content_height - self.viewport_height).max(0.0))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors used for rendering the minimap.
struct MinimapColors {
    background: Color32,
    border: Color32,
    text: Color32,
    heading: Color32,
    code_marker: Color32,
    comment: Color32,
    list: Color32,
    blockquote: Color32,
    link: Color32,
    empty_line: Color32,
    viewport_fill: Color32,
    viewport_border: Color32,
    current_match: Color32,
    other_match: Color32,
}

impl MinimapColors {
    fn new(is_dark: bool) -> Self {
        if is_dark {
            Self {
                background: Color32::from_rgb(25, 25, 25),
                border: Color32::from_rgb(50, 50, 50),
                text: Color32::from_rgba_unmultiplied(180, 180, 180, 200),
                heading: Color32::from_rgba_unmultiplied(100, 180, 255, 220),
                code_marker: Color32::from_rgba_unmultiplied(150, 120, 200, 200),
                comment: Color32::from_rgba_unmultiplied(100, 110, 120, 180),
                list: Color32::from_rgba_unmultiplied(120, 200, 120, 200),
                blockquote: Color32::from_rgba_unmultiplied(140, 140, 160, 180),
                link: Color32::from_rgba_unmultiplied(100, 180, 255, 180),
                empty_line: Color32::from_rgba_unmultiplied(60, 60, 60, 100),
                viewport_fill: Color32::from_rgba_unmultiplied(100, 100, 120, 40),
                viewport_border: Color32::from_rgba_unmultiplied(100, 180, 255, 150),
                current_match: Color32::from_rgba_unmultiplied(255, 200, 0, 255),
                other_match: Color32::from_rgba_unmultiplied(255, 200, 0, 120),
            }
        } else {
            Self {
                background: Color32::from_rgb(245, 245, 245),
                border: Color32::from_rgb(200, 200, 200),
                text: Color32::from_rgba_unmultiplied(80, 80, 80, 200),
                heading: Color32::from_rgba_unmultiplied(0, 90, 165, 220),
                code_marker: Color32::from_rgba_unmultiplied(120, 80, 160, 200),
                comment: Color32::from_rgba_unmultiplied(120, 120, 120, 180),
                list: Color32::from_rgba_unmultiplied(60, 140, 60, 200),
                blockquote: Color32::from_rgba_unmultiplied(100, 100, 120, 180),
                link: Color32::from_rgba_unmultiplied(0, 100, 200, 180),
                empty_line: Color32::from_rgba_unmultiplied(200, 200, 200, 100),
                viewport_fill: Color32::from_rgba_unmultiplied(100, 150, 200, 50),
                viewport_border: Color32::from_rgba_unmultiplied(0, 120, 212, 180),
                current_match: Color32::from_rgba_unmultiplied(255, 180, 0, 255),
                other_match: Color32::from_rgba_unmultiplied(255, 200, 100, 150),
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Minimap Settings
// ─────────────────────────────────────────────────────────────────────────────

/// Settings for the minimap feature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapSettings {
    /// Whether the minimap is enabled
    pub enabled: bool,
    /// Width of the minimap in pixels
    pub width: f32,
}

impl Default for MinimapSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            width: DEFAULT_MINIMAP_WIDTH,
        }
    }
}

impl MinimapSettings {
    /// Create new minimap settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create disabled minimap settings.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Set whether the minimap is enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the minimap width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(MIN_MINIMAP_WIDTH, MAX_MINIMAP_WIDTH);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimap_new() {
        let minimap = Minimap::new("Hello, World!");
        assert_eq!(minimap.content, "Hello, World!");
        assert_eq!(minimap.width, DEFAULT_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_width_clamping() {
        let minimap = Minimap::new("test").width(20.0);
        assert_eq!(minimap.width, MIN_MINIMAP_WIDTH);

        let minimap = Minimap::new("test").width(200.0);
        assert_eq!(minimap.width, MAX_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_settings_default() {
        let settings = MinimapSettings::default();
        assert!(settings.enabled);
        assert_eq!(settings.width, DEFAULT_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_settings_disabled() {
        let settings = MinimapSettings::disabled();
        assert!(!settings.enabled);
    }

    #[test]
    fn test_minimap_settings_width_clamping() {
        let settings = MinimapSettings::new().width(20.0);
        assert_eq!(settings.width, MIN_MINIMAP_WIDTH);

        let settings = MinimapSettings::new().width(200.0);
        assert_eq!(settings.width, MAX_MINIMAP_WIDTH);
    }

    #[test]
    fn test_minimap_colors_dark() {
        let colors = MinimapColors::new(true);
        // Dark theme should have dark background
        assert!(colors.background.r() < 50);
    }

    #[test]
    fn test_minimap_colors_light() {
        let colors = MinimapColors::new(false);
        // Light theme should have light background
        assert!(colors.background.r() > 200);
    }

    #[test]
    fn test_minimap_output_default() {
        let output = MinimapOutput::default();
        assert!(output.scroll_to_offset.is_none());
        assert!(!output.clicked);
        assert!(!output.dragging);
    }

    #[test]
    fn test_line_color_detection() {
        let minimap = Minimap::new("");
        let colors = MinimapColors::new(false);

        // Test heading detection
        assert_eq!(minimap.get_line_color("# Heading", &colors), colors.heading);
        assert_eq!(minimap.get_line_color("## Heading", &colors), colors.heading);

        // Test code marker detection
        assert_eq!(minimap.get_line_color("```rust", &colors), colors.code_marker);

        // Test comment detection
        assert_eq!(minimap.get_line_color("// comment", &colors), colors.comment);

        // Test list detection
        assert_eq!(minimap.get_line_color("- item", &colors), colors.list);
        assert_eq!(minimap.get_line_color("* item", &colors), colors.list);
        assert_eq!(minimap.get_line_color("1. item", &colors), colors.list);

        // Test blockquote detection
        assert_eq!(minimap.get_line_color("> quote", &colors), colors.blockquote);

        // Test link detection
        assert_eq!(minimap.get_line_color("[link](url)", &colors), colors.link);

        // Test empty line
        assert_eq!(minimap.get_line_color("", &colors), colors.empty_line);
        assert_eq!(minimap.get_line_color("   ", &colors), colors.empty_line);

        // Test regular text
        assert_eq!(minimap.get_line_color("Hello world", &colors), colors.text);
    }
}
