//! Wire-level DTOs for the HTTP transport.
//!
//! Each wire DTO is a thin Rust struct with `Serialize`,
//! `Deserialize`, and `ToSchema`. JSON field names use `camelCase`
//! (`#[serde(rename_all = "camelCase")]`) per the public API
//! conventions. Handler code translates JSON ↔ apis DTOs at the
//! boundary; the apis crate deliberately has no serde / utoipa
//! derives.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// -- requests -------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// -- responses ------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthClaimsResponse {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

// -- Role -----------------------------------------------------------------

/// Wire-level mirror of `apis::user::Role`. The two enums have
/// identical variants; the conversion is a single 3-arm `match` in
/// `auth.rs`. Kept separate so the apis crate stays free of
/// serde / utoipa derives.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Root,
    Admin,
    General,
}

impl From<apis::user::Role> for Role {
    fn from(r: apis::user::Role) -> Self {
        match r {
            apis::user::Role::Root => Role::Root,
            apis::user::Role::Admin => Role::Admin,
            apis::user::Role::General => Role::General,
        }
    }
}

impl From<Role> for apis::user::Role {
    fn from(r: Role) -> Self {
        match r {
            Role::Root => apis::user::Role::Root,
            Role::Admin => apis::user::Role::Admin,
            Role::General => apis::user::Role::General,
        }
    }
}

// -- user requests / responses ---------------------------------------------

/// Wire-level request body for `POST /api/user`. Mirrors
/// `apis::user::CreateUserRequest`; the handler translates at the
/// boundary so the apis crate stays free of serde / utoipa.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

/// Wire-level request body for `PATCH /api/user/{code}`. Every field
/// is optional — only the fields that actually changed need to be
/// supplied. Deliberately omits `id`: the handler resolves the URL
/// `{code}` to a `UserView` via `get_by_code` and threads the
/// resulting `id` into `apis::user::UpdateUserRequest` internally.
///
/// Each `Option` field is `skip_serializing_if = "Option::is_none"`
/// so a partial update round-trips losslessly: deserializing
/// `{"name":"Alice"}` and re-serializing it produces the same JSON.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

/// Wire-level extractor for the `{code}` URL parameter.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathCode {
    pub code: String,
}

/// Wire-level projection of a user — mirrors `apis::user::UserView`
/// field-for-field. Carries `Serialize` / `Deserialize` / `ToSchema`
/// so utoipa can document the response shape and the handler can
/// return it directly via `Json<UserViewResponse>`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Wire-level wrapper for `GET /api/user` responses. Wrapping the
/// vector in a struct leaves room for future pagination metadata
/// (`total`, `next_cursor`, …) without breaking the response shape.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserListResponse {
    pub users: Vec<UserViewResponse>,
}

impl From<apis::user::UserView> for UserViewResponse {
    fn from(view: apis::user::UserView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            name: view.name,
            role: view.role.into(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

// -- user-credential requests / responses -----------------------------------

/// Wire-level request body for `PATCH /api/auth/user-credential`.
///
/// `password` is the only mutable field today. `user_code` is
/// implied by [`AuthClaims`](crate::transport::http::auth::middleware::AuthClaims)
/// — a user can only update their own credential. The
/// `skip_serializing_if` keeps a partial update round-trip lossless
/// (a `{}` body stays `{}` on re-serialization).
///
/// There is no `CreateUserCredentialRequest` — credential creation
/// happens out of band (seed script / admin tool), so this route
/// only handles rotation of an existing password.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserCredentialRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Wire-level projection of a user's credential.
///
/// `password_hash` is always a hashed representation (Argon2 in the
/// canonical backend) — the wire API never exposes the plaintext
/// password. The handler translates from the apis view via the
/// `From` impl below.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserCredentialViewResponse {
    pub user_code: String,
    pub password_hash: String,
    pub token_version: u32,
}

impl From<apis::auth::UserCredentialView> for UserCredentialViewResponse {
    fn from(view: apis::auth::UserCredentialView) -> Self {
        Self {
            user_code: view.user_code,
            password_hash: view.password_hash,
            token_version: view.token_version,
        }
    }
}

/// Wire-level request body for `POST /api/auth/user-credential`.
///
/// Authentication is enforced by the `BearerAuth` middleware; the
/// handler additionally rejects callers whose role is not
/// `Root`/`Admin`. The server hashes `password` before persisting and
/// never returns the value.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserRequest {
    pub user_code: String,
    pub user_name: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
    pub password: String,
}

/// Wire-level response body for `POST /api/auth/user-credential`.
///
/// Never includes a password or password hash. Translates from
/// [`apis::auth::RegisterUserResponse`] via the `From` impl below.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserResponse {
    pub user_code: String,
    pub user_name: String,
    pub role: Role,
    pub active: bool,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

impl From<apis::auth::RegisterUserResponse> for RegisterUserResponse {
    fn from(view: apis::auth::RegisterUserResponse) -> Self {
        Self {
            user_code: view.user_code,
            user_name: view.user_name,
            role: view.role.into(),
            active: view.active,
            domain_name: view.domain_name,
            hostname: view.hostname,
            sid: view.sid,
        }
    }
}

// -- tag DTOs --------------------------------------------------------------

/// Wire-level request body for a single tag. Mirrors
/// `apis::project::TagData` field-for-field. Two strings, both
/// required (and validated non-empty after trim by the domain layer).
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TagDataRequest {
    pub key: String,
    pub value: String,
}

/// Wire-level projection of a single tag. Mirrors
/// `apis::project::TagView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TagViewResponse {
    pub key: String,
    pub value: String,
}

impl From<apis::project::TagView> for TagViewResponse {
    fn from(view: apis::project::TagView) -> Self {
        Self {
            key: view.key,
            value: view.value,
        }
    }
}

/// Wire-level request body for the SDTM-IG version pointer a
/// project targets. Mirrors `apis::project::ModelVersionData`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

impl From<ModelVersionRequest> for apis::project::ModelVersionData {
    fn from(v: ModelVersionRequest) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Wire-level projection of the SDTM-IG version pointer. Mirrors
/// `apis::project::ModelVersionView`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}

impl From<apis::project::ModelVersionView> for ModelVersionResponse {
    fn from(v: apis::project::ModelVersionView) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Wire-level request body for the SDTM controlled-terminology
/// version pointer a project pins. Mirrors
/// `apis::project::TerminologyVersionData`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

impl From<TerminologyVersionRequest> for apis::project::TerminologyVersionData {
    fn from(v: TerminologyVersionRequest) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Wire-level projection of the SDTM controlled-terminology version
/// pointer. Mirrors `apis::project::TerminologyVersionView`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}

impl From<apis::project::TerminologyVersionView> for TerminologyVersionResponse {
    fn from(v: apis::project::TerminologyVersionView) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

// -- project configuration DTOs ---------------------------------------------

/// Wire-level mirror of [`apis::project::ProjectLanguage`]. The two
/// enums have identical variants; the wire spelling follows the IETF
/// language-tag contract (`"en"`, `"zh-CN"`) used by the apis layer's
/// `as_str`. Conversion between the two layers is a single 2-arm
/// `match`. Kept separate so the apis crate stays free of serde /
/// utoipa derives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ProjectLanguage {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl From<apis::project::ProjectLanguage> for ProjectLanguage {
    fn from(l: apis::project::ProjectLanguage) -> Self {
        match l {
            apis::project::ProjectLanguage::English => Self::English,
            apis::project::ProjectLanguage::SimplifiedChinese => Self::SimplifiedChinese,
        }
    }
}

impl From<ProjectLanguage> for apis::project::ProjectLanguage {
    fn from(l: ProjectLanguage) -> Self {
        match l {
            ProjectLanguage::English => apis::project::ProjectLanguage::English,
            ProjectLanguage::SimplifiedChinese => apis::project::ProjectLanguage::SimplifiedChinese,
        }
    }
}

/// Wire-level request body for a project's configuration. Mirrors
/// `apis::project::ProjectConfigurationData` field-for-field. The
/// `language` and `tags` fields are individually optional on the
/// wire so absent values stay absent on re-serialization.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtm_terminology: Option<TerminologyVersionRequest>,
}

impl From<ProjectConfigurationDataRequest> for apis::project::ProjectConfigurationData {
    fn from(c: ProjectConfigurationDataRequest) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c
                .tags
                .into_iter()
                .map(|t| apis::project::TagData {
                    key: t.key,
                    value: t.value,
                })
                .collect(),
            sdtmig: c.sdtmig.map(Into::into),
            sdtm_terminology: c.sdtm_terminology.map(Into::into),
        }
    }
}

/// Wire-level projection of a project's configuration. Mirrors
/// `apis::project::ProjectConfigurationView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtm_terminology: Option<TerminologyVersionResponse>,
}

impl From<apis::project::ProjectConfigurationView> for ProjectConfigurationViewResponse {
    fn from(c: apis::project::ProjectConfigurationView) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c.tags.into_iter().map(Into::into).collect(),
            sdtmig: c.sdtmig.map(Into::into),
            sdtm_terminology: c.sdtm_terminology.map(Into::into),
        }
    }
}

// -- project membership DTOs -----------------------------------------------

/// Wire-level request payload for a project's membership. `default`
/// on each vector lets a JSON `{}` deserialize to a present-but-empty
/// membership, which is the difference between "leave alone" and
/// "wipe the team" during project update. `skip_serializing_if` on
/// each vector keeps empty membership objects round-tripping as `{}`
/// rather than `{"leaders":[],"workers":[]}`.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberDataRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserSummaryViewResponse {
    pub code: String,
    pub name: String,
}

impl From<apis::project::UserSummaryView> for UserSummaryViewResponse {
    fn from(view: apis::project::UserSummaryView) -> Self {
        Self {
            code: view.code,
            name: view.name,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberViewResponse {
    pub leaders: Vec<UserSummaryViewResponse>,
    pub workers: Vec<UserSummaryViewResponse>,
}

impl From<apis::project::ProjectMemberView> for ProjectMemberViewResponse {
    fn from(view: apis::project::ProjectMemberView) -> Self {
        Self {
            leaders: view.leaders.into_iter().map(Into::into).collect(),
            workers: view.workers.into_iter().map(Into::into).collect(),
        }
    }
}

// -- project requests / responses ------------------------------------------

/// Wire-level request body for `POST /api/project`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
}

/// Wire-level request body for `PATCH /api/project/{code}`.
///
/// Membership fields preserve the missing-vs-empty distinction the
/// usecase relies on: `None` (field absent) leaves the team alone;
/// `Some(empty)` (a present `{}`) wipes the team. Both vector
/// fields use `#[serde(default)]` so a present `{}` deserializes to
/// `Some(ProjectMemberDataRequest { leaders: vec![], workers: vec![] })`
/// rather than failing on missing keys. `configurations` follows
/// the same `None`-vs-`Some(empty)` semantics: missing leaves the
/// configuration alone, present-with-payload replaces it whole.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub configurations: ProjectConfigurationViewResponse,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::project::ProjectView> for ProjectViewResponse {
    fn from(view: apis::project::ProjectView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            description: view.description,
            members: view.members.into(),
            unblind_members: view.unblind_members.into(),
            configurations: view.configurations.into(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectViewResponse>,
}

// -- terminology kind --------------------------------------------------------

/// Wire-level mirror of `apis::terminology::TerminologyKind`. The two
/// enums have identical variants; the conversion is a single 2-arm
/// `match`. Kept separate so the apis crate stays free of serde /
/// utoipa derives. `Default` is derived (defaulting to `Sdtm`, the
/// more common standard) so query DTOs that wrap it can derive
/// `Default` for serde-deserialization of partial query strings.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminologyKind {
    #[default]
    Sdtm,
    Adam,
}

impl From<apis::terminology::TerminologyKind> for TerminologyKind {
    fn from(k: apis::terminology::TerminologyKind) -> Self {
        match k {
            apis::terminology::TerminologyKind::Sdtm => TerminologyKind::Sdtm,
            apis::terminology::TerminologyKind::Adam => TerminologyKind::Adam,
        }
    }
}

impl From<TerminologyKind> for apis::terminology::TerminologyKind {
    fn from(k: TerminologyKind) -> Self {
        match k {
            TerminologyKind::Sdtm => apis::terminology::TerminologyKind::Sdtm,
            TerminologyKind::Adam => apis::terminology::TerminologyKind::Adam,
        }
    }
}

// -- terminology view DTOs ---------------------------------------------------

/// Wire-level extractor for the `{id}` URL parameter used by every
/// version / code-list / code-item path.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathId {
    pub id: i64,
}

/// Wire-level projection of a `TerminologyVersion`. Mirrors
/// `apis::terminology::TerminologyVersionView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionViewResponse {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::terminology::TerminologyVersionView> for TerminologyVersionViewResponse {
    fn from(view: apis::terminology::TerminologyVersionView) -> Self {
        Self {
            id: view.id,
            kind: view.kind.into(),
            name: view.name,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Wire-level wrapper for `GET /api/terminology/versions`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionListResponse {
    pub versions: Vec<TerminologyVersionViewResponse>,
}

/// Wire-level projection of a `CodeList`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeListViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::terminology::CodeListView> for CodeListViewResponse {
    fn from(view: apis::terminology::CodeListView) -> Self {
        Self {
            id: view.id,
            version_id: view.version_id,
            code: view.code,
            extensible: view.extensible,
            name: view.name,
            submission_value: view.submission_value,
            synonym: view.synonym,
            definition: view.definition,
            nci_preferred_term: view.nci_preferred_term,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Paged envelope for `GET /api/terminology/code-lists`. `items`
/// carries the page's rows; `nextOffset = Some(n)` tells the client
/// the next call should pass `offset = n`; `nextOffset = None` means
/// the caller has reached the end of the result set.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeListListResponse {
    pub items: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

impl From<apis::terminology::Page<apis::terminology::CodeListView>> for PagedCodeListListResponse {
    fn from(page: apis::terminology::Page<apis::terminology::CodeListView>) -> Self {
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        }
    }
}

/// Wire-level projection of a `CodeItem`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemViewResponse {
    pub id: i64,
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::terminology::CodeItemView> for CodeItemViewResponse {
    fn from(view: apis::terminology::CodeItemView) -> Self {
        Self {
            id: view.id,
            codelist_id: view.codelist_id,
            version_id: view.version_id,
            code: view.code,
            submission_value: view.submission_value,
            synonym: view.synonym,
            definition: view.definition,
            nci_preferred_term: view.nci_preferred_term,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Paged envelope for `GET /api/terminology/code-items`. Mirrors
/// [`PagedCodeListListResponse`] but scopes to a codelist.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

impl From<apis::terminology::Page<apis::terminology::CodeItemView>> for PagedCodeItemListResponse {
    fn from(page: apis::terminology::Page<apis::terminology::CodeItemView>) -> Self {
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        }
    }
}

/// Non-paginated list wrapper used by natural-key lookups
/// (`GET /api/terminology/code-items/by-version-and-code`) where
/// pagination makes no sense — every row that matches the natural
/// key is returned in one shot.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
}

// -- terminology request DTOs -----------------------------------------------

/// Wire-level request body for `POST /api/terminology/versions`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminologyVersionRequest {
    pub kind: TerminologyKind,
    pub name: String,
}

/// Wire-level request body for `PATCH /api/terminology/versions/{id}`.
/// Every field is optional. `skip_serializing_if` keeps a
/// partial update round-trip lossless. Deliberately omits `id`: the
/// handler reads it from the `{id}` URL parameter and threads it
/// into `apis::terminology::UpdateTerminologyVersionRequest`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminologyVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TerminologyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Wire-level request body for `POST /api/terminology/code-lists`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeListRequest {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level request body for `PATCH /api/terminology/code-lists/{id}`.
///
/// Deliberately omits `id`: the handler reads it from the `{id}`
/// URL parameter and threads it into
/// `apis::terminology::UpdateCodeListRequest`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

/// Wire-level request body for `POST /api/terminology/code-items`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeItemRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level request body for `PATCH /api/terminology/code-items/{id}`.
///
/// Deliberately omits `id`: the handler reads it from the `{id}`
/// URL parameter and threads it into
/// `apis::terminology::UpdateCodeItemRequest`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

/// Wire-level request body for `POST /api/terminology/code-items/batch`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

/// Wire-level entry inside a batch request.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level response for `POST /api/terminology/code-items/batch`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}

// -- terminology list / search query DTOs -----------------------------------

/// Query string for `GET /api/terminology/code-lists`. Unified list
/// + search: `fragment = None` (or empty) yields a plain
///   `ORDER BY id ASC` list; `fragment = Some(_)` runs the FTS
///   prefix-match path with `ts_rank DESC, id ASC` ordering.
///   `offset` / `limit` are clamped by the backend (default 50,
///   max 500).
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeListListQuery {
    pub version_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

/// Query string for `GET /api/terminology/code-items`. Mirrors
/// [`CodeListListQuery`] but scoped to a `codelistId`. Both
/// `versionId` and `codelistId` are optional on the wire: omit
/// (or send `null`) to list every code item known to the backend;
/// supply an id to restrict to a single owning version /
/// codelist (the typical per-version or per-codelist browse path).
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codelist_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

/// Query string for `GET /api/terminology/code-items/by-version-and-code`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemByVersionAndCodeQuery {
    pub version_id: i64,
    pub code: String,
}

// ---- domain_model wire DTOs ----------------------------------------

// The DTOs below mirror the apis::domain_model types. Each request
// / view / query lives in its own struct so the wire format (field
// names, optionality, casing) can evolve independently of the
// backend contract. Handlers translate via `From` / `.into()` so
// they stay one-liners.

/// Wire enum mirroring [`apis::domain_model::DomainCategory`].
/// The apis contract pins the JSON spelling per variant (e.g.
/// `"Special Purpose"` with a space); the round-trip test pins
/// the wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

/// Wire enum mirroring [`apis::domain_model::SdtmVariableType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

/// Wire enum mirroring [`apis::domain_model::SdtmVariableCore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

/// Wire enum mirroring [`apis::domain_model::SdtmRole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

impl From<apis::domain_model::DomainCategory> for DomainCategory {
    fn from(c: apis::domain_model::DomainCategory) -> Self {
        match c {
            apis::domain_model::DomainCategory::SpecialPurpose => Self::SpecialPurpose,
            apis::domain_model::DomainCategory::Interventions => Self::Interventions,
            apis::domain_model::DomainCategory::Events => Self::Events,
            apis::domain_model::DomainCategory::Findings => Self::Findings,
            apis::domain_model::DomainCategory::TrialDesign => Self::TrialDesign,
            apis::domain_model::DomainCategory::Relationships => Self::Relationships,
            apis::domain_model::DomainCategory::StudyReference => Self::StudyReference,
        }
    }
}

impl From<DomainCategory> for apis::domain_model::DomainCategory {
    fn from(c: DomainCategory) -> Self {
        match c {
            DomainCategory::SpecialPurpose => Self::SpecialPurpose,
            DomainCategory::Interventions => Self::Interventions,
            DomainCategory::Events => Self::Events,
            DomainCategory::Findings => Self::Findings,
            DomainCategory::TrialDesign => Self::TrialDesign,
            DomainCategory::Relationships => Self::Relationships,
            DomainCategory::StudyReference => Self::StudyReference,
        }
    }
}

impl From<apis::domain_model::SdtmVariableType> for SdtmVariableType {
    fn from(t: apis::domain_model::SdtmVariableType) -> Self {
        match t {
            apis::domain_model::SdtmVariableType::Numeric => Self::Numeric,
            apis::domain_model::SdtmVariableType::Character => Self::Character,
        }
    }
}

impl From<SdtmVariableType> for apis::domain_model::SdtmVariableType {
    fn from(t: SdtmVariableType) -> Self {
        match t {
            SdtmVariableType::Numeric => Self::Numeric,
            SdtmVariableType::Character => Self::Character,
        }
    }
}

impl From<apis::domain_model::SdtmVariableCore> for SdtmVariableCore {
    fn from(c: apis::domain_model::SdtmVariableCore) -> Self {
        match c {
            apis::domain_model::SdtmVariableCore::Req => Self::Req,
            apis::domain_model::SdtmVariableCore::Exp => Self::Exp,
            apis::domain_model::SdtmVariableCore::Perm => Self::Perm,
            apis::domain_model::SdtmVariableCore::Supp => Self::Supp,
        }
    }
}

impl From<SdtmVariableCore> for apis::domain_model::SdtmVariableCore {
    fn from(c: SdtmVariableCore) -> Self {
        match c {
            SdtmVariableCore::Req => Self::Req,
            SdtmVariableCore::Exp => Self::Exp,
            SdtmVariableCore::Perm => Self::Perm,
            SdtmVariableCore::Supp => Self::Supp,
        }
    }
}

impl From<apis::domain_model::SdtmRole> for SdtmRole {
    fn from(r: apis::domain_model::SdtmRole) -> Self {
        match r {
            apis::domain_model::SdtmRole::Identifier => Self::Identifier,
            apis::domain_model::SdtmRole::Topic => Self::Topic,
            apis::domain_model::SdtmRole::Timing => Self::Timing,
            apis::domain_model::SdtmRole::RecordQualifier => Self::RecordQualifier,
            apis::domain_model::SdtmRole::SynonymQualifier => Self::SynonymQualifier,
            apis::domain_model::SdtmRole::VariableQualifier => Self::VariableQualifier,
            apis::domain_model::SdtmRole::GroupingQualifier => Self::GroupingQualifier,
            apis::domain_model::SdtmRole::Rule => Self::Rule,
        }
    }
}

impl From<SdtmRole> for apis::domain_model::SdtmRole {
    fn from(r: SdtmRole) -> Self {
        match r {
            SdtmRole::Identifier => Self::Identifier,
            SdtmRole::Topic => Self::Topic,
            SdtmRole::Timing => Self::Timing,
            SdtmRole::RecordQualifier => Self::RecordQualifier,
            SdtmRole::SynonymQualifier => Self::SynonymQualifier,
            SdtmRole::VariableQualifier => Self::VariableQualifier,
            SdtmRole::GroupingQualifier => Self::GroupingQualifier,
            SdtmRole::Rule => Self::Rule,
        }
    }
}

/// Wire projection of an SDTM domain description. Carried on
/// [`SdtmDomainView`] and round-trips through the backend as a
/// single JSONB blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// Wire projection of an SDTM variable description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

impl From<apis::domain_model::SdtmDomainDescription> for SdtmDomainDescription {
    fn from(d: apis::domain_model::SdtmDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        }
    }
}

impl From<SdtmDomainDescription> for apis::domain_model::SdtmDomainDescription {
    fn from(d: SdtmDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: apis::domain_model::SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        }
    }
}

impl From<apis::domain_model::SdtmVariableDescription> for SdtmVariableDescription {
    fn from(d: apis::domain_model::SdtmVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

impl From<SdtmVariableDescription> for apis::domain_model::SdtmVariableDescription {
    fn from(d: SdtmVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: apis::domain_model::SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmVariableControlled`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}

impl From<apis::domain_model::SdtmVariableControlled> for SdtmVariableControlled {
    fn from(v: apis::domain_model::SdtmVariableControlled) -> Self {
        Self {
            code: v.code,
            format: v.format,
        }
    }
}

impl From<SdtmVariableControlled> for apis::domain_model::SdtmVariableControlled {
    fn from(v: SdtmVariableControlled) -> Self {
        Self {
            code: v.code,
            format: v.format,
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmVersionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVersionViewResponse {
    pub id: i64,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::domain_model::SdtmVersionView> for SdtmVersionViewResponse {
    fn from(v: apis::domain_model::SdtmVersionView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmDomainView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::domain_model::SdtmDomainView> for SdtmDomainViewResponse {
    fn from(v: apis::domain_model::SdtmDomainView) -> Self {
        Self {
            id: v.id,
            version_id: v.version_id,
            name: v.name,
            category: v.category.into(),
            descriptions: v.descriptions.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmVariableView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableViewResponse {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::domain_model::SdtmVariableView> for SdtmVariableViewResponse {
    fn from(v: apis::domain_model::SdtmVariableView) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            name: v.name,
            variable_controlled: v.variable_controlled.map(Into::into),
            variable_type: v.variable_type.into(),
            variable_core: v.variable_core.into(),
            variable_role: v.variable_role.map(Into::into),
            variable_sequence: v.variable_sequence,
            descriptions: v.descriptions.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

// ---- list wrappers ----

/// Wire projection of `GET /api/domain-model/versions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVersionListResponse {
    pub versions: Vec<SdtmVersionViewResponse>,
}

impl From<apis::domain_model::SdtmVersionList> for SdtmVersionListResponse {
    fn from(l: apis::domain_model::SdtmVersionList) -> Self {
        Self {
            versions: l.versions.into_iter().map(Into::into).collect(),
        }
    }
}

/// Wire projection of `GET /api/domain-model/versions/{version_id}/domains`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainListResponse {
    pub domains: Vec<SdtmDomainViewResponse>,
}

impl From<apis::domain_model::SdtmDomainList> for SdtmDomainListResponse {
    fn from(l: apis::domain_model::SdtmDomainList) -> Self {
        Self {
            domains: l.domains.into_iter().map(Into::into).collect(),
        }
    }
}

/// Wire projection of `GET /api/domain-model/domains/{domain_id}/variables`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableListResponse {
    pub variables: Vec<SdtmVariableViewResponse>,
}

impl From<apis::domain_model::SdtmVariableList> for SdtmVariableListResponse {
    fn from(l: apis::domain_model::SdtmVariableList) -> Self {
        Self {
            variables: l.variables.into_iter().map(Into::into).collect(),
        }
    }
}

// ---- request bodies ----

/// `POST /api/domain-model/versions` body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmVersionRequest {
    pub name: String,
}

/// `PUT /api/domain-model/versions/{id}` body. All fields are
/// optional; absent fields are not touched (partial update).
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmVersionRequest {
    pub name: Option<String>,
}

/// `POST /api/domain-model/domains` body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmDomainRequest {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// `PUT /api/domain-model/domains/{id}` body. All fields optional
/// except via the apis partial-update contract.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmDomainRequest {
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

/// `POST /api/domain-model/variables` body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmVariableRequest {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// `PUT /api/domain-model/variables/{id}` body.
///
/// `variable_controlled` is a three-state field: a missing key means
/// "leave the column alone", an explicit `null` means "clear the
/// controlled value", and a value means "replace it". It uses the
/// `double_option` deserializer so serde does not collapse the first
/// two — a plain `Option<Option<T>>` derive maps `null` to the outer
/// `None`, which would make "clear" unreachable from a client.
///
/// `variable_role` still uses the plain derive, so for that field both
/// an absent key and an explicit `null` mean "don't change".
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmVariableRequest {
    pub name: Option<String>,
    /// Three-state: absent = unchanged, `null` = clear, value =
    /// replace. See [`double_option`].
    #[serde(default, deserialize_with = "double_option")]
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

// ---- adapters between wire DTOs and apis DTOs ----
//
// Create requests convert via `From` (one-arg, no `id`). Update
// requests are built inline by the handler because the apis DTO
// carries the `id` (read from the URL path), not the wire DTO.

impl From<CreateSdtmVersionRequest> for apis::domain_model::CreateSdtmVersionRequest {
    fn from(r: CreateSdtmVersionRequest) -> Self {
        Self { name: r.name }
    }
}

impl From<CreateSdtmDomainRequest> for apis::domain_model::CreateSdtmDomainRequest {
    fn from(r: CreateSdtmDomainRequest) -> Self {
        Self {
            version_id: r.version_id,
            name: r.name,
            category: r.category.into(),
            descriptions: r.descriptions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CreateSdtmVariableRequest> for apis::domain_model::CreateSdtmVariableRequest {
    fn from(r: CreateSdtmVariableRequest) -> Self {
        Self {
            domain_id: r.domain_id,
            name: r.name,
            variable_controlled: r.variable_controlled.map(Into::into),
            variable_type: r.variable_type.into(),
            variable_core: r.variable_core.into(),
            variable_role: r.variable_role.map(Into::into),
            variable_sequence: r.variable_sequence,
            descriptions: r.descriptions.into_iter().map(Into::into).collect(),
        }
    }
}

// ===========================================================================
// CRF (Case Report Form) wire DTOs
// ===========================================================================

/// Wire projection of [`apis::crf::CrfItemKind`]. Flat enum — the
/// server side carries the same discriminant set; `From`/`Into`
/// conversions between the two layers are lossless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CrfItemKind {
    Text,
    Selection,
    Checkbox,
    Datetime,
    Label,
}

impl From<apis::crf::CrfItemKind> for CrfItemKind {
    fn from(k: apis::crf::CrfItemKind) -> Self {
        match k {
            apis::crf::CrfItemKind::Text => Self::Text,
            apis::crf::CrfItemKind::Selection => Self::Selection,
            apis::crf::CrfItemKind::Checkbox => Self::Checkbox,
            apis::crf::CrfItemKind::Datetime => Self::Datetime,
            apis::crf::CrfItemKind::Label => Self::Label,
        }
    }
}

impl From<CrfItemKind> for apis::crf::CrfItemKind {
    fn from(k: CrfItemKind) -> Self {
        match k {
            CrfItemKind::Text => Self::Text,
            CrfItemKind::Selection => Self::Selection,
            CrfItemKind::Checkbox => Self::Checkbox,
            CrfItemKind::Datetime => Self::Datetime,
            CrfItemKind::Label => Self::Label,
        }
    }
}

/// Wire projection of [`apis::crf::AnnotationOwner`]. The DB-side
/// tuple variant `Form(i32)` becomes a struct-shaped variant so the
/// TS client can `switch (owner.kind)` cleanly. Each variant carries
/// the owning row's id under `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AnnotationOwner {
    Form {
        id: i64,
    },
    Item {
        id: i64,
    },
    #[serde(rename = "option")]
    Option {
        id: i64,
    },
    Unit {
        id: i64,
    },
}

impl From<apis::crf::AnnotationOwner> for AnnotationOwner {
    fn from(o: apis::crf::AnnotationOwner) -> Self {
        match o {
            apis::crf::AnnotationOwner::Form(id) => Self::Form { id },
            apis::crf::AnnotationOwner::Item(id) => Self::Item { id },
            apis::crf::AnnotationOwner::Option(id) => Self::Option { id },
            apis::crf::AnnotationOwner::Unit(id) => Self::Unit { id },
        }
    }
}

impl From<AnnotationOwner> for apis::crf::AnnotationOwner {
    fn from(o: AnnotationOwner) -> Self {
        match o {
            AnnotationOwner::Form { id } => Self::Form(id),
            AnnotationOwner::Item { id } => Self::Item(id),
            AnnotationOwner::Option { id } => Self::Option(id),
            AnnotationOwner::Unit { id } => Self::Unit(id),
        }
    }
}

/// Path parameter for CRF routes. CRF uses `i64` ids to match the
/// underlying Postgres BIGSERIAL/BIGINT columns; kept as its own
/// type to avoid a footgun against the `i64`-based terminology
/// routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CrfPathId {
    pub id: i64,
}

/// Wire-level extractor for the `{project_code}` URL parameter
/// used by `/api/crf/projects/{project_code}/versions` and friends.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProjectPathCode {
    pub project_code: String,
}

// ---- structures -------------------------------------------------------
//
// Wire types for the directory-structure namespace. They mirror
// `apis::structures::*` field for field.

/// Which kind of structure this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum StructureLevelResponse {
    Template,
    Project,
}

/// What to copy into a directory once it has been created.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MigrationViewResponse {
    pub directory: String,
    pub filter: Vec<String>,
}

/// One node of the effective tree.
///
/// The schema is written by hand rather than derived: the type is
/// recursive (a node's children are nodes), and utoipa 5's `ToSchema`
/// derive cannot express that — it collects child schemas eagerly and
/// recurses forever. The manual implementation below expresses the
/// recursion the way OpenAPI 3 does: the `children` items are a `$ref`
/// back to this schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeViewResponse {
    pub node_id: String,
    pub description: String,
    pub path: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
    pub children: Vec<NodeViewResponse>,
}

impl utoipa::PartialSchema for NodeViewResponse {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{ArrayBuilder, Object, ObjectBuilder, SchemaType, Type};

        let nullable_string = Object::with_type(SchemaType::from_iter([Type::String, Type::Null]));
        ObjectBuilder::new()
            .property("nodeId", Object::with_type(SchemaType::Type(Type::String)))
            .property(
                "description",
                Object::with_type(SchemaType::Type(Type::String)),
            )
            .property("path", Object::with_type(SchemaType::Type(Type::String)))
            .property(
                "directory",
                Object::with_type(SchemaType::Type(Type::Boolean)),
            )
            .property("parent", nullable_string.clone())
            .property("reference", nullable_string)
            .property("migration", MigrationViewResponse::schema())
            .property(
                "children",
                ArrayBuilder::new()
                    .items(utoipa::openapi::Ref::from_schema_name("NodeViewResponse")),
            )
            .required("nodeId")
            .required("description")
            .required("path")
            .required("directory")
            .required("children")
            .into()
    }
}

impl utoipa::ToSchema for NodeViewResponse {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("NodeViewResponse")
    }

    fn schemas(
        schemas: &mut Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) {
        // Register this schema exactly once — the `children` property
        // refers back to it by name rather than re-collecting it.
        if !schemas.iter().any(|(name, _)| name == "NodeViewResponse") {
            schemas.push((
                "NodeViewResponse".to_string(),
                <Self as utoipa::PartialSchema>::schema(),
            ));
        }
    }
}

/// A structure and its effective tree.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureViewResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
    pub nodes: Vec<NodeViewResponse>,
}

/// A structure's identity, without its tree.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureSummaryResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// A structure's identity and level, without the tree.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureMetaResponse {
    pub id: i64,
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// The effective tree of a structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureTreeResponse {
    pub structure_id: i64,
    pub nodes: Vec<NodeViewResponse>,
}

/// One of a structure's own records.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OwnNodeResponse {
    pub node_id: String,
    pub description: String,
    pub path: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
    pub hidden: bool,
    pub overrides_template: bool,
}

/// A page-less list of structures.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StructureListResponse {
    pub structures: Vec<StructureSummaryResponse>,
}

/// A page-less list of a structure's own records.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OwnNodeListResponse {
    pub nodes: Vec<OwnNodeResponse>,
}

/// Create a structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateStructureRequest {
    pub name: String,
    pub level: StructureLevelResponse,
    pub project_code: Option<String>,
    pub template_id: Option<i64>,
}

/// Rename a structure.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameStructureRequest {
    pub name: String,
}

/// Place a node at a path. `parent` is always present; `null` means
/// "at the root".
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AddNodeRequest {
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
    pub hidden: Option<bool>,
}

/// Replace a node's mutable attributes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNodeRequest {
    pub path: String,
    pub description: String,
    pub directory: bool,
    pub parent: Option<String>,
    pub reference: Option<String>,
    pub migration: Option<MigrationViewResponse>,
    pub hidden: Option<bool>,
}

/// The optional filters for the structure list.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListStructuresQuery {
    pub level: Option<StructureLevelResponse>,
    pub project_code: Option<String>,
}

/// Wire-level extractor for the `{id}` and `{node_id}` URL parameters.
///
/// No `rename_all` here: this is an extractor, not a wire body, and a
/// camelCase rename would turn `node_id` into `nodeId` — which no
/// longer matches the `{node_id}` path parameter.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StructureNodePath {
    pub id: i64,
    pub node_id: String,
}

// ---- structures: view conversions ------------------------------------

impl From<apis::structures::StructureLevel> for StructureLevelResponse {
    fn from(v: apis::structures::StructureLevel) -> Self {
        match v {
            apis::structures::StructureLevel::Template => StructureLevelResponse::Template,
            apis::structures::StructureLevel::Project => StructureLevelResponse::Project,
        }
    }
}

impl From<apis::structures::MigrationView> for MigrationViewResponse {
    fn from(v: apis::structures::MigrationView) -> Self {
        Self {
            directory: v.directory,
            filter: v.filter,
        }
    }
}

impl From<apis::structures::NodeView> for NodeViewResponse {
    fn from(v: apis::structures::NodeView) -> Self {
        Self {
            node_id: v.node_id,
            description: v.description,
            path: v.path,
            directory: v.directory,
            parent: v.parent,
            reference: v.reference,
            migration: v.migration.map(Into::into),
            children: v.children.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::structures::StructureView> for StructureViewResponse {
    fn from(v: apis::structures::StructureView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            level: v.level.into(),
            project_code: v.project_code,
            template_id: v.template_id,
            nodes: v.nodes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::structures::StructureSummaryView> for StructureSummaryResponse {
    fn from(v: apis::structures::StructureSummaryView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            level: v.level.into(),
            project_code: v.project_code,
            template_id: v.template_id,
        }
    }
}

impl From<apis::structures::StructureMetaView> for StructureMetaResponse {
    fn from(v: apis::structures::StructureMetaView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            level: v.level.into(),
            project_code: v.project_code,
            template_id: v.template_id,
        }
    }
}

impl From<apis::structures::StructureTreeView> for StructureTreeResponse {
    fn from(v: apis::structures::StructureTreeView) -> Self {
        Self {
            structure_id: v.structure_id,
            nodes: v.nodes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::structures::OwnNodeView> for OwnNodeResponse {
    fn from(v: apis::structures::OwnNodeView) -> Self {
        Self {
            node_id: v.node_id,
            description: v.description,
            path: v.path,
            directory: v.directory,
            parent: v.parent,
            reference: v.reference,
            migration: v.migration.map(Into::into),
            hidden: v.hidden,
            overrides_template: v.overrides_template,
        }
    }
}

// ---- view projections ----

/// Wire projection of [`apis::crf::CrfVersionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfVersionView> for CrfVersionViewResponse {
    fn from(v: apis::crf::CrfVersionView) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfFormView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub approved: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfFormView> for CrfFormViewResponse {
    fn from(v: apis::crf::CrfFormView) -> Self {
        Self {
            id: v.id,
            version_id: v.version_id,
            code: v.code,
            name: v.name,
            order: v.order,
            not_submitted: v.not_submitted,
            approved: v.approved,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfItemView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfItemView> for CrfItemViewResponse {
    fn from(v: apis::crf::CrfItemView) -> Self {
        Self {
            id: v.id,
            form_id: v.form_id,
            code: v.code,
            name: v.name,
            kind: v.kind.into(),
            order: v.order,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfOptionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfOptionView> for CrfOptionViewResponse {
    fn from(v: apis::crf::CrfOptionView) -> Self {
        Self {
            id: v.id,
            item_id: v.item_id,
            value: v.value,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfUnitView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfUnitView> for CrfUnitViewResponse {
    fn from(v: apis::crf::CrfUnitView) -> Self {
        Self {
            id: v.id,
            item_id: v.item_id,
            value: v.value,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::DomainAnnotationView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::DomainAnnotationView> for DomainAnnotationViewResponse {
    fn from(v: apis::crf::DomainAnnotationView) -> Self {
        Self {
            id: v.id,
            form_id: v.form_id,
            name: v.name,
            description: v.description,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::AnnotationView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationViewResponse {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::AnnotationView> for AnnotationViewResponse {
    fn from(v: apis::crf::AnnotationView) -> Self {
        Self {
            id: v.id,
            domain_annotation_id: v.domain_annotation_id,
            content: v.content,
            assign: v.assign,
            owner: v.owner.into(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

// ---- list response wrappers ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionListResponse {
    pub versions: Vec<CrfVersionViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormListResponse {
    pub forms: Vec<CrfFormViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemListResponse {
    pub items: Vec<CrfItemViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionListResponse {
    pub options: Vec<CrfOptionViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitListResponse {
    pub units: Vec<CrfUnitViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationListResponse {
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationListResponse {
    pub annotations: Vec<AnnotationViewResponse>,
}

// ---- request DTOs ----

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfVersionRequest {
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfFormRequest {
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub approved: bool,
}

/// Body for `POST /api/crf/versions/{version_id}/forms/bulk`. The
/// owning `version_id` is supplied via the path segment, so it is
/// intentionally absent from the body — the request shape mirrors
/// the single-row `CreateCrfFormRequest` plus a list of items
/// (each carrying its own options + units).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCrfFormRequest {
    pub form: CreateCrfFormRequest,
    pub items: Vec<BulkCreateCrfItemInput>,
}

/// One item under a [`BulkCreateCrfFormRequest`], with its own
/// options / units subtree. Reuses the single-row
/// `CreateCrfItemRequest` / `CreateCrfOptionRequest` /
/// `CreateCrfUnitRequest` shapes; the bulk port stamps
/// `form_id` / `item_id` at insert time.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCrfItemInput {
    pub item: CreateCrfItemRequest,
    pub options: Vec<CreateCrfOptionRequest>,
    pub units: Vec<CreateCrfUnitRequest>,
}

/// Wire projection of
/// [`apis::crf::BulkCreateCrfFormResult`]. Returns the freshly
/// created form plus every item's view in input order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BulkCreateCrfFormResponse {
    pub form: CrfFormViewResponse,
    pub items: Vec<CrfItemViewResponse>,
}

impl From<apis::crf::BulkCreateCrfFormResult> for BulkCreateCrfFormResponse {
    fn from(r: apis::crf::BulkCreateCrfFormResult) -> Self {
        Self {
            form: r.form.into(),
            items: r.items.into_iter().map(Into::into).collect(),
        }
    }
}

/// Wire projection of [`apis::crf::CrfFormDetailView`]. Returned
/// by `GET /api/crf/forms/{id}/details`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormDetailResponse {
    pub form: CrfFormViewResponse,
    pub form_annotations: Vec<AnnotationViewResponse>,
    pub items: Vec<CrfItemDetailResponse>,
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

/// Wire projection of [`apis::crf::CrfItemDetailView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemDetailResponse {
    pub item: CrfItemViewResponse,
    pub options: Vec<CrfOptionDetailResponse>,
    pub units: Vec<CrfUnitDetailResponse>,
    pub annotations: Vec<AnnotationViewResponse>,
}

/// Wire projection of [`apis::crf::CrfOptionDetailView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionDetailResponse {
    pub option: CrfOptionViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

/// Wire projection of [`apis::crf::CrfUnitDetailView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitDetailResponse {
    pub unit: CrfUnitViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

impl From<apis::crf::CrfFormDetailView> for CrfFormDetailResponse {
    fn from(v: apis::crf::CrfFormDetailView) -> Self {
        Self {
            form: v.form.into(),
            form_annotations: v.form_annotations.into_iter().map(Into::into).collect(),
            items: v.items.into_iter().map(Into::into).collect(),
            domain_annotations: v.domain_annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::crf::CrfItemDetailView> for CrfItemDetailResponse {
    fn from(v: apis::crf::CrfItemDetailView) -> Self {
        Self {
            item: v.item.into(),
            options: v.options.into_iter().map(Into::into).collect(),
            units: v.units.into_iter().map(Into::into).collect(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::crf::CrfOptionDetailView> for CrfOptionDetailResponse {
    fn from(v: apis::crf::CrfOptionDetailView) -> Self {
        Self {
            option: v.option.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::crf::CrfUnitDetailView> for CrfUnitDetailResponse {
    fn from(v: apis::crf::CrfUnitDetailView) -> Self {
        Self {
            unit: v.unit.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfFormRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

/// Body for `POST /api/crf/forms/{id}/approval`. The id is in
/// the path; the body carries only the desired flag value.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetCrfApprovedRequest {
    pub approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfItemRequest {
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<CrfItemKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfOptionRequest {
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfOptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfUnitRequest {
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfUnitRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainAnnotationRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDomainAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assign: Option<bool>,
}

// ---- search query DTOs ----
//
// Each search endpoint takes a `versionId` from the URL path (the
// owning version is required for version-scoped substring search)
// and a `fragment` query parameter. The fragment is required — an
// empty / whitespace-only fragment is rejected at the usecase layer
// with `EmptySearchFragment`.

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFragmentQuery {
    pub fragment: String,
}

// ===========================================================================
// Mission wire DTOs
// ===========================================================================
//
// Wire projections of `apis::mission::*`. Handlers translate at the
// boundary so the apis crate stays free of serde / utoipa derives.
// Mission handlers authorize by checking leader membership of the
// owning project — there is no separate role gate.

/// Wire enum mirroring [`apis::mission::MissionKind`]. JSON spelling
/// is `snake_case` (`crf`, `sdtm`, `adam`, `tfl`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

impl From<apis::mission::MissionKind> for MissionKind {
    fn from(k: apis::mission::MissionKind) -> Self {
        match k {
            apis::mission::MissionKind::Crf => Self::Crf,
            apis::mission::MissionKind::Sdtm => Self::Sdtm,
            apis::mission::MissionKind::Adam => Self::Adam,
            apis::mission::MissionKind::Tfl => Self::Tfl,
        }
    }
}

impl From<MissionKind> for apis::mission::MissionKind {
    fn from(k: MissionKind) -> Self {
        match k {
            MissionKind::Crf => apis::mission::MissionKind::Crf,
            MissionKind::Sdtm => apis::mission::MissionKind::Sdtm,
            MissionKind::Adam => apis::mission::MissionKind::Adam,
            MissionKind::Tfl => apis::mission::MissionKind::Tfl,
        }
    }
}

/// Wire enum mirroring [`apis::mission::MissionRole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissionRole {
    Dev,
    Qc,
}

impl From<apis::mission::MissionRole> for MissionRole {
    fn from(r: apis::mission::MissionRole) -> Self {
        match r {
            apis::mission::MissionRole::Dev => Self::Dev,
            apis::mission::MissionRole::Qc => Self::Qc,
        }
    }
}

impl From<MissionRole> for apis::mission::MissionRole {
    fn from(r: MissionRole) -> Self {
        match r {
            MissionRole::Dev => apis::mission::MissionRole::Dev,
            MissionRole::Qc => apis::mission::MissionRole::Qc,
        }
    }
}

/// One assignee entry inside a [`CreateMissionRequest`] or a
/// standalone `add_assignee` call.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeDataRequest {
    pub user_code: String,
    pub role: MissionRole,
}

/// Body for `POST /api/mission`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    #[serde(default)]
    pub assignees: Vec<AssigneeDataRequest>,
}

/// Wire projection of [`apis::mission::AssigneeView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeViewResponse {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::mission::AssigneeView> for AssigneeViewResponse {
    fn from(v: apis::mission::AssigneeView) -> Self {
        Self {
            id: v.id,
            user_code: v.user_code,
            role: v.role.into(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::mission::MissionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeViewResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::mission::MissionView> for MissionViewResponse {
    fn from(v: apis::mission::MissionView) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            mission_kind: v.mission_kind.into(),
            mission_code: v.mission_code,
            assignees: v.assignees.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wrapper for the two mission list endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionListResponse {
    pub missions: Vec<MissionViewResponse>,
}

/// Query for `GET /api/mission/by-project/{project_code}`. The
/// optional `kind` query parameter narrows the result set to a
/// single `MissionKind`. `kind` is captured as `String` at the wire
/// so a malformed value can be surfaced as a `400` via
/// `MissionApiError::Validation` instead of a deserializer 422 —
/// see the handler.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionByProjectQuery {
    pub project_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

// ===========================================================================
// Mission issue DTOs
// ===========================================================================
//
// Wire projections of `apis::mission::{IssueState, IssueView,
// IssueCommentView}`. The open/closed state uses `snake_case` so the
// TS client can `switch (state)` on the string directly; the rest of
// the response follows the camelCase convention used elsewhere.

/// Wire enum mirroring [`apis::mission::IssueState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Opened,
    Closed,
}

impl From<apis::mission::IssueState> for IssueState {
    fn from(s: apis::mission::IssueState) -> Self {
        match s {
            apis::mission::IssueState::Opened => Self::Opened,
            apis::mission::IssueState::Closed => Self::Closed,
        }
    }
}

impl From<IssueState> for apis::mission::IssueState {
    fn from(s: IssueState) -> Self {
        match s {
            IssueState::Opened => apis::mission::IssueState::Opened,
            IssueState::Closed => apis::mission::IssueState::Closed,
        }
    }
}

/// Wire projection of [`apis::mission::IssueCommentView`]. Each
/// comment is an immutable record of "who said what when" appended to
/// the issue's thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueCommentViewResponse {
    pub user: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::mission::IssueCommentView> for IssueCommentViewResponse {
    fn from(c: apis::mission::IssueCommentView) -> Self {
        Self {
            user: c.user,
            content: c.content,
            created_at: c.created_at,
        }
    }
}

/// Wire projection of [`apis::mission::IssueView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueViewResponse {
    pub id: i64,
    pub mission_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueCommentViewResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::mission::IssueView> for IssueViewResponse {
    fn from(v: apis::mission::IssueView) -> Self {
        Self {
            id: v.id,
            mission_id: v.mission_id,
            target_item: v.target_item,
            issuer: v.issuer,
            description: v.description,
            state: v.state.into(),
            comments: v.comments.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wrapper for `GET /api/mission/by-mission/{mission_id}/issues`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueListResponse {
    pub issues: Vec<IssueViewResponse>,
}

/// Body for `POST /api/mission/by-mission/{mission_id}/issues`.
/// `targetItem` is optional on the wire — an issue can describe the
/// whole mission (no specific row) or a single row inside it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateIssueRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_item: Option<String>,
    pub description: String,
}

/// Body for `PATCH /api/mission/issues/{issue_id}/state`. Empty
/// marker struct — the state is supplied via the `?state=opened|closed`
/// query parameter, and the body is left empty.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchIssueStateRequest {}

/// Body for `PATCH /api/mission/issues/{issue_id}/description`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIssueDescriptionRequest {
    pub description: String,
}

/// Body for `POST /api/mission/issues/{issue_id}/comments`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppendCommentRequest {
    pub content: String,
}

/// Query for `GET /api/mission/by-mission/{mission_id}/issues`. The
/// optional `state` query parameter narrows the result set to a single
/// `IssueState`. `state` is captured as `String` at the wire so a
/// malformed value can be surfaced as a `400` via
/// `MissionApiError::Validation` instead of a deserializer 422.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssueListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// Wire shape for `POST /api/log-ingest/submit`. Mirrors
/// `logging_utils::LogIngestor::submit` semantics: `batch_id` is
/// the dedup key, `entries` are the UTF-8 log lines (one per
/// element).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogIngestRequest {
    pub batch_id: String,
    pub entries: Vec<String>,
}

/// Wire shape returned from `POST /api/log-ingest/submit`.
/// `accepted` is the number of entries the ingestor accepted
/// (always equals `entries.len()` on the happy path; on duplicate
/// `batch_id` the request is rejected before reaching the
/// ingestor, so this DTO is not returned).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogIngestResponse {
    pub batch_id: String,
    pub accepted: usize,
}

// -- sdtm-project ---------------------------------------------------------
//
// The wire types for the per-project SDTM model tree. They mirror
// `apis::sdtm_project` and deliberately do NOT reuse the
// `domain-model` section's enums above: the CDISC standard and a
// project's overlay are separate features and are allowed to diverge.

// Wire enum mirroring [`apis::sdtm_project::DomainCategory`]. The
// apis contract pins the JSON spelling per variant (e.g. `"Special
// Purpose"` with a space).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SdtmProjectDomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

/// Wire enum mirroring [`apis::sdtm_project::SdtmVariableType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmProjectVariableType {
    Numeric,
    Character,
}

/// Wire enum mirroring [`apis::sdtm_project::SdtmVariableCore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmProjectVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

/// Wire enum mirroring [`apis::sdtm_project::SdtmRole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SdtmProjectRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

impl From<apis::sdtm_project::DomainCategory> for SdtmProjectDomainCategory {
    fn from(c: apis::sdtm_project::DomainCategory) -> Self {
        match c {
            apis::sdtm_project::DomainCategory::SpecialPurpose => Self::SpecialPurpose,
            apis::sdtm_project::DomainCategory::Interventions => Self::Interventions,
            apis::sdtm_project::DomainCategory::Events => Self::Events,
            apis::sdtm_project::DomainCategory::Findings => Self::Findings,
            apis::sdtm_project::DomainCategory::TrialDesign => Self::TrialDesign,
            apis::sdtm_project::DomainCategory::Relationships => Self::Relationships,
            apis::sdtm_project::DomainCategory::StudyReference => Self::StudyReference,
        }
    }
}

impl From<SdtmProjectDomainCategory> for apis::sdtm_project::DomainCategory {
    fn from(c: SdtmProjectDomainCategory) -> Self {
        match c {
            SdtmProjectDomainCategory::SpecialPurpose => Self::SpecialPurpose,
            SdtmProjectDomainCategory::Interventions => Self::Interventions,
            SdtmProjectDomainCategory::Events => Self::Events,
            SdtmProjectDomainCategory::Findings => Self::Findings,
            SdtmProjectDomainCategory::TrialDesign => Self::TrialDesign,
            SdtmProjectDomainCategory::Relationships => Self::Relationships,
            SdtmProjectDomainCategory::StudyReference => Self::StudyReference,
        }
    }
}

impl From<apis::sdtm_project::SdtmVariableType> for SdtmProjectVariableType {
    fn from(t: apis::sdtm_project::SdtmVariableType) -> Self {
        match t {
            apis::sdtm_project::SdtmVariableType::Numeric => Self::Numeric,
            apis::sdtm_project::SdtmVariableType::Character => Self::Character,
        }
    }
}

impl From<SdtmProjectVariableType> for apis::sdtm_project::SdtmVariableType {
    fn from(t: SdtmProjectVariableType) -> Self {
        match t {
            SdtmProjectVariableType::Numeric => Self::Numeric,
            SdtmProjectVariableType::Character => Self::Character,
        }
    }
}

impl From<apis::sdtm_project::SdtmVariableCore> for SdtmProjectVariableCore {
    fn from(c: apis::sdtm_project::SdtmVariableCore) -> Self {
        match c {
            apis::sdtm_project::SdtmVariableCore::Req => Self::Req,
            apis::sdtm_project::SdtmVariableCore::Exp => Self::Exp,
            apis::sdtm_project::SdtmVariableCore::Perm => Self::Perm,
            apis::sdtm_project::SdtmVariableCore::Supp => Self::Supp,
        }
    }
}

impl From<SdtmProjectVariableCore> for apis::sdtm_project::SdtmVariableCore {
    fn from(c: SdtmProjectVariableCore) -> Self {
        match c {
            SdtmProjectVariableCore::Req => Self::Req,
            SdtmProjectVariableCore::Exp => Self::Exp,
            SdtmProjectVariableCore::Perm => Self::Perm,
            SdtmProjectVariableCore::Supp => Self::Supp,
        }
    }
}

impl From<apis::sdtm_project::SdtmRole> for SdtmProjectRole {
    fn from(r: apis::sdtm_project::SdtmRole) -> Self {
        match r {
            apis::sdtm_project::SdtmRole::Identifier => Self::Identifier,
            apis::sdtm_project::SdtmRole::Topic => Self::Topic,
            apis::sdtm_project::SdtmRole::Timing => Self::Timing,
            apis::sdtm_project::SdtmRole::RecordQualifier => Self::RecordQualifier,
            apis::sdtm_project::SdtmRole::SynonymQualifier => Self::SynonymQualifier,
            apis::sdtm_project::SdtmRole::VariableQualifier => Self::VariableQualifier,
            apis::sdtm_project::SdtmRole::GroupingQualifier => Self::GroupingQualifier,
            apis::sdtm_project::SdtmRole::Rule => Self::Rule,
        }
    }
}

impl From<SdtmProjectRole> for apis::sdtm_project::SdtmRole {
    fn from(r: SdtmProjectRole) -> Self {
        match r {
            SdtmProjectRole::Identifier => Self::Identifier,
            SdtmProjectRole::Topic => Self::Topic,
            SdtmProjectRole::Timing => Self::Timing,
            SdtmProjectRole::RecordQualifier => Self::RecordQualifier,
            SdtmProjectRole::SynonymQualifier => Self::SynonymQualifier,
            SdtmProjectRole::VariableQualifier => Self::VariableQualifier,
            SdtmProjectRole::GroupingQualifier => Self::GroupingQualifier,
            SdtmProjectRole::Rule => Self::Rule,
        }
    }
}

// -- description DTOs ------------------------------------------------------

/// Wire mirror of `apis::sdtm_project::SdtmDomainDescription`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SdtmProjectDomainDescription {
    pub lang: String,
    pub details: SdtmProjectDomainDescriptionDetail,
}

/// Wire mirror of `apis::sdtm_project::SdtmDomainDescriptionDetail`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SdtmProjectDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// Wire mirror of `apis::sdtm_project::SdtmVariableDescription`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SdtmProjectVariableDescription {
    pub lang: String,
    pub details: SdtmProjectVariableDescriptionDetail,
}

/// Wire mirror of `apis::sdtm_project::SdtmVariableDescriptionDetail`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SdtmProjectVariableDescriptionDetail {
    pub label: String,
}

impl From<apis::sdtm_project::SdtmDomainDescription> for SdtmProjectDomainDescription {
    fn from(d: apis::sdtm_project::SdtmDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: SdtmProjectDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        }
    }
}

impl From<SdtmProjectDomainDescription> for apis::sdtm_project::SdtmDomainDescription {
    fn from(d: SdtmProjectDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: apis::sdtm_project::SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        }
    }
}

impl From<apis::sdtm_project::SdtmVariableDescription> for SdtmProjectVariableDescription {
    fn from(d: apis::sdtm_project::SdtmVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: SdtmProjectVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

impl From<SdtmProjectVariableDescription> for apis::sdtm_project::SdtmVariableDescription {
    fn from(d: SdtmProjectVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: apis::sdtm_project::SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

// -- requests --------------------------------------------------------------

/// Deserialise a nullable field into `Option<Option<T>>` so all three
/// states stay distinguishable over the wire:
///
/// - a missing key is `None` — "leave the column alone"
/// - an explicit `null` is `Some(None)` — "clear the column"
/// - a value is `Some(Some(v))` — "replace the column"
///
/// A plain `Option<Option<T>>` derive collapses the first two: serde
/// maps `null` to `None`, so "clear" would be unreachable from a
/// client.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmProjectVersionRequest {
    pub project_code: String,
    pub version_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmProjectVersionRequest {
    pub version_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmProjectDomainRequest {
    pub version_id: i64,
    pub domain_code: String,
    pub key_variables: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmProjectDomainRequest {
    pub domain_code: Option<String>,
    pub key_variables: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmProjectDomainOverrideRequest {
    pub category: SdtmProjectDomainCategory,
    pub descriptions: Vec<SdtmProjectDomainDescription>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmProjectDomainOverrideRequest {
    pub category: Option<SdtmProjectDomainCategory>,
    pub descriptions: Option<Vec<SdtmProjectDomainDescription>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmProjectVariableRequest {
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmProjectVariableRequest {
    pub variable_code: Option<String>,
    pub variable_sequence: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmProjectVariableOverrideRequest {
    pub variable_type: SdtmProjectVariableType,
    pub variable_core: SdtmProjectVariableCore,
    pub variable_role: Option<SdtmProjectRole>,
    pub descriptions: Vec<SdtmProjectVariableDescription>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmProjectVariableOverrideRequest {
    pub variable_type: Option<SdtmProjectVariableType>,
    pub variable_core: Option<SdtmProjectVariableCore>,
    /// Three-state: absent = unchanged, `null` = clear, value =
    /// replace. See [`double_option`].
    #[serde(default, deserialize_with = "double_option")]
    pub variable_role: Option<Option<SdtmProjectRole>>,
    pub descriptions: Option<Vec<SdtmProjectVariableDescription>>,
}

/// Query for `GET /api/sdtm-project/versions`. The parameter is
/// required: versions are always scoped to a project.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVersionListQuery {
    pub project_code: String,
}

// -- responses -------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub version_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectDomainViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub domain_code: String,
    /// The variable codes, in the order the caller supplied them.
    pub key_variables: Vec<String>,
    /// The domain's one override, when it has one. The wire key is
    /// `override`, so the field is renamed on the way out.
    #[serde(rename = "override")]
    pub domain_override: Option<SdtmProjectDomainOverrideViewResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectDomainOverrideViewResponse {
    pub id: i64,
    pub domain_id: i64,
    pub category: SdtmProjectDomainCategory,
    pub descriptions: Vec<SdtmProjectDomainDescription>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// The body of `GET /domains/{domain_id}/override`.
///
/// The override is optional, so the response is always an object with
/// an `override` key: `null` when the domain has no override, the
/// override itself when it has one. The key is present either way, so
/// a client can read `body.override` without a null check on the
/// envelope.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectDomainOverrideResponse {
    #[serde(rename = "override")]
    pub domain_override: Option<SdtmProjectDomainOverrideViewResponse>,
}

/// The body of `GET /variables/{variable_id}/override` — the variable
/// twin of [`SdtmProjectDomainOverrideResponse`].
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVariableOverrideResponse {
    #[serde(rename = "override")]
    pub variable_override: Option<SdtmProjectVariableOverrideViewResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVariableViewResponse {
    pub id: i64,
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
    #[serde(rename = "override")]
    pub variable_override: Option<SdtmProjectVariableOverrideViewResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVariableOverrideViewResponse {
    pub id: i64,
    pub variable_id: i64,
    pub variable_type: SdtmProjectVariableType,
    pub variable_core: SdtmProjectVariableCore,
    pub variable_role: Option<SdtmProjectRole>,
    pub descriptions: Vec<SdtmProjectVariableDescription>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVersionListResponse {
    pub versions: Vec<SdtmProjectVersionViewResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectDomainListResponse {
    pub domains: Vec<SdtmProjectDomainViewResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmProjectVariableListResponse {
    pub variables: Vec<SdtmProjectVariableViewResponse>,
}

impl From<apis::sdtm_project::SdtmProjectVersionView> for SdtmProjectVersionViewResponse {
    fn from(v: apis::sdtm_project::SdtmProjectVersionView) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            version_name: v.version_name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<apis::sdtm_project::SdtmProjectDomainView> for SdtmProjectDomainViewResponse {
    fn from(v: apis::sdtm_project::SdtmProjectDomainView) -> Self {
        Self {
            id: v.id,
            version_id: v.version_id,
            domain_code: v.domain_code,
            key_variables: v.key_variables,
            domain_override: v.domain_override.map(Into::into),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<apis::sdtm_project::SdtmProjectDomainOverrideView>
    for SdtmProjectDomainOverrideViewResponse
{
    fn from(v: apis::sdtm_project::SdtmProjectDomainOverrideView) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            category: v.category.into(),
            descriptions: v.descriptions.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<apis::sdtm_project::SdtmProjectVariableView> for SdtmProjectVariableViewResponse {
    fn from(v: apis::sdtm_project::SdtmProjectVariableView) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            variable_code: v.variable_code,
            variable_sequence: v.variable_sequence,
            variable_override: v.variable_override.map(Into::into),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<apis::sdtm_project::SdtmProjectVariableOverrideView>
    for SdtmProjectVariableOverrideViewResponse
{
    fn from(v: apis::sdtm_project::SdtmProjectVariableOverrideView) -> Self {
        Self {
            id: v.id,
            variable_id: v.variable_id,
            variable_type: v.variable_type.into(),
            variable_core: v.variable_core.into(),
            variable_role: v.variable_role.map(Into::into),
            descriptions: v.descriptions.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    /// The nullable `variableRole` update field must distinguish all
    /// three states over the wire: a missing key means "leave it
    /// alone", an explicit `null` means "clear it", and a value means
    /// "replace it". A plain `Option<Option<T>>` derive collapses
    /// `null` and "missing" into the same value, which makes the
    /// clear state unreachable from a client.
    #[test]
    fn sdtm_project_variable_override_update_distinguishes_absent_null_and_value() {
        let absent: UpdateSdtmProjectVariableOverrideRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(
            absent.variable_role, None,
            "missing key must mean 'unchanged'"
        );

        let null: UpdateSdtmProjectVariableOverrideRequest =
            serde_json::from_str(r#"{"variableRole":null}"#).unwrap();
        assert_eq!(
            null.variable_role,
            Some(None),
            "explicit null must mean 'clear the role'"
        );

        let value: UpdateSdtmProjectVariableOverrideRequest =
            serde_json::from_str(r#"{"variableRole":"Topic"}"#).unwrap();
        assert_eq!(
            value.variable_role,
            Some(Some(SdtmProjectRole::Topic)),
            "a value must mean 'replace the role'"
        );
    }

    /// The Rust field is `domain_override`, but the wire key is
    /// `override` — the explicit `#[serde(rename)]` wins over the
    /// struct's `camelCase` rule, so `domainOverride` must never
    /// appear. The nested override must serialise as `"override"`, and
    /// the key variables must keep the caller's order.
    #[test]
    fn sdtm_project_domain_view_serialises_the_override_and_key_order() {
        let ts = chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let view = SdtmProjectDomainViewResponse {
            id: 7,
            version_id: 3,
            domain_code: "AE".into(),
            key_variables: vec!["AESEV".into(), "AETERM".into()],
            domain_override: Some(SdtmProjectDomainOverrideViewResponse {
                id: 1,
                domain_id: 7,
                category: SdtmProjectDomainCategory::Events,
                descriptions: Vec::new(),
                created_at: ts,
                updated_at: ts,
            }),
            created_at: ts,
            updated_at: ts,
        };
        let json = serde_json::to_value(&view).unwrap();
        assert!(
            json.get("override").is_some(),
            "missing `override` key: {json}"
        );
        assert!(
            json.get("domainOverride").is_none(),
            "leaked `domainOverride` key: {json}"
        );
        assert_eq!(json["override"]["category"], serde_json::json!("Events"));
        assert_eq!(json["override"]["domainId"], serde_json::json!(7));
        // The caller's order survives the wire.
        assert_eq!(json["keyVariables"], serde_json::json!(["AESEV", "AETERM"]));
    }

    /// The variable view is the twin of the domain view: the Rust field
    /// is `variable_override`, the wire key is `override`.
    #[test]
    fn sdtm_project_variable_view_serialises_the_override_under_the_override_key() {
        let ts = chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let view = SdtmProjectVariableViewResponse {
            id: 8,
            domain_id: 7,
            variable_code: "AETERM".into(),
            variable_sequence: 11,
            variable_override: Some(SdtmProjectVariableOverrideViewResponse {
                id: 2,
                variable_id: 8,
                variable_type: SdtmProjectVariableType::Character,
                variable_core: SdtmProjectVariableCore::Req,
                variable_role: None,
                descriptions: Vec::new(),
                created_at: ts,
                updated_at: ts,
            }),
            created_at: ts,
            updated_at: ts,
        };
        let json = serde_json::to_value(&view).unwrap();
        assert!(
            json.get("override").is_some(),
            "missing `override` key: {json}"
        );
        assert!(
            json.get("variableOverride").is_none(),
            "leaked `variableOverride` key: {json}"
        );
        assert_eq!(
            json["override"]["variableType"],
            serde_json::json!("Character")
        );
        assert_eq!(json["override"]["variableId"], serde_json::json!(8));
    }

    /// The `GET …/override` envelope always carries an `override` key:
    /// `null` when there is no override, the override when there is
    /// one. It is never a bare `null` body.
    #[test]
    fn sdtm_project_override_response_wraps_the_override_in_an_override_key() {
        let absent = SdtmProjectDomainOverrideResponse {
            domain_override: None,
        };
        let json = serde_json::to_value(&absent).unwrap();
        assert_eq!(json, serde_json::json!({ "override": null }));

        let ts = chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let present = SdtmProjectDomainOverrideResponse {
            domain_override: Some(SdtmProjectDomainOverrideViewResponse {
                id: 1,
                domain_id: 7,
                category: SdtmProjectDomainCategory::Events,
                descriptions: Vec::new(),
                created_at: ts,
                updated_at: ts,
            }),
        };
        let json = serde_json::to_value(&present).unwrap();
        assert_eq!(json["override"]["category"], serde_json::json!("Events"));
        assert_eq!(json["override"]["domainId"], serde_json::json!(7));
        assert!(
            json.get("domainOverride").is_none(),
            "leaked `domainOverride` key: {json}"
        );

        let absent = SdtmProjectVariableOverrideResponse {
            variable_override: None,
        };
        let json = serde_json::to_value(&absent).unwrap();
        assert_eq!(json, serde_json::json!({ "override": null }));
    }

    use super::*;

    #[test]
    fn login_request_roundtrip() {
        let json = r#"{"code":"u1","password":"p"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.password, "p");
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn login_domain_request_roundtrip() {
        let json = r#"{"code":"u1","domain_name":"d","hostname":"h","sid":"s"}"#;
        let req: LoginDomainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.domain_name, "d");
        assert_eq!(req.hostname, "h");
        assert_eq!(req.sid, "s");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn refresh_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn logout_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn token_pair_response_roundtrip() {
        let json = r#"{"access_token":"a","refresh_token":"r"}"#;
        let res: TokenPairResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
        assert_eq!(res.refresh_token, "r");
    }

    #[test]
    fn access_token_response_roundtrip() {
        let json = r#"{"access_token":"a"}"#;
        let res: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
    }

    #[test]
    fn logout_response_roundtrip() {
        let res: LogoutResponse = serde_json::from_str("{}").unwrap();
        let out = serde_json::to_string(&res).unwrap();
        assert_eq!(out, "{}");
    }

    // ---- log-ingest DTO round-trips (new) -----

    #[test]
    fn log_ingest_request_roundtrip() {
        let json = r#"{"batchId":"b-1","entries":["a","b","c"]}"#;
        let req: LogIngestRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.batch_id, "b-1");
        assert_eq!(req.entries, vec!["a", "b", "c"]);
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn log_ingest_response_roundtrip() {
        let json = r#"{"batchId":"b-1","accepted":3}"#;
        let resp: LogIngestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.batch_id, "b-1");
        assert_eq!(resp.accepted, 3);
        let out = serde_json::to_string(&resp).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn auth_claims_response_roundtrip() {
        let json = r#"{"code":"u1","role":"admin","token_version":7}"#;
        let res: AuthClaimsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.code, "u1");
        assert!(matches!(res.role, Role::Admin));
        assert_eq!(res.token_version, 7);
        assert_eq!(serde_json::to_string(&res).unwrap(), json);
    }

    #[test]
    fn role_round_trip_all_variants() {
        for r in [Role::Root, Role::Admin, Role::General] {
            let s = serde_json::to_string(&r).unwrap();
            let back: Role = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{r:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn role_from_apis_role_all_variants() {
        assert!(matches!(Role::from(apis::user::Role::Root), Role::Root));
        assert!(matches!(Role::from(apis::user::Role::Admin), Role::Admin));
        assert!(matches!(
            Role::from(apis::user::Role::General),
            Role::General
        ));
    }

    // ---- user DTO round-trips (new) -----

    #[test]
    fn create_user_request_roundtrip() {
        let json = r#"{"code":"u1","name":"Alice","role":"admin"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.name, "Alice");
        assert!(matches!(req.role, Role::Admin));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_user_request_partial_roundtrip() {
        let json = r#"{"name":"Alice"}"#;
        let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.code.is_none());
        assert_eq!(req.name.as_deref(), Some("Alice"));
        assert!(req.role.is_none());
        assert!(req.active.is_none());
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn update_user_request_full_roundtrip() {
        let json = r#"{"code":"u2","name":"Bob","role":"root","active":true}"#;
        let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code.as_deref(), Some("u2"));
        assert_eq!(req.name.as_deref(), Some("Bob"));
        assert!(matches!(req.role, Some(Role::Root)));
        assert_eq!(req.active, Some(true));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn path_code_roundtrip() {
        let json = r#"{"code":"u1"}"#;
        let p: PathCode = serde_json::from_str(json).unwrap();
        assert_eq!(p.code, "u1");
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
    }

    #[test]
    fn user_view_response_roundtrip() {
        let json = r#"{"id":42,"code":"u1","name":"Alice","role":"admin","active":true,"created_at":"2026-01-02T03:04:05Z","updated_at":"2026-01-02T03:04:05Z"}"#;
        let v: UserViewResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.id, 42);
        assert_eq!(v.code, "u1");
        assert_eq!(v.name, "Alice");
        assert!(matches!(v.role, Role::Admin));
        assert!(v.active);
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_list_response_roundtrip() {
        let json = r#"{"users":[{"id":1,"code":"u1","name":"A","role":"admin","active":true,"created_at":"2026-01-02T03:04:05Z","updated_at":"2026-01-02T03:04:05Z"}]}"#;
        let v: UserListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.users.len(), 1);
        assert_eq!(v.users[0].code, "u1");
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_view_response_from_apis_user_view() {
        let apis_view = apis::user::UserView {
            id: 7,
            code: "u7".into(),
            name: "Seven".into(),
            role: apis::user::Role::General,
            active: false,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let resp: UserViewResponse = apis_view.into();
        assert_eq!(resp.id, 7);
        assert_eq!(resp.code, "u7");
        assert_eq!(resp.name, "Seven");
        assert!(matches!(resp.role, Role::General));
        assert!(!resp.active);
    }

    // ---- user-credential DTO round-trips (new) -----

    #[test]
    fn update_user_credential_request_partial_roundtrip() {
        // An empty update body must round-trip losslessly — the
        // handler reads `user_code` from `AuthClaims`, and an
        // absent `password` means "no change".
        let json = r#"{}"#;
        let req: UpdateUserCredentialRequest = serde_json::from_str(json).unwrap();
        assert!(req.password.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_user_credential_request_full_roundtrip() {
        let json = r#"{"password":"hunter2"}"#;
        let req: UpdateUserCredentialRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.password.as_deref(), Some("hunter2"));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn user_credential_view_response_roundtrip() {
        // The response carries the *hashed* password (Argon2 in the
        // canonical backend) — never the plaintext.
        let json = r#"{"user_code":"u1","password_hash":"argon2id$...","token_version":7}"#;
        let v: UserCredentialViewResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.user_code, "u1");
        assert_eq!(v.password_hash, "argon2id$...");
        assert_eq!(v.token_version, 7);
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_credential_view_response_from_apis_view() {
        let apis_view = apis::auth::UserCredentialView {
            user_code: "u7".into(),
            password_hash: "argon2id$...".into(),
            token_version: 5,
        };
        let resp: UserCredentialViewResponse = apis_view.into();
        assert_eq!(resp.user_code, "u7");
        assert_eq!(resp.password_hash, "argon2id$...");
        assert_eq!(resp.token_version, 5);
    }

    #[test]
    fn register_user_request_roundtrip() {
        let json = r#"{"user_code":"u1","user_name":"Alice","domain_name":"d","hostname":"h","sid":"s","password":"p"}"#;
        let req: RegisterUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_code, "u1");
        assert_eq!(req.password, "p");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn register_user_response_from_apis_view() {
        let apis_view = apis::auth::RegisterUserResponse {
            user_code: "u1".into(),
            user_name: "Alice".into(),
            role: apis::user::Role::Admin,
            active: true,
            domain_name: "d".into(),
            hostname: "h".into(),
            sid: "s".into(),
        };
        let resp: RegisterUserResponse = apis_view.into();
        assert_eq!(resp.user_code, "u1");
        assert!(matches!(resp.role, Role::Admin));
        assert!(resp.active);
        assert_eq!(resp.domain_name, "d");
    }

    // ---- project DTO round-trips -----

    fn sample_project_view() -> apis::project::ProjectView {
        apis::project::ProjectView {
            id: 2,
            code: "project-1".into(),
            description: "alpha".into(),
            members: apis::project::ProjectMemberView {
                leaders: vec![apis::project::UserSummaryView {
                    code: "leader-1".into(),
                    name: "Leader One".into(),
                }],
                workers: vec![],
            },
            unblind_members: apis::project::ProjectMemberView {
                leaders: vec![],
                workers: vec![apis::project::UserSummaryView {
                    code: "worker-2".into(),
                    name: "Worker Two".into(),
                }],
            },
            configurations: apis::project::ProjectConfigurationView {
                language: Some(apis::project::ProjectLanguage::English),
                tags: vec![
                    apis::project::TagView {
                        key: "Product".into(),
                        value: "DEMO-001".into(),
                    },
                    apis::project::TagView {
                        key: "Region".into(),
                        value: "EU".into(),
                    },
                ],
                sdtmig: None,
                sdtm_terminology: None,
            },
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn create_project_request_minimal_roundtrip() {
        let json = r#"{"code":"pr1","description":"x"}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "pr1");
        assert_eq!(req.description, "x");
        assert!(req.members.is_none());
        assert!(req.unblind_members.is_none());
        assert!(req.configurations.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_project_request_with_empty_members_roundtrip() {
        let json = r#"{"code":"pr1","description":"x","members":{},"unblindMembers":{}}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        let members = req.members.as_ref().expect("members present");
        assert!(members.leaders.is_empty());
        assert!(members.workers.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_project_request_with_configuration_roundtrip() {
        let json = r#"{"code":"pr1","description":"x","configurations":{"language":"en","tags":[{"key":"Product","value":"DEMO-001"}]}}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        let configurations = req.configurations.as_ref().expect("configurations present");
        assert!(matches!(
            configurations.language,
            Some(ProjectLanguage::English)
        ));
        assert_eq!(configurations.tags.len(), 1);
        assert_eq!(configurations.tags[0].key, "Product");
        assert_eq!(configurations.tags[0].value, "DEMO-001");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_omitted_membership_keeps_none() {
        let json = r#"{"description":"new"}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        assert!(req.members.is_none());
        assert!(req.unblind_members.is_none());
        assert!(req.configurations.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_empty_membership_becomes_some_empty() {
        let json = r#"{"members":{},"unblindMembers":{}}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        let members = req.members.as_ref().expect("members present");
        assert!(members.leaders.is_empty());
        assert!(members.workers.is_empty());
        let unblind = req
            .unblind_members
            .as_ref()
            .expect("unblind members present");
        assert!(unblind.leaders.is_empty());
        assert!(unblind.workers.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_partial_with_membership_roundtrip() {
        let json = r#"{"description":"y","active":false,"members":{"leaders":["l1"]}}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description.as_deref(), Some("y"));
        assert_eq!(req.active, Some(false));
        let members = req.members.as_ref().expect("members present");
        assert_eq!(members.leaders, vec!["l1".to_string()]);
        assert!(members.workers.is_empty());
        assert!(req.unblind_members.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_empty_configuration_becomes_some_empty() {
        let json = r#"{"configurations":{}}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        let configurations = req.configurations.as_ref().expect("configurations present");
        assert!(configurations.language.is_none());
        assert!(configurations.tags.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn tag_view_response_from_apis_view() {
        let view = apis::project::TagView {
            key: "Product".into(),
            value: "DEMO-001".into(),
        };
        let resp: TagViewResponse = view.into();
        assert_eq!(resp.key, "Product");
        assert_eq!(resp.value, "DEMO-001");
    }

    #[test]
    fn user_summary_view_response_from_apis_view() {
        let view = apis::project::UserSummaryView {
            code: "u1".into(),
            name: "Alice".into(),
        };
        let resp: UserSummaryViewResponse = view.into();
        assert_eq!(resp.code, "u1");
        assert_eq!(resp.name, "Alice");
    }

    #[test]
    fn project_member_view_response_from_apis_view() {
        let view = apis::project::ProjectMemberView {
            leaders: vec![apis::project::UserSummaryView {
                code: "l1".into(),
                name: "Leader".into(),
            }],
            workers: vec![apis::project::UserSummaryView {
                code: "w1".into(),
                name: "Worker".into(),
            }],
        };
        let resp: ProjectMemberViewResponse = view.into();
        assert_eq!(resp.leaders.len(), 1);
        assert_eq!(resp.leaders[0].code, "l1");
        assert_eq!(resp.workers.len(), 1);
        assert_eq!(resp.workers[0].code, "w1");
    }

    #[test]
    fn project_view_response_from_apis_view() {
        let view = sample_project_view();
        let resp: ProjectViewResponse = view.into();
        assert_eq!(resp.id, 2);
        assert_eq!(resp.code, "project-1");
        assert_eq!(resp.members.leaders.len(), 1);
        assert_eq!(resp.members.leaders[0].code, "leader-1");
        assert!(resp.members.workers.is_empty());
        assert!(resp.unblind_members.leaders.is_empty());
        assert_eq!(resp.unblind_members.workers.len(), 1);
        assert_eq!(resp.unblind_members.workers[0].code, "worker-2");
        assert!(matches!(
            resp.configurations.language,
            Some(ProjectLanguage::English)
        ));
        assert_eq!(resp.configurations.tags.len(), 2);
        assert_eq!(resp.configurations.tags[0].key, "Product");
        assert_eq!(resp.configurations.tags[1].value, "EU");
    }

    #[test]
    fn project_list_response_roundtrip() {
        let json = r#"{"projects":[]}"#;
        let resp: ProjectListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.projects.is_empty());
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    // ---- terminology DTO round-trips -----

    fn sample_terminology_version_view() -> apis::terminology::TerminologyVersionView {
        apis::terminology::TerminologyVersionView {
            id: 1,
            kind: apis::terminology::TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_code_list_view() -> apis::terminology::CodeListView {
        apis::terminology::CodeListView {
            id: 11,
            version_id: 1,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: String::new(),
            definition: String::new(),
            nci_preferred_term: "Age".into(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_code_item_view() -> apis::terminology::CodeItemView {
        apis::terminology::CodeItemView {
            id: 100,
            codelist_id: 11,
            version_id: 1,
            code: "C1".into(),
            submission_value: "Y".into(),
            synonym: String::new(),
            definition: String::new(),
            nci_preferred_term: "Yes".into(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn terminology_kind_round_trip_all_variants() {
        for k in [TerminologyKind::Sdtm, TerminologyKind::Adam] {
            let s = serde_json::to_string(&k).unwrap();
            let back: TerminologyKind = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{k:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn terminology_kind_from_apis_all_variants() {
        assert!(matches!(
            TerminologyKind::from(apis::terminology::TerminologyKind::Sdtm),
            TerminologyKind::Sdtm
        ));
        assert!(matches!(
            TerminologyKind::from(apis::terminology::TerminologyKind::Adam),
            TerminologyKind::Adam
        ));
    }

    #[test]
    fn path_id_roundtrip() {
        let json = r#"{"id":42}"#;
        let p: PathId = serde_json::from_str(json).unwrap();
        assert_eq!(p.id, 42);
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
    }

    #[test]
    fn terminology_version_view_response_from_apis_view() {
        let resp: TerminologyVersionViewResponse = sample_terminology_version_view().into();
        assert_eq!(resp.id, 1);
        assert!(matches!(resp.kind, TerminologyKind::Sdtm));
        assert_eq!(resp.name, "2026-03-27");
    }

    #[test]
    fn terminology_version_list_response_roundtrip() {
        let json = r#"{"versions":[]}"#;
        let resp: TerminologyVersionListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.versions.is_empty());
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn code_list_view_response_from_apis_view() {
        let resp: CodeListViewResponse = sample_code_list_view().into();
        assert_eq!(resp.id, 11);
        assert_eq!(resp.version_id, 1);
        assert_eq!(resp.code, "C66741");
        assert!(resp.extensible);
        assert_eq!(resp.name, "AGE");
    }

    #[test]
    fn paged_code_list_list_response_roundtrip() {
        let json = r#"{"items":[]}"#;
        let resp: PagedCodeListListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
        assert_eq!(resp.next_offset, None);
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_list_list_response_with_next_offset_roundtrip() {
        let json = r#"{"items":[],"nextOffset":50}"#;
        let resp: PagedCodeListListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_offset, Some(50));
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_list_list_response_from_apis_page() {
        let page = apis::terminology::Page {
            items: vec![sample_code_list_view()],
            next_offset: Some(2),
        };
        let resp: PagedCodeListListResponse = page.into();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "C66741");
        assert_eq!(resp.next_offset, Some(2));
    }

    #[test]
    fn code_item_view_response_from_apis_view() {
        let resp: CodeItemViewResponse = sample_code_item_view().into();
        assert_eq!(resp.id, 100);
        assert_eq!(resp.codelist_id, 11);
        assert_eq!(resp.version_id, 1);
        assert_eq!(resp.code, "C1");
        assert_eq!(resp.nci_preferred_term, "Yes");
    }

    #[test]
    fn paged_code_item_list_response_roundtrip() {
        let json = r#"{"items":[]}"#;
        let resp: PagedCodeItemListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
        assert_eq!(resp.next_offset, None);
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_item_list_response_from_apis_page() {
        let page = apis::terminology::Page {
            items: vec![sample_code_item_view()],
            next_offset: None,
        };
        let resp: PagedCodeItemListResponse = page.into();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "C1");
        assert_eq!(resp.next_offset, None);
    }

    #[test]
    fn create_terminology_version_request_roundtrip() {
        let json = r#"{"kind":"sdtm","name":"2026-03-27"}"#;
        let req: CreateTerminologyVersionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req.kind, TerminologyKind::Sdtm));
        assert_eq!(req.name, "2026-03-27");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_terminology_version_request_partial_roundtrip() {
        let json = r#"{"name":"new"}"#;
        let req: UpdateTerminologyVersionRequest = serde_json::from_str(json).unwrap();
        assert!(req.kind.is_none());
        assert_eq!(req.name.as_deref(), Some("new"));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_code_list_request_roundtrip() {
        let json = r#"{"versionId":1,"code":"C66741","extensible":true,"name":"AGE","submissionValue":"AGE","synonym":"","definition":"","nciPreferredTerm":"Age"}"#;
        let req: CreateCodeListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.version_id, 1);
        assert_eq!(req.code, "C66741");
        assert!(req.extensible);
        assert_eq!(req.nci_preferred_term, "Age");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_code_list_request_partial_roundtrip() {
        let json = r#"{"name":"new"}"#;
        let req: UpdateCodeListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("new"));
        assert!(req.code.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_code_item_request_roundtrip() {
        let json = r#"{"codelistId":11,"versionId":1,"code":"C1","submissionValue":"Y","synonym":"","definition":"","nciPreferredTerm":"Yes"}"#;
        let req: CreateCodeItemRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.codelist_id, 11);
        assert_eq!(req.version_id, 1);
        assert_eq!(req.code, "C1");
        assert_eq!(req.nci_preferred_term, "Yes");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_code_item_request_partial_roundtrip() {
        let json = r#"{"code":"C2"}"#;
        let req: UpdateCodeItemRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code.as_deref(), Some("C2"));
        assert!(req.synonym.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn code_list_list_query_with_fragment_roundtrip() {
        let json = r#"{"versionId":1,"fragment":"age","offset":0,"limit":50}"#;
        let q: CodeListListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert_eq!(q.fragment.as_deref(), Some("age"));
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 50);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn code_list_list_query_minimal_roundtrip() {
        let json = r#"{"versionId":1}"#;
        let q: CodeListListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert!(q.fragment.is_none());
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 0);
        // Optional fragment is omitted from the serialized form so
        // `?versionId=1` stays minimal; numeric `offset`/`limit`
        // serialize as `0` rather than being skipped.
        assert_eq!(
            serde_json::to_string(&q).unwrap(),
            r#"{"versionId":1,"offset":0,"limit":0}"#
        );
    }

    #[test]
    fn code_item_list_query_with_fragment_roundtrip() {
        let json = r#"{"codelistId":11,"fragment":"yes","offset":50,"limit":25}"#;
        let q: CodeItemListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.codelist_id, Some(11));
        assert_eq!(q.fragment.as_deref(), Some("yes"));
        assert_eq!(q.offset, 50);
        assert_eq!(q.limit, 25);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn code_item_list_query_without_codelist_id_roundtrip() {
        // Omitting `codelistId` (or sending `null`) must default to
        // `None` and serialize back without the key, preserving
        // the optional semantics on the wire.
        let json = r#"{"fragment":"x","offset":0,"limit":50}"#;
        let q: CodeItemListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.codelist_id, None);
        assert_eq!(q.fragment.as_deref(), Some("x"));
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 50);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);

        // Explicit `null` is also accepted.
        let q_null: CodeItemListQuery =
            serde_json::from_str(r#"{"codelistId":null,"offset":0,"limit":50}"#).unwrap();
        assert_eq!(q_null.codelist_id, None);
    }

    #[test]
    fn code_item_by_version_and_code_query_roundtrip() {
        let json = r#"{"versionId":1,"code":"C1"}"#;
        let q: CodeItemByVersionAndCodeQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert_eq!(q.code, "C1");
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn batch_create_code_items_request_roundtrip() {
        let json = r#"{"codelistId":11,"versionId":1,"items":[{"code":"C1","submissionValue":"Y","synonym":"","definition":"","nciPreferredTerm":"Yes"}]}"#;
        let req: BatchCreateCodeItemsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.codelist_id, 11);
        assert_eq!(req.version_id, 1);
        assert_eq!(req.items.len(), 1);
        assert_eq!(req.items[0].code, "C1");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn batch_create_code_items_response_roundtrip() {
        let json = r#"{"count":3,"codelistId":11,"versionId":1}"#;
        let resp: BatchCreateCodeItemsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 3);
        assert_eq!(resp.codelist_id, 11);
        assert_eq!(resp.version_id, 1);
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    // ---- mission issue DTO round-trips (new) -----

    #[test]
    fn issue_state_round_trip_all_variants() {
        for s in [IssueState::Opened, IssueState::Closed] {
            let json = serde_json::to_string(&s).unwrap();
            let back: IssueState = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{s:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn issue_state_from_apis_state_all_variants() {
        assert!(matches!(
            IssueState::from(apis::mission::IssueState::Opened),
            IssueState::Opened
        ));
        assert!(matches!(
            IssueState::from(apis::mission::IssueState::Closed),
            IssueState::Closed
        ));
    }

    #[test]
    fn create_issue_request_roundtrip() {
        let json = r#"{"targetItem":"dm.x","description":"d"}"#;
        let req: CreateIssueRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.target_item.as_deref(), Some("dm.x"));
        assert_eq!(req.description, "d");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_issue_request_omits_target_item_when_none() {
        let req = CreateIssueRequest {
            target_item: None,
            description: "d".into(),
        };
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, r#"{"description":"d"}"#);
    }

    #[test]
    fn update_issue_description_request_roundtrip() {
        let json = r#"{"description":"d"}"#;
        let req: UpdateIssueDescriptionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description, "d");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn append_comment_request_roundtrip() {
        let json = r#"{"content":"c"}"#;
        let req: AppendCommentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "c");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn issue_list_query_round_trips_state() {
        let json = r#"{"state":"closed"}"#;
        let q: IssueListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.state.as_deref(), Some("closed"));
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn issue_list_query_omits_state_when_none() {
        let q = IssueListQuery::default();
        assert_eq!(serde_json::to_string(&q).unwrap(), "{}");
    }

    #[test]
    fn issue_view_response_roundtrip() {
        let json = r#"{"id":1,"missionId":1,"targetItem":"dm.x","issuer":"u1","description":"d","state":"opened","comments":[{"user":"u2","content":"c","createdAt":"2026-01-02T03:04:05Z"}],"createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-02T03:04:05Z"}"#;
        let v: IssueViewResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.id, 1);
        assert_eq!(v.mission_id, 1);
        assert_eq!(v.target_item.as_deref(), Some("dm.x"));
        assert_eq!(v.issuer, "u1");
        assert_eq!(v.description, "d");
        assert!(matches!(v.state, IssueState::Opened));
        assert_eq!(v.comments.len(), 1);
        assert_eq!(v.comments[0].user, "u2");
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn issue_view_response_omits_target_item_when_none() {
        let json = r#"{"id":1,"missionId":1,"issuer":"u1","description":"d","state":"closed","comments":[],"createdAt":"2026-01-02T03:04:05Z","updatedAt":"2026-01-02T03:04:05Z"}"#;
        let v: IssueViewResponse = serde_json::from_str(json).unwrap();
        assert!(v.target_item.is_none());
        assert!(matches!(v.state, IssueState::Closed));
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn issue_view_response_from_apis_issue_view() {
        let apis_view = apis::mission::IssueView {
            id: 7,
            mission_id: 1,
            target_item: None,
            issuer: "u7".into(),
            description: "d".into(),
            state: apis::mission::IssueState::Closed,
            comments: vec![apis::mission::IssueCommentView {
                user: "u9".into(),
                content: "c".into(),
                created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            }],
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let resp: IssueViewResponse = apis_view.into();
        assert_eq!(resp.id, 7);
        assert!(resp.target_item.is_none());
        assert!(matches!(resp.state, IssueState::Closed));
        assert_eq!(resp.comments.len(), 1);
        assert_eq!(resp.comments[0].user, "u9");
    }

    #[test]
    fn issue_list_response_roundtrip() {
        let json = r#"{"issues":[]}"#;
        let r: IssueListResponse = serde_json::from_str(json).unwrap();
        assert!(r.issues.is_empty());
        assert_eq!(serde_json::to_string(&r).unwrap(), json);
    }

    /// The server accepts only the structured shape for
    /// `variableControlled`. The retired string shape must be a
    /// deserialization error rather than a silently coerced value.
    #[test]
    fn sdtm_variable_controlled_rejects_the_legacy_string_shape() {
        let legacy = r#"{"domainId":1,"name":"AESEV","variableControlled":"C66731","variableType":"Character","variableCore":"Req","variableSequence":1,"descriptions":[]}"#;
        let err = serde_json::from_str::<CreateSdtmVariableRequest>(legacy);
        assert!(
            err.is_err(),
            "the legacy string shape must not deserialize into the object type"
        );
    }

    /// The structured shape deserializes, and both lists are optional.
    #[test]
    fn sdtm_variable_controlled_accepts_the_object_shape() {
        let body = r#"{"domainId":1,"name":"AESEV","variableControlled":{"code":["C66731"]},"variableType":"Character","variableCore":"Req","variableSequence":1,"descriptions":[]}"#;
        let req: CreateSdtmVariableRequest = serde_json::from_str(body).unwrap();
        assert_eq!(
            req.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["C66731".into()]),
                format: None,
            })
        );
    }

    /// The update field must keep all three states distinguishable over
    /// the wire: a missing key is "leave the column alone", an explicit
    /// `null` is "clear the column", and a value is "replace the
    /// column". A plain `Option<Option<T>>` derive collapses the first
    /// two — serde maps `null` to the outer `None` — so the field uses
    /// the `double_option` deserializer.
    #[test]
    fn sdtm_variable_update_distinguishes_absent_null_and_value() {
        let absent: UpdateSdtmVariableRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(
            absent.variable_controlled, None,
            "a missing key must mean 'unchanged'"
        );

        let null: UpdateSdtmVariableRequest =
            serde_json::from_str(r#"{"variableControlled":null}"#).unwrap();
        assert_eq!(
            null.variable_controlled,
            Some(None),
            "an explicit null must mean 'clear the controlled value'"
        );

        let value: UpdateSdtmVariableRequest =
            serde_json::from_str(r#"{"variableControlled":{"code":["C66731"]}}"#).unwrap();
        assert_eq!(
            value.variable_controlled,
            Some(Some(SdtmVariableControlled {
                code: Some(vec!["C66731".into()]),
                format: None,
            })),
            "a value must mean 'replace the controlled value'"
        );
    }
}
