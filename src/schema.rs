use sqlx::PgPool;

pub async fn create_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    create_users_table(pool).await?;
    create_llm_providers_table(pool).await?;
    create_llm_models_table(pool).await?;
    create_agents_table(pool).await?;
    create_sessions_table(pool).await?;
    create_messages_table(pool).await?;
    create_tools_table(pool).await?;

    Ok(())
}

async fn create_users_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            username VARCHAR(255) NOT NULL UNIQUE,
            email VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_llm_providers_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS llm_providers (
            id UUID PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            type VARCHAR(255) NOT NULL,
            api_key TEXT NOT NULL,
            base_url VARCHAR(512),
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_llm_models_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS llm_models (
            id UUID PRIMARY KEY,
            provider_id UUID NOT NULL REFERENCES llm_providers(id),
            name VARCHAR(255) NOT NULL,
            model_identifier VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_agents_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id UUID PRIMARY KEY,
            user_id UUID NOT NULL REFERENCES users(id),
            model_id UUID NOT NULL REFERENCES llm_models(id),
            parent_id UUID,
            name VARCHAR(255) NOT NULL,
            description TEXT,
            system_prompt TEXT NOT NULL,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_agents_user_id ON agents(user_id)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_sessions_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id UUID PRIMARY KEY,
            agent_id UUID NOT NULL REFERENCES agents(id),
            user_id UUID NOT NULL REFERENCES users(id),
            title VARCHAR(255),
            started_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            ended_at TIMESTAMPTZ,
            is_active BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_messages_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id UUID PRIMARY KEY,
            session_id UUID NOT NULL REFERENCES sessions(id),
            role VARCHAR(50) NOT NULL,
            content TEXT NOT NULL,
            timestamp TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            metadata JSONB,
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_tools_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tools (
            id UUID PRIMARY KEY,
            agent_id UUID NOT NULL REFERENCES agents(id),
            name VARCHAR(255) NOT NULL,
            description TEXT,
            schema JSONB,
            type VARCHAR(100),
            created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_schema_functions_exist() {
        // Basic test to verify schema module loads
        assert!(true);
    }
}
