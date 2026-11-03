//! The crate's own `MissionService` facade. `MissionServiceImpl`
//! implements `apis::mission::MissionService` on top of
//! `MissionUsecase` + `MissionIssueUsecase`.

pub mod issue_service;
pub mod service;
