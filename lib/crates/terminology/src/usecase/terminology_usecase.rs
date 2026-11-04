use crate::domain::{
    CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, CodeListListQuery,
    CodeListNew, CodeListRepository, CodeListUpdate, DomainError, Page, TerminologyVersionNew,
    TerminologyVersionRepository, TerminologyVersionUpdate,
};

use super::commands::{
    BatchCreateCodeItems, CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeItem,
    UpdateCodeList, UpdateTerminologyVersion,
};
use super::error::UsecaseError;
use super::views::{
    BatchCreateCodeItemsResponse, CodeItemView, CodeListView, TerminologyVersionView,
};

/// Configuration for `TerminologyUsecase::new`. Wraps the three
/// concrete (or fake) repositories so the constructor stays
/// readable.
pub struct TerminologyUsecaseConfig<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    pub version_repo: V,
    pub code_list_repo: L,
    pub code_item_repo: I,
}

/// Async orchestration for terminology lifecycle operations.
///
/// Generic over the three repository ports so tests can inject
/// in-memory fakes. Domain → view projection runs through the
/// `From` impls in `super::views`.
pub struct TerminologyUsecase<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    version_repo: V,
    code_list_repo: L,
    code_item_repo: I,
}

impl<V, L, I> TerminologyUsecase<V, L, I>
where
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
{
    pub fn new(cfg: TerminologyUsecaseConfig<V, L, I>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            code_list_repo: cfg.code_list_repo,
            code_item_repo: cfg.code_item_repo,
        }
    }

    // ---- TerminologyVersion ----

    pub async fn create_version(
        &self,
        cmd: CreateTerminologyVersion,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        let version = self
            .version_repo
            .create(TerminologyVersionNew {
                kind: cmd.kind,
                name: cmd.name,
            })
            .await?;
        Ok(version.into())
    }

    pub async fn get_version_by_id(&self, id: i64) -> Result<TerminologyVersionView, UsecaseError> {
        let v = self.version_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn list_versions(&self) -> Result<Vec<TerminologyVersionView>, UsecaseError> {
        let versions = self.version_repo.list().await?;
        Ok(versions.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self,
        cmd: UpdateTerminologyVersion,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(TerminologyVersionUpdate {
                id: cmd.id,
                kind: cmd.kind,
                name: cmd.name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- CodeList ----

    pub async fn create_code_list(
        &self,
        cmd: CreateCodeList,
    ) -> Result<CodeListView, UsecaseError> {
        validate_create_code_list(&cmd)?;
        let cl = self
            .code_list_repo
            .create(CodeListNew {
                version_id: cmd.version_id,
                code: cmd.code,
                extensible: cmd.extensible,
                name: cmd.name,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(cl.into())
    }

    pub async fn get_code_list_by_id(&self, id: i64) -> Result<CodeListView, UsecaseError> {
        let cl = self.code_list_repo.find_by_id(id).await?;
        Ok(cl.into())
    }

    pub async fn list_code_lists(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeListView>, UsecaseError> {
        let limit = clamp_limit(query.limit);
        let fragment = normalize_fragment(query.fragment.as_deref())?;
        let q = CodeListListQuery {
            fragment,
            offset: query.offset,
            limit,
            ..query
        };
        let page = self.code_list_repo.search_or_list(q).await?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        })
    }

    pub async fn update_code_list(
        &self,
        cmd: UpdateCodeList,
    ) -> Result<CodeListView, UsecaseError> {
        validate_update_code_list(&cmd)?;
        let cl = self
            .code_list_repo
            .update(CodeListUpdate {
                id: cmd.id,
                code: cmd.code,
                extensible: cmd.extensible,
                name: cmd.name,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(cl.into())
    }

    pub async fn delete_code_list(&self, id: i64) -> Result<(), UsecaseError> {
        self.code_list_repo.delete(id).await?;
        Ok(())
    }

    // ---- CodeItem ----

    pub async fn create_code_item(
        &self,
        cmd: CreateCodeItem,
    ) -> Result<CodeItemView, UsecaseError> {
        validate_create_code_item(&cmd)?;
        let item = self
            .code_item_repo
            .create(CodeItemNew {
                codelist_id: cmd.codelist_id,
                version_id: cmd.version_id,
                code: cmd.code,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(item.into())
    }

    pub async fn list_code_items(
        &self,
        query: CodeItemListQuery,
    ) -> Result<Page<CodeItemView>, UsecaseError> {
        let limit = clamp_limit(query.limit);
        let fragment = normalize_fragment(query.fragment.as_deref())?;
        let q = CodeItemListQuery {
            fragment,
            offset: query.offset,
            limit,
            ..query
        };
        let page = self.code_item_repo.search_or_list(q).await?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        })
    }

    /// Natural-key lookup on the `code_items` table. Returns every
    /// item whose `version_id` matches and whose `code` matches —
    /// i.e. all rows sharing the same item value code across the
    /// codelists of a single version. Multiple rows are expected
    /// when one item code appears in more than one codelist of the
    /// version.
    pub async fn list_code_items_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItemView>, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let items = self
            .code_item_repo
            .list_by_version_and_code(version_id, code)
            .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    pub async fn update_code_item(
        &self,
        cmd: UpdateCodeItem,
    ) -> Result<CodeItemView, UsecaseError> {
        validate_update_code_item(&cmd)?;
        let item = self
            .code_item_repo
            .update(CodeItemUpdate {
                id: cmd.id,
                code: cmd.code,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(item.into())
    }

    pub async fn delete_code_item(&self, id: i64) -> Result<(), UsecaseError> {
        self.code_item_repo.delete(id).await?;
        Ok(())
    }

    pub async fn batch_create_code_items(
        &self,
        cmd: BatchCreateCodeItems,
    ) -> Result<BatchCreateCodeItemsResponse, UsecaseError> {
        validate_batch_code_items(&cmd)?;

        let inputs: Vec<CodeItemNew> = cmd
            .items
            .iter()
            .map(|item| CodeItemNew {
                codelist_id: cmd.codelist_id,
                version_id: cmd.version_id,
                code: item.code.clone(),
                submission_value: item.submission_value.clone(),
                synonym: item.synonym.clone(),
                definition: item.definition.clone(),
                nci_preferred_term: item.nci_preferred_term.clone(),
            })
            .collect();

        let count = self.code_item_repo.bulk_create(inputs).await?;

        Ok(BatchCreateCodeItemsResponse {
            count,
            codelist_id: cmd.codelist_id,
            version_id: cmd.version_id,
        })
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateTerminologyVersion) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateTerminologyVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_code_list(cmd: &CreateCodeList) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_update_code_list(cmd: &UpdateCodeList) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_create_code_item(cmd: &CreateCodeItem) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_update_code_item(cmd: &UpdateCodeItem) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_batch_code_items(cmd: &BatchCreateCodeItems) -> Result<(), UsecaseError> {
    for (i, item) in cmd.items.iter().enumerate() {
        if item.code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCodeAtPosition(
                i,
            )));
        }
    }
    Ok(())
}

// ---- search-query sanitation ----

/// Normalise a fragment for the list+search path:
/// - `None` or `Some("")` (whitespace-only after trim) → `None`
///   (treat as "no fragment", plain list path).
/// - `Some(non-empty)` → run `validate_tsquery_fragment` and pass
///   the trimmed fragment to the repo.
fn normalize_fragment(fragment: Option<&str>) -> Result<Option<String>, UsecaseError> {
    match fragment {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => Ok(Some(validate_tsquery_fragment(s)?.to_owned())),
    }
}

/// Reject any fragment containing characters reserved by Postgres
/// `to_tsquery` (`& | ! ( ) :`) or the quote characters that would
/// otherwise open a quoted lexeme (`' "`). The usecase passes
/// `fragment = None` (or `Some("")`) when the caller wants the plain
/// list path, so empty / whitespace-only fragments never reach this
/// helper.
///
/// Every other character is not an error: the adapter splits the
/// fragment into terms on them, so `"vital sign"` is a two-term
/// search and `"non-small"` is a two-term search as well.
fn validate_tsquery_fragment(s: &str) -> Result<&str, UsecaseError> {
    if s.chars()
        .any(|c| matches!(c, '&' | '|' | '!' | '(' | ')' | ':' | '\'' | '"'))
    {
        return Err(UsecaseError::Validation(DomainError::InvalidFragment));
    }
    Ok(s.trim())
}

/// Apply the default + hard cap to the `limit` field of a list+search
/// query. The Postgres implementation reads the clamped value, so
/// the cap is enforced even when tests pass an unbounded `u32::MAX`.
fn clamp_limit(limit: u32) -> u32 {
    if limit == 0 {
        50
    } else if limit > 500 {
        500
    } else {
        limit
    }
}
