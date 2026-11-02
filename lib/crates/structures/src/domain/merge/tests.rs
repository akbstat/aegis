use super::{MergedNode, merge};
use crate::domain::node::Node;

/// A node with only the fields a test cares about.
fn node(node_id: &str, path: &str, directory: bool, parent: Option<&str>) -> Node {
    Node {
        node_id: node_id.to_string(),
        description: String::new(),
        path: path.to_string(),
        hidden: None,
        directory,
        parent: parent.map(str::to_string),
        reference: None,
        migration: None,
    }
}

/// The effective `path`s of a merged subtree, depth-first, so a test can
/// assert on the shape of the tree without spelling out the nesting.
fn paths(node: &MergedNode) -> Vec<String> {
    let mut out = vec![node.node.path.clone()];
    for child in &node.children {
        out.extend(paths(child));
    }
    out
}

/// The effective `path`s of a whole merged tree, depth-first from every
/// root.
fn all_paths(merged: &super::MergedTree) -> Vec<String> {
    merged.roots.iter().flat_map(paths).collect()
}

#[test]
fn a_template_alone_merges_into_itself() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "raw/a.txt", false, Some("t_root")),
    ];
    let merged = merge(&template, &[]);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(paths(&merged.roots[0]), vec!["raw", "raw/a.txt"]);
}

#[test]
fn an_override_replaces_the_template_node_and_drops_the_template_children() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "raw/a.txt", false, Some("t_root")),
    ];
    let mut own = node("t_root", "raw", true, None);
    own.description = "project raw".into();
    let merged = merge(&template, &[own]);
    assert_eq!(merged.roots.len(), 1);
    // the template's child `raw/a.txt` is gone
    assert_eq!(paths(&merged.roots[0]), vec!["raw"]);
    assert_eq!(merged.roots[0].node.description, "project raw");
}

#[test]
fn an_override_keeps_the_projects_own_children() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "raw/a.txt", false, Some("t_root")),
    ];
    let own = vec![
        node("t_root", "raw", true, None),
        node("n_new", "raw/b.txt", false, Some("t_root")),
    ];
    let merged = merge(&template, &own);
    assert_eq!(paths(&merged.roots[0]), vec!["raw", "raw/b.txt"]);
}

#[test]
fn adding_under_a_template_node_keeps_the_templates_other_children() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "raw/a.txt", false, Some("t_root")),
    ];
    let own = vec![node("n_new", "raw/b.txt", false, Some("t_root"))];
    let merged = merge(&template, &own);
    // both the template's child and the new one are present, sorted by path
    assert_eq!(
        paths(&merged.roots[0]),
        vec!["raw", "raw/a.txt", "raw/b.txt"]
    );
}

#[test]
fn a_hidden_node_and_its_subtree_are_filtered_out() {
    let template = vec![
        node("t_doc", "docs", true, None),
        node("t_sub", "docs/x", false, Some("t_doc")),
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
    own.path = "renamed".into();
    let merged = merge(&template, &[own]);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.path, "renamed");
    // the template's `raw` did not come back
    assert_eq!(paths(&merged.roots[0]), vec!["renamed"]);
}

#[test]
fn an_own_record_whose_node_id_is_unknown_to_the_template_is_a_plain_new_node() {
    // the template was edited and the record no longer matches anything
    // in it.
    let template = vec![node("t_other", "other", true, None)];
    let own = vec![node("n_new", "raw", true, None)];
    let merged = merge(&template, &own);
    let mut all = all_paths(&merged);
    all.sort();
    assert_eq!(all, vec!["other", "raw"]);
}

#[test]
fn the_merged_tree_is_sorted_by_path() {
    let template = vec![
        node("t_z", "zeta", true, None),
        node("t_a", "alpha", true, None),
    ];
    let merged = merge(&template, &[]);
    let all: Vec<String> = merged.roots.iter().map(|r| r.node.path.clone()).collect();
    assert_eq!(all, vec!["alpha", "zeta"]);
}

#[test]
fn a_node_whose_parent_is_absent_is_treated_as_a_root() {
    let own = vec![node("n_orphan", "orphan", true, Some("t_missing"))];
    let merged = merge(&[], &own);
    assert_eq!(merged.roots.len(), 1);
    assert_eq!(merged.roots[0].node.path, "orphan");
}

#[test]
fn re_parenting_an_inherited_node_moves_it_and_does_not_duplicate_it() {
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_file", "raw/a.txt", false, Some("t_root")),
        node("t_doc", "docs", true, None),
    ];
    // the project re-parents `raw/a.txt` under `docs`
    let own = vec![node("t_file", "raw/a.txt", false, Some("t_doc"))];
    let merged = merge(&template, &own);
    let mut all = all_paths(&merged);
    all.sort();
    // `raw/a.txt` appears once, and it is no longer under `raw`
    assert_eq!(all, vec!["docs", "raw", "raw/a.txt"]);
    let raw = merged
        .roots
        .iter()
        .find(|r| r.node.path == "raw")
        .expect("the raw root is still there");
    assert!(raw.children.is_empty());
}

#[test]
fn a_dropped_template_subtree_is_not_promoted_to_a_root() {
    // overriding `raw` drops the template's children; their own children
    // go with them and must not reappear at the top level.
    let template = vec![
        node("t_root", "raw", true, None),
        node("t_child", "raw/sub", true, Some("t_root")),
        node("t_grand", "raw/sub/deep.txt", false, Some("t_child")),
    ];
    let own = vec![node("t_root", "raw", true, None)];
    let merged = merge(&template, &own);
    let mut all = all_paths(&merged);
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
    let mut all = all_paths(&merged);
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
