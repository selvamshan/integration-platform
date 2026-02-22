#!/bin/bash
# Implementation script for Flow Designer enhancements

echo "════════════════════════════════════════════════════════════════"
echo "  Flow Designer Enhancements Implementation Guide"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Step 1: Add Backend API Endpoint"
echo "──────────────────────────────────────────────────────────────────"
echo ""
echo "Add to crates/control-plane/src/main.rs after list_connector_instances:"
echo ""
cat << 'EOFHANDLER'
/// GET /connector-instances/type/:connector_type
/// Returns connector instances filtered by type
async fn list_connector_instances_by_type(
    State(state): State<Arc<AppState>>,
    Path(connector_type): Path<String>,
) -> Json<Value> {
    let instances = state.connector_instances.read().await;
    
    let filtered: Vec<Value> = instances
        .iter()
        .filter(|c| c.connector_type == connector_type)
        .map(|c| json!({
            "id": c.id,
            "name": c.name,
            "connector_type": c.connector_type,
            "host": c.host,
            "port": c.port,
            "database": c.database,
            "username": c.username,
            "active": c.active,
        }))
        .collect();

    Json(json!({ 
        "instances": filtered,
        "count": filtered.len()
    }))
}
EOFHANDLER
echo ""
echo "Add route:"
echo '.route("/connector-instances/type/:connector_type", get(list_connector_instances_by_type))'
echo ""
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Step 2: Frontend Components"
echo "══════════════════════════════════════════════════════════════════"
echo ""
echo "See the following files in frontend/src/components/Flows/:"
echo "  • CustomNode.tsx          - Node with delete button"
echo "  • NodePropertiesPanel.tsx - Right panel for editing"
echo "  • FlowDesigner.tsx        - Updated designer"
echo ""
echo "Complete implementation provided in archive!"
echo ""
