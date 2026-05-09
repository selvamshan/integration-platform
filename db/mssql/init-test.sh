#!/bin/bash
# Starts SQL Server and creates the testdb database once the server is ready.
# Used as the ENTRYPOINT in the test-mssql docker-compose service.

/opt/mssql/bin/sqlservr &
SERVER_PID=$!

echo "Waiting for SQL Server to start..."
for i in $(seq 1 30); do
    sleep 2
    /opt/mssql-tools18/bin/sqlcmd \
        -S localhost -U sa -P "${SA_PASSWORD}" -No \
        -Q "SELECT 1" > /dev/null 2>&1 && break
    echo "  attempt $i/30..."
done

echo "Creating testdb..."
/opt/mssql-tools18/bin/sqlcmd \
    -S localhost -U sa -P "${SA_PASSWORD}" -No \
    -Q "IF NOT EXISTS (SELECT name FROM sys.databases WHERE name = 'testdb') CREATE DATABASE testdb"

echo "SQL Server ready."
wait $SERVER_PID
