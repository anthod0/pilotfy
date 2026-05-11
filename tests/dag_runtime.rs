use llmparty::{
    application::{
        DagPatch, DagService, PatchOperation, SubmitPlanPayload, WorkItemDraft, WorkItemEdgeDraft,
    },
    ids::new_task_id,
    storage::sqlite::{connect_sqlite, run_migrations},
};
use serde_json::json;
use sqlx::SqlitePool;

async fn test_pool() -> SqlitePool {
    let db = connect_sqlite("sqlite://:memory:").await.expect("connect");
    run_migrations(&db).await.expect("migrate");
    db
}

async fn insert_task(pool: &SqlitePool) -> String {
    let task_id = new_task_id().to_string();
    sqlx::query("INSERT INTO tasks (task_id, state, input) VALUES (?, 'running', 'test task')")
        .bind(&task_id)
        .execute(pool)
        .await
        .expect("insert task");
    task_id
}

fn draft(temp_id: &str, profile: &str) -> WorkItemDraft {
    WorkItemDraft {
        temp_id: Some(temp_id.to_string()),
        title: format!("{temp_id} title"),
        description: format!("{temp_id} description"),
        kind: "implementation".to_string(),
        action: "agent_turn".to_string(),
        execution_profile_id: profile.to_string(),
        execution_profile_version: None,
        priority: 0,
        optional: false,
        parallelizable: true,
        acceptance_criteria: vec!["done".to_string()],
        metadata: json!({}),
    }
}

fn edge(from: &str, to: &str) -> WorkItemEdgeDraft {
    WorkItemEdgeDraft {
        from_work_item_id: from.to_string(),
        to_work_item_id: to.to_string(),
        edge_type: "depends_on".to_string(),
    }
}

fn initial_plan(
    work_items: Vec<WorkItemDraft>,
    edges: Vec<WorkItemEdgeDraft>,
) -> SubmitPlanPayload {
    SubmitPlanPayload {
        mode: "initial_dag".to_string(),
        summary: "initial plan".to_string(),
        work_items,
        edges,
        assumptions: vec![],
        risks: vec![],
    }
}

#[tokio::test]
async fn saves_dag_proposal_without_applying_it() {
    let pool = test_pool().await;
    let task_id = insert_task(&pool).await;
    let service = DagService::new(pool.clone());
    let payload = initial_plan(vec![draft("a", "implementer")], vec![]);

    let proposal = service
        .save_proposal(&task_id, &payload, Some("sess_planner"))
        .await
        .expect("save proposal");

    assert_eq!(proposal.task_id, task_id);
    assert_eq!(proposal.mode, "initial_dag");
    assert_eq!(proposal.state, "proposed");

    let work_item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM work_items")
        .fetch_one(&pool)
        .await
        .expect("count work items");
    assert_eq!(work_item_count, 0);
}

#[tokio::test]
async fn applies_initial_dag_and_initializes_ready_projection() {
    let pool = test_pool().await;
    let task_id = insert_task(&pool).await;
    let service = DagService::new(pool.clone());
    let payload = initial_plan(
        vec![draft("design", "planner"), draft("impl", "implementer")],
        vec![edge("design", "impl")],
    );

    service
        .apply_initial_dag(&task_id, &payload)
        .await
        .expect("apply initial dag");

    let states: Vec<(String, String)> = sqlx::query_as(
        r#"SELECT wi.title, p.current_state
           FROM work_items wi
           JOIN work_item_runtime_projection p ON p.work_item_id = wi.work_item_id
           ORDER BY wi.title"#,
    )
    .fetch_all(&pool)
    .await
    .expect("projection states");

    assert_eq!(states.len(), 2);
    assert_eq!(states[0].1, "ready");
    assert_eq!(states[1].1, "blocked");
}

#[tokio::test]
async fn rejects_initial_dag_with_cycle() {
    let pool = test_pool().await;
    let task_id = insert_task(&pool).await;
    let service = DagService::new(pool);
    let payload = initial_plan(
        vec![draft("a", "implementer"), draft("b", "implementer")],
        vec![edge("a", "b"), edge("b", "a")],
    );

    let error = service
        .apply_initial_dag(&task_id, &payload)
        .await
        .expect_err("cycle should fail");

    assert!(error.to_string().contains("cycle"));
}

#[tokio::test]
async fn applies_patch_with_added_work_item_and_edge() {
    let pool = test_pool().await;
    let task_id = insert_task(&pool).await;
    let service = DagService::new(pool.clone());
    let payload = initial_plan(vec![draft("a", "implementer")], vec![]);
    service
        .apply_initial_dag(&task_id, &payload)
        .await
        .expect("apply initial dag");
    let existing_id: String = sqlx::query_scalar("SELECT work_item_id FROM work_items LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("existing work item");

    let patch = DagPatch {
        summary: "add review".to_string(),
        operations: vec![
            PatchOperation::AddWorkItem {
                work_item: draft("review", "reviewer"),
            },
            PatchOperation::AddEdge {
                edge: edge(&existing_id, "review"),
            },
        ],
    };

    service
        .apply_patch(&task_id, &patch)
        .await
        .expect("apply patch");

    let edge_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM work_item_edges")
        .fetch_one(&pool)
        .await
        .expect("edge count");
    assert_eq!(edge_count, 1);

    let review_state: String = sqlx::query_scalar(
        "SELECT p.current_state FROM work_items wi JOIN work_item_runtime_projection p ON p.work_item_id = wi.work_item_id WHERE wi.title = 'review title'",
    )
    .fetch_one(&pool)
    .await
    .expect("review state");
    assert_eq!(review_state, "blocked");
}

#[tokio::test]
async fn rejects_superseding_running_work_item() {
    let pool = test_pool().await;
    let task_id = insert_task(&pool).await;
    let service = DagService::new(pool.clone());
    service
        .apply_initial_dag(
            &task_id,
            &initial_plan(vec![draft("a", "implementer")], vec![]),
        )
        .await
        .expect("apply initial dag");
    let work_item_id: String = sqlx::query_scalar("SELECT work_item_id FROM work_items LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("work item");
    sqlx::query(
        "UPDATE work_item_runtime_projection SET current_state = 'running' WHERE work_item_id = ?",
    )
    .bind(&work_item_id)
    .execute(&pool)
    .await
    .expect("mark running");

    let patch = DagPatch {
        summary: "supersede running".to_string(),
        operations: vec![PatchOperation::SupersedeWorkItem {
            work_item_id,
            reason: "obsolete".to_string(),
        }],
    };

    let error = service
        .apply_patch(&task_id, &patch)
        .await
        .expect_err("running supersede should fail");

    assert!(error.to_string().contains("running"));
}
