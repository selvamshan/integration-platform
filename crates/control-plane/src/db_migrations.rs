use anyhow::Result;
use sqlx::PgPool;

pub async fn run_migrations(db: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations...");

    sqlx::migrate!("./migrations").run(db).await?;

    // Seed sample users if the table is empty
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(db)
        .await?;
    if count.0 == 0 {
        sqlx::query(
            "INSERT INTO users (name, email) VALUES
                ('Alice Johnson',  'alice@example.com'),
                ('Bob Smith',      'bob@example.com'),
                ('Charlie Brown',  'charlie@example.com'),
                ('Diana Prince',   'diana@example.com'),
                ('Eve Wilson',     'eve@example.com')",
        )
        .execute(db)
        .await?;
        tracing::info!("Sample data inserted");
    }

    tracing::info!("Migrations completed");
    Ok(())
}
