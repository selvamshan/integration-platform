use std::collections::{HashMap, HashSet, VecDeque};
use common::{Error, FlowDefinition, FlowEdge, FlowNode, EdgeCondition, Message, Result};
use tracing::{info, warn};

use crate::templates::evaluate_condition;

/// Outcome of executing a single node
#[derive(Debug, Clone, PartialEq)]
pub enum StepOutcome {
    Success,
    Failure,
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
    execute_graph_nodes(&flow.nodes, &flow.edges, input, execute_node).await
}

/// Execute a DAG expressed as bare node and edge slices.
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
    if nodes.is_empty() {
        return Ok(input);
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
    // A node failure always propagates unless the flow has no more nodes to run.
    let mut last_error: Option<Error> = None;

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
        // For fan-in nodes we could merge; for now we use the most-recent predecessor output.
        let node_input = outputs
            .get(node_id)
            .cloned()
            .unwrap_or_else(|| last_output.clone());

        info!("graph_executor: executing node '{}'", node_id);
        let (node_output, outcome) = match execute_node(node, node_input).await {
            Ok(msg) => {
                info!("graph_executor: node '{}' succeeded", node_id);
                (msg, StepOutcome::Success)
            }
            Err(e) => {
                warn!("graph_executor: node '{}' failed: {}", node_id, e);
                last_error = Some(e);
                (last_output.clone(), StepOutcome::Failure)
            }
        };

        last_output = node_output.clone();
        outputs.insert(node_id, node_output.clone());
        outcomes.insert(node_id, outcome.clone());

        // Walk outgoing edges
        let edges = outgoing.get(node_id).cloned().unwrap_or_default();
        for edge in edges {
            let traverse = should_traverse(edge, &outcome, &node_output);
            if !traverse {
                // Edge not taken — count it as resolved so fan-in can still fire
                // if all other predecessors do traverse.
                *resolved_in.entry(edge.to.as_str()).or_default() += 1;
                continue;
            }

            has_active_predecessor.insert(edge.to.as_str());
            // Propagate the output to the target (last writer wins for fan-in)
            outputs.insert(edge.to.as_str(), node_output.clone());

            let resolved = {
                let r = resolved_in.entry(edge.to.as_str()).or_default();
                *r += 1;
                *r
            };

            // Enqueue when all predecessors have been processed
            if resolved >= in_degree[edge.to.as_str()] {
                if has_active_predecessor.contains(edge.to.as_str()) {
                    queue.push_back(edge.to.as_str());
                }
            }
        }
    }

    if let Some(e) = last_error {
        return Err(e);
    }
    Ok(last_output)
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
