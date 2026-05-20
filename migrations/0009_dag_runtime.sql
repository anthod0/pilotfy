CREATE TABLE graph_tasks (
    task_id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    ref TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id)
);

CREATE TABLE graph_work_items (
    work_item_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    kind TEXT NOT NULL,
    action TEXT NOT NULL,
    execution_profile_id TEXT NOT NULL,
    execution_profile_version TEXT,
    review_policy TEXT,
    execution_policy TEXT,
    escalation_policy TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    optional INTEGER NOT NULL DEFAULT 0 CHECK(optional IN (0, 1)),
    parallelizable INTEGER NOT NULL DEFAULT 1 CHECK(parallelizable IN (0, 1)),
    acceptance_criteria TEXT NOT NULL DEFAULT '[]',
    active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0, 1)),
    ref TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id)
);

CREATE INDEX idx_graph_work_items_task_active ON graph_work_items(task_id, active, priority, work_item_id);
CREATE INDEX idx_graph_work_items_profile ON graph_work_items(execution_profile_id, execution_profile_version);

CREATE TABLE graph_work_item_edges (
    edge_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    from_work_item_id TEXT NOT NULL,
    to_work_item_id TEXT NOT NULL,
    edge_type TEXT NOT NULL CHECK(edge_type IN ('depends_on', 'reviews', 'supersedes', 'caused_by')),
    ref TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(from_work_item_id) REFERENCES graph_work_items(work_item_id),
    FOREIGN KEY(to_work_item_id) REFERENCES graph_work_items(work_item_id),
    UNIQUE(task_id, from_work_item_id, to_work_item_id, edge_type)
);

CREATE INDEX idx_graph_work_item_edges_to ON graph_work_item_edges(task_id, to_work_item_id, edge_type);
CREATE INDEX idx_graph_work_item_edges_from ON graph_work_item_edges(task_id, from_work_item_id, edge_type);

CREATE TABLE graph_signals (
    signal_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    work_item_id TEXT,
    run_id TEXT,
    source_session_id TEXT,
    source TEXT NOT NULL DEFAULT 'system',
    kind TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail TEXT,
    severity TEXT NOT NULL DEFAULT 'medium' CHECK(severity IN ('low', 'medium', 'high')),
    related_refs TEXT NOT NULL DEFAULT '[]',
    state TEXT NOT NULL DEFAULT 'open' CHECK(state IN ('open', 'acknowledged', 'resolved', 'dismissed')),
    ref TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(source_session_id) REFERENCES sessions(session_id)
);

CREATE INDEX idx_graph_signals_task_state ON graph_signals(task_id, state, created_at, signal_id);
CREATE INDEX idx_graph_signals_work_item ON graph_signals(work_item_id, created_at, signal_id);

CREATE TABLE work_item_runs (
    run_id TEXT PRIMARY KEY NOT NULL,
    work_item_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('queued', 'running', 'completed', 'failed', 'blocked', 'needs_input', 'cancelled')),
    session_id TEXT,
    turn_id TEXT,
    client_type TEXT,
    execution_profile_id TEXT NOT NULL,
    execution_profile_version TEXT,
    rendered_prompt_ref TEXT,
    output_summary TEXT,
    failure TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    started_at TEXT,
    completed_at TEXT,
    lease_owner TEXT,
    lease_expires_at TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(session_id) REFERENCES sessions(session_id),
    FOREIGN KEY(turn_id) REFERENCES turns(turn_id)
);

CREATE INDEX idx_work_item_runs_work_item ON work_item_runs(work_item_id, attempt DESC, run_id);
CREATE INDEX idx_work_item_runs_task_state ON work_item_runs(task_id, state, updated_at, run_id);
CREATE INDEX idx_work_item_runs_turn ON work_item_runs(turn_id);

CREATE TABLE work_item_runtime_projection (
    work_item_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    current_run_id TEXT,
    current_state TEXT NOT NULL CHECK(current_state IN ('pending', 'ready', 'blocked', 'running', 'completed', 'failed', 'needs_input', 'cancelled', 'superseded', 'replan_anchor')),
    current_attempt INTEGER NOT NULL DEFAULT 0,
    ready_at TEXT,
    blocked_reason TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 0,
    priority INTEGER NOT NULL DEFAULT 0,
    optional INTEGER NOT NULL DEFAULT 0 CHECK(optional IN (0, 1)),
    parallelizable INTEGER NOT NULL DEFAULT 1 CHECK(parallelizable IN (0, 1)),
    session_id TEXT,
    turn_id TEXT,
    lease_owner TEXT,
    lease_expires_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(current_run_id) REFERENCES work_item_runs(run_id),
    FOREIGN KEY(session_id) REFERENCES sessions(session_id),
    FOREIGN KEY(turn_id) REFERENCES turns(turn_id)
);

CREATE INDEX idx_work_item_runtime_ready ON work_item_runtime_projection(task_id, current_state, priority, ready_at, work_item_id);

CREATE TABLE dag_proposals (
    proposal_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    mode TEXT NOT NULL CHECK(mode IN ('initial_dag', 'patch')),
    state TEXT NOT NULL CHECK(state IN ('proposed', 'validated', 'rejected', 'applied')),
    summary TEXT NOT NULL,
    proposal_json TEXT NOT NULL,
    validation_json TEXT NOT NULL DEFAULT '{}',
    created_by_session_id TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id)
);

CREATE INDEX idx_dag_proposals_task ON dag_proposals(task_id, created_at, proposal_id);

CREATE TABLE dag_signals (
    signal_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL,
    work_item_id TEXT,
    run_id TEXT,
    source_session_id TEXT,
    kind TEXT NOT NULL,
    summary TEXT NOT NULL,
    detail TEXT,
    severity TEXT NOT NULL DEFAULT 'medium' CHECK(severity IN ('low', 'medium', 'high')),
    related_refs TEXT NOT NULL DEFAULT '[]',
    state TEXT NOT NULL DEFAULT 'open' CHECK(state IN ('open', 'acknowledged', 'resolved', 'dismissed')),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY(task_id) REFERENCES tasks(task_id),
    FOREIGN KEY(run_id) REFERENCES work_item_runs(run_id),
    FOREIGN KEY(source_session_id) REFERENCES sessions(session_id)
);

CREATE INDEX idx_dag_signals_task_state ON dag_signals(task_id, state, created_at, signal_id);
CREATE INDEX idx_dag_signals_work_item ON dag_signals(work_item_id, created_at, signal_id);
