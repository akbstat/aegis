use super::{MergedNode, merge};
use crate::domain::node::Node;

/// A node with only the fields a test cares about.
fn node(node_id: &str, entry_name: &str, directory: bool, parent: Option<&str>) -> Node {
    Node {
        node_id: node_id.to_string(),
        description: String::new(),
        entry_name: entry_name.to_string(),
        hidden: None,
        directory,
        parent: parent.map(str::to_string),
        reference: None,
        migration: None,
    }
}

/// The effective names of a merged subtree, depth-first, so a test can
/// assert on the shape of the tree without spelling out the nesting.
fn names(node: &MergedNode) -> Vec<String> {
    let mut out = vec![node.node.entry_name.clone()];
    for child in &node.children {
        out.extend(names(child));
    }
    out
}

/// The effective names of a whole merged tree, depth-first from every
/// root.
fn all_names(merged: &super::MergedTree) -> Vec<String> {
    merged.roots.iter().flat_map(names).collect()
}

#[test]
fn a_template_alone_merges_into_itself() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "a.txt", false, Some("t_root")),
    ];
    let merged = merge(&template, &[]);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(names(&merged.roots[0]), vec!["raw", "a.txt"]);
}

#[test]
fn an_override_replaces_the_template_node_and_drops_the_template_children() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "a.txt", false, Some("t_root")),
    ];
    let mut own = node("t_root", "raw", true, None);
    own.description = "project raw".into();
    let merged = merge(&template, &[own]);
    assert_eq!(merged.roots.len(), 1);
    // the template's child `a.txt` is gone
    assert_eq!(names(&merged.roots[0]), vec!["raw"]);
    assert_eq!(merged.roots[0].node.description, "project raw");
}

#[test]
fn an_override_keeps_the_projects_own_children() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "a.txt", false, Some("t_root")),
    ];
    let own = vec![
        node("t_root", "raw", true, None),
        node("n_new", "b.txt", false, Some("t_root")),
    ];
    let merged = merge(&template, &own);
    assert_eq!(names(&merged.roots[0]), vec!["raw", "b.txt"]);
}

#[test]
fn adding_under_a_template_node_keeps_the_templates_other_children() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "a.txt", false, Some("t_root")),
    ];
    let own = vec![node("n_new", "b.txt", false, Some("t_root"))];
    let merged = merge(&template, &own);
    // both the template's child and the new one are present, sorted by name
    assert_eq!(names(&merged.roots[0]), vec!["raw", "a.txt", "b.txt"]);
}

#[test]
fn a_hidden_node_and_its_subtree_are_filtered_out() {
    let template = vec![
        node("t_doc", "docs", true, None),
        node("t_sub", "x", false, Some("t_doc")),
    ];
    let mut own = node("t_doc", "docs", true, None);
    own.hidden = Some(true);
    let merged = merge(&template, &[own]);
    assert!(merged.roots.is_empty());
}

#[test]
fn renaming_an_inherited_node_wins_and_does_not_resurrect_the_template_original() {
    let template = vec![node("t_root", "raw", true, None)];
    let mut own = node("t_root", "renamed", true, None);
    own.entry_name = "renamed".into();
    let merged = merge(&template, &[own]);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.entry_name, "renamed");
    // the template's `raw` did not come back
    assert_eq!(names(&merged.roots[0]), vec!["renamed"]);
}

#[test]
fn an_own_record_whose_node_id_is_unknown_to_the_template_is_a_plain_new_node() {
    // the template was edited and the record no longer matches anything
    // in it.
    let template = vec![node("t_other", "other", true, None)];
    let own = vec![node("n_new", "raw", true, None)];
    let merged = merge(&template, &own);
    let mut all = all_names(&merged);
    all.sort();
    assert_eq!(all, vec!["other", "raw"]);
}

#[test]
fn the_merged_tree_is_sorted_by_name() {
    let template = vec![
        node("t_z", "zeta", true, None),
        node("t_a", "alpha", true, None),
    ];
    let merged = merge(&template, &[]);
    let all: Vec<String> = merged
        .roots
        .iter()
        .map(|r| r.node.entry_name.clone())
        .collect();
    assert_eq!(all, vec!["alpha", "zeta"]);
}

#[test]
fn the_same_name_may_appear_under_two_different_parents() {
    // A name is unique among the *siblings*, not across the structure.
    let template = vec![
        node("t_one", "one", true, None),
        node("t_two", "two", true, None),
        node("t_a", "same.txt", false, Some("t_one")),
        node("t_b", "same.txt", false, Some("t_two")),
    ];
    let merged = merge(&template, &[]);
    assert_eq!(merged.roots.len(), 2);
    let one = merged
        .roots
        .iter()
        .find(|r| r.node.entry_name == "one")
        .expect("the `one` root is there");
    let two = merged
        .roots
        .iter()
        .find(|r| r.node.entry_name == "two")
        .expect("the `two` root is there");
    assert_eq!(names(one), vec!["one", "same.txt"]);
    assert_eq!(names(two), vec!["two", "same.txt"]);
}

#[test]
fn a_node_whose_parent_is_absent_is_treated_as_a_root() {
    let own = vec![node("n_orphan", "orphan", true, Some("t_missing"))];
    let merged = merge(&[], &own);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.entry_name, "orphan");
}

#[test]
fn re_parenting_an_inherited_node_moves_it_and_does_not_duplicate_it() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "a.txt", false, Some("t_root")),
        node("t_doc", "docs", true, None),
    ];
    // the project re-parents `a.txt` under `docs`
    let own = vec![node("t_file", "a.txt", false, Some("t_doc"))];
    let merged = merge(&template, &own);
    let mut all = all_names(&merged);
    all.sort();
    // `a.txt` appears once, and it is no longer under `raw`
    assert_eq!(all, vec!["a.txt", "docs", "raw"]);
    let raw = merged
        .roots
        .iter()
        .find(|r| r.node.entry_name == "raw")
        .expect("the raw root is still there");
    assert!(raw.children.is_empty());
}

#[test]
fn a_dropped_template_subtree_is_not_promoted_to_a_root() {
    // overriding `raw` drops the template's children; their own children
    // go with them and must not reappear at the top level.
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_child", "sub", true, Some("t_root")),
        node("t_grand", "deep.txt", false, Some("t_child")),
    ];
    let own = vec![node("t_root", "raw", true, None)];
    let merged = merge(&template, &own);
    let mut all = all_names(&merged);
    all.sort();
    assert_eq!(all, vec!["raw"]);
}

#[test]
fn a_cycle_does_not_recurse_forever() {
    let own = vec![
        node("a", "a", true, Some("b")),
        node("b", "b", true, Some("a")),
    ];
    let merged = merge(&[], &own);
    // the merge terminates; every node appears at most once
    let mut all = all_names(&merged);
    all.sort();
    all.dedup();
    assert!(all.len() <= 2);
}

#[test]
fn an_own_record_with_a_dangling_parent_is_still_rendered() {
    // the parent was dropped by an override elsewhere; the record is
    // surfaced rather than silently lost.
    let own = vec![node("n_orphan", "orphan", true, Some("t_missing"))];
    let merged = merge(&[], &own);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.node_id, "n_orphan");
}
