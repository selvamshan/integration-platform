export function Dashboard() {
  return (
    <div>
      <h1 className="text-3xl font-bold mb-6">Dashboard</h1>
      <div className="grid gap-4 md:grid-cols-3">
        <div className="card">
          <h2 className="text-xl font-bold">Flows</h2>
          <p className="text-3xl mt-2">0</p>
        </div>
        <div className="card">
          <h2 className="text-xl font-bold">Connectors</h2>
          <p className="text-3xl mt-2">0</p>
        </div>
        <div className="card">
          <h2 className="text-xl font-bold">Executions</h2>
          <p className="text-3xl mt-2">0</p>
        </div>
      </div>
    </div>
  )
}
