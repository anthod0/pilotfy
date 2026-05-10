use std::{collections::HashMap, env, net::SocketAddr};

use crate::{
    application::{
        GraphRuntimeConfig, PlannerRuntimeConfig, WorkspaceBrowserConfig, WorkspaceRootConfig,
    },
    error::{Error, Result},
};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
const DEFAULT_DATABASE_URL: &str = "sqlite://~/.local/share/llmparty/llmparty.db";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub bind_addr: SocketAddr,
    pub database_url: String,
    pub external_api_token: Option<String>,
    pub run_migrations: bool,
    pub planner: PlannerRuntimeConfig,
    pub graph: GraphRuntimeConfig,
    pub workspace_browser: WorkspaceBrowserConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let _ = dotenvy::dotenv();
        let vars: HashMap<String, String> = env::vars().collect();
        Self::from_vars(&vars)
    }

    pub fn from_vars(vars: &HashMap<String, String>) -> Result<Self> {
        let bind_addr = get(vars, "LLMPARTY_BIND_ADDR")
            .unwrap_or(DEFAULT_BIND_ADDR)
            .parse::<SocketAddr>()
            .map_err(|err| Error::InvalidConfig {
                key: "LLMPARTY_BIND_ADDR",
                message: err.to_string(),
            })?;

        let database_url = get(vars, "LLMPARTY_DATABASE_URL")
            .unwrap_or(DEFAULT_DATABASE_URL)
            .to_string();

        let external_api_token = get(vars, "LLMPARTY_EXTERNAL_API_TOKEN")
            .filter(|value| !value.trim().is_empty())
            .map(ToString::to_string);

        let run_migrations = match get(vars, "LLMPARTY_RUN_MIGRATIONS") {
            Some(value) => parse_bool("LLMPARTY_RUN_MIGRATIONS", value)?,
            None => true,
        };

        let planner = PlannerRuntimeConfig {
            enabled: match get(vars, "LLMPARTY_PLANNER_ENABLED") {
                Some(value) => parse_bool("LLMPARTY_PLANNER_ENABLED", value)?,
                None => false,
            },
            client_type: get(vars, "LLMPARTY_PLANNER_CLIENT_TYPE")
                .unwrap_or("pi")
                .to_string(),
            timeout_ms: match get(vars, "LLMPARTY_PLANNER_TIMEOUT_MS") {
                Some(value) => value.parse::<u64>().map_err(|err| Error::InvalidConfig {
                    key: "LLMPARTY_PLANNER_TIMEOUT_MS",
                    message: err.to_string(),
                })?,
                None => 30_000,
            },
            compatibility_direct_dispatch: match get(
                vars,
                "LLMPARTY_PLANNER_COMPAT_DIRECT_DISPATCH",
            ) {
                Some(value) => parse_bool("LLMPARTY_PLANNER_COMPAT_DIRECT_DISPATCH", value)?,
                None => false,
            },
        };

        let graph_enabled = match get(vars, "LLMPARTY_GRAPH_ENABLED") {
            Some(value) => parse_bool("LLMPARTY_GRAPH_ENABLED", value)?,
            None => false,
        };
        let graph = GraphRuntimeConfig {
            enabled: graph_enabled,
            db_dir: get(vars, "LLMPARTY_GRAPH_DB_DIR")
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
                .or_else(|| graph_enabled.then(|| default_graph_db_dir(&database_url))),
        };

        let workspace_browser = WorkspaceBrowserConfig {
            roots: parse_workspace_roots(get(vars, "LLMPARTY_WORKSPACE_ROOTS").unwrap_or(""))?,
        };

        Ok(Self {
            bind_addr,
            database_url,
            external_api_token,
            run_migrations,
            planner,
            graph,
            workspace_browser,
        })
    }
}

fn get<'a>(vars: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    vars.get(key).map(String::as_str)
}

fn default_graph_db_dir(database_url: &str) -> String {
    let path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(database_url);
    let parent = std::path::Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    parent.join("graph").join("lbug").display().to_string()
}

fn parse_workspace_roots(value: &str) -> Result<Vec<WorkspaceRootConfig>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    trimmed
        .split(';')
        .filter(|entry| !entry.trim().is_empty())
        .map(|entry| {
            let parts = entry.split('|').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(Error::InvalidConfig {
                    key: "LLMPARTY_WORKSPACE_ROOTS",
                    message:
                        "expected entries formatted as root_id|label|path separated by semicolons"
                            .to_string(),
                });
            }
            let root_id = parts[0].trim();
            let label = parts[1].trim();
            let path = parts[2].trim();
            if root_id.is_empty() || label.is_empty() || path.is_empty() {
                return Err(Error::InvalidConfig {
                    key: "LLMPARTY_WORKSPACE_ROOTS",
                    message: "root_id, label, and path must be non-empty".to_string(),
                });
            }
            Ok(WorkspaceRootConfig {
                root_id: root_id.to_string(),
                label: label.to_string(),
                path: path.to_string(),
            })
        })
        .collect()
}

fn parse_bool(key: &'static str, value: &str) -> Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(Error::InvalidConfig {
            key,
            message: format!("expected boolean, got {value:?}"),
        }),
    }
}
