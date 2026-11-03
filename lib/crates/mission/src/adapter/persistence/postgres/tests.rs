use std::convert::TryFrom;
use std::fs;

use chrono::{DateTime, Utc};
use sqlx::types::Json;

use crate::domain::{
    Assignee, IssueComment, IssueState, Mission, MissionIssue, MissionKind, MissionRole,
};

use super::row::{AssigneeRow, IssueRow, MissionRow};

#[test]
fn mission_row_to_domain() {
    let row = MissionRow {
        id: 1,
        project_code: "p1".into(),
        mission_kind: "crf".into(),
        mission_code: "c1".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    let m: Mission = Mission::try_from((row, vec![])).unwrap();
    assert_eq!(m.id, 1);
    assert_eq!(m.mission_kind, MissionKind::Crf);
    assert_eq!(m.mission_code, "c1");
    assert!(m.assignees.is_empty());
}

#[test]
fn assignee_row_to_domain() {
    let row = AssigneeRow {
        id: 7,
        mission_id: 1,
        user_code: "u1".into(),
        role: "qc".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    let a: Assignee = Assignee::try_from(row).unwrap();
    assert_eq!(a.id, 7);
    assert_eq!(a.role, MissionRole::Qc);
}

#[test]
fn mission_row_rejects_unknown_kind() {
    let row = MissionRow {
        id: 1,
        project_code: "p1".into(),
        mission_kind: "not-a-kind".into(),
        mission_code: "c1".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    assert!(Mission::try_from((row, vec![])).is_err());
}

#[test]
fn mission_migration_has_natural_key_unique() {
    let sql = read_migration("20261103135404_create_missions.sql");
    assert!(sql.contains("missions_natural_key"));
    assert!(sql.contains("UNIQUE (project_code, mission_kind, mission_code)"));
    assert!(sql.contains("missions_kind_check"));
    assert!(sql.contains("CHECK (mission_kind IN ('crf', 'sdtm', 'adam', 'tfl'))"));
    assert!(sql.contains("missions_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON missions"));
}

#[test]
fn assignee_migration_has_per_mission_unique_and_cascade() {
    let sql = read_migration("20261103135405_create_assignees.sql");
    assert!(sql.contains("assignees_per_mission_unique"));
    assert!(sql.contains("UNIQUE (mission_id, user_code, role)"));
    assert!(sql.contains("assignees_role_check"));
    assert!(sql.contains("CHECK (role IN ('dev', 'qc'))"));
    assert!(sql.contains("assignees_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON assignees"));
    assert!(sql.contains("REFERENCES missions(id) ON DELETE CASCADE"));
}

#[test]
fn issue_row_to_domain() {
    let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let row = IssueRow {
        id: 1,
        mission_id: 2,
        target_item: Some("form AE".into()),
        issuer: "u1".into(),
        description: "desc".into(),
        state: "opened".into(),
        comments: Json(vec![IssueComment {
            user: "u1".into(),
            content: "first".into(),
            created_at: now,
        }]),
        created_at: now,
        updated_at: now,
    };
    let issue: MissionIssue = MissionIssue::try_from(row).unwrap();
    assert_eq!(issue.id, 1);
    assert_eq!(issue.mission_id, 2);
    assert_eq!(issue.target_item.as_deref(), Some("form AE"));
    assert_eq!(issue.issuer, "u1");
    assert_eq!(issue.state, IssueState::Opened);
    assert_eq!(issue.comments.len(), 1);
    assert_eq!(issue.comments[0].user, "u1");
}

#[test]
fn issue_row_rejects_unknown_state() {
    let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let row = IssueRow {
        id: 1,
        mission_id: 2,
        target_item: None,
        issuer: "u1".into(),
        description: "desc".into(),
        state: "pending".into(),
        comments: Json(vec![]),
        created_at: now,
        updated_at: now,
    };
    assert!(MissionIssue::try_from(row).is_err());
}

#[test]
fn mission_issues_migration_has_checks_and_trigger() {
    let sql = read_migration("20261103135406_create_mission_issues.sql");
    assert!(sql.contains("CREATE TABLE mission_issues"));
    assert!(sql.contains("REFERENCES missions(id) ON DELETE CASCADE"));
    assert!(sql.contains("mission_issues_state_check"));
    assert!(sql.contains("CHECK (state IN ('opened', 'closed'))"));
    assert!(sql.contains("mission_issues_description_nonempty"));
    assert!(sql.contains("length(btrim(description)) > 0"));
    assert!(sql.contains("mission_issues_issuer_nonempty"));
    assert!(sql.contains("JSONB NOT NULL DEFAULT '[]'::jsonb"));
    assert!(sql.contains("mission_issues_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON mission_issues"));
    assert!(sql.contains("mission_issues_by_mission_state"));
}

fn read_migration(name: &str) -> String {
    let path = format!("{}/migrations/{}", env!("CARGO_MANIFEST_DIR"), name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path, e))
}
