use chrono::{TimeZone, Utc};

use super::*;

fn test_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

#[test]
fn team_type_as_str_maps_to_lowercase() {
    assert_eq!(TeamType::Members.as_str(), "members");
    assert_eq!(TeamType::UnblindMembers.as_str(), "unblind_members");
}

#[test]
fn team_type_try_from_str_parses_known_values() {
    assert_eq!(TeamType::try_from("members").unwrap(), TeamType::Members);
    assert_eq!(
        TeamType::try_from("unblind_members").unwrap(),
        TeamType::UnblindMembers
    );
}

#[test]
fn team_type_try_from_str_rejects_unknown_value() {
    let err = TeamType::try_from("admins").unwrap_err();
    assert!(matches!(err, DomainError::UnknownTeamType(ref s) if s == "admins"));
}

#[test]
fn role_type_as_str_maps_to_lowercase() {
    assert_eq!(RoleType::Leader.as_str(), "leader");
    assert_eq!(RoleType::Worker.as_str(), "worker");
}

#[test]
fn role_type_try_from_str_parses_known_values() {
    assert_eq!(RoleType::try_from("leader").unwrap(), RoleType::Leader);
    assert_eq!(RoleType::try_from("worker").unwrap(), RoleType::Worker);
}

#[test]
fn role_type_try_from_str_rejects_unknown_value() {
    let err = RoleType::try_from("admin").unwrap_err();
    assert!(matches!(err, DomainError::UnknownRoleType(ref s) if s == "admin"));
}

#[test]
fn project_member_accepts_clean_input() {
    let m = ProjectMember::new(vec!["u1".into()], vec!["u2".into()]).unwrap();
    assert_eq!(m.leaders, vec!["u1".to_string()]);
    assert_eq!(m.workers, vec!["u2".to_string()]);
}

#[test]
fn project_member_rejects_duplicate_leader() {
    let err = ProjectMember::new(vec!["u1".into(), "u1".into()], vec![]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateLeader(ref s) if s == "u1"));
}

#[test]
fn project_member_rejects_duplicate_worker() {
    let err = ProjectMember::new(vec![], vec!["u2".into(), "u2".into()]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateWorker(ref s) if s == "u2"));
}

#[test]
fn project_member_allows_same_code_in_leaders_and_workers() {
    let m = ProjectMember::new(vec!["u1".into()], vec!["u1".into()]).unwrap();
    assert_eq!(m.leaders, vec!["u1".to_string()]);
    assert_eq!(m.workers, vec!["u1".to_string()]);
}

#[test]
fn project_member_accepts_empty_lists() {
    let m = ProjectMember::new(vec![], vec![]).unwrap();
    assert!(m.leaders.is_empty());
    assert!(m.workers.is_empty());
}

#[test]
fn project_new_rejects_empty_code() {
    let m = ProjectMember::default();
    let err = Project::new(
        1,
        "".into(),
        "desc".into(),
        m.clone(),
        m,
        ProjectConfiguration::default(),
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn project_new_accepts_valid_input() {
    let m = ProjectMember::default();
    let p = Project::new(
        9,
        "proj9".into(),
        "desc".into(),
        m.clone(),
        m,
        ProjectConfiguration::default(),
        true,
        test_now(),
        test_now(),
    )
    .unwrap();
    assert_eq!(p.id, 9);
    assert!(p.configurations.tags.is_empty());
    assert!(p.configurations.language.is_none());
}

#[test]
fn project_tag_new_rejects_empty_key() {
    let err = ProjectTag::new("".into(), "v".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyTagKey));
}

#[test]
fn project_tag_new_rejects_empty_value() {
    let err = ProjectTag::new("k".into(), "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyTagValue));
}

#[test]
fn project_tag_new_accepts_valid_input() {
    let t = ProjectTag::new("Product".into(), "DEMO-001".into()).unwrap();
    assert_eq!(t.key, "Product");
    assert_eq!(t.value, "DEMO-001");
}

#[test]
fn project_language_as_str_maps_to_wire_codes() {
    assert_eq!(ProjectLanguage::English.as_str(), "en");
    assert_eq!(ProjectLanguage::SimplifiedChinese.as_str(), "zh-CN");
}

#[test]
fn project_language_try_from_str_parses_known_values() {
    assert_eq!(
        ProjectLanguage::try_from("en").unwrap(),
        ProjectLanguage::English
    );
    assert_eq!(
        ProjectLanguage::try_from("zh-CN").unwrap(),
        ProjectLanguage::SimplifiedChinese
    );
}

#[test]
fn project_language_try_from_str_rejects_unknown_value() {
    let err = ProjectLanguage::try_from("ja").unwrap_err();
    assert!(matches!(err, DomainError::UnknownLanguage(ref s) if s == "ja"));
}

#[test]
fn project_configuration_default_has_no_language_and_no_tags() {
    let c = ProjectConfiguration::default();
    assert!(c.language.is_none());
    assert!(c.tags.is_empty());
}

#[test]
fn project_configuration_for_repository_carries_language_and_tags() {
    let c = ProjectConfiguration::for_repository(
        Some(ProjectLanguage::SimplifiedChinese),
        vec![ProjectTag::for_repository("k".into(), "v".into())],
        None,
        None,
    );
    assert_eq!(c.language, Some(ProjectLanguage::SimplifiedChinese));
    assert_eq!(c.tags.len(), 1);
    assert_eq!(c.tags[0].key, "k");
}

#[test]
fn project_configuration_default_has_no_sdtmig() {
    let c = ProjectConfiguration::default();
    assert!(c.sdtmig.is_none());
}

#[test]
fn project_configuration_for_repository_carries_sdtmig() {
    let c = ProjectConfiguration::for_repository(
        None,
        vec![],
        Some(ModelVersion::for_repository(3, "2024-03-29".into())),
        None,
    );
    assert_eq!(
        c.sdtmig,
        Some(ModelVersion::for_repository(3, "2024-03-29".into()))
    );
}

#[test]
fn model_version_new_rejects_empty_name() {
    let err = ModelVersion::new(1, "".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmigName));
}

#[test]
fn model_version_new_rejects_whitespace_name() {
    let err = ModelVersion::new(1, "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmigName));
}

#[test]
fn model_version_new_accepts_valid_input() {
    let v = ModelVersion::new(7, "2024-03-29".into()).unwrap();
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn model_version_for_repository_carries_fields() {
    let v = ModelVersion::for_repository(7, "2024-03-29".into());
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn terminology_version_data_new_rejects_empty_name() {
    let err = TerminologyVersionData::new(1, "".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmTerminologyName));
}

#[test]
fn terminology_version_data_new_rejects_whitespace_name() {
    let err = TerminologyVersionData::new(1, "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmTerminologyName));
}

#[test]
fn terminology_version_data_new_accepts_valid_input() {
    let v = TerminologyVersionData::new(7, "2024-03-29".into()).unwrap();
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn terminology_version_data_for_repository_carries_fields() {
    let v = TerminologyVersionData::for_repository(7, "2024-03-29".into());
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn project_configuration_default_has_no_sdtm_terminology() {
    let c = ProjectConfiguration::default();
    assert!(c.sdtm_terminology.is_none());
}

#[test]
fn project_branch_new_rejects_an_empty_name() {
    let err = ProjectBranch::new(1, 1, "".into(), false, test_now(), test_now()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyBranchName));
}

#[test]
fn project_branch_new_rejects_a_whitespace_only_name() {
    let err = ProjectBranch::new(1, 1, "   ".into(), false, test_now(), test_now()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyBranchName));
}

#[test]
fn project_branch_new_accepts_a_valid_name() {
    let b = ProjectBranch::new(7, 3, "2026Q1".into(), true, test_now(), test_now()).unwrap();
    assert_eq!(b.id, 7);
    assert_eq!(b.project_id, 3);
    assert_eq!(b.branch, "2026Q1");
    assert!(b.active);
}

#[test]
fn project_branch_for_repository_bypasses_validation() {
    let b = ProjectBranch::for_repository(1, 1, "".into(), false, test_now(), test_now());
    assert_eq!(b.branch, "");
}

#[test]
fn project_configuration_for_repository_carries_sdtm_terminology() {
    let c = ProjectConfiguration::for_repository(
        None,
        vec![],
        None,
        Some(TerminologyVersionData::for_repository(
            7,
            "2024-03-29".into(),
        )),
    );
    assert_eq!(
        c.sdtm_terminology,
        Some(TerminologyVersionData::for_repository(
            7,
            "2024-03-29".into()
        ))
    );
}
