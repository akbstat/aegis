//! In-memory `MissionIssueRepository` for tests and any
//! non-Postgres backend the facade is reused on. Mirrors
//! `test_support::FakeIssueRepo` but lives here so the facade's
//! own `tests` block can reach it via `super`.

#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    DomainError, IssueComment, IssueState, MissionIssue, MissionIssueNew, MissionIssueRepository,
};

type IssueStore = Mutex<Vec<MissionIssue>>;

#[derive(Default)]
pub struct InMemoryIssueRepo {
    pub next_id: AtomicI32,
    pub issues: IssueStore,
}

#[async_trait]
impl MissionIssueRepository for InMemoryIssueRepo {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError> {
        let now = Utc::now();
        let issue = MissionIssue::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.mission_id,
            input.target_item,
            input.issuer,
            input.description,
            IssueState::Opened,
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
        state: Option<IssueState>,
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
        i.state = IssueState::Closed;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = IssueState::Opened;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError> {
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
