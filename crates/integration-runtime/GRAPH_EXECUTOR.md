# Graph Executor — Technical Documentation

## Overview

The graph executor is a DAG (Directed Acyclic Graph) execution engine for the integration platform. It enables conditional, parallel-path, and error-recovery flow routing — replacing the legacy linear step executor when a `FlowDefinition` contains `nodes` and `edges`.

A `Loop` node can also use a sub-graph (nodes + edges) as its body, enabling conditional branching **inside** each loop iteration.

**Location:** `crates/integration-runtime/src/graph_executor.rs`

---

## Architecture

```
FlowExecutor::execute_flow()
        │
        ├── flow.is_graph_flow()? ──Yes──▶ execute_graph(flow, …)
        │                                          │
        │                                          └── execute_graph_nodes(&flow.nodes, &flow.edges, …)
        │                                                    │
        │                                             execute_node callback
        │                                                    │
        │                                             FlowExecutor::execute_step()
        │                                                    │
        │                                             FlowStep::Loop with nodes? ──Yes──▶ LoopBody::Graph
        │                                                    │                                    │
        │                                                    │                         run_graph() per iteration
        │                                                    │                                    │
        │                                                    │                         execute_graph_nodes() [recursive]
        │                                                    │
        │                                             FlowStep::Loop steps only ──▶ LoopBody::Steps
        │                                                                                   │
        │                                                                           run_steps() per iteration
        │
        └── No ──▶ execute_steps() [linear / legacy]
```

---

## Core Data Structures

### Message
Flows through the entire pipeline as the primary data carrier.

```rust
pub struct Message {
    pub id: String,
    pub headers: HashMap<String, String>,
    pub payload: serde_json::Value,   // main data — mutated by steps
    pub attributes: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}
```

### FlowNode
A single vertex in the DAG, wrapping a `FlowStep` with editor metadata.

```rust
pub struct FlowNode {
    pub id: String,      // unique node identifier (used in edges)
    pub step: FlowStep,  // the operation this node performs
    pub position_x: f64, // visual editor only
    pub position_y: f64,
}
```

### FlowStep (node types)

| Variant | Purpose |
|---------|---------|
| `Log { name, message }` | Log a message; templates resolved against payload |
| `Call { name, connector, operation, params }` | Invoke a registered connector |
| `Transform { name, spec }` | Apply a JSON transformation spec |
| `Loop { name, loop_mode, … }` | Nested while / foreach / count loop — body can be linear steps **or a sub-graph** |

### FlowStep::Loop fields

| Field | Type | Purpose |
|-------|------|---------|
| `loop_mode` | `String` | `"while"` / `"foreach"` / `"count"` |
| `condition` | `Option<String>` | Expression for `while` mode |
| `iterate_over` | `Option<String>` | Template path to array for `foreach` |
| `count` | `Option<usize>` | Iteration count for `count` mode |
| `steps` | `Vec<FlowStep>` | Linear body (legacy / simple) |
| `nodes` | `Vec<FlowNode>` | Sub-graph body — used when non-empty |
| `edges` | `Vec<FlowEdge>` | Sub-graph edges — used when `nodes` is non-empty |
| `max_iterations` | `Option<usize>` | Safety cap (default 1000) |

When `nodes` is non-empty, the sub-graph is executed each iteration via `execute_graph_nodes`. `steps` is ignored.

### FlowEdge
A directed edge between two nodes with a traversal condition.

```rust
pub struct FlowEdge {
    pub id: String,
    pub from: String,               // source node id
    pub to: String,                 // target node id
    pub condition: EdgeCondition,
    pub expression: Option<String>, // used when condition == Expression
}
```

### EdgeCondition

| Variant | Traversal rule |
|---------|----------------|
| `Always` | Always traverse |
| `OnSuccess` | Only when source node succeeded |
| `OnError` | Only when source node failed |
| `Expression` | Evaluate template expression against current payload |

### StepOutcome
Result tracked per executed node for edge-condition evaluation.

```rust
pub enum StepOutcome { Success, Failure }
```

### LoopBody
Selects between a linear step list and a sub-graph as the loop body.

```rust
pub enum LoopBody<'a> {
    Steps(&'a [FlowStep]),
    Graph { nodes: &'a [FlowNode], edges: &'a [FlowEdge] },
}
```

---

## Flow Diagram

```
                        ┌────────────────────────────────────────────┐
                        │       execute_graph_nodes()                │
                        │                                            │
  nodes + edges ───────▶│  1. Build node_map (id → &FlowNode)       │
  + input Message        │  2. Build outgoing (id → Vec<&FlowEdge>)  │
                        │  3. Compute in_degree per node             │
                        │  4. Seed queue ← nodes with in_degree == 0│
                        └────────────────┬───────────────────────────┘
                                         │
                          ┌──────────────▼───────────────┐
                          │     queue.pop_front()         │
                          │   (next ready node_id)        │
                          └──────────────┬────────────────┘
                                         │
                          ┌──────────────▼───────────────────────┐
                          │  Resolve input message for node       │
                          │  (outputs[node_id] ?? last_output)    │
                          └──────────────┬────────────────────────┘
                                         │
                          ┌──────────────▼───────────────────────┐
                          │  execute_node(node, input)            │
                          │  ┌──────────────────────────────┐    │
                          │  │  FlowExecutor::execute_step() │    │
                          │  │  • Log / Call / Transform     │    │
                          │  │  • Loop (steps or sub-graph)  │    │
                          │  └──────────────────────────────┘    │
                          └──────────┬──────────────┬────────────┘
                                     │              │
                               Ok(msg)           Err(e)
                                     │              │
                          ┌──────────▼──┐  ┌────────▼────────────┐
                          │ outcome =   │  │ outcome = Failure    │
                          │ Success     │  │ propagate last_output│
                          │ store msg   │  │ (error swallowed)    │
                          └──────┬──────┘  └────────┬────────────┘
                                 └────────┬──────────┘
                                          │
                          ┌───────────────▼──────────────────────┐
                          │  For each outgoing edge:              │
                          │                                       │
                          │  should_traverse(edge, outcome, msg)? │
                          │                                       │
                          │  ┌─── Always ──────────────▶ true    │
                          │  ├─── OnSuccess ──▶ Success? → true  │
                          │  ├─── OnError ────▶ Failure? → true  │
                          │  └─── Expression ─▶ eval template    │
                          └──────────┬────────────────┬──────────┘
                                     │                │
                                  traverse         skip
                                     │                │
                          ┌──────────▼──┐  ┌─────────▼──────────┐
                          │ mark target │  │ resolved_in[target] │
                          │ active      │  │ += 1                │
                          │ propagate   │  │ (fan-in accounting) │
                          │ output      │  └─────────────────────┘
                          │ resolved_in │
                          │ [target]+=1 │
                          └──────┬──────┘
                                 │
                    resolved >= in_degree[target]
                    AND has_active_predecessor?
                                 │
                         Yes ────▶ queue.push_back(target)
                                 │
                          ┌──────▼──────────────────────────┐
                          │  queue empty?                    │
                          │  Yes → return last_output        │
                          │  No  → loop back to pop_front()  │
                          └─────────────────────────────────┘
```

---

## Loop Node with Sub-Graph Body

### Dispatch in execute_step

```rust
let body = if !nodes.is_empty() {
    LoopBody::Graph { nodes, edges }   // sub-graph: execute_graph_nodes per iteration
} else {
    LoopBody::Steps(steps)             // legacy: run_steps per iteration
};
loop_executor.execute(name, loop_type, body, current, self).await
```

### Per-iteration execution

```rust
// loop_executor.rs — run_body helper
async fn run_body(body: &LoopBody<'_>, message: Message, executor: &dyn StepExecutor) -> Result<Message> {
    match body {
        LoopBody::Steps(steps) => executor.run_steps(steps, message).await,
        LoopBody::Graph { nodes, edges } => executor.run_graph(nodes, edges, message).await,
    }
}
```

### Sub-graph loop example

A `foreach` loop where each iteration runs a conditional DAG:

```json
{
  "type": "loop",
  "name": "process_orders",
  "loop_mode": "foreach",
  "iterate_over": "{{orders}}",
  "nodes": [
    { "id": "validate", "step": { "type": "call", "name": "validate", "connector": "validator", "operation": "check", "params": {} } },
    { "id": "enrich",   "step": { "type": "call", "name": "enrich",   "connector": "enricher",  "operation": "add",   "params": {} } },
    { "id": "log_ok",   "step": { "type": "log",  "name": "log_ok",   "message": "Order {{item.id}} enriched" } },
    { "id": "log_err",  "step": { "type": "log",  "name": "log_err",  "message": "Order {{item.id}} failed validation" } }
  ],
  "edges": [
    { "id": "e1", "from": "validate", "to": "enrich",  "condition": "OnSuccess" },
    { "id": "e2", "from": "validate", "to": "log_err", "condition": "OnError"   },
    { "id": "e3", "from": "enrich",   "to": "log_ok",  "condition": "Always"    }
  ]
}
```

```
  Each iteration executes this sub-graph:

        ┌───────────┐
        │ validate  │
        └─────┬─────┘
              │
    ┌─────────┴───────────┐
    │ OnSuccess            │ OnError
    ▼                      ▼
┌────────┐           ┌──────────┐
│ enrich │           │ log_err  │
└────┬───┘           └──────────┘
     │ Always
     ▼
┌──────────┐
│  log_ok  │
└──────────┘
```

---

## Execution Algorithm

### Step 1 — Initialization

| Structure | Type | Purpose |
|-----------|------|---------|
| `node_map` | `HashMap<&str, &FlowNode>` | O(1) lookup by node id |
| `outgoing` | `HashMap<&str, Vec<&FlowEdge>>` | adjacency list |
| `in_degree` | `HashMap<&str, usize>` | incoming edge count per node |
| `resolved_in` | `HashMap<&str, usize>` | predecessors already processed |
| `has_active_predecessor` | `HashSet<&str>` | at least one edge was traversed to this node |
| `outputs` | `HashMap<&str, Message>` | last message produced at each node |
| `outcomes` | `HashMap<&str, StepOutcome>` | result of each node for edge evaluation |
| `queue` | `VecDeque<&str>` | FIFO queue of nodes ready to execute |

Root nodes (in-degree == 0) are inserted into the queue first.

### Step 2 — Main Loop

For each node popped from the queue:

1. **Resolve input** — use `outputs[node_id]` if a predecessor wrote to it, else `last_output`.
2. **Execute** — call the `execute_node` callback, which routes to `FlowExecutor::execute_step`.
3. **Record** — store output message and outcome; update `last_output`.
4. **Walk edges** — evaluate each outgoing edge's condition via `should_traverse`.

### Step 3 — Fan-in Gate

Every outgoing edge (traversed or not) increments `resolved_in[target]`. A target is enqueued **only when**:

```
resolved_in[target] >= in_degree[target]
AND has_active_predecessor contains target
```

This ensures a node with multiple incoming edges waits for **all** predecessors before executing, while a node whose only incoming edges were all skipped never runs.

### Step 4 — Termination

When the queue is empty, return `last_output` (the message produced by the most recently executed node).

---

## Edge Condition Evaluation

`should_traverse(edge, outcome, output)` in `graph_executor.rs`

```
EdgeCondition::Always      → true
EdgeCondition::OnSuccess   → outcome == Success
EdgeCondition::OnError     → outcome == Failure
EdgeCondition::Expression  → evaluate_condition(expr, &output.payload)
```

Expression evaluation (`templates.rs`) supports:
- `{{path.to.value}}` — JSON path extraction
- `{{field || 'default'}}` — default value
- `"true"` / `"false"` string results → boolean
- `"<a> == <b>"` / `"<a> != <b>"` — string comparison

---

## Error Handling Strategy

Node failures are **non-fatal** to the graph:

```
execute_node fails
       │
       ▼
outcome = Failure
node_output = last_output (context preserved)
       │
       ▼
OnError edges activated → recovery nodes can handle the failure
Always edges still fire
OnSuccess edges suppressed
```

This works at every nesting level — a failure inside a sub-graph loop body triggers `OnError` edges within that sub-graph for the current iteration.

---

## Concurrency Model

- **Sequential node execution** — one node at a time per graph invocation (no intra-graph parallelism).
- **Async I/O** — the `execute_node` callback is `async`, allowing connectors and transformers to do non-blocking I/O.
- **Recursive** — a `Loop` node with a graph body calls `execute_graph_nodes` re-entrantly; each iteration is its own independent DAG execution.

---

## Public API

```rust
// crates/integration-runtime/src/graph_executor.rs

/// Entry point for full FlowDefinition callers (execute_flow)
pub async fn execute_graph<'a, F, Fut>(
    flow: &'a FlowDefinition,
    input: Message,
    execute_node: F,
) -> Result<Message>

/// Entry point for sub-graph callers (loop body, nested graphs)
pub async fn execute_graph_nodes<'a, F, Fut>(
    nodes: &'a [FlowNode],
    edges: &'a [FlowEdge],
    input: Message,
    execute_node: F,
) -> Result<Message>
```

`execute_graph` is a thin wrapper — it delegates to `execute_graph_nodes(&flow.nodes, &flow.edges, …)`.

### StepExecutor trait

```rust
#[async_trait]
pub trait StepExecutor: Send + Sync {
    async fn run_steps(&self, steps: &[FlowStep], message: Message) -> Result<Message>;
    async fn run_graph(&self, nodes: &[FlowNode], edges: &[FlowEdge], message: Message) -> Result<Message>;
}
```

---

## Example: Conditional Branching Flow

```
        ┌────────────┐
        │  Fetch API │  (Call node)
        └─────┬──────┘
              │
    ┌─────────┴──────────┐
    │ OnSuccess           │ OnError
    ▼                     ▼
┌──────────┐       ┌────────────┐
│ Transform│       │ Log Error  │
└──────────┘       └────────────┘
    │
    ▼ Always
┌──────────┐
│  Log OK  │
└──────────┘
```

FlowEdge definitions:
```json
[
  { "from": "fetch", "to": "transform", "condition": "OnSuccess" },
  { "from": "fetch", "to": "log_error",  "condition": "OnError"   },
  { "from": "transform", "to": "log_ok", "condition": "Always"    }
]
```

---

## Key Constraints & Limitations

| Constraint | Detail |
|-----------|--------|
| No cycle detection | Graph is assumed to be a valid DAG; cycles will deadlock the queue |
| Fan-in message merge | Last-writer-wins — no multi-input merge strategy |
| No intra-graph parallelism | Nodes with no dependency execute sequentially |
| Expression edge with no expression | Logs a warning and treats edge as not traversed |
| Node id not in node_map | Logs a warning and continues |
| Sub-graph loop body | `steps` field is ignored when `nodes` is non-empty |

---

## Related Files

| File | Purpose |
|------|---------|
| [graph_executor.rs](src/graph_executor.rs) | DAG execution engine (`execute_graph`, `execute_graph_nodes`) |
| [lib.rs](src/lib.rs) | `FlowExecutor` — dispatch, step execution, `StepExecutor` impl |
| [loop_executor.rs](src/loop_executor.rs) | Loop modes, `LoopBody` enum, `StepExecutor` trait |
| [templates.rs](src/templates.rs) | Template resolution and condition evaluation |
| [transformers/json.rs](src/transformers/json.rs) | JSON transformation engine |
| `crates/common/src/lib.rs` | All shared data structures |
