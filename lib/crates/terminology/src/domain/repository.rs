use async_trait::async_trait;

use super::code_item::{CodeItem, CodeItemListQuery, CodeItemNew, CodeItemUpdate};
use super::code_list::{CodeList, CodeListListQuery, CodeListNew, CodeListUpdate};
use super::error::DomainError;
use super::paging::Page;
use super::terminology_kind::TerminologyKind;
use super::terminology_version::{
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionUpdate,
};

/// Outbound port for persistence of `TerminologyVersion`
/// aggregates. Implementations live in the adapter layer.
#[async_trait]
pub trait TerminologyVersionRepository: Send + Sync {
    async fn create(&self, input: TerminologyVersionNew)
    -> Result<TerminologyVersion, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError>;

    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError>;

    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError>;

    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError>;

    /// Hard delete; cascades to child code_lists (and via them to
    /// code_items) via the schema's `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `CodeList` aggregates.
#[async_trait]
pub trait CodeListRepository: Send + Sync {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError>;
    /// Unified list+search under a version. Returns a single page.
    /// - `fragment = None`           → `WHERE version_id = $1 ORDER BY id ASC`
    /// - `fragment = Some(_)`        → the fragment is split into terms; every term
    ///                                  becomes a prefix-matched lexeme and the terms are
    ///                                  ANDed, so `"vital sign"` becomes `vital:* & sign:*`:
    ///                                  `WHERE version_id = $1 AND tsv @@ to_tsquery('english', $2)
    ///                                  ORDER BY ts_rank(tsv, to_tsquery('english', $2)) DESC, id ASC`
    /// Implementations fetch `limit + 1` rows to compute `next_offset`.
    async fn search_or_list(&self, query: CodeListListQuery)
    -> Result<Page<CodeList>, DomainError>;
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError>;
    /// Hard delete; cascades to code_items via the schema's
    /// `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `CodeItem` aggregates.
#[async_trait]
pub trait CodeItemRepository: Send + Sync {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError>;
    /// Unified list+search under a codelist. Returns a single page.
    /// Same shape semantics as
    /// [`CodeListRepository::search_or_list`].
    async fn search_or_list(&self, query: CodeItemListQuery)
    -> Result<Page<CodeItem>, DomainError>;
    /// Natural-key lookup on the `code_items` table itself. Returns
    /// every item whose `version_id` matches the given value and
    /// whose `code` matches the given value — i.e. all items with
    /// the same value code across the codelists of a single
    /// version. Multiple rows are expected when the same item
    /// code appears in more than one codelist of the version.
    /// Backed by the composite index
    /// `code_items_version_id_code_idx (version_id, code)`.
    async fn list_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItem>, DomainError>;
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;

    /// Insert several `CodeItem` rows in a single SQL statement.
    /// Returns the number of rows inserted on success. The backend
    /// must execute this atomically — if any row violates a constraint
    /// the entire call fails and zero rows are inserted.
    async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError>;
}
