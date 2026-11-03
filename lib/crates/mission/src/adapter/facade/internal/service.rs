use async_trait::async_trait;

use apis::mission::{
    Actor, AppendCommentRequest, AssigneeData, AssigneeView as ApiAssigneeView, CloseIssueRequest,
    CreateIssueRequest, CreateMissionRequest, IssueCommentView as ApiIssueCommentView,
    IssueState as ApiIssueState, IssueView as ApiIssueView, ListIssuesByMissionRequest,
    ListMissionsByProjectRequest, ListMissionsByUserRequest, MissionApiError,
    MissionKind as ApiKind, MissionRole as ApiRole, MissionService, MissionView as ApiMissionView,
    ReopenIssueRequest, UpdateIssueDescriptionRequest,
};

use crate::domain::{
    AssigneeRepository, DomainError, MissionIssueRepository, MissionRepository, ProjectLookup,
    UserLookup,
};
use crate::usecase::{
    AssigneeData as UcAssigneeData, CreateIssue as UcCreateIssue, CreateMission as UcCreateMission,
    MissionIssueUsecase, MissionUsecase, UsecaseError,
};

use crate::usecase::AssigneeView as UcAssigneeView;
use crate::usecase::IssueCommentView as UcIssueCommentView;
use crate::usecase::IssueView as UcIssueView;
use crate::usecase::MissionView as UcMissionView;
use crate::usecase::{MissionIssueUsecaseConfig, MissionUsecaseConfig};

pub struct MissionServiceImpl<M, A, P, U, I> {
    usecase: MissionUsecase<M, A, P, U>,
    issue_usecase: MissionIssueUsecase<M, A, P, I>,
}

impl<M, A, P, U, I> MissionServiceImpl<M, A, P, U, I>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
    I: MissionIssueRepository,
{
    pub fn from_usecase(
        usecase: MissionUsecase<M, A, P, U>,
        issue_usecase: MissionIssueUsecase<M, A, P, I>,
    ) -> Self {
        Self {
            usecase,
            issue_usecase,
        }
    }

    /// Convenience constructor: builds both usecases from the
    /// five backing pieces and returns a single facade. Mirrors
    /// the legacy `from_repos(mission_repo, assignee_repo,
    /// projects, users)` so the server wiring stays a one-liner.
    pub fn from_repos(
        mission_repo: std::sync::Arc<M>,
        assignee_repo: std::sync::Arc<A>,
        projects: std::sync::Arc<P>,
        users: std::sync::Arc<U>,
        issue_repo: std::sync::Arc<I>,
    ) -> Self
    where
        M: Clone,
        A: Clone,
        P: Clone,
        U: Clone,
        I: Clone,
    {
        let mission_usecase = MissionUsecase::new(MissionUsecaseConfig {
            mission_repo: (*mission_repo).clone(),
            assignee_repo: (*assignee_repo).clone(),
            project_lookup: (*projects).clone(),
            user_lookup: (*users).clone(),
        });
        let issue_usecase = MissionIssueUsecase::new(MissionIssueUsecaseConfig {
            mission_repo: (*mission_repo).clone(),
            assignee_repo: (*assignee_repo).clone(),
            project_lookup: (*projects).clone(),
            issue_repo: (*issue_repo).clone(),
        });
        Self::from_usecase(mission_usecase, issue_usecase)
    }
}

#[async_trait]
impl<M, A, P, U, I> MissionService for MissionServiceImpl<M, A, P, U, I>
where
    M: MissionRepository + 'static,
    A: AssigneeRepository + 'static,
    P: ProjectLookup + 'static,
    U: UserLookup + 'static,
    I: MissionIssueRepository + 'static,
{
    async fn create_mission(
        &self,
        actor: &Actor,
        req: CreateMissionRequest,
    ) -> Result<ApiMissionView, MissionApiError> {
        self.usecase
            .create_mission(
                actor,
                UcCreateMission {
                    project_code: req.project_code,
                    mission_kind: req.mission_kind.into(),
                    mission_code: req.mission_code,
                    assignees: req
                        .assignees
                        .into_iter()
                        .map(|a| UcAssigneeData {
                            user_code: a.user_code,
                            role: a.role.into(),
                        })
                        .collect(),
                },
            )
            .await
            .map(into_api_mission)
            .map_err(map_error)
    }

    async fn get_mission_by_id(&self, id: i64) -> Result<ApiMissionView, MissionApiError> {
        self.usecase
            .get_mission_by_id(id)
            .await
            .map(into_api_mission)
            .map_err(map_error)
    }

    async fn list_missions_by_project(
        &self,
        req: ListMissionsByProjectRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_project(&req.project_code, req.kind.map(Into::into))
            .await
            .map(|v| v.into_iter().map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn list_missions_by_user(
        &self,
        req: ListMissionsByUserRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_user(&req.user_code)
            .await
            .map(|v| v.into_iter().map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn delete_mission(&self, actor: &Actor, id: i64) -> Result<(), MissionApiError> {
        self.usecase
            .delete_mission(actor, id)
            .await
            .map_err(map_error)
    }

    async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<ApiAssigneeView, MissionApiError> {
        self.usecase
            .add_assignee(
                actor,
                mission_id,
                UcAssigneeData {
                    user_code: data.user_code,
                    role: data.role.into(),
                },
            )
            .await
            .map(into_api_assignee)
            .map_err(map_error)
    }

    async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), MissionApiError> {
        self.usecase
            .remove_assignee(actor, mission_id, assignee_id)
            .await
            .map_err(map_error)
    }

    async fn list_issues_by_mission(
        &self,
        req: ListIssuesByMissionRequest,
    ) -> Result<Vec<ApiIssueView>, MissionApiError> {
        self.issue_usecase
            .list_issues_by_mission(req.mission_id, req.state.map(Into::into))
            .await
            .map(|v| v.into_iter().map(into_api_issue).collect())
            .map_err(map_error)
    }

    async fn create_issue(
        &self,
        actor: &Actor,
        req: CreateIssueRequest,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .create_issue(
                actor,
                UcCreateIssue {
                    mission_id: req.mission_id,
                    target_item: req.target_item,
                    description: req.description,
                },
            )
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn close_issue(
        &self,
        actor: &Actor,
        _req: CloseIssueRequest,
        issue_id: i64,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .close_issue(actor, issue_id)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn reopen_issue(
        &self,
        actor: &Actor,
        _req: ReopenIssueRequest,
        issue_id: i64,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .reopen_issue(actor, issue_id)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn update_issue_description(
        &self,
        actor: &Actor,
        issue_id: i64,
        req: UpdateIssueDescriptionRequest,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .update_issue_description(actor, issue_id, req.description)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn append_comment(
        &self,
        actor: &Actor,
        issue_id: i64,
        req: AppendCommentRequest,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .append_comment(actor, issue_id, req.content)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }
}

// ---- apis <-> domain enum bridges ----

impl From<ApiKind> for crate::domain::MissionKind {
    fn from(k: ApiKind) -> Self {
        match k {
            ApiKind::Crf => crate::domain::MissionKind::Crf,
            ApiKind::Sdtm => crate::domain::MissionKind::Sdtm,
            ApiKind::Adam => crate::domain::MissionKind::Adam,
            ApiKind::Tfl => crate::domain::MissionKind::Tfl,
        }
    }
}

impl From<crate::domain::MissionKind> for ApiKind {
    fn from(k: crate::domain::MissionKind) -> Self {
        match k {
            crate::domain::MissionKind::Crf => ApiKind::Crf,
            crate::domain::MissionKind::Sdtm => ApiKind::Sdtm,
            crate::domain::MissionKind::Adam => ApiKind::Adam,
            crate::domain::MissionKind::Tfl => ApiKind::Tfl,
        }
    }
}

impl From<ApiRole> for crate::domain::MissionRole {
    fn from(r: ApiRole) -> Self {
        match r {
            ApiRole::Dev => crate::domain::MissionRole::Dev,
            ApiRole::Qc => crate::domain::MissionRole::Qc,
        }
    }
}

impl From<crate::domain::MissionRole> for ApiRole {
    fn from(r: crate::domain::MissionRole) -> Self {
        match r {
            crate::domain::MissionRole::Dev => ApiRole::Dev,
            crate::domain::MissionRole::Qc => ApiRole::Qc,
        }
    }
}

impl From<ApiIssueState> for crate::domain::IssueState {
    fn from(s: ApiIssueState) -> Self {
        match s {
            ApiIssueState::Opened => crate::domain::IssueState::Opened,
            ApiIssueState::Closed => crate::domain::IssueState::Closed,
        }
    }
}

impl From<crate::domain::IssueState> for ApiIssueState {
    fn from(s: crate::domain::IssueState) -> Self {
        match s {
            crate::domain::IssueState::Opened => ApiIssueState::Opened,
            crate::domain::IssueState::Closed => ApiIssueState::Closed,
        }
    }
}

// ---- view bridges ----

fn into_api_mission(m: UcMissionView) -> ApiMissionView {
    ApiMissionView {
        id: m.id,
        project_code: m.project_code,
        mission_kind: m.mission_kind.into(),
        mission_code: m.mission_code,
        assignees: m.assignees.into_iter().map(into_api_assignee).collect(),
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn into_api_assignee(a: UcAssigneeView) -> ApiAssigneeView {
    ApiAssigneeView {
        id: a.id,
        user_code: a.user_code,
        role: a.role.into(),
        created_at: a.created_at,
        updated_at: a.updated_at,
    }
}

fn into_api_issue(i: UcIssueView) -> ApiIssueView {
    ApiIssueView {
        id: i.id,
        mission_id: i.mission_id,
        target_item: i.target_item,
        issuer: i.issuer,
        description: i.description,
        state: i.state.into(),
        comments: i.comments.into_iter().map(into_api_issue_comment).collect(),
        created_at: i.created_at,
        updated_at: i.updated_at,
    }
}

fn into_api_issue_comment(c: UcIssueCommentView) -> ApiIssueCommentView {
    ApiIssueCommentView {
        user: c.user,
        content: c.content,
        created_at: c.created_at,
    }
}

fn map_error(e: UsecaseError) -> MissionApiError {
    match e {
        UsecaseError::Forbidden {
            user_code,
            project_code,
        } => MissionApiError::Forbidden {
            user_code,
            project_code,
        },
        UsecaseError::Domain(d) => match d {
            DomainError::EmptyMissionCode
            | DomainError::EmptyUserCode
            | DomainError::UnknownMissionKind(_)
            | DomainError::UnknownMissionRole(_) => MissionApiError::Validation(d.to_string()),
            DomainError::NotFound => MissionApiError::NotFound,
            DomainError::AssigneeNotFound => MissionApiError::AssigneeNotFound,
            DomainError::MissionIssueNotFound => MissionApiError::IssueNotFound,
            DomainError::ProjectNotFound(c) => MissionApiError::ProjectNotFound(c),
            DomainError::UserNotFound(c) => MissionApiError::UserNotFound(c),
            DomainError::DuplicateMission {
                project_code,
                mission_kind,
                mission_code,
            } => MissionApiError::DuplicateMission {
                project_code,
                mission_kind: mission_kind.into(),
                mission_code,
            },
            DomainError::DuplicateAssignee {
                mission_id,
                user_code,
                role,
            } => MissionApiError::DuplicateAssignee {
                mission_id,
                user_code,
                role: role.into(),
            },
            DomainError::EmptyIssueDescription
            | DomainError::EmptyIssueIssuer
            | DomainError::EmptyCommentContent => MissionApiError::Validation(d.to_string()),
            DomainError::Repository(s) => MissionApiError::Repository(s),
        },
    }
}
