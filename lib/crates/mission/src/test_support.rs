//! Shared test fakes for the mission crate. Used by
//! `usecase::tests` and `adapter::facade::internal::tests` so
//! the fake definitions live in one place.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, AssigneeNew, AssigneeRepository, DomainError, IssueComment, Mission, MissionIssue,
    MissionIssueNew, MissionIssueRepository, MissionKind, MissionNew, MissionRepository,
    MissionRole, ProjectLookup, UserLookup,
};

/// Shared `(mission_id, assignee)` store used by both
/// `FakeMissionRepo` (initial-assignees at create time) and
/// `FakeAssigneeRepo` (everything else). Mirrors what the live
/// Postgres `MissionRepo::create` transaction does in one
/// statement.
type AssigneeStore = Arc<Mutex<Vec<(i64, Assignee)>>>;

#[derive(Default)]
pub struct FakeMissionRepo {
    pub next_id: AtomicI32,
    pub missions: Arc<Mutex<Vec<Mission>>>,
    /// Optional side-channel used by the fake to populate the
    /// shared assignee store whenever a mission is created with
    /// initial assignees — mirrors what the live Postgres repo
    /// does inside its create transaction.
    pub assignee_repo: Option<AssigneeStore>,
}

impl Clone for FakeMissionRepo {
    fn clone(&self) -> Self {
        Self {
            next_id: AtomicI32::new(self.next_id.load(Ordering::SeqCst)),
            missions: self.missions.clone(),
            assignee_repo: self.assignee_repo.clone(),
        }
    }
}

#[async_trait]
impl MissionRepository for FakeMissionRepo {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as i64;
        let now: DateTime<Utc> = Utc::now();
        let assignees = input
            .assignees
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                Assignee::for_repository(
                    id * 1000 + idx as i64,
                    a.user_code.clone(),
                    a.role,
                    now,
                    now,
                )
            })
            .collect();
        if let Some(store) = &self.assignee_repo {
            for (idx, a) in input.assignees.iter().enumerate() {
                let assignee = Assignee::for_repository(
                    id * 1000 + idx as i64,
                    a.user_code.clone(),
                    a.role,
                    now,
                    now,
                );
                store.lock().unwrap().push((id, assignee));
            }
        }
        let m = Mission::for_repository(
            id,
            input.project_code,
            input.mission_kind,
            input.mission_code,
            assignees,
            now,
            now,
        );
        self.missions.lock().unwrap().push(m.clone());
        Ok(m)
    }
    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError> {
        self.missions
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.project_code == project_code && kind.is_none_or(|k| k == m.mission_kind))
            .cloned()
            .collect())
    }
    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.assignees.iter().any(|a| a.user_code == user_code))
            .cloned()
            .collect())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.missions.lock().unwrap();
        let before = g.len();
        g.retain(|m| m.id != id);
        if g.len() == before {
            Err(DomainError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct FakeAssigneeRepo {
    pub next_id: AtomicI32,
    /// `(mission_id, assignee)` rows. The Arc is shared with the
    /// companion `FakeMissionRepo` so initial-assignees added at
    /// mission creation land in the same store the issue usecase
    /// reads via `is_assignee`.
    pub assignees: AssigneeStore,
}

impl Clone for FakeAssigneeRepo {
    fn clone(&self) -> Self {
        Self {
            next_id: AtomicI32::new(self.next_id.load(Ordering::SeqCst)),
            assignees: self.assignees.clone(),
        }
    }
}

#[async_trait]
impl AssigneeRepository for FakeAssigneeRepo {
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError> {
        let now = Utc::now();
        let a = Assignee::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.user_code,
            input.role,
            now,
            now,
        )?;
        let mut g = self.assignees.lock().unwrap();
        if g.iter()
            .any(|(mid, x)| *mid == mission_id && x.user_code == a.user_code && x.role == a.role)
        {
            return Err(DomainError::DuplicateAssignee {
                mission_id,
                user_code: a.user_code.clone(),
                role: a.role,
            });
        }
        g.push((mission_id, a.clone()));
        Ok(a)
    }
    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let mut g = self.assignees.lock().unwrap();
        let before = g.len();
        g.retain(|(mid, a)| !(*mid == mission_id && a.id == assignee_id));
        if g.len() == before {
            Err(DomainError::AssigneeNotFound)
        } else {
            Ok(())
        }
    }
    async fn is_assignee(
        &self,
        mission_id: i64,
        user_code: &str,
        role: MissionRole,
    ) -> Result<bool, DomainError> {
        Ok(self
            .assignees
            .lock()
            .unwrap()
            .iter()
            .any(|(mid, a)| *mid == mission_id && a.user_code == user_code && a.role == role))
    }
}

#[derive(Default)]
pub struct FakeIssueRepo {
    pub next_id: AtomicI32,
    pub issues: Arc<Mutex<Vec<MissionIssue>>>,
}

impl Clone for FakeIssueRepo {
    fn clone(&self) -> Self {
        Self {
            next_id: AtomicI32::new(self.next_id.load(Ordering::SeqCst)),
            issues: self.issues.clone(),
        }
    }
}

#[async_trait]
impl MissionIssueRepository for FakeIssueRepo {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError> {
        let now = Utc::now();
        let issue = MissionIssue::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.mission_id,
            input.target_item,
            input.issuer,
            input.description,
            crate::domain::IssueState::Opened,
            vec![],
            now,
            now,
        )?;
        self.issues.lock().unwrap().push(issue.clone());
        Ok(issue)
    }
    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError> {
        self.issues
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or(DomainError::MissionIssueNotFound)
    }
    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<crate::domain::IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError> {
        Ok(self
            .issues
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.mission_id == mission_id && state.is_none_or(|s| s == i.state))
            .cloned()
            .collect())
    }
    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = crate::domain::IssueState::Closed;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = crate::domain::IssueState::Opened;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError> {
        // Validate before mutating so the contract is the same
        // as the live repo's CHECK constraint.
        if description.trim().is_empty() {
            return Err(DomainError::EmptyIssueDescription);
        }
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.description = description;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.comments.push(comment);
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
}

pub struct FakeProject {
    pub leader_for: Vec<&'static str>,
}

#[async_trait]
impl ProjectLookup for FakeProject {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code == "p1" {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.into()))
        }
    }
    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError> {
        Ok(project_code == "p1" && self.leader_for.contains(&user_code))
    }
}

pub struct FakeUser;

#[async_trait]
impl UserLookup for FakeUser {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code.starts_with('u') {
            Ok(())
        } else {
            Err(DomainError::UserNotFound(code.into()))
        }
    }
}
