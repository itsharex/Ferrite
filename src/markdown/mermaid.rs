//! Native Mermaid Diagram Rendering
//!
//! This module provides native rendering of MermaidJS diagrams without external
//! dependencies. Diagrams are parsed and rendered directly using egui primitives.
//!
//! # Supported Diagram Types
//!
//! - **Flowchart** (TD, TB, LR, RL, BT) - Nodes and edges with various shapes
//! - **Sequence Diagram** - Participants and message flows (planned)
//!
//! # Architecture
//!
//! 1. `parser` - Parse mermaid source into AST
//! 2. `layout` - Compute node positions using layout algorithms
//! 3. `renderer` - Draw the diagram using egui painter
//!
//! # Example
//!
//! ```ignore
//! use crate::markdown::mermaid::{parse_flowchart, layout_flowchart, render_flowchart};
//!
//! let source = "flowchart TD\n  A[Start] --> B[End]";
//! if let Ok(flowchart) = parse_flowchart(source) {
//!     let layout = layout_flowchart(&flowchart, available_width);
//!     render_flowchart(ui, &flowchart, &layout, colors);
//! }
//! ```

use egui::{Color32, FontId, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Flowchart AST Types
// ─────────────────────────────────────────────────────────────────────────────

/// Direction of the flowchart layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowDirection {
    #[default]
    TopDown,  // TD or TB
    BottomUp, // BT
    LeftRight, // LR
    RightLeft, // RL
}

/// Shape of a flowchart node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeShape {
    #[default]
    Rectangle,    // [text]
    RoundRect,    // (text)
    Stadium,      // ([text])
    Diamond,      // {text}
    Hexagon,      // {{text}}
    Parallelogram, // [/text/]
    Circle,       // ((text))
    Cylinder,     // [(text)]
    Subroutine,   // [[text]]
    Asymmetric,   // >text]
}

/// A node in the flowchart.
#[derive(Debug, Clone)]
pub struct FlowNode {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
}

/// Style of an edge line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeStyle {
    #[default]
    Solid,   // ---
    Dotted,  // -.-
    Thick,   // ===
}

/// Type of arrow head.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowHead {
    #[default]
    Arrow,  // >
    Circle, // o
    Cross,  // x
    None,
}

/// An edge connecting two nodes.
#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub style: EdgeStyle,
    pub arrow_start: ArrowHead,
    pub arrow_end: ArrowHead,
}

/// A parsed flowchart.
#[derive(Debug, Clone, Default)]
pub struct Flowchart {
    pub direction: FlowDirection,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Flowchart Parser
// ─────────────────────────────────────────────────────────────────────────────

/// Parse mermaid flowchart source into a Flowchart AST.
pub fn parse_flowchart(source: &str) -> Result<Flowchart, String> {
    let mut flowchart = Flowchart::default();
    let mut lines = source.lines().peekable();
    let mut node_map: HashMap<String, usize> = HashMap::new();

    // Parse header line (skip comments and empty lines)
    let mut found_header = false;
    while let Some(header) = lines.next() {
        let header_trimmed = header.trim();
        // Skip empty lines and comments
        if header_trimmed.is_empty() || header_trimmed.starts_with("%%") {
            continue;
        }
        let header_lower = header_trimmed.to_lowercase();
        if header_lower.starts_with("flowchart") || header_lower.starts_with("graph") {
            flowchart.direction = parse_direction(&header_lower);
            found_header = true;
            break;
        } else {
            return Err("Expected 'flowchart' or 'graph' declaration".to_string());
        }
    }
    
    if !found_header {
        return Err("Empty flowchart source".to_string());
    }

    // Parse body lines
    for line in lines {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Try to parse as edge (contains arrow)
        if let Some((nodes, edge)) = parse_edge_line(line) {
            for (id, label, shape) in nodes {
                if !node_map.contains_key(&id) {
                    node_map.insert(id.clone(), flowchart.nodes.len());
                    flowchart.nodes.push(FlowNode { id, label, shape });
                }
            }
            if let Some(e) = edge {
                flowchart.edges.push(e);
            }
        } else if let Some(node) = parse_node_definition(line) {
            // Standalone node definition
            if !node_map.contains_key(&node.id) {
                node_map.insert(node.id.clone(), flowchart.nodes.len());
                flowchart.nodes.push(node);
            }
        }
    }

    Ok(flowchart)
}

fn parse_direction(header: &str) -> FlowDirection {
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() > 1 {
        match parts[1].to_uppercase().as_str() {
            "TD" | "TB" => FlowDirection::TopDown,
            "BT" => FlowDirection::BottomUp,
            "LR" => FlowDirection::LeftRight,
            "RL" => FlowDirection::RightLeft,
            _ => FlowDirection::TopDown,
        }
    } else {
        FlowDirection::TopDown
    }
}

fn parse_edge_line(line: &str) -> Option<(Vec<(String, String, NodeShape)>, Option<FlowEdge>)> {
    // Find arrow patterns
    let arrow_patterns = [
        ("-->", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Arrow),
        ("--->", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Arrow),
        ("---", EdgeStyle::Solid, ArrowHead::None, ArrowHead::None),
        ("-.->", EdgeStyle::Dotted, ArrowHead::None, ArrowHead::Arrow),
        ("-.-", EdgeStyle::Dotted, ArrowHead::None, ArrowHead::None),
        ("==>", EdgeStyle::Thick, ArrowHead::None, ArrowHead::Arrow),
        ("===", EdgeStyle::Thick, ArrowHead::None, ArrowHead::None),
        ("--o", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Circle),
        ("--x", EdgeStyle::Solid, ArrowHead::None, ArrowHead::Cross),
        ("<-->", EdgeStyle::Solid, ArrowHead::Arrow, ArrowHead::Arrow),
        ("o--o", EdgeStyle::Solid, ArrowHead::Circle, ArrowHead::Circle),
        ("x--x", EdgeStyle::Solid, ArrowHead::Cross, ArrowHead::Cross),
    ];

    for (pattern, style, arrow_start, arrow_end) in arrow_patterns {
        // Check for labeled edges: A -->|label| B
        if let Some(pos) = line.find(pattern) {
            let left = line[..pos].trim();
            let right_part = &line[pos + pattern.len()..];
            
            // Check for label
            let (label, right) = if let Some(label_start) = right_part.find('|') {
                if let Some(label_end) = right_part[label_start + 1..].find('|') {
                    let label = right_part[label_start + 1..label_start + 1 + label_end].trim().to_string();
                    let rest = right_part[label_start + 2 + label_end..].trim();
                    (Some(label), rest)
                } else {
                    (None, right_part.trim())
                }
            } else {
                (None, right_part.trim())
            };

            // Parse left and right nodes
            let left_node = parse_node_from_text(left);
            let right_node = parse_node_from_text(right);

            if let (Some((from_id, from_label, from_shape)), Some((to_id, to_label, to_shape))) = 
                (left_node, right_node) 
            {
                let nodes = vec![
                    (from_id.clone(), from_label, from_shape),
                    (to_id.clone(), to_label, to_shape),
                ];
                let edge = FlowEdge {
                    from: from_id,
                    to: to_id,
                    label,
                    style,
                    arrow_start,
                    arrow_end,
                };
                return Some((nodes, Some(edge)));
            }
        }
    }

    None
}

fn parse_node_from_text(text: &str) -> Option<(String, String, NodeShape)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // Try various shape patterns
    // Stadium: ([text])
    if text.contains("([") && text.contains("])") {
        if let Some(start) = text.find("([") {
            let id = text[..start].trim();
            let id = if id.is_empty() { &text[..start.max(1)] } else { id };
            if let Some(end) = text.find("])") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Stadium));
            }
        }
    }

    // Circle: ((text))
    if text.contains("((") && text.contains("))") {
        if let Some(start) = text.find("((") {
            let id = text[..start].trim();
            if let Some(end) = text.find("))") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Circle));
            }
        }
    }

    // Cylinder: [(text)]
    if text.contains("[(") && text.contains(")]") {
        if let Some(start) = text.find("[(") {
            let id = text[..start].trim();
            if let Some(end) = text.find(")]") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Cylinder));
            }
        }
    }

    // Subroutine: [[text]]
    if text.contains("[[") && text.contains("]]") {
        if let Some(start) = text.find("[[") {
            let id = text[..start].trim();
            if let Some(end) = text.find("]]") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Subroutine));
            }
        }
    }

    // Hexagon: {{text}}
    if text.contains("{{") && text.contains("}}") {
        if let Some(start) = text.find("{{") {
            let id = text[..start].trim();
            if let Some(end) = text.find("}}") {
                let label = text[start + 2..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Hexagon));
            }
        }
    }

    // Diamond: {text}
    if text.contains('{') && text.contains('}') && !text.contains("{{") {
        if let Some(start) = text.find('{') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind('}') {
                let label = text[start + 1..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Diamond));
            }
        }
    }

    // Round rect: (text)
    if text.contains('(') && text.contains(')') && !text.contains("((") && !text.contains("([") && !text.contains("[(") {
        if let Some(start) = text.find('(') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind(')') {
                let label = text[start + 1..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::RoundRect));
            }
        }
    }

    // Rectangle: [text]
    if text.contains('[') && text.contains(']') && !text.contains("[[") && !text.contains("[(") && !text.contains("([") {
        if let Some(start) = text.find('[') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind(']') {
                let label = text[start + 1..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Rectangle));
            }
        }
    }

    // Asymmetric: >text]
    if text.contains('>') && text.contains(']') {
        if let Some(start) = text.find('>') {
            let id = text[..start].trim();
            if let Some(end) = text.rfind(']') {
                let label = text[start + 1..end].trim();
                return Some((extract_id(id, text), label.to_string(), NodeShape::Asymmetric));
            }
        }
    }

    // Just an ID (no shape specified)
    let id = text.split_whitespace().next().unwrap_or(text);
    Some((id.to_string(), id.to_string(), NodeShape::Rectangle))
}

fn extract_id(id: &str, full_text: &str) -> String {
    if id.is_empty() {
        // Generate ID from first part of text
        full_text.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
    } else {
        id.to_string()
    }
}

fn parse_node_definition(line: &str) -> Option<FlowNode> {
    parse_node_from_text(line).map(|(id, label, shape)| FlowNode { id, label, shape })
}

// ─────────────────────────────────────────────────────────────────────────────
// Layout Engine
// ─────────────────────────────────────────────────────────────────────────────

/// Layout information for a node.
#[derive(Debug, Clone)]
pub struct NodeLayout {
    pub pos: Pos2,
    pub size: Vec2,
}

/// Complete layout for a flowchart.
#[derive(Debug, Clone, Default)]
pub struct FlowchartLayout {
    pub nodes: HashMap<String, NodeLayout>,
    pub total_size: Vec2,
}

/// Compute layout for a flowchart.
pub fn layout_flowchart(flowchart: &Flowchart, available_width: f32, font_size: f32) -> FlowchartLayout {
    if flowchart.nodes.is_empty() {
        return FlowchartLayout::default();
    }

    let node_padding = Vec2::new(24.0, 12.0);
    let node_spacing = Vec2::new(40.0, 50.0);
    
    // Calculate node sizes based on text
    let mut node_sizes: HashMap<String, Vec2> = HashMap::new();
    for node in &flowchart.nodes {
        let text_width = node.label.len() as f32 * font_size * 0.6;
        let text_height = font_size;
        let size = Vec2::new(
            (text_width + node_padding.x * 2.0).max(80.0),
            (text_height + node_padding.y * 2.0).max(40.0),
        );
        node_sizes.insert(node.id.clone(), size);
    }

    // Build adjacency for layer assignment
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    
    for node in &flowchart.nodes {
        outgoing.insert(node.id.clone(), Vec::new());
        incoming.insert(node.id.clone(), Vec::new());
    }
    
    for edge in &flowchart.edges {
        if let Some(out) = outgoing.get_mut(&edge.from) {
            out.push(edge.to.clone());
        }
        if let Some(inc) = incoming.get_mut(&edge.to) {
            inc.push(edge.from.clone());
        }
    }

    // Assign layers using topological sort
    let mut layers: Vec<Vec<String>> = Vec::new();
    let mut node_layer: HashMap<String, usize> = HashMap::new();
    let mut remaining: Vec<String> = flowchart.nodes.iter().map(|n| n.id.clone()).collect();

    while !remaining.is_empty() {
        // Find nodes with no remaining incoming edges
        let layer_nodes: Vec<String> = remaining
            .iter()
            .filter(|id| {
                incoming.get(*id).map_or(true, |inc| {
                    inc.iter().all(|from| node_layer.contains_key(from))
                })
            })
            .cloned()
            .collect();

        if layer_nodes.is_empty() {
            // Cycle detected or remaining nodes, just add them
            for id in remaining.drain(..) {
                let layer_idx = layers.len();
                node_layer.insert(id.clone(), layer_idx);
                if layers.len() <= layer_idx {
                    layers.push(Vec::new());
                }
                layers[layer_idx].push(id);
            }
            break;
        }

        let layer_idx = layers.len();
        layers.push(layer_nodes.clone());
        for id in &layer_nodes {
            node_layer.insert(id.clone(), layer_idx);
            remaining.retain(|r| r != id);
        }
    }

    // Calculate positions
    let mut layout = FlowchartLayout::default();
    let is_horizontal = matches!(flowchart.direction, FlowDirection::LeftRight | FlowDirection::RightLeft);
    
    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = 0.0;
    let start_margin = 20.0_f32;
    let mut current_main_pos = start_margin;

    // First pass: calculate total cross-axis size needed
    let mut max_cross_size: f32 = 0.0;
    for layer in &layers {
        let mut layer_cross_size: f32 = 0.0;
        for id in layer {
            if let Some(size) = node_sizes.get(id) {
                layer_cross_size += if is_horizontal { size.y } else { size.x };
            }
        }
        layer_cross_size += (layer.len().saturating_sub(1)) as f32 * 
            if is_horizontal { node_spacing.y } else { node_spacing.x };
        max_cross_size = max_cross_size.max(layer_cross_size);
    }

    for layer in &layers {
        let mut layer_cross_size: f32 = 0.0;
        
        // Calculate total cross-axis size of this layer
        for id in layer {
            if let Some(size) = node_sizes.get(id) {
                layer_cross_size += if is_horizontal { size.y } else { size.x };
            }
        }
        layer_cross_size += (layer.len().saturating_sub(1)) as f32 * 
            if is_horizontal { node_spacing.y } else { node_spacing.x };

        // For vertical layouts (TD/BT): center horizontally within available_width
        // For horizontal layouts (LR/RL): start from top, center within calculated height
        let start_cross = if is_horizontal {
            // For LR/RL: center vertically within the max cross size
            start_margin + (max_cross_size - layer_cross_size) / 2.0
        } else {
            // For TD/BT: center horizontally within available width
            (available_width - layer_cross_size).max(start_margin * 2.0) / 2.0
        };

        let mut cross_pos = start_cross;
        
        for id in layer {
            if let Some(size) = node_sizes.get(id) {
                let pos = if is_horizontal {
                    Pos2::new(current_main_pos, cross_pos)
                } else {
                    Pos2::new(cross_pos, current_main_pos)
                };
                
                layout.nodes.insert(id.clone(), NodeLayout { pos, size: *size });
                
                max_x = max_x.max(pos.x + size.x);
                max_y = max_y.max(pos.y + size.y);
                
                cross_pos += if is_horizontal { 
                    size.y + node_spacing.y 
                } else { 
                    size.x + node_spacing.x 
                };
            }
        }

        // Move to next layer along main axis
        let max_main_size = layer.iter()
            .filter_map(|id| node_sizes.get(id))
            .map(|s| if is_horizontal { s.x } else { s.y })
            .fold(0.0_f32, |a, b| a.max(b));
        
        current_main_pos += max_main_size + if is_horizontal { node_spacing.x } else { node_spacing.y };
    }

    // Handle reverse directions
    if matches!(flowchart.direction, FlowDirection::BottomUp | FlowDirection::RightLeft) {
        let total = if is_horizontal { max_x } else { max_y };
        for node_layout in layout.nodes.values_mut() {
            if is_horizontal {
                node_layout.pos.x = total - node_layout.pos.x - node_layout.size.x + start_margin;
            } else {
                node_layout.pos.y = total - node_layout.pos.y - node_layout.size.y + start_margin;
            }
        }
    }

    layout.total_size = Vec2::new(max_x + start_margin, max_y + start_margin);
    layout
}

// ─────────────────────────────────────────────────────────────────────────────
// Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for rendering the flowchart.
#[derive(Debug, Clone)]
pub struct FlowchartColors {
    pub node_fill: Color32,
    pub node_stroke: Color32,
    pub node_text: Color32,
    pub edge_stroke: Color32,
    pub edge_label_bg: Color32,
    pub edge_label_text: Color32,
    pub diamond_fill: Color32,
    pub circle_fill: Color32,
}

impl FlowchartColors {
    pub fn dark() -> Self {
        Self {
            node_fill: Color32::from_rgb(45, 55, 72),
            node_stroke: Color32::from_rgb(100, 140, 180),
            node_text: Color32::from_rgb(220, 230, 240),
            edge_stroke: Color32::from_rgb(120, 150, 180),
            edge_label_bg: Color32::from_rgb(35, 45, 55),
            edge_label_text: Color32::from_rgb(180, 190, 200),
            diamond_fill: Color32::from_rgb(60, 50, 70),
            circle_fill: Color32::from_rgb(50, 65, 75),
        }
    }

    pub fn light() -> Self {
        Self {
            node_fill: Color32::from_rgb(240, 245, 250),
            node_stroke: Color32::from_rgb(100, 140, 180),
            node_text: Color32::from_rgb(30, 40, 50),
            edge_stroke: Color32::from_rgb(100, 130, 160),
            edge_label_bg: Color32::from_rgb(255, 255, 255),
            edge_label_text: Color32::from_rgb(60, 70, 80),
            diamond_fill: Color32::from_rgb(255, 250, 240),
            circle_fill: Color32::from_rgb(240, 250, 255),
        }
    }
}

/// Render a flowchart to the UI.
pub fn render_flowchart(
    ui: &mut Ui,
    flowchart: &Flowchart,
    layout: &FlowchartLayout,
    colors: &FlowchartColors,
    font_size: f32,
) {
    if flowchart.nodes.is_empty() {
        return;
    }

    // Allocate space for the diagram
    let (response, painter) = ui.allocate_painter(
        layout.total_size,
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw edges first (behind nodes)
    for edge in &flowchart.edges {
        if let (Some(from_layout), Some(to_layout)) = (layout.nodes.get(&edge.from), layout.nodes.get(&edge.to)) {
            draw_edge(&painter, edge, from_layout, to_layout, offset, colors, font_size, flowchart.direction);
        }
    }

    // Draw nodes
    for node in &flowchart.nodes {
        if let Some(node_layout) = layout.nodes.get(&node.id) {
            draw_node(&painter, node, node_layout, offset, colors, font_size);
        }
    }
}

fn draw_node(
    painter: &egui::Painter,
    node: &FlowNode,
    layout: &NodeLayout,
    offset: Vec2,
    colors: &FlowchartColors,
    font_size: f32,
) {
    let rect = Rect::from_min_size(layout.pos + offset, layout.size);
    let center = rect.center();
    let stroke = Stroke::new(2.0, colors.node_stroke);

    match node.shape {
        NodeShape::Rectangle | NodeShape::Subroutine => {
            painter.rect(rect, Rounding::same(4.0), colors.node_fill, stroke);
            if matches!(node.shape, NodeShape::Subroutine) {
                // Draw double vertical lines
                let inset = 8.0;
                painter.line_segment(
                    [Pos2::new(rect.left() + inset, rect.top()), Pos2::new(rect.left() + inset, rect.bottom())],
                    stroke,
                );
                painter.line_segment(
                    [Pos2::new(rect.right() - inset, rect.top()), Pos2::new(rect.right() - inset, rect.bottom())],
                    stroke,
                );
            }
        }
        NodeShape::RoundRect | NodeShape::Stadium => {
            let rounding = if matches!(node.shape, NodeShape::Stadium) {
                Rounding::same(layout.size.y / 2.0)
            } else {
                Rounding::same(12.0)
            };
            painter.rect(rect, rounding, colors.node_fill, stroke);
        }
        NodeShape::Diamond => {
            let points = [
                Pos2::new(center.x, rect.top()),
                Pos2::new(rect.right(), center.y),
                Pos2::new(center.x, rect.bottom()),
                Pos2::new(rect.left(), center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                colors.diamond_fill,
                stroke,
            ));
        }
        NodeShape::Circle => {
            let radius = layout.size.x.min(layout.size.y) / 2.0;
            painter.circle(center, radius, colors.circle_fill, stroke);
        }
        NodeShape::Hexagon => {
            let inset = layout.size.x * 0.15;
            let points = [
                Pos2::new(rect.left() + inset, rect.top()),
                Pos2::new(rect.right() - inset, rect.top()),
                Pos2::new(rect.right(), center.y),
                Pos2::new(rect.right() - inset, rect.bottom()),
                Pos2::new(rect.left() + inset, rect.bottom()),
                Pos2::new(rect.left(), center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                colors.node_fill,
                stroke,
            ));
        }
        NodeShape::Cylinder => {
            // Simplified cylinder as rounded rect with ellipse hints
            painter.rect(rect, Rounding::same(4.0), colors.node_fill, stroke);
            let ellipse_height = 8.0;
            painter.line_segment(
                [
                    Pos2::new(rect.left(), rect.top() + ellipse_height),
                    Pos2::new(rect.right(), rect.top() + ellipse_height),
                ],
                Stroke::new(1.0, colors.node_stroke.gamma_multiply(0.5)),
            );
        }
        NodeShape::Parallelogram => {
            let skew = layout.size.x * 0.15;
            let points = [
                Pos2::new(rect.left() + skew, rect.top()),
                Pos2::new(rect.right(), rect.top()),
                Pos2::new(rect.right() - skew, rect.bottom()),
                Pos2::new(rect.left(), rect.bottom()),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                colors.node_fill,
                stroke,
            ));
        }
        NodeShape::Asymmetric => {
            let indent = layout.size.y * 0.3;
            let points = [
                Pos2::new(rect.left() + indent, rect.top()),
                Pos2::new(rect.right(), rect.top()),
                Pos2::new(rect.right(), rect.bottom()),
                Pos2::new(rect.left() + indent, rect.bottom()),
                Pos2::new(rect.left(), center.y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                colors.node_fill,
                stroke,
            ));
        }
    }

    // Draw text
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        &node.label,
        FontId::proportional(font_size),
        colors.node_text,
    );
}

fn draw_edge(
    painter: &egui::Painter,
    edge: &FlowEdge,
    from_layout: &NodeLayout,
    to_layout: &NodeLayout,
    offset: Vec2,
    colors: &FlowchartColors,
    font_size: f32,
    direction: FlowDirection,
) {
    let from_rect = Rect::from_min_size(from_layout.pos + offset, from_layout.size);
    let to_rect = Rect::from_min_size(to_layout.pos + offset, to_layout.size);

    // Calculate connection points based on direction
    let (start, end) = match direction {
        FlowDirection::TopDown => {
            (
                Pos2::new(from_rect.center().x, from_rect.bottom()),
                Pos2::new(to_rect.center().x, to_rect.top()),
            )
        }
        FlowDirection::BottomUp => {
            (
                Pos2::new(from_rect.center().x, from_rect.top()),
                Pos2::new(to_rect.center().x, to_rect.bottom()),
            )
        }
        FlowDirection::LeftRight => {
            (
                Pos2::new(from_rect.right(), from_rect.center().y),
                Pos2::new(to_rect.left(), to_rect.center().y),
            )
        }
        FlowDirection::RightLeft => {
            (
                Pos2::new(from_rect.left(), from_rect.center().y),
                Pos2::new(to_rect.right(), to_rect.center().y),
            )
        }
    };

    // Edge style
    let stroke_width = match edge.style {
        EdgeStyle::Solid => 2.0,
        EdgeStyle::Dotted => 1.5,
        EdgeStyle::Thick => 3.0,
    };

    let stroke = Stroke::new(stroke_width, colors.edge_stroke);

    // Draw the line
    if matches!(edge.style, EdgeStyle::Dotted) {
        // Draw dashed line
        draw_dashed_line(painter, start, end, stroke, 5.0, 3.0);
    } else {
        painter.line_segment([start, end], stroke);
    }

    // Draw arrow head at end
    if !matches!(edge.arrow_end, ArrowHead::None) {
        draw_arrow_head(painter, start, end, &edge.arrow_end, colors.edge_stroke, stroke_width);
    }

    // Draw arrow head at start (for bidirectional)
    if !matches!(edge.arrow_start, ArrowHead::None) {
        draw_arrow_head(painter, end, start, &edge.arrow_start, colors.edge_stroke, stroke_width);
    }

    // Draw edge label
    if let Some(label) = &edge.label {
        let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        let label_size = Vec2::new(label.len() as f32 * font_size * 0.5 + 8.0, font_size + 4.0);
        let label_rect = Rect::from_center_size(mid, label_size);
        
        painter.rect_filled(label_rect, Rounding::same(3.0), colors.edge_label_bg);
        painter.text(
            mid,
            egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(font_size - 2.0),
            colors.edge_label_text,
        );
    }
}

fn draw_dashed_line(painter: &egui::Painter, start: Pos2, end: Pos2, stroke: Stroke, dash_len: f32, gap_len: f32) {
    let dir = (end - start).normalized();
    let total_len = (end - start).length();
    let mut pos = 0.0;
    
    while pos < total_len {
        let seg_start = start + dir * pos;
        let seg_end_pos = (pos + dash_len).min(total_len);
        let seg_end = start + dir * seg_end_pos;
        painter.line_segment([seg_start, seg_end], stroke);
        pos = seg_end_pos + gap_len;
    }
}

fn draw_arrow_head(painter: &egui::Painter, from: Pos2, to: Pos2, head_type: &ArrowHead, color: Color32, stroke_width: f32) {
    let dir = (to - from).normalized();
    let perp = Vec2::new(-dir.y, dir.x);
    let arrow_size = 8.0 + stroke_width;
    
    match head_type {
        ArrowHead::Arrow => {
            let tip = to;
            let left = to - dir * arrow_size + perp * (arrow_size * 0.5);
            let right = to - dir * arrow_size - perp * (arrow_size * 0.5);
            painter.add(egui::Shape::convex_polygon(
                vec![tip, left, right],
                color,
                Stroke::NONE,
            ));
        }
        ArrowHead::Circle => {
            painter.circle_filled(to - dir * 4.0, 4.0, color);
        }
        ArrowHead::Cross => {
            let center = to - dir * 4.0;
            let size = 4.0;
            painter.line_segment(
                [center - perp * size - dir * size, center + perp * size + dir * size],
                Stroke::new(stroke_width, color),
            );
            painter.line_segment(
                [center + perp * size - dir * size, center - perp * size + dir * size],
                Stroke::new(stroke_width, color),
            );
        }
        ArrowHead::None => {}
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sequence Diagram Types
// ─────────────────────────────────────────────────────────────────────────────

/// A participant in a sequence diagram.
#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub is_actor: bool,
}

/// Type of message arrow in sequence diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageType {
    #[default]
    Solid,       // ->>
    SolidOpen,   // ->
    Dotted,      // -->>
    DottedOpen,  // -->
}

/// A message between participants.
#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub to: String,
    pub label: String,
    pub message_type: MessageType,
}

/// A parsed sequence diagram.
#[derive(Debug, Clone, Default)]
pub struct SequenceDiagram {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Sequence Diagram Parser
// ─────────────────────────────────────────────────────────────────────────────

/// Parse mermaid sequence diagram source.
pub fn parse_sequence_diagram(source: &str) -> Result<SequenceDiagram, String> {
    let mut diagram = SequenceDiagram::default();
    let mut participant_map: HashMap<String, usize> = HashMap::new();
    let lines = source.lines().skip(1); // Skip "sequenceDiagram" header

    for line in lines {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse participant declaration
        if line.starts_with("participant ") || line.starts_with("actor ") {
            let is_actor = line.starts_with("actor ");
            let rest = if is_actor { &line[6..] } else { &line[12..] };
            let rest = rest.trim();
            
            // Check for "as" alias: participant A as Alice
            let (id, label) = if let Some(as_pos) = rest.find(" as ") {
                let id = rest[..as_pos].trim().to_string();
                let label = rest[as_pos + 4..].trim().to_string();
                (id, label)
            } else {
                (rest.to_string(), rest.to_string())
            };
            
            if !participant_map.contains_key(&id) {
                participant_map.insert(id.clone(), diagram.participants.len());
                diagram.participants.push(Participant { id, label, is_actor });
            }
            continue;
        }

        // Parse message: A->>B: Message or A-->>B: Message
        if let Some(msg) = parse_sequence_message(line) {
            // Auto-add participants if not declared
            for id in [&msg.from, &msg.to] {
                if !participant_map.contains_key(id) {
                    participant_map.insert(id.clone(), diagram.participants.len());
                    diagram.participants.push(Participant {
                        id: id.clone(),
                        label: id.clone(),
                        is_actor: false,
                    });
                }
            }
            diagram.messages.push(msg);
        }
    }

    if diagram.participants.is_empty() {
        return Err("No participants found in sequence diagram".to_string());
    }

    Ok(diagram)
}

fn parse_sequence_message(line: &str) -> Option<Message> {
    // Arrow patterns to check (order matters - check longer patterns first)
    let arrow_patterns = [
        ("-->>", MessageType::Dotted),
        ("->>", MessageType::Solid),
        ("-->", MessageType::DottedOpen),
        ("->", MessageType::SolidOpen),
    ];

    for (pattern, msg_type) in arrow_patterns {
        if let Some(arrow_pos) = line.find(pattern) {
            let from = line[..arrow_pos].trim();
            let rest = &line[arrow_pos + pattern.len()..];
            
            // Find the colon for the message label
            let (to, label) = if let Some(colon_pos) = rest.find(':') {
                let to = rest[..colon_pos].trim();
                let label = rest[colon_pos + 1..].trim();
                (to, label)
            } else {
                (rest.trim(), "")
            };

            if !from.is_empty() && !to.is_empty() {
                return Some(Message {
                    from: from.to_string(),
                    to: to.to_string(),
                    label: label.to_string(),
                    message_type: msg_type,
                });
            }
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Sequence Diagram Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// Render a sequence diagram to the UI.
pub fn render_sequence_diagram(
    ui: &mut Ui,
    diagram: &SequenceDiagram,
    dark_mode: bool,
    font_size: f32,
) {
    if diagram.participants.is_empty() {
        return;
    }

    // Layout constants
    let participant_width = 100.0_f32;
    let participant_height = 40.0_f32;
    let participant_spacing = 50.0_f32;
    let message_height = 40.0_f32;
    let margin = 20.0_f32;
    let lifeline_extend = 30.0_f32;

    // Calculate total size
    let total_width = margin * 2.0 + 
        diagram.participants.len() as f32 * participant_width +
        (diagram.participants.len().saturating_sub(1)) as f32 * participant_spacing;
    let total_height = margin * 2.0 + 
        participant_height + 
        diagram.messages.len() as f32 * message_height +
        lifeline_extend;

    // Colors
    let (bg_color, stroke_color, text_color, lifeline_color) = if dark_mode {
        (
            Color32::from_rgb(45, 55, 72),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(80, 100, 120),
        )
    } else {
        (
            Color32::from_rgb(240, 245, 250),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(180, 190, 200),
        )
    };

    let actor_color = if dark_mode {
        Color32::from_rgb(100, 160, 220)
    } else {
        Color32::from_rgb(50, 120, 180)
    };

    // Allocate space
    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width, total_height),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Calculate participant positions
    let mut participant_x: HashMap<String, f32> = HashMap::new();
    let mut current_x = margin + participant_width / 2.0;
    
    for participant in &diagram.participants {
        participant_x.insert(participant.id.clone(), current_x);
        current_x += participant_width + participant_spacing;
    }

    // Draw lifelines first (behind everything)
    let lifeline_start_y = margin + participant_height;
    let lifeline_end_y = total_height - margin;
    
    for participant in &diagram.participants {
        if let Some(&x) = participant_x.get(&participant.id) {
            painter.line_segment(
                [
                    Pos2::new(x, lifeline_start_y) + offset,
                    Pos2::new(x, lifeline_end_y) + offset,
                ],
                Stroke::new(1.0, lifeline_color),
            );
        }
    }

    // Draw participants
    for participant in &diagram.participants {
        if let Some(&center_x) = participant_x.get(&participant.id) {
            let rect = Rect::from_center_size(
                Pos2::new(center_x, margin + participant_height / 2.0) + offset,
                Vec2::new(participant_width, participant_height),
            );

            if participant.is_actor {
                // Draw stick figure for actor
                let head_y = rect.top() + 10.0;
                let body_y = head_y + 15.0;
                let legs_y = body_y + 12.0;
                
                // Head
                painter.circle_stroke(Pos2::new(center_x + offset.x, head_y), 6.0, Stroke::new(2.0, actor_color));
                // Body
                painter.line_segment(
                    [Pos2::new(center_x + offset.x, head_y + 6.0), Pos2::new(center_x + offset.x, body_y)],
                    Stroke::new(2.0, actor_color),
                );
                // Arms
                painter.line_segment(
                    [Pos2::new(center_x - 10.0 + offset.x, head_y + 12.0), Pos2::new(center_x + 10.0 + offset.x, head_y + 12.0)],
                    Stroke::new(2.0, actor_color),
                );
                // Legs
                painter.line_segment(
                    [Pos2::new(center_x + offset.x, body_y), Pos2::new(center_x - 8.0 + offset.x, legs_y)],
                    Stroke::new(2.0, actor_color),
                );
                painter.line_segment(
                    [Pos2::new(center_x + offset.x, body_y), Pos2::new(center_x + 8.0 + offset.x, legs_y)],
                    Stroke::new(2.0, actor_color),
                );
                // Label below
                painter.text(
                    Pos2::new(center_x + offset.x, rect.bottom() - 2.0),
                    egui::Align2::CENTER_BOTTOM,
                    &participant.label,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
            } else {
                // Draw rectangle for participant
                painter.rect(rect, Rounding::same(4.0), bg_color, Stroke::new(2.0, stroke_color));
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &participant.label,
                    FontId::proportional(font_size),
                    text_color,
                );
            }
        }
    }

    // Draw messages
    let mut current_y = margin + participant_height + message_height / 2.0;
    
    for message in &diagram.messages {
        if let (Some(&from_x), Some(&to_x)) = (participant_x.get(&message.from), participant_x.get(&message.to)) {
            let y = current_y + offset.y;
            let from_pos = Pos2::new(from_x + offset.x, y);
            let to_pos = Pos2::new(to_x + offset.x, y);
            
            // Determine stroke style
            let stroke = match message.message_type {
                MessageType::Solid | MessageType::SolidOpen => Stroke::new(1.5, stroke_color),
                MessageType::Dotted | MessageType::DottedOpen => Stroke::new(1.5, stroke_color),
            };

            // Draw arrow line
            if matches!(message.message_type, MessageType::Dotted | MessageType::DottedOpen) {
                draw_dashed_line(&painter, from_pos, to_pos, stroke, 5.0, 3.0);
            } else {
                painter.line_segment([from_pos, to_pos], stroke);
            }

            // Draw arrow head (solid or open)
            let is_solid_head = matches!(message.message_type, MessageType::Solid | MessageType::Dotted);
            let dir = (to_pos - from_pos).normalized();
            let arrow_size = 8.0;
            let perp = Vec2::new(-dir.y, dir.x);
            
            let arrow_tip = to_pos;
            let arrow_left = to_pos - dir * arrow_size + perp * (arrow_size * 0.4);
            let arrow_right = to_pos - dir * arrow_size - perp * (arrow_size * 0.4);

            if is_solid_head {
                painter.add(egui::Shape::convex_polygon(
                    vec![arrow_tip, arrow_left, arrow_right],
                    stroke_color,
                    Stroke::NONE,
                ));
            } else {
                painter.line_segment([arrow_tip, arrow_left], stroke);
                painter.line_segment([arrow_tip, arrow_right], stroke);
            }

            // Draw message label
            if !message.label.is_empty() {
                let label_pos = Pos2::new((from_x + to_x) / 2.0 + offset.x, y - 8.0);
                painter.text(
                    label_pos,
                    egui::Align2::CENTER_BOTTOM,
                    &message.label,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
            }

            current_y += message_height;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pie Chart Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A slice of a pie chart.
#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
}

/// A parsed pie chart.
#[derive(Debug, Clone, Default)]
pub struct PieChart {
    pub title: Option<String>,
    pub slices: Vec<PieSlice>,
}

/// Parse mermaid pie chart source.
pub fn parse_pie_chart(source: &str) -> Result<PieChart, String> {
    let mut chart = PieChart::default();
    
    for line in source.lines().skip(1) {
        let line = line.trim();
        
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse title
        if line.starts_with("title ") {
            chart.title = Some(line[6..].trim().to_string());
            continue;
        }

        // Parse slice: "Label" : value
        if let Some(colon_pos) = line.find(':') {
            let label = line[..colon_pos].trim().trim_matches('"').to_string();
            let value_str = line[colon_pos + 1..].trim();
            if let Ok(value) = value_str.parse::<f32>() {
                chart.slices.push(PieSlice { label, value });
            }
        }
    }

    if chart.slices.is_empty() {
        return Err("No data found in pie chart".to_string());
    }

    Ok(chart)
}

/// Render a pie chart to the UI.
pub fn render_pie_chart(
    ui: &mut Ui,
    chart: &PieChart,
    dark_mode: bool,
    font_size: f32,
) {
    use std::f32::consts::PI;

    let margin = 20.0_f32;
    let pie_radius = 80.0_f32;
    let legend_width = 120.0_f32;
    
    let total_width = margin * 3.0 + pie_radius * 2.0 + legend_width;
    let total_height = margin * 2.0 + pie_radius * 2.0 + if chart.title.is_some() { 30.0 } else { 0.0 };

    let text_color = if dark_mode {
        Color32::from_rgb(220, 230, 240)
    } else {
        Color32::from_rgb(30, 40, 50)
    };

    // Pie colors
    let colors = [
        Color32::from_rgb(66, 133, 244),   // Blue
        Color32::from_rgb(234, 67, 53),    // Red
        Color32::from_rgb(251, 188, 4),    // Yellow
        Color32::from_rgb(52, 168, 83),    // Green
        Color32::from_rgb(155, 89, 182),   // Purple
        Color32::from_rgb(230, 126, 34),   // Orange
        Color32::from_rgb(26, 188, 156),   // Teal
        Color32::from_rgb(241, 196, 15),   // Gold
    ];

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width, total_height),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    let mut y_offset = margin;
    if let Some(title) = &chart.title {
        painter.text(
            Pos2::new(total_width / 2.0, margin / 2.0) + offset,
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
        y_offset += 20.0;
    }

    // Calculate total and draw pie
    let total: f32 = chart.slices.iter().map(|s| s.value).sum();
    if total <= 0.0 {
        return;
    }

    let center = Pos2::new(margin + pie_radius, y_offset + pie_radius) + offset;
    let mut start_angle = -PI / 2.0; // Start from top
    let border_color = if dark_mode { Color32::from_rgb(25, 30, 40) } else { Color32::WHITE };

    // Draw each slice as a filled path
    for (i, slice) in chart.slices.iter().enumerate() {
        let sweep_angle = (slice.value / total) * 2.0 * PI;
        let color = colors[i % colors.len()];
        let end_angle = start_angle + sweep_angle;

        // Build path for this slice
        let mut path = vec![center];
        
        // Add arc points - use enough segments for smooth curve
        let arc_segments = ((sweep_angle / (2.0 * PI)) * 100.0).max(8.0) as usize;
        for j in 0..=arc_segments {
            let t = j as f32 / arc_segments as f32;
            let angle = start_angle + sweep_angle * t;
            path.push(center + Vec2::new(angle.cos(), angle.sin()) * pie_radius);
        }

        // Draw the slice as a filled mesh (handles non-convex shapes)
        // We'll use triangles from center to each pair of adjacent arc points
        for j in 0..path.len() - 2 {
            let p0 = path[0]; // center
            let p1 = path[j + 1];
            let p2 = path[j + 2];
            
            // Create a mesh for this triangle
            let mesh = egui::Mesh {
                indices: vec![0, 1, 2],
                vertices: vec![
                    egui::epaint::Vertex { pos: p0, uv: egui::epaint::WHITE_UV, color },
                    egui::epaint::Vertex { pos: p1, uv: egui::epaint::WHITE_UV, color },
                    egui::epaint::Vertex { pos: p2, uv: egui::epaint::WHITE_UV, color },
                ],
                texture_id: egui::TextureId::default(),
            };
            painter.add(egui::Shape::mesh(mesh));
        }

        // Draw slice border lines
        let start_edge = center + Vec2::new(start_angle.cos(), start_angle.sin()) * pie_radius;
        let end_edge = center + Vec2::new(end_angle.cos(), end_angle.sin()) * pie_radius;
        painter.line_segment([center, start_edge], Stroke::new(1.5, border_color));
        painter.line_segment([center, end_edge], Stroke::new(1.5, border_color));

        start_angle = end_angle;
    }
    
    // Draw outer circle border
    painter.circle_stroke(center, pie_radius, Stroke::new(1.5, border_color));

    // Draw legend
    let legend_x = margin * 2.0 + pie_radius * 2.0 + offset.x;
    let mut legend_y = y_offset + 10.0 + offset.y;

    for (i, slice) in chart.slices.iter().enumerate() {
        let color = colors[i % colors.len()];
        let percentage = (slice.value / total * 100.0).round();

        // Color box
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(legend_x, legend_y), Vec2::new(12.0, 12.0)),
            2.0,
            color,
        );

        // Label
        painter.text(
            Pos2::new(legend_x + 18.0, legend_y + 6.0),
            egui::Align2::LEFT_CENTER,
            format!("{} ({}%)", slice.label, percentage),
            FontId::proportional(font_size - 2.0),
            text_color,
        );

        legend_y += 20.0;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// State Diagram Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A state in a state diagram.
#[derive(Debug, Clone)]
pub struct State {
    pub id: String,
    pub label: String,
    pub is_start: bool,
    pub is_end: bool,
}

/// A transition between states.
#[derive(Debug, Clone)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

/// A parsed state diagram.
#[derive(Debug, Clone, Default)]
pub struct StateDiagram {
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

/// Parse mermaid state diagram source.
pub fn parse_state_diagram(source: &str) -> Result<StateDiagram, String> {
    let mut diagram = StateDiagram::default();
    let mut state_map: HashMap<String, usize> = HashMap::new();

    for line in source.lines().skip(1) {
        let line = line.trim();
        
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse transitions: State1 --> State2 or State1 --> State2: label
        if line.contains("-->") {
            if let Some(arrow_pos) = line.find("-->") {
                let from_part = line[..arrow_pos].trim();
                let rest = &line[arrow_pos + 3..];
                
                // Check for label after colon
                let (to_part, label) = if let Some(colon_pos) = rest.find(':') {
                    (rest[..colon_pos].trim(), Some(rest[colon_pos + 1..].trim().to_string()))
                } else {
                    (rest.trim(), None)
                };

                // Handle [*] for start/end states
                let from_id = if from_part == "[*]" { "__start__".to_string() } else { from_part.to_string() };
                let to_id = if to_part == "[*]" { "__end__".to_string() } else { to_part.to_string() };

                // Add states if not exists
                for (id, is_start, is_end) in [
                    (&from_id, from_part == "[*]", false),
                    (&to_id, false, to_part == "[*]"),
                ] {
                    if !state_map.contains_key(id) {
                        state_map.insert(id.clone(), diagram.states.len());
                        let label = if is_start { "●".to_string() } 
                                   else if is_end { "◉".to_string() }
                                   else { id.clone() };
                        diagram.states.push(State {
                            id: id.clone(),
                            label,
                            is_start,
                            is_end,
                        });
                    }
                }

                diagram.transitions.push(Transition { from: from_id, to: to_id, label });
            }
        }
        // Parse state definition: state "Label" as StateName
        else if line.starts_with("state ") {
            let rest = &line[6..].trim();
            if let Some(as_pos) = rest.find(" as ") {
                let label = rest[..as_pos].trim().trim_matches('"').to_string();
                let id = rest[as_pos + 4..].trim().to_string();
                if !state_map.contains_key(&id) {
                    state_map.insert(id.clone(), diagram.states.len());
                    diagram.states.push(State { id, label, is_start: false, is_end: false });
                }
            }
        }
    }

    if diagram.states.is_empty() {
        return Err("No states found in state diagram".to_string());
    }

    Ok(diagram)
}

/// Render a state diagram to the UI.
pub fn render_state_diagram(
    ui: &mut Ui,
    diagram: &StateDiagram,
    dark_mode: bool,
    font_size: f32,
) {
    if diagram.states.is_empty() {
        return;
    }

    // Layout: use topological sort similar to flowchart
    let state_width = 100.0_f32;
    let state_height = 36.0_f32;
    let spacing_x = 80.0_f32;
    let spacing_y = 70.0_f32;
    let margin = 30.0_f32;

    // Build adjacency for layer assignment
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
    
    for state in &diagram.states {
        outgoing.insert(state.id.clone(), Vec::new());
        incoming.insert(state.id.clone(), Vec::new());
    }
    
    for trans in &diagram.transitions {
        if let Some(out) = outgoing.get_mut(&trans.from) {
            out.push(trans.to.clone());
        }
        if let Some(inc) = incoming.get_mut(&trans.to) {
            inc.push(trans.from.clone());
        }
    }

    // Assign layers
    let mut layers: Vec<Vec<String>> = Vec::new();
    let mut state_layer: HashMap<String, usize> = HashMap::new();
    let mut remaining: Vec<String> = diagram.states.iter().map(|s| s.id.clone()).collect();

    while !remaining.is_empty() {
        let layer_states: Vec<String> = remaining
            .iter()
            .filter(|id| {
                incoming.get(*id).map_or(true, |inc| {
                    inc.iter().all(|from| state_layer.contains_key(from))
                })
            })
            .cloned()
            .collect();

        if layer_states.is_empty() {
            // Cycle - just add remaining
            for id in remaining.drain(..) {
                let idx = layers.len();
                state_layer.insert(id.clone(), idx);
                if layers.len() <= idx { layers.push(Vec::new()); }
                layers[idx].push(id);
            }
            break;
        }

        let idx = layers.len();
        layers.push(layer_states.clone());
        for id in &layer_states {
            state_layer.insert(id.clone(), idx);
            remaining.retain(|r| r != id);
        }
    }

    // Calculate positions
    let mut state_pos: HashMap<String, Pos2> = HashMap::new();
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;

    for (layer_idx, layer) in layers.iter().enumerate() {
        let x = margin + layer_idx as f32 * (state_width + spacing_x) + state_width / 2.0;
        let layer_height = layer.len() as f32 * (state_height + spacing_y) - spacing_y;
        let start_y = margin + (if layer.len() > 1 { 0.0 } else { spacing_y / 2.0 });

        for (i, id) in layer.iter().enumerate() {
            let y = start_y + i as f32 * (state_height + spacing_y) + state_height / 2.0;
            state_pos.insert(id.clone(), Pos2::new(x, y));
            max_x = max_x.max(x + state_width / 2.0);
            max_y = max_y.max(y + state_height / 2.0);
        }
    }

    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let (state_fill, state_stroke, text_color, arrow_color, start_color, label_bg) = if dark_mode {
        (
            Color32::from_rgb(45, 55, 72),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(120, 150, 180),
            Color32::from_rgb(80, 180, 120),
            Color32::from_rgb(35, 40, 50),
        )
    } else {
        (
            Color32::from_rgb(240, 245, 250),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(100, 130, 160),
            Color32::from_rgb(50, 150, 80),
            Color32::from_rgb(255, 255, 255),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw transitions first (behind states)
    for transition in &diagram.transitions {
        if let (Some(&from_pos), Some(&to_pos)) = (state_pos.get(&transition.from), state_pos.get(&transition.to)) {
            let from_state = diagram.states.iter().find(|s| s.id == transition.from);
            let to_state = diagram.states.iter().find(|s| s.id == transition.to);
            
            let from_radius = if from_state.map(|s| s.is_start || s.is_end).unwrap_or(false) { 12.0 } else { state_width / 2.0 };
            let to_radius = if to_state.map(|s| s.is_start || s.is_end).unwrap_or(false) { 12.0 } else { state_width / 2.0 };
            
            let from = from_pos + offset;
            let to = to_pos + offset;
            
            let dir = (to - from).normalized();
            let start = from + dir * from_radius;
            let end = to - dir * to_radius;
            
            // Draw line
            painter.line_segment([start, end], Stroke::new(1.5, arrow_color));
            
            // Draw arrow head
            let arrow_size = 8.0;
            let perp = Vec2::new(-dir.y, dir.x);
            let arrow_left = end - dir * arrow_size + perp * (arrow_size * 0.4);
            let arrow_right = end - dir * arrow_size - perp * (arrow_size * 0.4);
            painter.add(egui::Shape::convex_polygon(
                vec![end, arrow_left, arrow_right],
                arrow_color,
                Stroke::NONE,
            ));
            
            // Draw label with background
            if let Some(label) = &transition.label {
                let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                let label_font_size = font_size - 2.0;
                let label_width = label.len() as f32 * label_font_size * 0.6 + 12.0;
                let label_height = label_font_size + 6.0;
                let label_pos = mid - Vec2::new(0.0, 14.0);
                let label_rect = Rect::from_center_size(label_pos, Vec2::new(label_width, label_height));
                painter.rect_filled(label_rect, 3.0, label_bg);
                painter.text(label_pos, egui::Align2::CENTER_CENTER, label, FontId::proportional(label_font_size), text_color);
            }
        }
    }

    // Draw states
    for state in &diagram.states {
        if let Some(&pos) = state_pos.get(&state.id) {
            let center = pos + offset;
            
            if state.is_start || state.is_end {
                let radius = if state.is_start { 10.0 } else { 14.0 };
                painter.circle_filled(center, radius, if state.is_start { start_color } else { state_stroke });
                if state.is_end {
                    painter.circle_filled(center, 7.0, if dark_mode { Color32::from_rgb(30, 35, 45) } else { Color32::WHITE });
                }
            } else {
                let rect = Rect::from_center_size(center, Vec2::new(state_width, state_height));
                painter.rect(rect, Rounding::same(8.0), state_fill, Stroke::new(2.0, state_stroke));
                painter.text(center, egui::Align2::CENTER_CENTER, &state.label, FontId::proportional(font_size), text_color);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mindmap Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A node in a mindmap.
#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub text: String,
    pub level: usize,
    pub children: Vec<MindmapNode>,
}

/// A parsed mindmap.
#[derive(Debug, Clone)]
pub struct Mindmap {
    pub root: Option<MindmapNode>,
}

/// Parse mermaid mindmap source.
pub fn parse_mindmap(source: &str) -> Result<Mindmap, String> {
    let mut root: Option<MindmapNode> = None;
    let mut stack: Vec<(usize, MindmapNode)> = Vec::new();

    for line in source.lines().skip(1) {
        if line.trim().is_empty() || line.trim().starts_with("%%") {
            continue;
        }

        // Count indentation (spaces)
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        let level = indent / 4; // Assume 4 spaces per level
        let text = line.trim();
        
        // Handle root node with (( )) or just text
        let text = if text.starts_with("root") {
            let inner = text.strip_prefix("root").unwrap_or(text).trim();
            if inner.starts_with("((") && inner.ends_with("))") {
                inner[2..inner.len()-2].to_string()
            } else if inner.starts_with('(') && inner.ends_with(')') {
                inner[1..inner.len()-1].to_string()
            } else {
                inner.to_string()
            }
        } else {
            text.to_string()
        };

        if text.is_empty() {
            continue;
        }

        let node = MindmapNode { text, level, children: Vec::new() };

        // Find parent
        while let Some((parent_level, _)) = stack.last() {
            if *parent_level >= level {
                let (_, finished_node) = stack.pop().unwrap();
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push(finished_node);
                } else {
                    root = Some(finished_node);
                }
            } else {
                break;
            }
        }

        stack.push((level, node));
    }

    // Pop remaining nodes
    while let Some((_, finished_node)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(finished_node);
        } else {
            root = Some(finished_node);
        }
    }

    if root.is_none() {
        return Err("No root node found in mindmap".to_string());
    }

    Ok(Mindmap { root })
}

/// Layout info for a mindmap node.
#[derive(Debug, Clone)]
struct MindmapLayout {
    center: Pos2,
    width: f32,
    children: Vec<MindmapLayout>,
}

/// Render a mindmap to the UI.
pub fn render_mindmap(
    ui: &mut Ui,
    mindmap: &Mindmap,
    dark_mode: bool,
    font_size: f32,
) {
    let root = match &mindmap.root {
        Some(r) => r,
        None => return,
    };

    let margin = 30.0_f32;
    let node_height = 28.0_f32;
    let level_width = 140.0_f32;
    let vertical_spacing = 12.0_f32;

    // First pass: calculate layout WITHOUT drawing
    fn calc_layout(
        node: &MindmapNode,
        x: f32,
        y: &mut f32,
        font_size: f32,
        node_height: f32,
        level_width: f32,
        vertical_spacing: f32,
    ) -> MindmapLayout {
        let node_width = (node.text.len() as f32 * font_size * 0.5 + 24.0).max(60.0).min(130.0);
        
        // First, layout all children
        let mut children_layouts: Vec<MindmapLayout> = Vec::new();
        for child in &node.children {
            let child_layout = calc_layout(child, x + level_width, y, font_size, node_height, level_width, vertical_spacing);
            children_layouts.push(child_layout);
        }
        
        // Calculate this node's center Y
        let center_y = if children_layouts.is_empty() {
            let cy = *y + node_height / 2.0;
            *y += node_height + vertical_spacing;
            cy
        } else {
            // Center among children
            let first_y = children_layouts.first().map(|c| c.center.y).unwrap_or(*y);
            let last_y = children_layouts.last().map(|c| c.center.y).unwrap_or(*y);
            (first_y + last_y) / 2.0
        };
        
        MindmapLayout {
            center: Pos2::new(x + node_width / 2.0, center_y),
            width: node_width,
            children: children_layouts,
        }
    }

    // Calculate layout
    let mut y = margin;
    let layout = calc_layout(root, margin, &mut y, font_size, node_height, level_width, vertical_spacing);

    // Calculate total size from layout
    fn calc_bounds(layout: &MindmapLayout, node_height: f32) -> (f32, f32) {
        let mut max_x = layout.center.x + layout.width / 2.0;
        let mut max_y = layout.center.y + node_height / 2.0;
        for child in &layout.children {
            let (cx, cy) = calc_bounds(child, node_height);
            max_x = max_x.max(cx);
            max_y = max_y.max(cy);
        }
        (max_x, max_y)
    }
    let (max_x, max_y) = calc_bounds(&layout, node_height);
    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let colors_by_level = if dark_mode {
        vec![
            Color32::from_rgb(100, 160, 220),
            Color32::from_rgb(120, 180, 140),
            Color32::from_rgb(200, 160, 100),
            Color32::from_rgb(180, 120, 160),
            Color32::from_rgb(140, 140, 180),
        ]
    } else {
        vec![
            Color32::from_rgb(50, 120, 180),
            Color32::from_rgb(60, 140, 80),
            Color32::from_rgb(180, 130, 50),
            Color32::from_rgb(150, 80, 130),
            Color32::from_rgb(100, 100, 150),
        ]
    };
    let text_color = if dark_mode { Color32::from_rgb(220, 230, 240) } else { Color32::from_rgb(30, 40, 50) };
    let bg_color = if dark_mode { Color32::from_rgb(40, 45, 55) } else { Color32::from_rgb(245, 248, 252) };

    // Allocate space based on calculated layout
    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Second pass: draw using pre-calculated layout
    fn draw_node(
        painter: &egui::Painter,
        node: &MindmapNode,
        layout: &MindmapLayout,
        offset: Vec2,
        level: usize,
        font_size: f32,
        node_height: f32,
        colors: &[Color32],
        text_color: Color32,
        bg_color: Color32,
    ) {
        let color = colors[level % colors.len()];
        let center = layout.center + offset;
        let rect = Rect::from_center_size(center, Vec2::new(layout.width, node_height));
        
        // Draw connections to children first (behind nodes)
        for (child_node, child_layout) in node.children.iter().zip(layout.children.iter()) {
            let child_center = child_layout.center + offset;
            let start = Pos2::new(rect.right(), center.y);
            let end = Pos2::new(child_center.x - child_layout.width / 2.0, child_center.y);
            painter.line_segment([start, end], Stroke::new(1.5, colors[(level + 1) % colors.len()].gamma_multiply(0.6)));
            
            // Recursively draw children
            draw_node(painter, child_node, child_layout, offset, level + 1, font_size, node_height, colors, text_color, bg_color);
        }
        
        // Draw this node
        painter.rect(rect, Rounding::same(node_height / 2.0), bg_color, Stroke::new(2.0, color));
        painter.text(center, egui::Align2::CENTER_CENTER, &node.text, FontId::proportional(font_size - 1.0), text_color);
    }

    draw_node(&painter, root, &layout, offset, 0, font_size, node_height, &colors_by_level, text_color, bg_color);
}

// ─────────────────────────────────────────────────────────────────────────────
// Class Diagram Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// Visibility modifier for class members.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Public,    // +
    Private,   // -
    Protected, // #
    Package,   // ~
}

/// A member of a class (attribute or method).
#[derive(Debug, Clone)]
pub struct ClassMember {
    pub visibility: Visibility,
    pub name: String,
    pub member_type: String, // return type or attribute type
    pub is_method: bool,
}

/// A class in the diagram.
#[derive(Debug, Clone)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub stereotype: Option<String>, // <<interface>>, <<abstract>>, etc.
    pub attributes: Vec<ClassMember>,
    pub methods: Vec<ClassMember>,
}

/// Relationship type between classes.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassRelationType {
    Inheritance,   // --|>
    Composition,   // *--
    Aggregation,   // o--
    Association,   // --
    Dependency,    // ..>
    Realization,   // ..|>
}

/// A relationship between classes.
#[derive(Debug, Clone)]
pub struct ClassRelation {
    pub from: String,
    pub to: String,
    pub relation_type: ClassRelationType,
    pub label: Option<String>,
    pub from_cardinality: Option<String>,
    pub to_cardinality: Option<String>,
}

/// A class diagram.
#[derive(Debug, Clone)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<ClassRelation>,
}

/// Parse a class diagram from source.
pub fn parse_class_diagram(source: &str) -> Result<ClassDiagram, String> {
    let mut classes: Vec<Class> = Vec::new();
    let mut relations: Vec<ClassRelation> = Vec::new();
    let mut current_class: Option<Class> = None;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Check for class definition start: class ClassName { or class ClassName
        if line.starts_with("class ") {
            // Save previous class if any
            if let Some(c) = current_class.take() {
                classes.push(c);
            }

            let rest = line[6..].trim();
            let (name, stereotype) = if rest.contains("~") {
                // class Animal~T~ for generics
                let parts: Vec<&str> = rest.splitn(2, '~').collect();
                (parts[0].trim().to_string(), None)
            } else if rest.contains("<<") && rest.contains(">>") {
                // class Interface <<interface>>
                let start = rest.find("<<").unwrap();
                let end = rest.find(">>").unwrap();
                let name = rest[..start].trim().trim_end_matches('{').trim().to_string();
                let stereo = rest[start+2..end].trim().to_string();
                (name, Some(stereo))
            } else {
                (rest.trim_end_matches('{').trim().to_string(), None)
            };

            current_class = Some(Class {
                id: name.clone(),
                name,
                stereotype,
                attributes: Vec::new(),
                methods: Vec::new(),
            });
            continue;
        }

        // Check for class definition end
        if line == "}" {
            if let Some(c) = current_class.take() {
                classes.push(c);
            }
            continue;
        }

        // Check for member definition inside class
        if current_class.is_some() && !line.contains("--") && !line.contains("..") {
            if let Some(member) = parse_class_member(line) {
                if let Some(ref mut c) = current_class {
                    if member.is_method {
                        c.methods.push(member);
                    } else {
                        c.attributes.push(member);
                    }
                }
            }
            continue;
        }

        // Check for relationship
        if let Some(relation) = parse_class_relation(line) {
            // Ensure classes exist
            for class_id in [&relation.from, &relation.to] {
                if !classes.iter().any(|c| &c.id == class_id) && 
                   current_class.as_ref().map(|c| &c.id != class_id).unwrap_or(true) {
                    classes.push(Class {
                        id: class_id.clone(),
                        name: class_id.clone(),
                        stereotype: None,
                        attributes: Vec::new(),
                        methods: Vec::new(),
                    });
                }
            }
            relations.push(relation);
        }
    }

    // Save last class
    if let Some(c) = current_class {
        classes.push(c);
    }

    if classes.is_empty() {
        return Err("No classes found in diagram".to_string());
    }

    Ok(ClassDiagram { classes, relations })
}

fn parse_class_member(line: &str) -> Option<ClassMember> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let (visibility, rest) = if line.starts_with('+') {
        (Visibility::Public, &line[1..])
    } else if line.starts_with('-') {
        (Visibility::Private, &line[1..])
    } else if line.starts_with('#') {
        (Visibility::Protected, &line[1..])
    } else if line.starts_with('~') {
        (Visibility::Package, &line[1..])
    } else {
        (Visibility::Public, line)
    };

    let is_method = rest.contains('(');
    
    // Parse type and name
    let (name, member_type) = if is_method {
        // method() ReturnType or method(): ReturnType
        let paren_idx = rest.find('(')?;
        let name_part = rest[..paren_idx].trim().to_string();
        let type_part = if rest.contains(':') {
            rest.rsplit(':').next().unwrap_or("void").trim().to_string()
        } else if rest.contains(')') {
            let after_paren = rest.rfind(')')?;
            rest[after_paren+1..].trim().to_string()
        } else {
            "void".to_string()
        };
        (name_part, type_part)
    } else {
        // attribute: Type or Type attribute
        if rest.contains(':') {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            (parts[0].trim().to_string(), parts.get(1).map(|s| s.trim()).unwrap_or("").to_string())
        } else {
            (rest.trim().to_string(), String::new())
        }
    };

    Some(ClassMember {
        visibility,
        name,
        member_type,
        is_method,
    })
}

fn parse_class_relation(line: &str) -> Option<ClassRelation> {
    // Patterns: A --|> B, A *-- B, A o-- B, A -- B, A ..> B, A ..|> B
    // With optional labels: A "label" --|> "label2" B
    
    let relation_patterns = [
        ("--|>", ClassRelationType::Inheritance),
        ("<|--", ClassRelationType::Inheritance),
        ("..|>", ClassRelationType::Realization),
        ("<|..", ClassRelationType::Realization),
        ("*--", ClassRelationType::Composition),
        ("--*", ClassRelationType::Composition),
        ("o--", ClassRelationType::Aggregation),
        ("--o", ClassRelationType::Aggregation),
        ("..>", ClassRelationType::Dependency),
        ("<..", ClassRelationType::Dependency),
        ("--", ClassRelationType::Association),
        ("..", ClassRelationType::Dependency),
    ];

    for (pattern, rel_type) in &relation_patterns {
        if line.contains(pattern) {
            let parts: Vec<&str> = line.split(pattern).collect();
            if parts.len() >= 2 {
                let from = parts[0].trim().trim_matches('"').split_whitespace().last()?.to_string();
                let to = parts[1].trim().trim_matches('"').split_whitespace().next()?.to_string();
                
                // Extract label if present (between quotes after relation)
                let label = if parts[1].contains(':') {
                    Some(parts[1].split(':').last()?.trim().to_string())
                } else {
                    None
                };
                
                return Some(ClassRelation {
                    from,
                    to,
                    relation_type: rel_type.clone(),
                    label,
                    from_cardinality: None,
                    to_cardinality: None,
                });
            }
        }
    }
    None
}

/// Render a class diagram to the UI.
pub fn render_class_diagram(
    ui: &mut Ui,
    diagram: &ClassDiagram,
    dark_mode: bool,
    font_size: f32,
) {
    let margin = 30.0_f32;
    let class_min_width = 120.0_f32;
    let member_height = font_size + 4.0;
    let header_height = font_size + 10.0;
    let spacing = Vec2::new(60.0, 50.0);

    // Calculate class sizes
    let mut class_sizes: HashMap<String, Vec2> = HashMap::new();
    for class in &diagram.classes {
        let name_width = class.name.len() as f32 * font_size * 0.55 + 20.0;
        let max_member_width = class.attributes.iter()
            .chain(class.methods.iter())
            .map(|m| (m.name.len() + m.member_type.len() + 4) as f32 * (font_size - 2.0) * 0.5 + 20.0)
            .fold(0.0_f32, f32::max);
        
        let width = name_width.max(max_member_width).max(class_min_width);
        let height = header_height 
            + (class.attributes.len().max(1) as f32 * member_height)
            + (class.methods.len().max(1) as f32 * member_height)
            + 10.0;
        
        class_sizes.insert(class.id.clone(), Vec2::new(width, height));
    }

    // Layout classes in grid
    let classes_per_row = 3.max((diagram.classes.len() as f32).sqrt().ceil() as usize);
    let mut class_pos: HashMap<String, Pos2> = HashMap::new();
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;
    let mut row_height = 0.0_f32;
    let mut x = margin;
    let mut y = margin;

    for (i, class) in diagram.classes.iter().enumerate() {
        let size = class_sizes.get(&class.id).copied().unwrap_or(Vec2::new(class_min_width, 80.0));
        
        if i > 0 && i % classes_per_row == 0 {
            x = margin;
            y += row_height + spacing.y;
            row_height = 0.0;
        }
        
        class_pos.insert(class.id.clone(), Pos2::new(x, y));
        max_x = max_x.max(x + size.x);
        max_y = max_y.max(y + size.y);
        row_height = row_height.max(size.y);
        x += size.x + spacing.x;
    }

    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let (class_fill, class_stroke, header_fill, text_color, line_color) = if dark_mode {
        (
            Color32::from_rgb(40, 48, 60),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(55, 70, 90),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(120, 150, 180),
        )
    } else {
        (
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(230, 240, 250),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(100, 130, 160),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw relations first
    for relation in &diagram.relations {
        if let (Some(&from_pos), Some(&to_pos)) = (class_pos.get(&relation.from), class_pos.get(&relation.to)) {
            let from_size = class_sizes.get(&relation.from).copied().unwrap_or(Vec2::new(100.0, 80.0));
            let to_size = class_sizes.get(&relation.to).copied().unwrap_or(Vec2::new(100.0, 80.0));
            
            let from_center = from_pos + from_size / 2.0 + offset;
            let to_center = to_pos + to_size / 2.0 + offset;
            
            let dir = (to_center - from_center).normalized();
            let start = from_center + dir * (from_size.x / 2.0).min(from_size.y / 2.0);
            let end = to_center - dir * (to_size.x / 2.0).min(to_size.y / 2.0);
            
            // Draw line based on type
            let is_dashed = matches!(relation.relation_type, ClassRelationType::Dependency | ClassRelationType::Realization);
            
            if is_dashed {
                draw_dashed_line(&painter, start, end, Stroke::new(1.5, line_color), 6.0, 4.0);
            } else {
                painter.line_segment([start, end], Stroke::new(1.5, line_color));
            }
            
            // Draw arrow/decoration at end
            let arrow_size = 10.0;
            let perp = Vec2::new(-dir.y, dir.x);
            
            match relation.relation_type {
                ClassRelationType::Inheritance | ClassRelationType::Realization => {
                    // Empty triangle
                    let tip = end;
                    let left = end - dir * arrow_size + perp * (arrow_size * 0.5);
                    let right = end - dir * arrow_size - perp * (arrow_size * 0.5);
                    painter.add(egui::Shape::convex_polygon(
                        vec![tip, left, right],
                        if dark_mode { Color32::from_rgb(40, 48, 60) } else { Color32::WHITE },
                        Stroke::new(1.5, line_color),
                    ));
                }
                ClassRelationType::Composition => {
                    // Filled diamond at start
                    let diamond_size = 8.0;
                    let d_center = start + dir * diamond_size;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            start,
                            d_center + perp * (diamond_size * 0.5),
                            start + dir * diamond_size * 2.0,
                            d_center - perp * (diamond_size * 0.5),
                        ],
                        line_color,
                        Stroke::NONE,
                    ));
                }
                ClassRelationType::Aggregation => {
                    // Empty diamond at start
                    let diamond_size = 8.0;
                    let d_center = start + dir * diamond_size;
                    painter.add(egui::Shape::convex_polygon(
                        vec![
                            start,
                            d_center + perp * (diamond_size * 0.5),
                            start + dir * diamond_size * 2.0,
                            d_center - perp * (diamond_size * 0.5),
                        ],
                        if dark_mode { Color32::from_rgb(40, 48, 60) } else { Color32::WHITE },
                        Stroke::new(1.5, line_color),
                    ));
                }
                ClassRelationType::Dependency => {
                    // Simple arrow
                    let left = end - dir * arrow_size + perp * (arrow_size * 0.4);
                    let right = end - dir * arrow_size - perp * (arrow_size * 0.4);
                    painter.line_segment([left, end], Stroke::new(1.5, line_color));
                    painter.line_segment([right, end], Stroke::new(1.5, line_color));
                }
                ClassRelationType::Association => {
                    // Simple line, no decoration
                }
            }
            
            // Draw label
            if let Some(label) = &relation.label {
                let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                painter.text(mid, egui::Align2::CENTER_CENTER, label, FontId::proportional(font_size - 2.0), text_color);
            }
        }
    }

    // Draw classes
    for class in &diagram.classes {
        if let Some(&pos) = class_pos.get(&class.id) {
            let size = class_sizes.get(&class.id).copied().unwrap_or(Vec2::new(100.0, 80.0));
            let rect = Rect::from_min_size(pos + offset, size);
            
            // Draw box
            painter.rect(rect, Rounding::same(4.0), class_fill, Stroke::new(2.0, class_stroke));
            
            // Draw header
            let header_rect = Rect::from_min_size(rect.min, Vec2::new(size.x, header_height));
            painter.rect_filled(header_rect, Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }, header_fill);
            
            // Draw stereotype
            let mut text_y = rect.min.y + 4.0;
            if let Some(stereo) = &class.stereotype {
                painter.text(
                    Pos2::new(rect.center().x, text_y + font_size * 0.35),
                    egui::Align2::CENTER_CENTER,
                    format!("<<{}>>", stereo),
                    FontId::proportional(font_size - 3.0),
                    text_color.gamma_multiply(0.7),
                );
                text_y += font_size * 0.7;
            }
            
            // Draw class name
            painter.text(
                Pos2::new(rect.center().x, text_y + font_size * 0.5),
                egui::Align2::CENTER_CENTER,
                &class.name,
                FontId::proportional(font_size),
                text_color,
            );
            
            // Draw separator after header
            let sep_y = rect.min.y + header_height;
            painter.line_segment(
                [Pos2::new(rect.min.x, sep_y), Pos2::new(rect.max.x, sep_y)],
                Stroke::new(1.0, class_stroke),
            );
            
            // Draw attributes
            let mut y = sep_y + 4.0;
            for attr in &class.attributes {
                let vis_char = match attr.visibility {
                    Visibility::Public => "+",
                    Visibility::Private => "-",
                    Visibility::Protected => "#",
                    Visibility::Package => "~",
                };
                let text = if attr.member_type.is_empty() {
                    format!("{} {}", vis_char, attr.name)
                } else {
                    format!("{} {}: {}", vis_char, attr.name, attr.member_type)
                };
                painter.text(
                    Pos2::new(rect.min.x + 6.0, y + member_height * 0.4),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
                y += member_height;
            }
            if class.attributes.is_empty() {
                y += member_height;
            }
            
            // Draw separator before methods
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(1.0, class_stroke),
            );
            y += 4.0;
            
            // Draw methods
            for method in &class.methods {
                let vis_char = match method.visibility {
                    Visibility::Public => "+",
                    Visibility::Private => "-",
                    Visibility::Protected => "#",
                    Visibility::Package => "~",
                };
                let text = if method.member_type.is_empty() || method.member_type == "void" {
                    format!("{} {}()", vis_char, method.name)
                } else {
                    format!("{} {}(): {}", vis_char, method.name, method.member_type)
                };
                painter.text(
                    Pos2::new(rect.min.x + 6.0, y + member_height * 0.4),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(font_size - 2.0),
                    text_color,
                );
                y += member_height;
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entity-Relationship Diagram Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// An entity in an ER diagram.
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub attributes: Vec<EntityAttribute>,
}

/// An attribute of an entity.
#[derive(Debug, Clone)]
pub struct EntityAttribute {
    pub name: String,
    pub attr_type: String,
    pub is_pk: bool,  // Primary key
    pub is_fk: bool,  // Foreign key
}

/// Cardinality in a relationship.
#[derive(Debug, Clone, PartialEq)]
pub enum Cardinality {
    ZeroOrOne,   // |o or o|
    ExactlyOne,  // ||
    ZeroOrMore,  // }o or o{
    OneOrMore,   // }|  or |{
}

/// A relationship between entities.
#[derive(Debug, Clone)]
pub struct ERRelation {
    pub from: String,
    pub to: String,
    pub from_cardinality: Cardinality,
    pub to_cardinality: Cardinality,
    pub label: Option<String>,
    pub identifying: bool,  // solid vs dashed line
}

/// An ER diagram.
#[derive(Debug, Clone)]
pub struct ERDiagram {
    pub entities: Vec<Entity>,
    pub relations: Vec<ERRelation>,
}

/// Parse an ER diagram from source.
pub fn parse_er_diagram(source: &str) -> Result<ERDiagram, String> {
    let mut entities: Vec<Entity> = Vec::new();
    let mut relations: Vec<ERRelation> = Vec::new();
    let mut current_entity: Option<Entity> = None;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Check for entity definition
        if line.ends_with('{') {
            if let Some(e) = current_entity.take() {
                entities.push(e);
            }
            let name = line.trim_end_matches('{').trim().to_string();
            current_entity = Some(Entity {
                name,
                attributes: Vec::new(),
            });
            continue;
        }

        // Check for entity end
        if line == "}" {
            if let Some(e) = current_entity.take() {
                entities.push(e);
            }
            continue;
        }

        // Check for attribute inside entity
        if current_entity.is_some() && !line.contains("||") && !line.contains("}") && !line.contains("{") {
            if let Some(attr) = parse_er_attribute(line) {
                if let Some(ref mut e) = current_entity {
                    e.attributes.push(attr);
                }
            }
            continue;
        }

        // Check for relationship
        if let Some(relation) = parse_er_relation(line) {
            // Ensure entities exist
            for entity_name in [&relation.from, &relation.to] {
                if !entities.iter().any(|e| &e.name == entity_name) &&
                   current_entity.as_ref().map(|e| &e.name != entity_name).unwrap_or(true) {
                    entities.push(Entity {
                        name: entity_name.clone(),
                        attributes: Vec::new(),
                    });
                }
            }
            relations.push(relation);
        }
    }

    // Save last entity
    if let Some(e) = current_entity {
        entities.push(e);
    }

    if entities.is_empty() {
        return Err("No entities found in diagram".to_string());
    }

    Ok(ERDiagram { entities, relations })
}

fn parse_er_attribute(line: &str) -> Option<EntityAttribute> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Format: type name PK/FK or type name "comment"
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let attr_type = parts[0].to_string();
    let name = parts[1].to_string();
    let is_pk = parts.get(2).map(|s| s.to_uppercase() == "PK").unwrap_or(false);
    let is_fk = parts.get(2).map(|s| s.to_uppercase() == "FK").unwrap_or(false);

    Some(EntityAttribute {
        name,
        attr_type,
        is_pk,
        is_fk,
    })
}

fn parse_er_relation(line: &str) -> Option<ERRelation> {
    // Patterns: ENTITY1 ||--o{ ENTITY2 : "label"
    // Cardinality markers: || (exactly one), |o (zero or one), }| (one or more), }o (zero or more)
    
    let relation_patterns = [
        ("||--||", Cardinality::ExactlyOne, Cardinality::ExactlyOne, true),
        ("||--|{", Cardinality::ExactlyOne, Cardinality::OneOrMore, true),
        ("||--o{", Cardinality::ExactlyOne, Cardinality::ZeroOrMore, true),
        ("||--o|", Cardinality::ExactlyOne, Cardinality::ZeroOrOne, true),
        ("|o--||", Cardinality::ZeroOrOne, Cardinality::ExactlyOne, true),
        ("|o--|{", Cardinality::ZeroOrOne, Cardinality::OneOrMore, true),
        ("|o--o{", Cardinality::ZeroOrOne, Cardinality::ZeroOrMore, true),
        ("|o--o|", Cardinality::ZeroOrOne, Cardinality::ZeroOrOne, true),
        ("}|--||", Cardinality::OneOrMore, Cardinality::ExactlyOne, true),
        ("}|--|{", Cardinality::OneOrMore, Cardinality::OneOrMore, true),
        ("}|--o{", Cardinality::OneOrMore, Cardinality::ZeroOrMore, true),
        ("}|--o|", Cardinality::OneOrMore, Cardinality::ZeroOrOne, true),
        ("}o--||", Cardinality::ZeroOrMore, Cardinality::ExactlyOne, true),
        ("}o--|{", Cardinality::ZeroOrMore, Cardinality::OneOrMore, true),
        ("}o--o{", Cardinality::ZeroOrMore, Cardinality::ZeroOrMore, true),
        ("}o--o|", Cardinality::ZeroOrMore, Cardinality::ZeroOrOne, true),
        // Dashed variants (non-identifying)
        ("||..||", Cardinality::ExactlyOne, Cardinality::ExactlyOne, false),
        ("||..|{", Cardinality::ExactlyOne, Cardinality::OneOrMore, false),
        ("||..o{", Cardinality::ExactlyOne, Cardinality::ZeroOrMore, false),
        ("||..o|", Cardinality::ExactlyOne, Cardinality::ZeroOrOne, false),
    ];

    for (pattern, from_card, to_card, identifying) in &relation_patterns {
        if line.contains(pattern) {
            let parts: Vec<&str> = line.split(pattern).collect();
            if parts.len() >= 2 {
                let from = parts[0].trim().to_string();
                let rest = parts[1].trim();
                
                // Extract entity name and label
                let (to, label) = if rest.contains(':') {
                    let label_parts: Vec<&str> = rest.splitn(2, ':').collect();
                    (
                        label_parts[0].trim().to_string(),
                        Some(label_parts[1].trim().trim_matches('"').to_string()),
                    )
                } else {
                    (rest.to_string(), None)
                };

                return Some(ERRelation {
                    from,
                    to,
                    from_cardinality: from_card.clone(),
                    to_cardinality: to_card.clone(),
                    label,
                    identifying: *identifying,
                });
            }
        }
    }
    None
}

/// Render an ER diagram to the UI.
pub fn render_er_diagram(
    ui: &mut Ui,
    diagram: &ERDiagram,
    dark_mode: bool,
    font_size: f32,
) {
    let margin = 30.0_f32;
    let entity_min_width = 140.0_f32;
    let attr_height = font_size + 4.0;
    let header_height = font_size + 12.0;
    let spacing = Vec2::new(80.0, 60.0);

    // Calculate entity sizes
    let mut entity_sizes: HashMap<String, Vec2> = HashMap::new();
    for entity in &diagram.entities {
        let name_width = entity.name.len() as f32 * font_size * 0.6 + 30.0;
        let max_attr_width = entity.attributes.iter()
            .map(|a| (a.attr_type.len() + a.name.len() + 6) as f32 * (font_size - 2.0) * 0.5 + 30.0)
            .fold(0.0_f32, f32::max);
        
        let width = name_width.max(max_attr_width).max(entity_min_width);
        let height = header_height + entity.attributes.len().max(1) as f32 * attr_height + 10.0;
        
        entity_sizes.insert(entity.name.clone(), Vec2::new(width, height));
    }

    // Layout entities
    let entities_per_row = 3.max((diagram.entities.len() as f32).sqrt().ceil() as usize);
    let mut entity_pos: HashMap<String, Pos2> = HashMap::new();
    let mut max_x = 0.0_f32;
    let mut max_y = 0.0_f32;
    let mut row_height = 0.0_f32;
    let mut x = margin;
    let mut y = margin;

    for (i, entity) in diagram.entities.iter().enumerate() {
        let size = entity_sizes.get(&entity.name).copied().unwrap_or(Vec2::new(entity_min_width, 80.0));
        
        if i > 0 && i % entities_per_row == 0 {
            x = margin;
            y += row_height + spacing.y;
            row_height = 0.0;
        }
        
        entity_pos.insert(entity.name.clone(), Pos2::new(x, y));
        max_x = max_x.max(x + size.x);
        max_y = max_y.max(y + size.y);
        row_height = row_height.max(size.y);
        x += size.x + spacing.x;
    }

    let total_width = max_x + margin;
    let total_height = max_y + margin;

    // Colors
    let (entity_fill, entity_stroke, header_fill, text_color, line_color, pk_color) = if dark_mode {
        (
            Color32::from_rgb(40, 48, 60),
            Color32::from_rgb(100, 160, 140),
            Color32::from_rgb(50, 70, 65),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(100, 150, 130),
            Color32::from_rgb(220, 180, 80),
        )
    } else {
        (
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(60, 140, 100),
            Color32::from_rgb(220, 245, 230),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(60, 120, 90),
            Color32::from_rgb(200, 150, 50),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw relations first
    for relation in &diagram.relations {
        if let (Some(&from_pos), Some(&to_pos)) = (entity_pos.get(&relation.from), entity_pos.get(&relation.to)) {
            let from_size = entity_sizes.get(&relation.from).copied().unwrap_or(Vec2::new(100.0, 80.0));
            let to_size = entity_sizes.get(&relation.to).copied().unwrap_or(Vec2::new(100.0, 80.0));
            
            let from_center = from_pos + from_size / 2.0 + offset;
            let to_center = to_pos + to_size / 2.0 + offset;
            
            let dir = (to_center - from_center).normalized();
            let start = from_center + dir * (from_size.x / 2.0).min(from_size.y / 2.0);
            let end = to_center - dir * (to_size.x / 2.0).min(to_size.y / 2.0);
            
            // Draw line
            if relation.identifying {
                painter.line_segment([start, end], Stroke::new(1.5, line_color));
            } else {
                draw_dashed_line(&painter, start, end, Stroke::new(1.5, line_color), 6.0, 4.0);
            }
            
            // Draw cardinality markers
            let marker_size = 8.0;
            let perp = Vec2::new(-dir.y, dir.x);
            
            // From side marker
            draw_cardinality_marker(&painter, start, dir, perp, &relation.from_cardinality, marker_size, line_color);
            
            // To side marker
            draw_cardinality_marker(&painter, end, -dir, -perp, &relation.to_cardinality, marker_size, line_color);
            
            // Draw label
            if let Some(label) = &relation.label {
                let mid = Pos2::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
                let label_bg = if dark_mode { Color32::from_rgb(35, 40, 50) } else { Color32::from_rgb(255, 255, 255) };
                let label_width = label.len() as f32 * (font_size - 2.0) * 0.5 + 10.0;
                let label_rect = Rect::from_center_size(mid, Vec2::new(label_width, font_size + 4.0));
                painter.rect_filled(label_rect, 2.0, label_bg);
                painter.text(mid, egui::Align2::CENTER_CENTER, label, FontId::proportional(font_size - 2.0), text_color);
            }
        }
    }

    // Draw entities
    for entity in &diagram.entities {
        if let Some(&pos) = entity_pos.get(&entity.name) {
            let size = entity_sizes.get(&entity.name).copied().unwrap_or(Vec2::new(100.0, 80.0));
            let rect = Rect::from_min_size(pos + offset, size);
            
            // Draw box
            painter.rect(rect, Rounding::same(4.0), entity_fill, Stroke::new(2.0, entity_stroke));
            
            // Draw header
            let header_rect = Rect::from_min_size(rect.min, Vec2::new(size.x, header_height));
            painter.rect_filled(header_rect, Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 }, header_fill);
            
            // Draw entity name
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + header_height / 2.0),
                egui::Align2::CENTER_CENTER,
                &entity.name,
                FontId::proportional(font_size),
                text_color,
            );
            
            // Draw separator
            let sep_y = rect.min.y + header_height;
            painter.line_segment(
                [Pos2::new(rect.min.x, sep_y), Pos2::new(rect.max.x, sep_y)],
                Stroke::new(1.0, entity_stroke),
            );
            
            // Draw attributes
            let mut y = sep_y + 4.0;
            for attr in &entity.attributes {
                let color = if attr.is_pk { pk_color } else { text_color };
                let prefix = if attr.is_pk { "🔑 " } else if attr.is_fk { "🔗 " } else { "" };
                let text = format!("{}{} {}", prefix, attr.attr_type, attr.name);
                
                painter.text(
                    Pos2::new(rect.min.x + 8.0, y + attr_height * 0.4),
                    egui::Align2::LEFT_CENTER,
                    text,
                    FontId::proportional(font_size - 2.0),
                    color,
                );
                y += attr_height;
            }
        }
    }
}

fn draw_cardinality_marker(
    painter: &egui::Painter,
    pos: Pos2,
    dir: Vec2,
    perp: Vec2,
    cardinality: &Cardinality,
    size: f32,
    color: Color32,
) {
    match cardinality {
        Cardinality::ExactlyOne => {
            // Two vertical lines ||
            let p1a = pos + perp * size * 0.4;
            let p1b = pos - perp * size * 0.4;
            let p2a = pos + dir * 4.0 + perp * size * 0.4;
            let p2b = pos + dir * 4.0 - perp * size * 0.4;
            painter.line_segment([p1a, p1b], Stroke::new(2.0, color));
            painter.line_segment([p2a, p2b], Stroke::new(2.0, color));
        }
        Cardinality::ZeroOrOne => {
            // Circle and line |o
            let line_p1 = pos + perp * size * 0.4;
            let line_p2 = pos - perp * size * 0.4;
            painter.line_segment([line_p1, line_p2], Stroke::new(2.0, color));
            painter.circle_stroke(pos + dir * 8.0, 4.0, Stroke::new(2.0, color));
        }
        Cardinality::ZeroOrMore => {
            // Circle and crow's foot }o
            painter.circle_stroke(pos + dir * 4.0, 4.0, Stroke::new(2.0, color));
            let foot_start = pos + dir * 12.0;
            painter.line_segment([foot_start, pos + perp * size * 0.5], Stroke::new(1.5, color));
            painter.line_segment([foot_start, pos - perp * size * 0.5], Stroke::new(1.5, color));
            painter.line_segment([foot_start, pos], Stroke::new(1.5, color));
        }
        Cardinality::OneOrMore => {
            // Line and crow's foot }|
            let line_p1 = pos + perp * size * 0.4;
            let line_p2 = pos - perp * size * 0.4;
            painter.line_segment([line_p1, line_p2], Stroke::new(2.0, color));
            let foot_start = pos + dir * 8.0;
            painter.line_segment([foot_start, pos + perp * size * 0.5], Stroke::new(1.5, color));
            painter.line_segment([foot_start, pos - perp * size * 0.5], Stroke::new(1.5, color));
            painter.line_segment([foot_start, pos], Stroke::new(1.5, color));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Git Graph Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A commit in a git graph.
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub id: String,
    pub branch: String,
    pub message: Option<String>,
    pub is_merge: bool,
    pub merge_from: Option<String>,
}

/// A branch in a git graph.
#[derive(Debug, Clone)]
pub struct GitBranch {
    pub name: String,
    pub color_idx: usize,
}

/// A git graph.
#[derive(Debug, Clone)]
pub struct GitGraph {
    pub commits: Vec<GitCommit>,
    pub branches: Vec<GitBranch>,
}

/// Parse a git graph from source.
pub fn parse_git_graph(source: &str) -> Result<GitGraph, String> {
    let mut commits: Vec<GitCommit> = Vec::new();
    let mut branches: Vec<GitBranch> = vec![GitBranch { name: "main".to_string(), color_idx: 0 }];
    let mut current_branch = "main".to_string();
    let mut commit_counter = 0;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        let line_lower = line.to_lowercase();

        // Parse commit
        if line_lower.starts_with("commit") {
            commit_counter += 1;
            let id = if line.contains("id:") {
                // commit id: "abc123"
                line.split("id:")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .unwrap_or_else(|| format!("c{}", commit_counter))
            } else {
                format!("c{}", commit_counter)
            };
            
            let message = if line.contains("msg:") {
                line.split("msg:")
                    .nth(1)
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            } else {
                None
            };

            commits.push(GitCommit {
                id,
                branch: current_branch.clone(),
                message,
                is_merge: false,
                merge_from: None,
            });
        }
        // Parse branch creation
        else if line_lower.starts_with("branch") {
            let name = line[6..].trim().to_string();
            if !branches.iter().any(|b| b.name == name) {
                branches.push(GitBranch {
                    name: name.clone(),
                    color_idx: branches.len(),
                });
            }
            current_branch = name;
        }
        // Parse checkout
        else if line_lower.starts_with("checkout") {
            let name = line[8..].trim().to_string();
            if !branches.iter().any(|b| b.name == name) {
                branches.push(GitBranch {
                    name: name.clone(),
                    color_idx: branches.len(),
                });
            }
            current_branch = name;
        }
        // Parse merge
        else if line_lower.starts_with("merge") {
            commit_counter += 1;
            let rest = line[5..].trim();
            let (merge_from, id) = if rest.contains("id:") {
                let parts: Vec<&str> = rest.split("id:").collect();
                let from = parts[0].trim().to_string();
                let id = parts[1].trim().trim_matches('"').trim_matches('\'').to_string();
                (from, id)
            } else {
                (rest.to_string(), format!("m{}", commit_counter))
            };

            commits.push(GitCommit {
                id,
                branch: current_branch.clone(),
                message: Some(format!("Merge {}", merge_from)),
                is_merge: true,
                merge_from: Some(merge_from),
            });
        }
    }

    if commits.is_empty() {
        return Err("No commits found in git graph".to_string());
    }

    Ok(GitGraph { commits, branches })
}

/// Render a git graph to the UI.
pub fn render_git_graph(
    ui: &mut Ui,
    graph: &GitGraph,
    dark_mode: bool,
    font_size: f32,
) {
    let margin = 30.0_f32;
    let commit_radius = 8.0_f32;
    let commit_spacing = 50.0_f32;
    let branch_spacing = 60.0_f32;  // Wider spacing between branches
    let label_width = 140.0_f32;    // Wider label area

    // Branch colors
    let branch_colors = if dark_mode {
        vec![
            Color32::from_rgb(100, 180, 100),  // green (main)
            Color32::from_rgb(100, 150, 220),  // blue
            Color32::from_rgb(220, 160, 100),  // orange
            Color32::from_rgb(180, 100, 180),  // purple
            Color32::from_rgb(220, 100, 100),  // red
            Color32::from_rgb(100, 200, 200),  // cyan
        ]
    } else {
        vec![
            Color32::from_rgb(60, 140, 60),
            Color32::from_rgb(60, 110, 180),
            Color32::from_rgb(180, 120, 60),
            Color32::from_rgb(140, 60, 140),
            Color32::from_rgb(180, 60, 60),
            Color32::from_rgb(60, 160, 160),
        ]
    };
    let text_color = if dark_mode { Color32::from_rgb(220, 230, 240) } else { Color32::from_rgb(30, 40, 50) };
    let line_bg = if dark_mode { Color32::from_rgb(50, 55, 65) } else { Color32::from_rgb(240, 245, 250) };

    // Calculate branch positions
    let mut branch_x: HashMap<String, f32> = HashMap::new();
    for (i, branch) in graph.branches.iter().enumerate() {
        branch_x.insert(branch.name.clone(), margin + label_width + i as f32 * branch_spacing);
    }

    let total_width = margin * 2.0 + label_width + graph.branches.len() as f32 * branch_spacing + 50.0;
    let total_height = margin * 2.0 + graph.commits.len() as f32 * commit_spacing;

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(300.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Track last commit position per branch for drawing lines
    let mut last_commit_pos: HashMap<String, Pos2> = HashMap::new();

    // Draw commits
    for (i, commit) in graph.commits.iter().enumerate() {
        let x = branch_x.get(&commit.branch).copied().unwrap_or(margin + label_width);
        let y = margin + i as f32 * commit_spacing;
        let pos = Pos2::new(x, y) + offset;

        let branch = graph.branches.iter().find(|b| b.name == commit.branch);
        let color = branch_colors[branch.map(|b| b.color_idx).unwrap_or(0) % branch_colors.len()];

        // Draw line from previous commit on same branch
        if let Some(prev_pos) = last_commit_pos.get(&commit.branch) {
            painter.line_segment([*prev_pos, pos], Stroke::new(3.0, color));
        }

        // Draw merge line
        if let Some(ref merge_from) = commit.merge_from {
            if let Some(merge_pos) = last_commit_pos.get(merge_from) {
                let merge_color = graph.branches.iter()
                    .find(|b| &b.name == merge_from)
                    .map(|b| branch_colors[b.color_idx % branch_colors.len()])
                    .unwrap_or(color);
                
                // Draw curved merge line
                let mid_y = (merge_pos.y + pos.y) / 2.0;
                let ctrl1 = Pos2::new(merge_pos.x, mid_y);
                let ctrl2 = Pos2::new(pos.x, mid_y);
                
                // Approximate bezier with line segments
                painter.line_segment([*merge_pos, ctrl1], Stroke::new(2.0, merge_color));
                painter.line_segment([ctrl1, ctrl2], Stroke::new(2.0, merge_color));
                painter.line_segment([ctrl2, pos], Stroke::new(2.0, merge_color));
            }
        }

        // Draw commit circle
        if commit.is_merge {
            // Merge commit - filled circle with border
            painter.circle_filled(pos, commit_radius, color);
            painter.circle_stroke(pos, commit_radius, Stroke::new(2.0, if dark_mode { Color32::WHITE } else { Color32::BLACK }));
        } else {
            // Regular commit - filled circle
            painter.circle_filled(pos, commit_radius, color);
        }

        // Draw commit label
        let label = commit.message.as_ref().unwrap_or(&commit.id);
        let label_bg_rect = Rect::from_min_size(
            Pos2::new(offset.x + margin - 5.0, pos.y - font_size * 0.5 - 2.0),
            Vec2::new(label_width - 10.0, font_size + 4.0),
        );
        painter.rect_filled(label_bg_rect, 3.0, line_bg);
        painter.text(
            Pos2::new(offset.x + margin, pos.y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(font_size - 2.0),
            text_color,
        );

        last_commit_pos.insert(commit.branch.clone(), pos);
    }

    // Draw branch labels at top
    for branch in &graph.branches {
        if let Some(&x) = branch_x.get(&branch.name) {
            let color = branch_colors[branch.color_idx % branch_colors.len()];
            let pos = Pos2::new(x, margin - 15.0) + offset;
            painter.text(
                pos,
                egui::Align2::CENTER_BOTTOM,
                &branch.name,
                FontId::proportional(font_size - 2.0),
                color,
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Gantt Chart Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A task in a Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttTask {
    pub id: String,
    pub name: String,
    pub section: Option<String>,
    pub start_day: i32,
    pub duration: i32,
    pub is_milestone: bool,
    pub is_done: bool,
    pub is_active: bool,
    pub is_crit: bool,
}

/// A section in a Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttSection {
    pub name: String,
}

/// A Gantt chart.
#[derive(Debug, Clone)]
pub struct GanttChart {
    pub title: Option<String>,
    pub tasks: Vec<GanttTask>,
    pub sections: Vec<GanttSection>,
}

/// Parse a Gantt chart from source.
pub fn parse_gantt_chart(source: &str) -> Result<GanttChart, String> {
    let mut title: Option<String> = None;
    let mut tasks: Vec<GanttTask> = Vec::new();
    let mut sections: Vec<GanttSection> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut task_map: HashMap<String, i32> = HashMap::new(); // task_id -> end_day
    let mut current_day = 0;
    let mut task_counter = 0;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse title
        if line.to_lowercase().starts_with("title") {
            title = Some(line[5..].trim().to_string());
            continue;
        }

        // Skip dateFormat and other directives
        if line.to_lowercase().starts_with("dateformat") 
            || line.to_lowercase().starts_with("excludes")
            || line.to_lowercase().starts_with("todaymarker")
            || line.to_lowercase().starts_with("axisformat") {
            continue;
        }

        // Parse section
        if line.to_lowercase().starts_with("section") {
            let name = line[7..].trim().to_string();
            sections.push(GanttSection { name: name.clone() });
            current_section = Some(name);
            continue;
        }

        // Parse task: name :id, start, duration or name :id, after id2, duration
        if line.contains(':') {
            task_counter += 1;
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() < 2 {
                continue;
            }

            let name = parts[0].trim().to_string();
            let spec = parts[1].trim();
            let spec_parts: Vec<&str> = spec.split(',').map(|s| s.trim()).collect();

            let mut id = format!("t{}", task_counter);
            let mut start_day = current_day;
            let mut duration = 1;
            let mut is_milestone = false;
            let mut is_done = false;
            let mut is_active = false;
            let mut is_crit = false;

            for (i, part) in spec_parts.iter().enumerate() {
                let part_lower = part.to_lowercase();
                
                if part_lower == "done" {
                    is_done = true;
                } else if part_lower == "active" {
                    is_active = true;
                } else if part_lower == "crit" {
                    is_crit = true;
                } else if part_lower == "milestone" {
                    is_milestone = true;
                    duration = 0;
                } else if part_lower.starts_with("after") {
                    // after task_id
                    let after_id = part[5..].trim();
                    if let Some(&end_day) = task_map.get(after_id) {
                        start_day = end_day;
                    }
                } else if part.ends_with('d') {
                    // Duration like "7d"
                    if let Ok(d) = part[..part.len()-1].parse::<i32>() {
                        duration = d;
                    }
                } else if i == 0 && !part.contains(' ') {
                    // First part might be ID
                    id = part.to_string();
                } else if let Ok(d) = part.parse::<i32>() {
                    // Plain number as duration
                    duration = d;
                }
            }

            let task = GanttTask {
                id: id.clone(),
                name,
                section: current_section.clone(),
                start_day,
                duration,
                is_milestone,
                is_done,
                is_active,
                is_crit,
            };

            task_map.insert(id, start_day + duration);
            current_day = start_day + duration;
            tasks.push(task);
        }
    }

    if tasks.is_empty() {
        return Err("No tasks found in Gantt chart".to_string());
    }

    Ok(GanttChart { title, tasks, sections })
}

/// Render a Gantt chart to the UI.
pub fn render_gantt_chart(
    ui: &mut Ui,
    chart: &GanttChart,
    dark_mode: bool,
    font_size: f32,
) {
    let margin = 30.0_f32;
    let row_height = 28.0_f32;
    let row_spacing = 6.0_f32;
    let label_width = 150.0_f32;
    let day_width = 20.0_f32;
    let header_height = 30.0_f32;

    // Find total duration
    let max_day = chart.tasks.iter()
        .map(|t| t.start_day + t.duration)
        .max()
        .unwrap_or(10);

    let total_width = margin * 2.0 + label_width + (max_day as f32 + 2.0) * day_width;
    let total_height = margin * 2.0 + header_height + chart.tasks.len() as f32 * (row_height + row_spacing);

    // Colors
    let (bg_color, grid_color, text_color, task_done, task_active, task_normal, task_crit, milestone_color) = if dark_mode {
        (
            Color32::from_rgb(35, 40, 50),
            Color32::from_rgb(60, 65, 75),
            Color32::from_rgb(220, 230, 240),
            Color32::from_rgb(80, 140, 80),
            Color32::from_rgb(100, 150, 200),
            Color32::from_rgb(80, 100, 140),
            Color32::from_rgb(200, 80, 80),
            Color32::from_rgb(220, 180, 60),
        )
    } else {
        (
            Color32::from_rgb(250, 252, 255),
            Color32::from_rgb(220, 225, 235),
            Color32::from_rgb(30, 40, 50),
            Color32::from_rgb(100, 180, 100),
            Color32::from_rgb(100, 160, 220),
            Color32::from_rgb(140, 160, 200),
            Color32::from_rgb(220, 100, 100),
            Color32::from_rgb(240, 200, 80),
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(400.0), total_height.max(100.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    let mut start_y = margin;
    if let Some(ref title) = chart.title {
        painter.text(
            Pos2::new(offset.x + total_width / 2.0, offset.y + margin / 2.0),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
        start_y += 10.0;
    }

    // Draw grid background
    let grid_rect = Rect::from_min_size(
        Pos2::new(offset.x + margin + label_width, offset.y + start_y),
        Vec2::new((max_day as f32 + 1.0) * day_width, total_height - margin - start_y),
    );
    painter.rect_filled(grid_rect, 0.0, bg_color);

    // Draw vertical grid lines (days)
    for day in 0..=max_day {
        let x = offset.x + margin + label_width + day as f32 * day_width;
        painter.line_segment(
            [Pos2::new(x, offset.y + start_y), Pos2::new(x, offset.y + total_height - margin)],
            Stroke::new(1.0, grid_color),
        );
        
        // Day labels (every 5 days or if small chart)
        if day % 5 == 0 || max_day <= 10 {
            painter.text(
                Pos2::new(x + day_width / 2.0, offset.y + start_y + header_height / 2.0),
                egui::Align2::CENTER_CENTER,
                format!("{}", day),
                FontId::proportional(font_size - 3.0),
                text_color.gamma_multiply(0.6),
            );
        }
    }

    // Draw tasks
    let mut y = start_y + header_height;
    let mut current_section: Option<String> = None;

    for task in &chart.tasks {
        // Draw section header if changed
        if task.section != current_section {
            if let Some(ref section) = task.section {
                painter.text(
                    Pos2::new(offset.x + margin, offset.y + y + row_height / 2.0),
                    egui::Align2::LEFT_CENTER,
                    section,
                    FontId::proportional(font_size - 1.0),
                    text_color.gamma_multiply(0.7),
                );
            }
            current_section = task.section.clone();
        }

        // Draw horizontal grid line
        painter.line_segment(
            [Pos2::new(offset.x + margin + label_width, offset.y + y + row_height + row_spacing / 2.0),
             Pos2::new(offset.x + margin + label_width + (max_day as f32 + 1.0) * day_width, offset.y + y + row_height + row_spacing / 2.0)],
            Stroke::new(0.5, grid_color),
        );

        // Draw task label
        painter.text(
            Pos2::new(offset.x + margin + label_width - 8.0, offset.y + y + row_height / 2.0),
            egui::Align2::RIGHT_CENTER,
            &task.name,
            FontId::proportional(font_size - 2.0),
            text_color,
        );

        // Draw task bar or milestone
        let task_x = offset.x + margin + label_width + task.start_day as f32 * day_width;
        let task_y = offset.y + y + 4.0;

        if task.is_milestone {
            // Diamond for milestone
            let center = Pos2::new(task_x, task_y + row_height / 2.0 - 4.0);
            let size = 8.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    center + Vec2::new(0.0, -size),
                    center + Vec2::new(size, 0.0),
                    center + Vec2::new(0.0, size),
                    center + Vec2::new(-size, 0.0),
                ],
                milestone_color,
                Stroke::NONE,
            ));
        } else {
            // Bar for task
            let bar_width = task.duration as f32 * day_width;
            let bar_height = row_height - 8.0;
            let bar_rect = Rect::from_min_size(
                Pos2::new(task_x, task_y),
                Vec2::new(bar_width.max(4.0), bar_height),
            );
            
            let bar_color = if task.is_crit {
                task_crit
            } else if task.is_done {
                task_done
            } else if task.is_active {
                task_active
            } else {
                task_normal
            };

            painter.rect_filled(bar_rect, 3.0, bar_color);
            
            // Progress indicator for done tasks
            if task.is_done {
                painter.text(
                    bar_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "✓",
                    FontId::proportional(font_size - 3.0),
                    Color32::WHITE,
                );
            }
        }

        y += row_height + row_spacing;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Timeline Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A period in a timeline.
#[derive(Debug, Clone)]
pub struct TimelinePeriod {
    pub label: String,
    pub events: Vec<String>,
}

/// A timeline diagram.
#[derive(Debug, Clone)]
pub struct Timeline {
    pub title: Option<String>,
    pub periods: Vec<TimelinePeriod>,
}

/// Parse a timeline from source.
pub fn parse_timeline(source: &str) -> Result<Timeline, String> {
    let mut title: Option<String> = None;
    let mut periods: Vec<TimelinePeriod> = Vec::new();
    let mut current_period: Option<TimelinePeriod> = None;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse title
        if line.to_lowercase().starts_with("title") {
            title = Some(line[5..].trim().to_string());
            continue;
        }

        // Check if this is a period label (doesn't start with whitespace in original)
        // or a section marker
        if line.to_lowercase().starts_with("section") {
            // Save previous period
            if let Some(p) = current_period.take() {
                if !p.events.is_empty() || periods.is_empty() {
                    periods.push(p);
                }
            }
            let label = line[7..].trim().to_string();
            current_period = Some(TimelinePeriod {
                label,
                events: Vec::new(),
            });
            continue;
        }

        // Check if line starts with a date/period pattern or is indented event
        if line.contains(':') {
            // Period with events: "2024-Q1 : Event 1 : Event 2"
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                // Save previous period
                if let Some(p) = current_period.take() {
                    periods.push(p);
                }
                
                let label = parts[0].trim().to_string();
                let events: Vec<String> = parts[1..].iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                periods.push(TimelinePeriod { label, events });
            }
        } else if current_period.is_some() {
            // Event for current period
            if let Some(ref mut p) = current_period {
                p.events.push(line.to_string());
            }
        } else {
            // New period without colon
            if let Some(p) = current_period.take() {
                periods.push(p);
            }
            current_period = Some(TimelinePeriod {
                label: line.to_string(),
                events: Vec::new(),
            });
        }
    }

    // Save last period
    if let Some(p) = current_period {
        periods.push(p);
    }

    if periods.is_empty() {
        return Err("No periods found in timeline".to_string());
    }

    Ok(Timeline { title, periods })
}

/// Render a timeline to the UI.
pub fn render_timeline(
    ui: &mut Ui,
    timeline: &Timeline,
    dark_mode: bool,
    font_size: f32,
) {
    let margin = 30.0_f32;
    let period_width = 160.0_f32;
    let period_spacing = 20.0_f32;
    let event_height = font_size + 6.0;
    let header_height = 40.0_f32;
    let timeline_y = 80.0_f32;

    // Calculate max events per period for height
    let max_events = timeline.periods.iter()
        .map(|p| p.events.len())
        .max()
        .unwrap_or(1)
        .max(1);

    let total_width = margin * 2.0 + timeline.periods.len() as f32 * (period_width + period_spacing);
    let total_height = margin * 2.0 + timeline_y + max_events as f32 * (event_height + 8.0) + 40.0;

    // Colors
    let (bg_color, line_color, text_color, period_colors) = if dark_mode {
        (
            Color32::from_rgb(35, 40, 50),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(220, 230, 240),
            vec![
                Color32::from_rgb(80, 140, 200),
                Color32::from_rgb(100, 180, 140),
                Color32::from_rgb(200, 160, 100),
                Color32::from_rgb(180, 120, 160),
                Color32::from_rgb(140, 160, 200),
            ],
        )
    } else {
        (
            Color32::from_rgb(250, 252, 255),
            Color32::from_rgb(100, 140, 180),
            Color32::from_rgb(30, 40, 50),
            vec![
                Color32::from_rgb(70, 130, 180),
                Color32::from_rgb(80, 160, 120),
                Color32::from_rgb(180, 140, 80),
                Color32::from_rgb(160, 100, 140),
                Color32::from_rgb(120, 140, 180),
            ],
        )
    };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(400.0), total_height.max(150.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    if let Some(ref title) = timeline.title {
        painter.text(
            Pos2::new(offset.x + total_width / 2.0, offset.y + margin),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
    }

    // Draw timeline line
    let line_y = offset.y + timeline_y;
    let line_start = offset.x + margin + period_width / 2.0;
    let line_end = offset.x + margin + (timeline.periods.len() - 1) as f32 * (period_width + period_spacing) + period_width / 2.0;
    painter.line_segment(
        [Pos2::new(line_start, line_y), Pos2::new(line_end, line_y)],
        Stroke::new(3.0, line_color),
    );

    // Draw periods and events
    for (i, period) in timeline.periods.iter().enumerate() {
        let x = offset.x + margin + i as f32 * (period_width + period_spacing) + period_width / 2.0;
        let color = period_colors[i % period_colors.len()];

        // Draw period marker (circle on timeline)
        painter.circle_filled(Pos2::new(x, line_y), 8.0, color);
        painter.circle_stroke(Pos2::new(x, line_y), 8.0, Stroke::new(2.0, if dark_mode { Color32::WHITE } else { Color32::BLACK }));

        // Draw period label above
        painter.text(
            Pos2::new(x, line_y - 20.0),
            egui::Align2::CENTER_BOTTOM,
            &period.label,
            FontId::proportional(font_size),
            color,
        );

        // Draw events below
        let mut event_y = line_y + 25.0;
        for event in &period.events {
            let event_rect = Rect::from_center_size(
                Pos2::new(x, event_y + event_height / 2.0),
                Vec2::new(period_width - 10.0, event_height),
            );
            painter.rect_filled(event_rect, 4.0, color.gamma_multiply(0.2));
            painter.rect_stroke(event_rect, 4.0, Stroke::new(1.0, color.gamma_multiply(0.5)));
            painter.text(
                event_rect.center(),
                egui::Align2::CENTER_CENTER,
                event,
                FontId::proportional(font_size - 2.0),
                text_color,
            );
            event_y += event_height + 8.0;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// User Journey Types and Renderer
// ─────────────────────────────────────────────────────────────────────────────

/// A task in a user journey.
#[derive(Debug, Clone)]
pub struct JourneyTask {
    pub name: String,
    pub score: i32,  // 1-5 satisfaction score
    pub actors: Vec<String>,
}

/// A section in a user journey.
#[derive(Debug, Clone)]
pub struct JourneySection {
    pub name: String,
    pub tasks: Vec<JourneyTask>,
}

/// A user journey diagram.
#[derive(Debug, Clone)]
pub struct UserJourney {
    pub title: Option<String>,
    pub sections: Vec<JourneySection>,
}

/// Parse a user journey from source.
pub fn parse_user_journey(source: &str) -> Result<UserJourney, String> {
    let mut title: Option<String> = None;
    let mut sections: Vec<JourneySection> = Vec::new();
    let mut current_section: Option<JourneySection> = None;

    for line in source.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        // Parse title
        if line.to_lowercase().starts_with("title") {
            title = Some(line[5..].trim().to_string());
            continue;
        }

        // Parse section
        if line.to_lowercase().starts_with("section") {
            // Save previous section
            if let Some(s) = current_section.take() {
                sections.push(s);
            }
            let name = line[7..].trim().to_string();
            current_section = Some(JourneySection {
                name,
                tasks: Vec::new(),
            });
            continue;
        }

        // Parse task: "Task name: score: Actor1, Actor2"
        if line.contains(':') {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 2 {
                let name = parts[0].trim().to_string();
                let score = parts[1].trim().parse::<i32>().unwrap_or(3);
                let actors: Vec<String> = if parts.len() > 2 {
                    parts[2].split(',').map(|s| s.trim().to_string()).collect()
                } else {
                    Vec::new()
                };

                let task = JourneyTask { name, score, actors };

                if let Some(ref mut s) = current_section {
                    s.tasks.push(task);
                } else {
                    // Create default section if none exists
                    current_section = Some(JourneySection {
                        name: "Journey".to_string(),
                        tasks: vec![task],
                    });
                }
            }
        }
    }

    // Save last section
    if let Some(s) = current_section {
        sections.push(s);
    }

    if sections.is_empty() {
        return Err("No sections found in user journey".to_string());
    }

    Ok(UserJourney { title, sections })
}

/// Render a user journey to the UI.
pub fn render_user_journey(
    ui: &mut Ui,
    journey: &UserJourney,
    dark_mode: bool,
    font_size: f32,
) {
    let margin = 30.0_f32;
    let task_width = 140.0_f32;
    let task_spacing = 15.0_f32;
    let section_spacing = 30.0_f32;
    let row_height = 80.0_f32;
    let header_height = 50.0_f32;

    // Count total tasks
    let total_tasks: usize = journey.sections.iter().map(|s| s.tasks.len()).sum();

    let total_width = margin * 2.0 + total_tasks as f32 * (task_width + task_spacing) 
        + journey.sections.len() as f32 * section_spacing;
    let total_height = margin * 2.0 + header_height + row_height + 60.0;

    // Colors - score based (1=red, 5=green)
    let score_colors = if dark_mode {
        vec![
            Color32::from_rgb(200, 80, 80),   // 1 - Bad
            Color32::from_rgb(200, 140, 80),  // 2 - Poor
            Color32::from_rgb(200, 200, 80),  // 3 - Neutral
            Color32::from_rgb(140, 200, 80),  // 4 - Good
            Color32::from_rgb(80, 200, 120),  // 5 - Great
        ]
    } else {
        vec![
            Color32::from_rgb(220, 100, 100),
            Color32::from_rgb(220, 160, 100),
            Color32::from_rgb(220, 220, 100),
            Color32::from_rgb(160, 200, 100),
            Color32::from_rgb(100, 180, 120),
        ]
    };
    let text_color = if dark_mode { Color32::from_rgb(220, 230, 240) } else { Color32::from_rgb(30, 40, 50) };
    let line_color = if dark_mode { Color32::from_rgb(80, 90, 100) } else { Color32::from_rgb(180, 190, 200) };
    let section_color = if dark_mode { Color32::from_rgb(100, 140, 180) } else { Color32::from_rgb(80, 120, 160) };

    let (response, painter) = ui.allocate_painter(
        Vec2::new(total_width.max(400.0), total_height.max(150.0)),
        egui::Sense::hover(),
    );
    let offset = response.rect.min.to_vec2();

    // Draw title
    if let Some(ref title) = journey.title {
        painter.text(
            Pos2::new(offset.x + total_width / 2.0, offset.y + margin),
            egui::Align2::CENTER_CENTER,
            title,
            FontId::proportional(font_size + 2.0),
            text_color,
        );
    }

    // Draw journey path
    let path_y = offset.y + header_height + row_height / 2.0 + 10.0;
    let mut x = offset.x + margin + task_width / 2.0;
    let mut prev_x: Option<f32> = None;
    let mut prev_score_y: Option<f32> = None;

    for (section_idx, section) in journey.sections.iter().enumerate() {
        // Draw section label
        let section_start_x = x;
        
        for (task_idx, task) in section.tasks.iter().enumerate() {
            let score_idx = (task.score.clamp(1, 5) - 1) as usize;
            let color = score_colors[score_idx];
            
            // Score affects Y position (higher score = higher position)
            let score_offset = (3 - task.score) as f32 * 10.0;
            let task_y = path_y + score_offset;

            // Draw connection line from previous task
            if let (Some(px), Some(py)) = (prev_x, prev_score_y) {
                painter.line_segment(
                    [Pos2::new(px, py), Pos2::new(x, task_y)],
                    Stroke::new(2.0, line_color),
                );
            }

            // Draw task card
            let card_rect = Rect::from_center_size(
                Pos2::new(x, task_y),
                Vec2::new(task_width - 10.0, 50.0),
            );
            painter.rect_filled(card_rect, 6.0, color.gamma_multiply(0.3));
            painter.rect_stroke(card_rect, 6.0, Stroke::new(2.0, color));

            // Draw task name
            painter.text(
                Pos2::new(x, task_y - 8.0),
                egui::Align2::CENTER_CENTER,
                &task.name,
                FontId::proportional(font_size - 2.0),
                text_color,
            );

            // Draw score indicator (emoji face)
            let face = match task.score {
                1 => "😫",
                2 => "😕",
                3 => "😐",
                4 => "🙂",
                _ => "😊",
            };
            painter.text(
                Pos2::new(x, task_y + 12.0),
                egui::Align2::CENTER_CENTER,
                face,
                FontId::proportional(font_size),
                text_color,
            );

            // Draw actors below
            if !task.actors.is_empty() {
                let actors_text = task.actors.join(", ");
                painter.text(
                    Pos2::new(x, card_rect.max.y + 10.0),
                    egui::Align2::CENTER_TOP,
                    &actors_text,
                    FontId::proportional(font_size - 3.0),
                    text_color.gamma_multiply(0.6),
                );
            }

            prev_x = Some(x);
            prev_score_y = Some(task_y);
            x += task_width + task_spacing;
        }

        // Draw section label above the section
        let section_end_x = x - task_spacing;
        let section_mid_x = (section_start_x + section_end_x) / 2.0;
        painter.text(
            Pos2::new(section_mid_x, offset.y + header_height - 5.0),
            egui::Align2::CENTER_BOTTOM,
            &section.name,
            FontId::proportional(font_size - 1.0),
            section_color,
        );

        // Add section spacing
        if section_idx < journey.sections.len() - 1 {
            x += section_spacing;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Result of attempting to render a mermaid diagram.
#[derive(Debug, Clone)]
pub enum RenderResult {
    /// Successfully rendered.
    Success,
    /// Parse error with message.
    ParseError(String),
    /// Diagram type not yet supported.
    Unsupported(String),
}

/// Render a mermaid diagram to the UI.
///
/// Returns a RenderResult indicating success or failure.
pub fn render_mermaid_diagram(
    ui: &mut Ui,
    source: &str,
    dark_mode: bool,
    font_size: f32,
) -> RenderResult {
    let source = source.trim();
    if source.is_empty() {
        return RenderResult::ParseError("Empty diagram source".to_string());
    }

    // Detect diagram type from first non-comment line
    let first_line = source.lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .unwrap_or("")
        .to_lowercase();
    
    if first_line.starts_with("flowchart") || first_line.starts_with("graph") {
        match parse_flowchart(source) {
            Ok(flowchart) => {
                let colors = if dark_mode {
                    FlowchartColors::dark()
                } else {
                    FlowchartColors::light()
                };
                let layout = layout_flowchart(&flowchart, ui.available_width(), font_size);
                render_flowchart(ui, &flowchart, &layout, &colors, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("sequencediagram") {
        match parse_sequence_diagram(source) {
            Ok(diagram) => {
                render_sequence_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("pie") {
        match parse_pie_chart(source) {
            Ok(chart) => {
                render_pie_chart(ui, &chart, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("statediagram") {
        match parse_state_diagram(source) {
            Ok(diagram) => {
                render_state_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("mindmap") {
        match parse_mindmap(source) {
            Ok(mindmap) => {
                render_mindmap(ui, &mindmap, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("classdiagram") {
        match parse_class_diagram(source) {
            Ok(diagram) => {
                render_class_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("erdiagram") {
        match parse_er_diagram(source) {
            Ok(diagram) => {
                render_er_diagram(ui, &diagram, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("gantt") {
        match parse_gantt_chart(source) {
            Ok(chart) => {
                render_gantt_chart(ui, &chart, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("gitgraph") {
        match parse_git_graph(source) {
            Ok(graph) => {
                render_git_graph(ui, &graph, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("timeline") {
        match parse_timeline(source) {
            Ok(timeline) => {
                render_timeline(ui, &timeline, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else if first_line.starts_with("journey") {
        match parse_user_journey(source) {
            Ok(journey) => {
                render_user_journey(ui, &journey, dark_mode, font_size);
                RenderResult::Success
            }
            Err(e) => RenderResult::ParseError(e),
        }
    } else {
        RenderResult::ParseError(format!("Unknown diagram type: {}", first_line))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_flowchart() {
        let source = "flowchart TD\n  A[Start] --> B[End]";
        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 2);
        assert_eq!(flowchart.edges.len(), 1);
    }

    #[test]
    fn test_parse_direction() {
        assert_eq!(parse_direction("flowchart TD"), FlowDirection::TopDown);
        assert_eq!(parse_direction("flowchart LR"), FlowDirection::LeftRight);
        assert_eq!(parse_direction("flowchart BT"), FlowDirection::BottomUp);
        assert_eq!(parse_direction("flowchart RL"), FlowDirection::RightLeft);
    }

    #[test]
    fn test_parse_node_shapes() {
        let rect = parse_node_from_text("A[Text]").unwrap();
        assert_eq!(rect.2, NodeShape::Rectangle);
        
        let round = parse_node_from_text("B(Text)").unwrap();
        assert_eq!(round.2, NodeShape::RoundRect);
        
        let diamond = parse_node_from_text("C{Decision}").unwrap();
        assert_eq!(diamond.2, NodeShape::Diamond);
        
        let circle = parse_node_from_text("D((Circle))").unwrap();
        assert_eq!(circle.2, NodeShape::Circle);
    }

    #[test]
    fn test_parse_edge_with_label() {
        let result = parse_edge_line("A -->|Yes| B");
        assert!(result.is_some());
        let (nodes, edge) = result.unwrap();
        assert_eq!(nodes.len(), 2);
        let edge = edge.unwrap();
        assert_eq!(edge.label, Some("Yes".to_string()));
    }

    #[test]
    fn test_parse_multiple_edges() {
        let source = r#"flowchart TD
            A[Start] --> B{Decision}
            B -->|Yes| C[Great!]
            B -->|No| D[Debug]
            D --> E[Fix]
            E --> B
            C --> F[End]"#;
        
        let result = parse_flowchart(source);
        assert!(result.is_ok());
        let flowchart = result.unwrap();
        assert_eq!(flowchart.nodes.len(), 6); // A, B, C, D, E, F
        assert_eq!(flowchart.edges.len(), 6);
    }

    #[test]
    fn test_layout_produces_valid_positions() {
        let source = "flowchart TD\n  A[Start] --> B[End]";
        let flowchart = parse_flowchart(source).unwrap();
        let layout = layout_flowchart(&flowchart, 400.0, 14.0);
        
        assert_eq!(layout.nodes.len(), 2);
        assert!(layout.nodes.contains_key("A"));
        assert!(layout.nodes.contains_key("B"));
        
        // In TD layout, B should be below A
        let a_pos = layout.nodes.get("A").unwrap().pos;
        let b_pos = layout.nodes.get("B").unwrap().pos;
        assert!(b_pos.y > a_pos.y);
    }
}
