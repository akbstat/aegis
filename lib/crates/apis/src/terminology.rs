//! Outbound port for the terminology service.
//!
//! Mirrors the surface of `terminology::TerminologyUsecase` so
//! adapters in any backend (in-memory, PostgreSQL, …) can adapt
//! their own types to the shared contract defined here. All
//! supporting DTOs (request shapes, view projections, search
//! queries, and [`TerminologyApiError`]) live alongside the trait
//! so a single `use apis::terminology::*;` brings the whole
//! contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Terminology flavour (e.g. SDTM, ADaM CDISC standards).
///
/// Mirrors `terminology::TerminologyKind` so backends can convert
/// losslessly; the two enums are kept in sync layer by layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminologyKind {
    Sdtm,
    Adam,
}

/// Error surface returned by every [`TerminologyService`] method.
///
/// Adapters translate backend-specific errors (e.g.
/// `terminology::UsecaseError`) into this type at the implementation
/// boundary. The shape intentionally combines validation, lookup,
/// duplicate, and infrastructure concerns into a single type so
/// handlers can match exhaustively.
#[derive(Debug, Clone, Error)]
pub enum TerminologyApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("terminology version already exists for {kind:?} / {name}")]
    DuplicateVersion { kind: TerminologyKind, name: String },

    #[error("code list already exists for version {version_id} / {code}")]
    DuplicateCodeList { version_id: i64, code: String },

    #[error("code item already exists for codelist {codelist_id} / {code}")]
    DuplicateCodeItem { codelist_id: i64, code: String },

    #[error("repository error: {0}")]
    Repository(String),
}

// ---- view projections ----

/// Safe projection of a `TerminologyVersion` aggregate — exactly
/// what adapters hand back to whatever consumes the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminologyVersionView {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `CodeList` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeListView {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `CodeItem` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeItemView {
    pub id: i64,
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- pagination envelope ----

/// One page of a paginated result set. Mirrors
/// `terminology::domain::Page<T>` field-for-field so the two
/// layers can `From`-convert without ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<u32>,
}

/// Query for [`TerminologyService::list_code_lists`]. Unified list
/// + search under a single signature: `fragment = None` is a plain
///   `ORDER BY id ASC` list; `fragment = Some(_)` is a FTS query with
///   `ts_rank DESC, id ASC` ordering.
///
/// The fragment is free text: it is split into terms, every term is
/// prefix-matched, and all of them have to match — `"vital sign"`
/// matches the rows whose text contains both `vital` and `sign`.
#[derive(Debug, Clone)]
pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

/// Query for [`TerminologyService::list_code_items`]. Mirrors
/// [`CodeListListQuery`] but scopes to a `codelist_id`. Both
/// `version_id` and `codelist_id` are optional: `Some(_)`
/// restricts to a single owning version / codelist, `None`
/// lists every code item known to the backend.
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    /// Optional owning version id. `Some(_)` restricts the
    /// result set to code items whose `version_id` matches;
    /// `None` lists every code item the backend knows about
    /// (optionally further narrowed by `codelist_id`).
    pub version_id: Option<i64>,
    pub codelist_id: Option<i64>,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

// ---- request DTOs ----

/// Input DTO for [`TerminologyService::create_version`].
#[derive(Debug, Clone)]
pub struct CreateTerminologyVersionRequest {
    pub kind: TerminologyKind,
    pub name: String,
}

/// Input DTO for [`TerminologyService::update_version`]. Every
/// field except `id` is optional; only the fields that actually
/// changed need to be supplied.
#[derive(Debug, Default, Clone)]
pub struct UpdateTerminologyVersionRequest {
    pub id: i64,
    pub kind: Option<TerminologyKind>,
    pub name: Option<String>,
}

/// Input DTO for [`TerminologyService::create_code_list`].
#[derive(Debug, Clone)]
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

/// Input DTO for [`TerminologyService::update_code_list`]. Every
/// field except `id` is optional; only the fields that actually
/// changed need to be supplied.
#[derive(Debug, Default, Clone)]
pub struct UpdateCodeListRequest {
    pub id: i64,
    pub code: Option<String>,
    pub extensible: Option<bool>,
    pub name: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

/// Input DTO for [`TerminologyService::create_code_item`].
#[derive(Debug, Clone)]
pub struct CreateCodeItemRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for [`TerminologyService::update_code_item`]. Every
/// field except `id` is optional; only the fields that actually
/// changed need to be supplied.
#[derive(Debug, Default, Clone)]
pub struct UpdateCodeItemRequest {
    pub id: i64,
    pub code: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

/// One entry inside a batch request. All fields map 1-to-1 with
/// `CreateCodeItemRequest` minus codelist_id/version_id
/// (those are fixed at the batch level).
#[derive(Debug, Clone)]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for [`TerminologyService::batch_create_code_items`].
#[derive(Debug, Clone)]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

/// Response for [`TerminologyService::batch_create_code_items`].
#[derive(Debug, Clone)]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}

/// Outbound port for terminology lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn TerminologyService>` can be shared
/// state in an async server (axum, tarpc, etc.). Object-safe: no
/// generic methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `terminology::TerminologyUsecase`) into this contract,
/// translating between backend-specific DTOs / errors and the
/// `apis` types defined above.
#[async_trait]
pub trait TerminologyService: Send + Sync {
    // ---- TerminologyVersion ----

    /// Create a new `(kind, name)` version. Returns
    /// `DuplicateVersion { kind, name }` if a version with that
    /// pair already exists.
    async fn create_version(
        &self,
        req: CreateTerminologyVersionRequest,
    ) -> Result<TerminologyVersionView, TerminologyApiError>;

    /// Look up a version by its surrogate id.
    async fn get_version_by_id(
        &self,
        id: i64,
    ) -> Result<TerminologyVersionView, TerminologyApiError>;

    /// List every version known to the backend. Order is
    /// backend-defined.
    async fn list_versions(&self) -> Result<Vec<TerminologyVersionView>, TerminologyApiError>;

    /// Apply the optional fields on `req` to the version identified
    /// by `req.id`. Returns `NotFound` if no such version exists.
    async fn update_version(
        &self,
        req: UpdateTerminologyVersionRequest,
    ) -> Result<TerminologyVersionView, TerminologyApiError>;

    /// Hard delete the version identified by `id`. Cascades to
    /// child code lists (and via them to code items) at the
    /// backend.
    async fn delete_version(&self, id: i64) -> Result<(), TerminologyApiError>;

    // ---- CodeList ----

    /// Create a new `(version_id, code)` codelist. Returns
    /// `DuplicateCodeList { version_id, code }` if a codelist with
    /// that pair already exists.
    async fn create_code_list(
        &self,
        req: CreateCodeListRequest,
    ) -> Result<CodeListView, TerminologyApiError>;

    /// Look up a codelist by its surrogate id.
    async fn get_code_list_by_id(&self, id: i64) -> Result<CodeListView, TerminologyApiError>;

    /// Unified list+search under a version. Returns one page of
    /// `CodeListView`s. `query.fragment = None` (or `Some("")`)
    /// returns the plain list path ordered by `id ASC`;
    /// `query.fragment = Some(non-empty)` runs the FTS prefix-match
    /// path with `ts_rank DESC, id ASC`. The fragment is split into
    /// terms and every term has to match, so `"vital sign"` matches
    /// the rows containing both `vital` and `sign`.
    /// `query.offset` / `query.limit`
    /// are clamped by the backend (default 50, max 500).
    async fn list_code_lists(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeListView>, TerminologyApiError>;

    /// Apply the optional fields on `req` to the codelist
    /// identified by `req.id`. Returns `NotFound` if no such
    /// codelist exists.
    async fn update_code_list(
        &self,
        req: UpdateCodeListRequest,
    ) -> Result<CodeListView, TerminologyApiError>;

    /// Hard delete the codelist identified by `id`.
    async fn delete_code_list(&self, id: i64) -> Result<(), TerminologyApiError>;

    // ---- CodeItem ----

    /// Create a new `(codelist_id, code)` item. Returns
    /// `DuplicateCodeItem { codelist_id, code }` if an item with
    /// that pair already exists.
    async fn create_code_item(
        &self,
        req: CreateCodeItemRequest,
    ) -> Result<CodeItemView, TerminologyApiError>;

    /// Unified list+search under a codelist. Mirrors
    /// [`TerminologyService::list_code_lists`] but scoped to
    /// `query.codelist_id`.
    async fn list_code_items(
        &self,
        query: CodeItemListQuery,
    ) -> Result<Page<CodeItemView>, TerminologyApiError>;

    /// Natural-key lookup on the `code_items` table. Returns every
    /// item whose `version_id` matches and whose `code` matches —
    /// i.e. all rows sharing the same value code across the
    /// codelists of a single version. Multiple rows are expected
    /// when one item code appears in more than one codelist of the
    /// version.
    async fn list_code_items_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItemView>, TerminologyApiError>;

    /// Apply the optional fields on `req` to the item identified
    /// by `req.id`. Returns `NotFound` if no such item exists.
    async fn update_code_item(
        &self,
        req: UpdateCodeItemRequest,
    ) -> Result<CodeItemView, TerminologyApiError>;

    /// Hard delete the item identified by `id`.
    async fn delete_code_item(&self, id: i64) -> Result<(), TerminologyApiError>;

    /// Create several `CodeItem`s in one logical operation. All items
    /// must share `req.codelist_id` and `req.version_id`. If any item
    /// fails validation the entire batch is rolled back and the first
    /// error is returned with the failing item's position annotated:
    /// `"item 23: code cannot be empty"`.
    async fn batch_create_code_items(
        &self,
        req: BatchCreateCodeItemsRequest,
    ) -> Result<BatchCreateCodeItemsResponse, TerminologyApiError>;
}
