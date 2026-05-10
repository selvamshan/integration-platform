use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;
use common::{Error, FlowDefinition, FlowEdge, FlowNode, EdgeCondition, Message, Result};
use tracing::{info, warn};

use crate::templates::evaluate_condition;

/// Outcome of executing a single node
#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    Success,
    Failure,
}

/// Per-node execution result captured during a flow run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeRunResult {
    pub node_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub output: Option<serde_json::Value>,
}

/// Execute a flow using the graph (DAG) executor.
/// Thin wrapper around [`execute_graph_nodes`] for callers that have a full [`FlowDefinition`].
pub async fn execute_graph<'a, F, Fut>(
    flow: &'a FlowDefinition,
    input: Message,
    execute_node: F,
) -> Result<Message>
where
    F: Fn(&'a FlowNode, Message) -> Fut,
    Fut: std::future::Future<Output = Result<Message>> + 'a,
{
    let (result, _) = execute_graph_nodes_inner(&flow.nodes, &flow.edges, input, execute_node).await;
    result
}

/// Execute a flow and return per-node execution results alongside the final message.
pub async fn execute_graph_with_node_results<'a, F, Fut>(
    flow: &'a FlowDefinition,
    input: Message,
    execute_node: F,
) -> (Result<Message>, Vec<NodeRunResult>)
where
    F: Fn(&'a FlowNode, Message) -> Fut,
    Fut: std::future::Future<Output = Result<Message>> + 'a,
{
    execute_graph_nodes_inner(&flow.nodes, &flow.edges, input, execute_node).await
}

/// Execute a DAG expressed as bare node and edge slices.
pub async fn execute_graph_nodes<'a, F, Fut>(
    nodes: &'a [FlowNode],
    edges: &'a [FlowEdge],
    input: Message,
    execute_node: F,
) -> Result<Message>
where
    F: Fn(&'a FlowNode, Message) -> Fut,
    Fut: std::future::Future<Output = Result<Message>> + 'a,
{
    let (result, _) = execute_graph_nodes_inner(nodes, edges, input, execute_node).await;
    result
}

/// Execute a DAG and also return per-node execution results.
pub async fn execute_graph_nodes_with_results<'a, F, Fut>(
    nodes: &'a [FlowNode],
    edges: &'a [FlowEdge],
    input: Message,
    execute_node: F,
) -> (Result<Message>, Vec<NodeRunResult>)
where
    F: Fn(&'a FlowNode, Message) -> Fut,
    Fut: std::future::Future<Output = Result<Message>> + 'a,
{
    execute_graph_nodes_inner(nodes, edges, input, execute_node).await
}

/// Core DAG execution. Always collects per-node results.
///
/// Algorithm:
///   1. Build in-degree map and adjacency list from `edges`.
///   2. Seed the ready queue with nodes that have in-degree 0.
///   3. Run each ready node, record its outcome + output message.
///   4. For each outgoing edge, check whether the edge condition matches
///      the node's outcome / expression.  Track how many predecessors have
///      *fired* toward each successor; only enqueue the successor once all
///      of its in-edges that were actually traversed have resolved (fan-in).
///   5. Repeat until the queue is empty.
///   6. Return the message produced by the last node to execute (or the
///      input message if no nodes ran).
async fn execute_graph_nodes_inner<'a, F, Fut>(
    nodes: &'a [FlowNode],
    edges: &'a [FlowEdge],
    input: Message,
    execute_node: F,
) -> (Result<Message>, Vec<NodeRunResult>)
where
    F: Fn(&'a FlowNode, Message) -> Fut,
    Fut: std::future::Future<Output = Result<Message>> + 'a,
{
    if nodes.is_empty() {
        return (Ok(input), vec![]);
    }

    // Index nodes by id
    let node_map: HashMap<&str, &FlowNode> = nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();

    // Build adjacency list: from_id -> Vec<&FlowEdge>
    let mut outgoing: HashMap<&str, Vec<&FlowEdge>> = HashMap::new();
    for edge in edges {
        outgoing.entry(edge.from.as_str()).or_default().push(edge);
    }

    // Compute in-degree for every node
    let mut in_degree: HashMap<&str, usize> = nodes
        .iter()
        .map(|n| (n.id.as_str(), 0usize))
        .collect();
    for edge in edges {
        *in_degree.entry(edge.to.as_str()).or_default() += 1;
    }

    // How many predecessors have already signalled each node
    let mut resolved_in: HashMap<&str, usize> = HashMap::new();
    // Whether at least one incoming edge was actually traversed toward a node
    let mut has_active_predecessor: HashSet<&str> = HashSet::new();

    // Messages produced by each node (keyed by node id)
    let mut outputs: HashMap<&str, Message> = HashMap::new();
    // Outcomes for edge-condition evaluation
    let mut outcomes: HashMap<&str, StepOutcome> = HashMap::new();
    // Track the last node error; never cleared by a subsequent node's success.
    let mut last_error: Option<Error> = None;
    // Per-node run results
    let mut node_results: Vec<NodeRunResult> = Vec::new();

    // Seed queue with root nodes (in-degree 0)
    let mut queue: VecDeque<&str> = nodes
        .iter()
        .filter(|n| in_degree[n.id.as_str()] == 0)
        .map(|n| n.id.as_str())
        .collect();

    let mut last_output: Message = input.clone();

    while let Some(node_id) = queue.pop_front() {
        let node = match node_map.get(node_id) {
            Some(n) => n,
            None => {
                warn!("graph_executor: node '{}' not found in node_map", node_id);
                continue;
            }
        };

        // Use the last output reaching this node as its input.
        let node_input = outputs
            .get(node_id)
            .cloned()
            .unwrap_or_else(|| last_output.clone());

        info!("graph_executor: executing node '{}'", node_id);
        let t0 = Instant::now();
        let (node_output, outcome, node_error) = match execute_node(node, node_input).await {
            Ok(msg) => {
                info!("graph_executor: node '{}' succeeded", node_id);
                (msg, StepOutcome::Success, None)
            }
            Err(e) => {
                warn!("graph_executor: node '{}' failed: {}", node_id, e);
                let err_str = e.to_string();
                last_error = Some(e);
                (last_output.clone(), StepOutcome::Failure, Some(err_str))
            }
        };
        let duration_ms = t0.elapsed().as_millis() as u64;

        node_results.push(NodeRunResult {
            node_id: node_id.to_string(),
            success: outcome == StepOutcome::Success,
            error: node_error,
            duration_ms,
            output: if outcome == StepOutcome::Success {
                Some(node_output.payload.clone())
            } else {
                None
            },
        });

        last_output = node_output.clone();
        outputs.insert(node_id, node_output.clone());
        outcomes.insert(node_id, outcome.clone());

        // Walk outgoing edges
        let edges = outgoing.get(node_id).cloned().unwrap_or_default();
        for edge in edges {
            let traverse = should_traverse(edge, &outcome, &node_output);
            if !traverse {
                *resolved_in.entry(edge.to.as_str()).or_default() += 1;
                continue;
            }

            has_active_predecessor.insert(edge.to.as_str());
            outputs.insert(edge.to.as_str(), node_output.clone());

            let resolved = {
                let r = resolved_in.entry(edge.to.as_str()).or_default();
                *r += 1;
                *r
            };

            if resolved >= in_degree[edge.to.as_str()] {
                if has_active_predecessor.contains(edge.to.as_str()) {
                    queue.push_back(edge.to.as_str());
                }
            }
        }
    }

    if let Some(e) = last_error {
        return (Err(e), node_results);
    }
    (Ok(last_output), node_results)
}

fn should_traverse(edge: &FlowEdge, outcome: &StepOutcome, output: &Message) -> bool {
    match &edge.condition {
        EdgeCondition::Always => true,
        EdgeCondition::OnSuccess => outcome == &StepOutcome::Success,
        EdgeCondition::OnError => outcome == &StepOutcome::Failure,
        EdgeCondition::Expression => {
            let expr = match &edge.expression {
                Some(e) => e.as_str(),
                None => {
                    warn!("graph_executor: edge '{}' has Expression condition but no expression string", edge.id);
                    return false;
                }
            };
            evaluate_condition(expr, &output.payload)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::{EdgeCondition, FlowDefinition, FlowEdge, FlowNode, FlowStep, Message, Trigger};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn node(id: &str) -> FlowNode {
        FlowNode {
            id: id.to_string(),
            step: FlowStep::Log {
                name: id.to_string(),
                message: id.to_string(),
            },
            position_x: 0.0,
            position_y: 0.0,
        }
    }

    fn edge(id: &str, from: &str, to: &str, cond: EdgeCondition) -> FlowEdge {
        FlowEdge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            condition: cond,
            expression: None,
        }
    }

    fn expr_edge(id: &str, from: &str, to: &str, expr: &str) -> FlowEdge {
        FlowEdge {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            condition: EdgeCondition::Expression,
            expression: Some(expr.to_string()),
        }
    }

    fn minimal_flow(nodes: Vec<FlowNode>, edges: Vec<FlowEdge>) -> FlowDefinition {
        FlowDefinition {
            id: "test-flow".to_string(),
            name: "Test Flow".to_string(),
            client_id: None,
            trigger: Trigger::Http {
                path: "/test".to_string(),
                method: "POST".to_string(),
            },
            nodes,
            edges,
            steps: vec![],
            rate_limit: None,
            circuit_breaker: None,
            retry: None,
        }
    }

    // Returns a closure that echoes input and records which nodes ran.
    fn tracking_executor(
        executed: Arc<Mutex<Vec<String>>>,
        failing: Vec<&'static str>,
    ) -> impl Fn(&FlowNode, Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Message>> + '_>>
    {
        move |node: &FlowNode, msg: Message| {
            let id = node.id.clone();
            let exec = Arc::clone(&executed);
            let should_fail = failing.contains(&id.as_str());
            Box::pin(async move {
                exec.lock().unwrap().push(id.clone());
                if should_fail {
                    Err(Error::Flow(format!("node {} failed", id)))
                } else {
                    Ok(msg)
                }
            })
        }
    }

    // Returns a closure that produces distinct payloads per node id.
    fn payload_executor(
        payloads: std::collections::HashMap<&'static str, serde_json::Value>,
        failing: Vec<&'static str>,
    ) -> impl Fn(&FlowNode, Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Message>> + '_>>
    {
        let payloads = Arc::new(payloads);
        move |node: &FlowNode, msg: Message| {
            let id = node.id.clone();
            let payloads = Arc::clone(&payloads);
            let should_fail = failing.contains(&id.as_str());
            Box::pin(async move {
                if should_fail {
                    return Err(Error::Flow(format!("node {} failed", id)));
                }
                match payloads.get(id.as_str()) {
                    Some(p) => Ok(Message::new(p.clone())),
                    None => Ok(msg),
                }
            })
        }
    }

    // ── Unit Tests: execute_graph_nodes ───────────────────────────────────────

    #[tokio::test]
    async fn test_empty_nodes_returns_input() {
        let input = Message::new(json!({"from": "input"}));
        let result = execute_graph_nodes(
            &[],
            &[],
            input.clone(),
            |_node: &FlowNode, msg: Message| Box::pin(async move { Ok(msg) }),
        )
        .await
        .unwrap();
        assert_eq!(result.payload, json!({"from": "input"}));
    }

    #[tokio::test]
    async fn test_single_node_success_returns_node_output() {
        let nodes = vec![node("a")];
        let result = execute_graph_nodes(
            &nodes,
            &[],
            Message::new(json!({})),
            |_node: &FlowNode, _msg: Message| {
                Box::pin(async move { Ok(Message::new(json!({"node": "a"}))) })
            },
        )
        .await
        .unwrap();
        assert_eq!(result.payload["node"], "a");
    }

    #[tokio::test]
    async fn test_single_node_failure_returns_err() {
        let nodes = vec![node("a")];
        let result = execute_graph_nodes(
            &nodes,
            &[],
            Message::new(json!({})),
            |_node: &FlowNode, _msg: Message| {
                Box::pin(async move { Err(Error::Flow("node a failed".into())) })
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_linear_chain_all_nodes_run() {
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![
            edge("e1", "a", "b", EdgeCondition::Always),
            edge("e2", "b", "c", EdgeCondition::Always),
        ];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        assert_eq!(*executed.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn test_output_propagates_along_chain() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::Always)];
        let mut payloads = std::collections::HashMap::new();
        payloads.insert("a", json!({"step": "a_output"}));

        let result = execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            payload_executor(payloads, vec![]),
        )
        .await
        .unwrap();
        // B echoes input (A's output), so final payload has "step": "a_output"
        assert_eq!(result.payload["step"], "a_output");
    }

    // ── Edge Condition: OnSuccess ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_on_success_edge_fires_when_source_succeeds() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::OnSuccess)];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        assert!(executed.lock().unwrap().contains(&"b".to_string()));
    }

    #[tokio::test]
    async fn test_on_success_edge_skips_when_source_fails() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::OnSuccess)];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        // A fails → OnSuccess edge not traversed → B never runs
        let result = execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            tracking_executor(Arc::clone(&executed), vec!["a"]),
        )
        .await;
        assert!(result.is_err());
        assert!(!executed.lock().unwrap().contains(&"b".to_string()));
    }

    // ── Edge Condition: OnError ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_on_error_edge_fires_when_source_fails() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::OnError)];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        // A fails → OnError edge IS traversed → B runs
        // B succeeds so overall result is Ok (last_error cleared? No — last_error persists)
        execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            tracking_executor(Arc::clone(&executed), vec!["a"]),
        )
        .await
        .unwrap_err(); // last_error from A is still returned
        assert!(executed.lock().unwrap().contains(&"b".to_string()));
    }

    #[tokio::test]
    async fn test_on_error_edge_skips_when_source_succeeds() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::OnError)];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        assert!(!executed.lock().unwrap().contains(&"b".to_string()));
    }

    // ── Edge Condition: Always ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_always_edge_fires_even_when_source_fails() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::Always)];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        // A fails but Always edge still fires → B runs
        execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            tracking_executor(Arc::clone(&executed), vec!["a"]),
        )
        .await
        .unwrap_err();
        assert!(executed.lock().unwrap().contains(&"b".to_string()));
    }

    // ── Edge Condition: Expression ────────────────────────────────────────────

    #[tokio::test]
    async fn test_expression_edge_traversed_when_condition_true() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![expr_edge("e1", "a", "b", "{{status}} == active")];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let mut payloads = std::collections::HashMap::new();
        payloads.insert("a", json!({"status": "active"}));

        execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            {
                let exec = Arc::clone(&executed);
                payload_executor_tracked(payloads, vec![], exec)
            },
        )
        .await
        .unwrap();
        assert!(executed.lock().unwrap().contains(&"b".to_string()));
    }

    #[tokio::test]
    async fn test_expression_edge_skipped_when_condition_false() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![expr_edge("e1", "a", "b", "{{status}} == active")];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let mut payloads = std::collections::HashMap::new();
        payloads.insert("a", json!({"status": "inactive"}));

        execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            {
                let exec = Arc::clone(&executed);
                payload_executor_tracked(payloads, vec![], exec)
            },
        )
        .await
        .unwrap();
        assert!(!executed.lock().unwrap().contains(&"b".to_string()));
    }

    #[tokio::test]
    async fn test_expression_edge_without_expression_string_skips() {
        let nodes = vec![node("a"), node("b")];
        // Edge has Expression condition but no expression string
        let edges = vec![FlowEdge {
            id: "e1".to_string(),
            from: "a".to_string(),
            to: "b".to_string(),
            condition: EdgeCondition::Expression,
            expression: None,
        }];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        // B should NOT run — missing expression string evaluates to false
        assert!(!executed.lock().unwrap().contains(&"b".to_string()));
    }

    // ── Graph Topology Tests ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_fan_out_both_branches_run() {
        // A → B
        // A → C
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![
            edge("e1", "a", "b", EdgeCondition::Always),
            edge("e2", "a", "c", EdgeCondition::Always),
        ];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        let ran = executed.lock().unwrap().clone();
        assert!(ran.contains(&"a".to_string()));
        assert!(ran.contains(&"b".to_string()));
        assert!(ran.contains(&"c".to_string()));
    }

    #[tokio::test]
    async fn test_fan_in_waits_for_all_predecessors() {
        // A → C
        // B → C   (C is fan-in; both A and B must fire before C runs)
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![
            edge("e1", "a", "c", EdgeCondition::Always),
            edge("e2", "b", "c", EdgeCondition::Always),
        ];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        let ran = executed.lock().unwrap().clone();
        assert!(ran.contains(&"a".to_string()));
        assert!(ran.contains(&"b".to_string()));
        assert!(ran.contains(&"c".to_string()));
        // C must appear after both A and B
        let pos_c = ran.iter().position(|x| x == "c").unwrap();
        let pos_a = ran.iter().position(|x| x == "a").unwrap();
        let pos_b = ran.iter().position(|x| x == "b").unwrap();
        assert!(pos_c > pos_a && pos_c > pos_b);
    }

    #[tokio::test]
    async fn test_diamond_graph_all_nodes_run() {
        // A → B
        // A → C
        // B → D
        // C → D
        let nodes = vec![node("a"), node("b"), node("c"), node("d")];
        let edges = vec![
            edge("e1", "a", "b", EdgeCondition::Always),
            edge("e2", "a", "c", EdgeCondition::Always),
            edge("e3", "b", "d", EdgeCondition::Always),
            edge("e4", "c", "d", EdgeCondition::Always),
        ];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        let ran = executed.lock().unwrap().clone();
        assert_eq!(ran.len(), 4);
        for id in &["a", "b", "c", "d"] {
            assert!(ran.contains(&id.to_string()), "node '{}' did not run", id);
        }
        // D must be last
        assert_eq!(ran.last().unwrap(), "d");
    }

    #[tokio::test]
    async fn test_multiple_roots_both_run() {
        // A (root) and B (root) — no edges connecting them
        let nodes = vec![node("a"), node("b")];
        let edges: Vec<FlowEdge> = vec![];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(&nodes, &edges, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        let ran = executed.lock().unwrap().clone();
        assert!(ran.contains(&"a".to_string()));
        assert!(ran.contains(&"b".to_string()));
    }

    #[tokio::test]
    async fn test_node_skipped_when_no_active_predecessor_fires() {
        // A → C (OnSuccess), B → C (OnSuccess)
        // Both A and B fail → no active predecessor → C must NOT run
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![
            edge("e1", "a", "c", EdgeCondition::OnSuccess),
            edge("e2", "b", "c", EdgeCondition::OnSuccess),
        ];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            tracking_executor(Arc::clone(&executed), vec!["a", "b"]),
        )
        .await
        .unwrap_err();
        assert!(!executed.lock().unwrap().contains(&"c".to_string()));
    }

    #[tokio::test]
    async fn test_last_error_propagated_even_when_later_node_succeeds() {
        // A fails → Always edge → B succeeds.
        // last_error from A is never cleared, so the overall result is Err.
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::Always)];
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let result = execute_graph_nodes(
            &nodes,
            &edges,
            Message::new(json!({})),
            tracking_executor(Arc::clone(&executed), vec!["a"]),
        )
        .await;
        assert!(result.is_err());
        // B still ran (Always edge)
        assert!(executed.lock().unwrap().contains(&"b".to_string()));
    }

    // ── Integration test: execute_graph wrapper ───────────────────────────────

    #[tokio::test]
    async fn test_execute_graph_delegates_to_execute_graph_nodes() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("e1", "a", "b", EdgeCondition::Always)];
        let flow = minimal_flow(nodes, edges);
        let executed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        execute_graph(&flow, Message::new(json!({})), tracking_executor(Arc::clone(&executed), vec![]))
            .await
            .unwrap();
        assert_eq!(*executed.lock().unwrap(), vec!["a", "b"]);
    }

    #[tokio::test]
    async fn test_execute_graph_empty_flow_returns_input() {
        let flow = minimal_flow(vec![], vec![]);
        let input = Message::new(json!({"source": "original"}));
        let result = execute_graph(
            &flow,
            input,
            |_node: &FlowNode, msg: Message| Box::pin(async move { Ok(msg) }),
        )
        .await
        .unwrap();
        assert_eq!(result.payload["source"], "original");
    }

    // ── StepOutcome tests ─────────────────────────────────────────────────────

    #[test]
    fn test_step_outcome_equality() {
        assert_eq!(StepOutcome::Success, StepOutcome::Success);
        assert_eq!(StepOutcome::Failure, StepOutcome::Failure);
        assert_ne!(StepOutcome::Success, StepOutcome::Failure);
    }

    // ── Helper used by expression tests ──────────────────────────────────────

    // Like payload_executor but also records execution to `executed`.
    fn payload_executor_tracked(
        payloads: std::collections::HashMap<&'static str, serde_json::Value>,
        failing: Vec<&'static str>,
        executed: Arc<Mutex<Vec<String>>>,
    ) -> impl Fn(&FlowNode, Message) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Message>> + '_>>
    {
        let payloads = Arc::new(payloads);
        move |node: &FlowNode, msg: Message| {
            let id = node.id.clone();
            let payloads = Arc::clone(&payloads);
            let exec = Arc::clone(&executed);
            let should_fail = failing.contains(&id.as_str());
            Box::pin(async move {
                exec.lock().unwrap().push(id.clone());
                if should_fail {
                    return Err(Error::Flow(format!("node {} failed", id)));
                }
                match payloads.get(id.as_str()) {
                    Some(p) => Ok(Message::new(p.clone())),
                    None => Ok(msg),
                }
            })
        }
    }
}
