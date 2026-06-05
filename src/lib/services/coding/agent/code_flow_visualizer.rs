use async_trait::async_trait;
use petgraph::algo::{connected_components, tarjan_scc, toposort};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::services::coding::agent::{
    code_intelligence::{Symbol, SymbolKind},
    code_review::CodeLocation,
    errors::{CodingAgentError, CodingAgentResult},
};

/// Visual code flow analyzer for creating diagrams and understanding code structure
#[derive(Clone)]
pub struct CodeFlowVisualizer {
    flow_analyzer: FlowAnalyzer,
    diagram_generator: DiagramGenerator,
    graph_builder: GraphBuilder,
    layout_engine: LayoutEngine,
    render_engine: RenderEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationRequest {
    pub target: VisualizationTarget,
    pub visualization_type: VisualizationType,
    pub options: VisualizationOptions,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisualizationTarget {
    File(PathBuf),
    Function(String),
    Class(String),
    Module(String),
    Project,
    CallChain(String, String), // From -> To
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VisualizationType {
    ControlFlow,
    DataFlow,
    CallGraph,
    ClassDiagram,
    SequenceDiagram,
    DependencyGraph,
    ArchitectureDiagram,
    StateFlow,
    ActivityDiagram,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationOptions {
    pub max_depth: usize,
    pub include_external_calls: bool,
    pub show_conditionals: bool,
    pub show_loops: bool,
    pub show_error_paths: bool,
    pub simplify: bool,
    pub cluster_by: Option<ClusteringStrategy>,
    pub highlight_patterns: Vec<String>,
    pub color_scheme: ColorScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusteringStrategy {
    Module,
    Package,
    Layer,
    Component,
    Functionality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorScheme {
    Default,
    HighContrast,
    Colorblind,
    Custom(HashMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    SVG,
    PNG,
    DOT,
    PlantUML,
    Mermaid,
    D2,
    JSON,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationResult {
    pub diagram: Diagram,
    pub metadata: DiagramMetadata,
    pub insights: Vec<CodeInsight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    pub format: OutputFormat,
    pub content: String,
    pub interactive_elements: Vec<InteractiveElement>,
    pub layers: Vec<Layer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveElement {
    pub id: String,
    pub element_type: ElementType,
    pub bounds: Bounds,
    pub action: InteractionAction,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Node,
    Edge,
    Cluster,
    Label,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionAction {
    Navigate(String),
    ShowDetails(String),
    Expand,
    Collapse,
    Highlight(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub elements: Vec<GraphElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphElement {
    pub id: String,
    pub element_type: GraphElementType,
    pub properties: HashMap<String, String>,
    pub style: ElementStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphElementType {
    Node(NodeElement),
    Edge(EdgeElement),
    Cluster(ClusterElement),
    Annotation(AnnotationElement),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeElement {
    pub label: String,
    pub node_type: NodeType,
    pub position: Position,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Function,
    Class,
    Module,
    Decision,
    Process,
    DataStore,
    Interface,
    State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeElement {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub edge_type: EdgeType,
    pub path: Vec<Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    Call,
    Return,
    DataFlow,
    ControlFlow,
    Dependency,
    Association,
    Inheritance,
    Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterElement {
    pub label: String,
    pub children: Vec<String>,
    pub cluster_type: ClusterType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterType {
    Module,
    Package,
    Namespace,
    Component,
    Layer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationElement {
    pub text: String,
    pub target: String,
    pub annotation_type: AnnotationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationType {
    Comment,
    Warning,
    Error,
    Info,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementStyle {
    pub color: String,
    pub fill: String,
    pub stroke: String,
    pub stroke_width: f32,
    pub font_size: f32,
    pub font_family: String,
    pub shape: Shape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Shape {
    Rectangle,
    RoundedRectangle,
    Circle,
    Diamond,
    Hexagon,
    Parallelogram,
    Cylinder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramMetadata {
    pub title: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub complexity_score: f32,
    pub node_count: usize,
    pub edge_count: usize,
    pub depth: usize,
    pub clusters: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeInsight {
    pub insight_type: InsightType,
    pub description: String,
    pub severity: InsightSeverity,
    pub location: Option<CodeLocation>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    Complexity,
    Coupling,
    Cohesion,
    Cycle,
    Bottleneck,
    DeadCode,
    Pattern,
    AntiPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightSeverity {
    Info,
    Warning,
    Critical,
}

/// Flow analyzer for understanding code flow
#[derive(Clone)]
pub struct FlowAnalyzer {
    graph: DiGraph<FlowNode, FlowEdge>,
    node_map: HashMap<String, NodeIndex>,
}

#[derive(Debug, Clone)]
pub struct FlowNode {
    pub id: String,
    pub node_type: FlowNodeType,
    pub label: String,
    pub code: String,
    pub location: CodeLocation,
}

#[derive(Debug, Clone)]
pub enum FlowNodeType {
    Entry,
    Exit,
    Statement,
    Condition,
    Loop,
    Call,
    Return,
    Exception,
}

#[derive(Debug, Clone)]
pub struct FlowEdge {
    pub edge_type: FlowEdgeType,
    pub condition: Option<String>,
    pub probability: f32,
}

#[derive(Debug, Clone)]
pub enum FlowEdgeType {
    Sequential,
    Conditional,
    Loop,
    Exception,
    Call,
    Return,
}

impl FlowAnalyzer {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    pub async fn analyze_control_flow(
        &mut self,
        code: &str,
    ) -> CodingAgentResult<ControlFlowGraph> {
        // Parse code and build control flow graph
        self.build_flow_graph(code)?;

        // Analyze graph properties
        let cycles = self.detect_cycles();
        let complexity = self.calculate_complexity();
        let paths = self.find_critical_paths();

        Ok(ControlFlowGraph {
            nodes: self.extract_nodes(),
            edges: self.extract_edges(),
            entry_points: self.find_entry_points(),
            exit_points: self.find_exit_points(),
            cycles,
            complexity,
            critical_paths: paths,
        })
    }

    fn build_flow_graph(&mut self, _code: &str) -> CodingAgentResult<()> {
        // Parse code and build graph
        // This would use AST parsing to build accurate flow
        Ok(())
    }

    fn detect_cycles(&self) -> Vec<Cycle> {
        let sccs = tarjan_scc(&self.graph);
        sccs.into_iter()
            .filter(|scc| scc.len() > 1)
            .map(|scc| Cycle {
                nodes: scc
                    .into_iter()
                    .map(|idx| self.graph[idx].id.clone())
                    .collect(),
            })
            .collect()
    }

    fn calculate_complexity(&self) -> f32 {
        // McCabe cyclomatic complexity
        let e = self.graph.edge_count() as f32;
        let n = self.graph.node_count() as f32;
        let p = connected_components(&self.graph) as f32;
        e - n + 2.0 * p
    }

    fn find_critical_paths(&self) -> Vec<Path> {
        // Find paths from entry to exit
        Vec::new()
    }

    fn extract_nodes(&self) -> Vec<FlowNode> {
        self.graph.node_weights().cloned().collect()
    }

    fn extract_edges(&self) -> Vec<(String, String, FlowEdge)> {
        self.graph
            .edge_indices()
            .filter_map(|e| {
                let (from, to) = self.graph.edge_endpoints(e)?;
                let edge = self.graph[e].clone();
                Some((self.graph[from].id.clone(), self.graph[to].id.clone(), edge))
            })
            .collect()
    }

    fn find_entry_points(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .filter(|&n| {
                self.graph
                    .edges_directed(n, petgraph::Direction::Incoming)
                    .count()
                    == 0
            })
            .map(|n| self.graph[n].id.clone())
            .collect()
    }

    fn find_exit_points(&self) -> Vec<String> {
        self.graph
            .node_indices()
            .filter(|&n| {
                self.graph
                    .edges_directed(n, petgraph::Direction::Outgoing)
                    .count()
                    == 0
            })
            .map(|n| self.graph[n].id.clone())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<(String, String, FlowEdge)>,
    pub entry_points: Vec<String>,
    pub exit_points: Vec<String>,
    pub cycles: Vec<Cycle>,
    pub complexity: f32,
    pub critical_paths: Vec<Path>,
}

#[derive(Debug, Clone)]
pub struct Cycle {
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Path {
    pub nodes: Vec<String>,
    pub length: usize,
    pub weight: f32,
}

/// Diagram generator for creating various diagram formats
#[derive(Clone)]
pub struct DiagramGenerator {
    templates: HashMap<VisualizationType, Template>,
}

#[derive(Clone)]
pub struct Template {
    pub name: String,
    pub format: String,
    pub variables: Vec<String>,
}

impl DiagramGenerator {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Add default templates
        templates.insert(
            VisualizationType::ControlFlow,
            Template {
                name: "control_flow".to_string(),
                format: "digraph".to_string(),
                variables: vec!["nodes".to_string(), "edges".to_string()],
            },
        );

        Self { templates }
    }

    pub async fn generate(
        &self,
        graph: &ControlFlowGraph,
        format: OutputFormat,
    ) -> CodingAgentResult<String> {
        match format {
            OutputFormat::DOT => self.generate_dot(graph),
            OutputFormat::PlantUML => self.generate_plantuml(graph),
            OutputFormat::Mermaid => self.generate_mermaid(graph),
            OutputFormat::D2 => self.generate_d2(graph),
            _ => Err(CodingAgentError::ConfigError {
                message: format!("Output format {:?} not supported", format),
            }),
        }
    }

    fn generate_dot(&self, graph: &ControlFlowGraph) -> CodingAgentResult<String> {
        let mut dot = String::from("digraph G {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Add nodes
        for node in &graph.nodes {
            let shape = match node.node_type {
                FlowNodeType::Condition => "diamond",
                FlowNodeType::Loop => "ellipse",
                _ => "box",
            };
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", shape={}];\n",
                node.id, node.label, shape
            ));
        }

        // Add edges
        for (from, to, edge) in &graph.edges {
            let label = edge.condition.as_deref().unwrap_or("");
            let style = match edge.edge_type {
                FlowEdgeType::Exception => "dashed",
                _ => "solid",
            };
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\", style={}];\n",
                from, to, label, style
            ));
        }

        dot.push_str("}\n");
        Ok(dot)
    }

    fn generate_plantuml(&self, graph: &ControlFlowGraph) -> CodingAgentResult<String> {
        let mut uml = String::from("@startuml\n");

        // Add nodes and edges
        for (from, to, edge) in &graph.edges {
            let arrow = match edge.edge_type {
                FlowEdgeType::Conditional => "->",
                FlowEdgeType::Loop => "-->",
                _ => "->",
            };
            let label = edge.condition.as_deref().unwrap_or("");
            uml.push_str(&format!("{} {} {} : {}\n", from, arrow, to, label));
        }

        uml.push_str("@enduml\n");
        Ok(uml)
    }

    fn generate_mermaid(&self, graph: &ControlFlowGraph) -> CodingAgentResult<String> {
        let mut mermaid = String::from("graph TD\n");

        // Add nodes
        for node in &graph.nodes {
            let shape = match node.node_type {
                FlowNodeType::Condition => format!("{{{}}}", node.label),
                FlowNodeType::Loop => format!("(({}))", node.label),
                _ => format!("[{}]", node.label),
            };
            mermaid.push_str(&format!("  {}[\"{}\"]\n", node.id, node.label));
        }

        // Add edges
        for (from, to, edge) in &graph.edges {
            let arrow = match edge.edge_type {
                FlowEdgeType::Conditional => "-->",
                _ => "-->",
            };
            let label = edge.condition.as_deref().unwrap_or("");
            if !label.is_empty() {
                mermaid.push_str(&format!("  {} {}|{}| {}\n", from, arrow, label, to));
            } else {
                mermaid.push_str(&format!("  {} {} {}\n", from, arrow, to));
            }
        }

        Ok(mermaid)
    }

    fn generate_d2(&self, graph: &ControlFlowGraph) -> CodingAgentResult<String> {
        let mut d2 = String::new();

        // Add nodes with styles
        for node in &graph.nodes {
            let shape = match node.node_type {
                FlowNodeType::Condition => "diamond",
                FlowNodeType::Loop => "circle",
                _ => "rectangle",
            };
            d2.push_str(&format!("{}: {{shape: {}}}\n", node.id, shape));
            d2.push_str(&format!("{}.label: \"{}\"\n", node.id, node.label));
        }

        // Add edges
        for (from, to, edge) in &graph.edges {
            if let Some(condition) = &edge.condition {
                d2.push_str(&format!("{} -> {}: \"{}\"\n", from, to, condition));
            } else {
                d2.push_str(&format!("{} -> {}\n", from, to));
            }
        }

        Ok(d2)
    }
}

/// Graph builder for constructing graph structures
#[derive(Clone)]
pub struct GraphBuilder;

impl GraphBuilder {
    pub fn new() -> Self {
        Self
    }

    pub async fn build_call_graph(&self, _code: &str) -> CodingAgentResult<CallGraph> {
        Ok(CallGraph {
            functions: Vec::new(),
            calls: Vec::new(),
        })
    }

    pub async fn build_dependency_graph(
        &self,
        _project: &str,
    ) -> CodingAgentResult<DependencyGraph> {
        Ok(DependencyGraph {
            modules: Vec::new(),
            dependencies: Vec::new(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CallGraph {
    pub functions: Vec<Function>,
    pub calls: Vec<Call>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub location: CodeLocation,
    pub complexity: f32,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub from: String,
    pub to: String,
    pub call_type: CallType,
}

#[derive(Debug, Clone)]
pub enum CallType {
    Direct,
    Indirect,
    Virtual,
    Recursive,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub modules: Vec<Module>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    pub dep_type: DependencyType,
}

#[derive(Debug, Clone)]
pub enum DependencyType {
    Import,
    Inheritance,
    Composition,
    Association,
}

/// Layout engine for graph layout algorithms
#[derive(Clone)]
pub struct LayoutEngine;

impl LayoutEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn layout(&self, graph: &ControlFlowGraph) -> LayoutResult {
        // Apply force-directed layout or hierarchical layout
        LayoutResult {
            positions: HashMap::new(),
            bounds: Bounds {
                x: 0.0,
                y: 0.0,
                width: 800.0,
                height: 600.0,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub positions: HashMap<String, Position>,
    pub bounds: Bounds,
}

/// Render engine for final output
#[derive(Clone)]
pub struct RenderEngine;

impl RenderEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn render(&self, diagram: &str, format: OutputFormat) -> CodingAgentResult<Vec<u8>> {
        match format {
            OutputFormat::SVG => Ok(diagram.as_bytes().to_vec()),
            OutputFormat::PNG => self.render_png(diagram).await,
            _ => Ok(diagram.as_bytes().to_vec()),
        }
    }

    async fn render_png(&self, _diagram: &str) -> CodingAgentResult<Vec<u8>> {
        // Would use a rendering library like resvg or graphviz
        Ok(Vec::new())
    }
}

impl CodeFlowVisualizer {
    pub fn new() -> Self {
        Self {
            flow_analyzer: FlowAnalyzer::new(),
            diagram_generator: DiagramGenerator::new(),
            graph_builder: GraphBuilder::new(),
            layout_engine: LayoutEngine::new(),
            render_engine: RenderEngine::new(),
        }
    }

    pub async fn visualize(
        &mut self,
        request: VisualizationRequest,
    ) -> CodingAgentResult<VisualizationResult> {
        // Analyze code flow based on visualization type
        let graph = match request.visualization_type {
            VisualizationType::ControlFlow => {
                // Read target code
                let code = self.read_target(&request.target).await?;
                self.flow_analyzer.analyze_control_flow(&code).await?
            }
            _ => {
                return Err(CodingAgentError::ConfigError {
                    message: format!(
                        "Visualization type {:?} not yet implemented",
                        request.visualization_type
                    ),
                });
            }
        };

        // Generate diagram
        let diagram_content = self
            .diagram_generator
            .generate(&graph, request.output_format.clone())
            .await?;

        // Apply layout
        let layout = self.layout_engine.layout(&graph);

        // Generate insights
        let insights = self.analyze_insights(&graph);

        // Create final diagram
        let diagram = Diagram {
            format: request.output_format,
            content: diagram_content,
            interactive_elements: Vec::new(),
            layers: Vec::new(),
        };

        // Create metadata
        let metadata = DiagramMetadata {
            title: format!("{:?} Diagram", request.visualization_type),
            description: String::new(),
            created_at: chrono::Utc::now(),
            complexity_score: graph.complexity,
            node_count: graph.nodes.len(),
            edge_count: graph.edges.len(),
            depth: 0,
            clusters: 0,
        };

        Ok(VisualizationResult {
            diagram,
            metadata,
            insights,
        })
    }

    async fn read_target(&self, target: &VisualizationTarget) -> CodingAgentResult<String> {
        match target {
            VisualizationTarget::File(path) => {
                tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| CodingAgentError::IoError {
                        message: e.to_string(),
                        path: None,
                    })
            }
            _ => Ok(String::new()),
        }
    }

    fn analyze_insights(&self, graph: &ControlFlowGraph) -> Vec<CodeInsight> {
        let mut insights = Vec::new();

        // Check for high complexity
        if graph.complexity > 10.0 {
            insights.push(CodeInsight {
                insight_type: InsightType::Complexity,
                description: format!("High cyclomatic complexity: {:.1}", graph.complexity),
                severity: InsightSeverity::Warning,
                location: None,
                suggestions: vec![
                    "Consider breaking down complex functions".to_string(),
                    "Extract conditional logic into separate functions".to_string(),
                ],
            });
        }

        // Check for cycles
        if !graph.cycles.is_empty() {
            insights.push(CodeInsight {
                insight_type: InsightType::Cycle,
                description: format!("Found {} cycles in control flow", graph.cycles.len()),
                severity: InsightSeverity::Info,
                location: None,
                suggestions: vec!["Review cycles for potential infinite loops".to_string()],
            });
        }

        insights
    }
}
