//! Runtime control boundary.
//!
//! The MVP generic runtime records a binding and immediately reports ready. This
//! module stays independent from HTTP transport details.

mod claude_code;
mod config;
mod paths;
mod script;
mod tmux;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;

use crate::{
    adapters::{AdapterCapabilities, AgentInputSink, GenericTestAdapter},
    agent_clients,
    application::SessionCapabilities,
    error::{Error, Result},
    ids::new_runtime_instance_id,
    time::utc_now,
};

pub use config::set_runtime_config;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeStartRequest {
    pub session_id: String,
    pub client_type: String,
    pub workspace: Option<String>,
    pub handle: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeStartResult {
    pub runtime_kind: String,
    pub runtime_ref: String,
    pub capabilities: SessionCapabilities,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInput {
    pub session_id: String,
    pub turn_id: String,
    pub input: String,
}

#[derive(Debug, Clone, Default)]
pub struct GenericRuntimeManager;

impl From<AdapterCapabilities> for SessionCapabilities {
    fn from(capabilities: AdapterCapabilities) -> Self {
        Self {
            accept_task: capabilities.accept_task,
            report_turn_started: capabilities.report_turn_started,
            report_turn_finished: capabilities.report_turn_finished,
            interrupt: capabilities.interrupt,
            stream_output: capabilities.stream_output,
            heartbeat: capabilities.heartbeat,
            artifact_sources: capabilities.artifact_sources,
        }
    }
}

impl GenericRuntimeManager {
    pub fn start_session(&self, request: RuntimeStartRequest) -> Result<RuntimeStartResult> {
        self.start_session_with_restart_count(request, 0)
    }

    pub fn start_session_with_restart_count(
        &self,
        request: RuntimeStartRequest,
        restart_count: i64,
    ) -> Result<RuntimeStartResult> {
        let client_spec =
            agent_clients::get_client_spec(&request.client_type).ok_or_else(|| {
                Error::Domain(format!("unsupported client_type: {}", request.client_type))
            })?;
        let capabilities = client_spec.capabilities.clone();
        let tmux_session = tmux::tmux_session_name(&request);
        let workspace = paths::workspace_path(&request)?;
        script::run_startup_hooks(client_spec.startup_hooks, &workspace)?;
        let runtime_dir = paths::runtime_dir(&request.session_id)?;
        std::fs::create_dir_all(&runtime_dir)?;
        let log_path = runtime_dir.join("runtime.log");
        let adapter_event_log = runtime_dir.join("adapter-events.jsonl");
        let current_turn_file = runtime_dir.join("current-turn.json");
        let pi_hook_log = runtime_dir.join("pi-hook.log");
        let claude_hook_log = runtime_dir.join("claude-hook.log");
        let internal_event_url = script::internal_event_url();
        let runtime_instance_id = new_runtime_instance_id().to_string();
        std::fs::File::create(&log_path)?;
        let script_path = runtime_dir.join("runtime.sh");
        let runtime_paths = script::RuntimePaths {
            runtime_dir: &runtime_dir,
            log_path: &log_path,
            adapter_event_log: &adapter_event_log,
            current_turn_file: &current_turn_file,
            pi_hook_log: &pi_hook_log,
            claude_hook_log: &claude_hook_log,
        };
        script::write_runtime_script(
            &script_path,
            &workspace,
            &runtime_paths,
            &request,
            &runtime_instance_id,
        )?;

        let status = tmux::spawn_tmux_session(&tmux_session, &workspace, &script_path)
            .map_err(|err| Error::Domain(format!("tmux runtime spawn failed: {err}")))?;
        if !status.success() {
            return Err(Error::Domain(format!(
                "tmux runtime spawn failed with status {status}"
            )));
        }

        let started_at = utc_now()
            .format(&Rfc3339)
            .map_err(|err| Error::Domain(format!("invalid runtime timestamp: {err}")))?;
        let workspace = workspace.display().to_string();
        let runtime_dir = runtime_dir.display().to_string();
        let log_path = log_path.display().to_string();
        let adapter_event_log = adapter_event_log.display().to_string();
        let current_turn_file = current_turn_file.display().to_string();
        let pi_hook_log = pi_hook_log.display().to_string();
        let claude_hook_log = claude_hook_log.display().to_string();
        Ok(RuntimeStartResult {
            runtime_kind: "tmux".to_string(),
            runtime_ref: tmux_session.clone(),
            capabilities: capabilities.into(),
            metadata: json!({
                "backend": "tmux",
                "tmux_session": tmux_session,
                "workspace": workspace,
                "runtime_dir": runtime_dir,
                "runtime_log": log_path,
                "log_path": log_path,
                "adapter_event_log": adapter_event_log,
                "current_turn_file": current_turn_file,
                "internal_event_url": internal_event_url,
                "pi_hook_log": pi_hook_log,
                "claude_hook_log": claude_hook_log,
                "handle": request.handle,
                "role": request.role,
                "started_at": started_at,
                "restart_count": restart_count,
                "runtime_instance_id": runtime_instance_id,
            }),
        })
    }

    pub fn submit_input(&self, input: AgentInput) -> Result<()> {
        GenericTestAdapter.accept_input(input)
    }

    pub fn dispatch_pi_turn(&self, runtime_ref: &str, input: &AgentInput) -> Result<()> {
        self.dispatch_tui_turn(runtime_ref, "pi", input)
    }

    pub fn dispatch_tui_turn(
        &self,
        runtime_ref: &str,
        client_type: &str,
        input: &AgentInput,
    ) -> Result<()> {
        tmux::dispatch_tui_turn(runtime_ref, client_type, input)
    }

    pub fn interrupt_session(&self, runtime_ref: &str) -> Result<()> {
        tmux::interrupt_session(runtime_ref)
    }

    pub fn terminate_session(&self, runtime_ref: &str) -> Result<()> {
        tmux::terminate_session(runtime_ref)
    }

    pub fn restart_session(&self, request: RuntimeStartRequest) -> Result<RuntimeStartResult> {
        self.start_session(request)
    }

    pub fn is_alive(&self, runtime_ref: &str) -> bool {
        tmux::is_alive(runtime_ref)
    }
}

impl RuntimeStartResult {
    pub fn binding_metadata(&self) -> serde_json::Value {
        let mut metadata = self.metadata.clone();
        if let Some(object) = metadata.as_object_mut() {
            object.insert("capabilities".to_string(), json!(self.capabilities));
        }
        metadata
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
