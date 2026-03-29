# Flow Test Endpoint - POST /flows/test

Test flow definitions without saving them to the database.

---

## Endpoint

**POST /flows/test**

### Request

```json
{
  "flow": {
    "name": "Test Flow",
    "trigger": {
      "type": "http",
      "path": "/api/users",
      "method": "POST"
    },
    "steps": [
      {
        "type": "log",
        "name": "log_start",
        "message": "Flow started"
      },
      {
        "type": "transform",
        "name": "select_fields",
        "spec": {
          "type": "select",
          "fields": ["name", "email"]
        }
      },
      {
        "type": "call",
        "name": "insert_user",
        "connector": "postgres",
        "operation": "execute",
        "params": {
          "sql": "INSERT INTO users (name, email) VALUES ($1, $2)"
        }
      }
    ]
  },
  "test_input": {
    "name": "John Doe",
    "email": "john@example.com"
  }
}
```

### Response

```json
{
  "success": true,
  "result": {
    "connector": "postgres",
    "operation": "execute",
    "test_mode": true,
    "result": "mock_connector_response"
  },
  "error": null,
  "execution": {
    "duration_ms": 45,
    "steps_executed": 3,
    "step_results": [
      {
        "name": "log_start",
        "step_type": "log",
        "success": true,
        "output": { "logged": "Flow started" },
        "error": null,
        "duration_ms": 2
      },
      {
        "name": "select_fields",
        "step_type": "transform",
        "success": true,
        "output": {
          "transform_type": "select",
          "result": "transformed_data"
        },
        "error": null,
        "duration_ms": 3
      },
      {
        "name": "insert_user",
        "step_type": "call",
        "success": true,
        "output": {
          "connector": "postgres",
          "operation": "execute",
          "test_mode": true,
          "result": "mock_connector_response"
        },
        "error": null,
        "duration_ms": 12
      }
    ],
    "output": {
      "connector": "postgres",
      "operation": "execute",
      "test_mode": true,
      "result": "mock_connector_response"
    }
  }
}
```

---

## Benefits

✅ **Test before saving** - Validate flow logic  
✅ **Catch errors early** - Before deployment  
✅ **Mock execution** - Doesn't call real APIs  
✅ **Detailed feedback** - Step-by-step results  
✅ **Fast iteration** - Test changes immediately  

---

## Frontend Integration

### Service Method

Add to `frontend/src/services/flow.ts`:

```typescript
export const flowService = {
  // ... existing methods

  async test(flow: Flow, testInput?: any) {
    const response = await api.post('/flows/test', {
      flow,
      test_input: testInput
    })
    return response.data
  },
}
```

### Component - Test Button

In `FlowDesigner.tsx`:

```typescript
const [testResult, setTestResult] = useState<any>(null)
const [testing, setTesting] = useState(false)

const handleTest = async () => {
  setTesting(true)
  setTestResult(null)
  
  try {
    // Build flow from current nodes/edges
    const flow = buildFlowFromNodes(nodes, edges)
    
    // Test the flow
    const result = await flowService.test(flow, {
      name: 'Test User',
      email: 'test@example.com'
    })
    
    setTestResult(result)
    
    if (result.success) {
      alert(`✅ Test passed! ${result.execution.steps_executed} steps executed`)
    } else {
      alert(`❌ Test failed: ${result.error}`)
    }
  } catch (error: any) {
    alert(`Error testing flow: ${error.message}`)
  } finally {
    setTesting(false)
  }
}

// In render:
<button
  onClick={handleTest}
  disabled={testing || nodes.length === 0}
  className="btn btn-secondary flex items-center gap-2"
>
  <Play className="w-4 h-4" />
  {testing ? 'Testing...' : 'Test Flow'}
</button>

{testResult && (
  <TestResultsPanel result={testResult} onClose={() => setTestResult(null)} />
)}
```

### Test Results Panel Component

```typescript
import { CheckCircle, XCircle, X } from 'lucide-react'

function TestResultsPanel({ result, onClose }: {
  result: any
  onClose: () => void
}) {
  return (
    <div className="fixed right-0 top-0 bottom-0 w-96 bg-white shadow-lg border-l overflow-y-auto z-50">
      <div className="p-4 border-b flex items-center justify-between bg-gray-50">
        <h3 className="font-bold">Test Results</h3>
        <button onClick={onClose} className="hover:bg-gray-200 p-1 rounded">
          <X className="w-4 h-4" />
        </button>
      </div>
      
      <div className="p-4 space-y-4">
        {/* Overall Status */}
        <div className={`p-4 rounded border ${
          result.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'
        }`}>
          <div className="flex items-center gap-2">
            {result.success ? (
              <CheckCircle className="w-5 h-5 text-green-600" />
            ) : (
              <XCircle className="w-5 h-5 text-red-600" />
            )}
            <span className="font-medium">
              {result.success ? 'Test Passed' : 'Test Failed'}
            </span>
          </div>
          {result.error && (
            <p className="text-sm text-red-700 mt-2">{result.error}</p>
          )}
        </div>
        
        {/* Execution Stats */}
        <div className="bg-gray-50 p-3 rounded">
          <div className="text-sm space-y-1">
            <div>Duration: {result.execution.duration_ms}ms</div>
            <div>Steps: {result.execution.steps_executed}</div>
          </div>
        </div>
        
        {/* Step Results */}
        <div>
          <h4 className="font-medium mb-2">Step Results:</h4>
          <div className="space-y-2">
            {result.execution.step_results.map((step: any, index: number) => (
              <div key={index} className={`p-3 rounded border ${
                step.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'
              }`}>
                <div className="flex items-center justify-between">
                  <span className="font-medium text-sm">{step.name}</span>
                  <span className="text-xs text-gray-500">{step.duration_ms}ms</span>
                </div>
                <div className="text-xs text-gray-600 mt-1">{step.step_type}</div>
                {step.error && (
                  <div className="text-xs text-red-600 mt-1">{step.error}</div>
                )}
                {step.output && (
                  <details className="mt-2">
                    <summary className="text-xs cursor-pointer text-gray-600">
                      Output
                    </summary>
                    <pre className="text-xs bg-white p-2 rounded mt-1 overflow-x-auto">
                      {JSON.stringify(step.output, null, 2)}
                    </pre>
                  </details>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
```

---

## Testing

```bash
# Build and deploy
docker-compose build control-plane
docker-compose up -d control-plane

# Test endpoint
curl -X POST http://localhost:8081/flows/test \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "flow": {
      "name": "Test Flow",
      "trigger": {"type": "http", "path": "/test"},
      "steps": [
        {"type": "log", "name": "test_log", "message": "Testing"}
      ]
    }
  }'
```

---

## Summary

✅ **POST /flows/test** — Test flows without saving  
✅ **Mock execution** — Safe testing environment  
✅ **Step-by-step results** — Detailed feedback  
✅ **Frontend integration** — Test button in designer  
✅ **Error detection** — Catch issues early  

**Test your flows before deploying them!** 🧪✅
