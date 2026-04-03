use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub dbtype: String,
    pub dbname: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct HikariConfig {
    pub maximum_pool_size: u32,
    pub minimum_idle: u32,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub db: DbConfig,
    pub hikari: HikariConfig,
}

fn get_env(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn load_config() -> Result<AppConfig> {
    dotenv::dotenv().ok();

    let server = ServerConfig {
        port: get_env("PORT", "8080")
            .parse()
            .unwrap_or(8080),
        host: get_env("HOST", "0.0.0.0"),
    };

    let db = DbConfig {
        dbtype: get_env("DB_TYPE", "postgresql"),
        dbname: get_env("DB_NAME", "agent_builder"),
        host: get_env("DB_HOST", "localhost"),
        port: get_env("DB_PORT", "5433")
            .parse()
            .unwrap_or(5433),
        user: get_env("DB_USER", "postgres"),
        password: get_env("DB_PASSWORD", "postgres"),
    };

    let hikari = HikariConfig {
        maximum_pool_size: 20,
        minimum_idle: 5,
        idle_timeout: 600000,
        max_lifetime: 1800000,
    };

    tracing::info!("Database config loaded: host={}, port={}, dbname={}, user={}",
        db.host, db.port, db.dbname, db.user);

    Ok(AppConfig { server, db, hikari })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_with_default() {
        let value = get_env("NONEXISTENT_VAR", "default_value");
        assert_eq!(value, "default_value");
    }

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig {
            port: 8080,
            host: "0.0.0.0".to_string(),
        };
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "0.0.0.0");
    }

    #[test]
    fn test_db_config_defaults() {
        let config = DbConfig {
            dbtype: "postgresql".to_string(),
            dbname: "agent_builder".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: "postgres".to_string(),
        };
        assert_eq!(config.dbname, "agent_builder");
        assert_eq!(config.port, 5432);
    }
}
