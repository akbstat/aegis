//! The merge algorithm.
//!
//! Joins a template's nodes with a project structure's extension
//! records on **`node_id`**, never on `entry_name`. See the spec's "The
//! merge algorithm" section for the seven rules this implements.
//!
//! The merge is **total** — it never fails and never panics. Malformed
//! input (a `parent` naming a node that is not in the merged set, a
//! cycle through `parent`) is resolved defensively rather than
//! rejected: a node whose parent is absent is treated as a root, and a
//! node already visited during the recursion is skipped. The usecase
//! validates those shapes before the merge ever sees them.

use std::collections::{HashMap, HashSet};

use super::node::Node;

#[cfg(test)]
mod tests;

/// A node in the merged tree: its effective attributes plus its
/// effective children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedNode {
    pub node: Node,
    pub children: Vec<MergedNode>,
}

/// The merged result: the effective tree's roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedTree {
    pub roots: Vec<MergedNode>,
}

/// The effective record for each `node_id`: the template's rows with
/// this structure's own rows layered on top. An own record wins.
///
/// This is the set of nodes the structure actually has — a template
/// node that the project has overridden does not also exist under its
/// template name.
pub fn effective_records<'a>(template: &'a [Node], own: &'a [Node]) -> HashMap<&'a str, &'a Node> {
    let mut effective: HashMap<&str, &Node> = HashMap::with_capacity(template.len() + own.len());
    for node in template {
        effective.insert(node.node_id.as_str(), node);
    }
    for node in own {
        effective.insert(node.node_id.as_str(), node);
    }
    effective
}

/// Combine a template's nodes with a project structure's extension
/// records.
///
/// The join key is `node_id`. An extension record whose `node_id` the
/// template does not know is an addition; one whose `node_id` the
/// template does know **overrides** that template node.
pub fn merge(template: &[Node], own: &[Node]) -> MergedTree {
    // 1. The effective record for each node_id. An own record wins.
    let effective = effective_records(template, own);

    // 2. The node_ids this structure has a record for.
    let own_ids: HashSet<&str> = own.iter().map(|n| n.node_id.as_str()).collect();

    // 3. The children of each node, keyed on the *effective* parent — the
    //    parent named by the winning record, so a re-parented node moves
    //    instead of appearing under two parents.
    let mut children: HashMap<&str, Vec<&Node>> = HashMap::new();
    for node in effective.values() {
        if let Some(parent) = node.parent.as_deref() {
            children.entry(parent).or_default().push(node);
        }
    }

    // 4. Overriding a node drops the template's children: only the
    //    extension records under it survive.
    for (parent, kids) in children.iter_mut() {
        if own_ids.contains(parent) {
            kids.retain(|kid| own_ids.contains(kid.node_id.as_str()));
        }
    }

    // 5. The roots: a node with no parent, plus an own record whose
    //    parent is not in the merged set at all. A *template* node whose
    //    parent is missing is not a root — it was dropped with its
    //    parent and is simply unreachable.
    let mut roots: Vec<&Node> = Vec::new();
    for (id, node) in &effective {
        match node.parent.as_deref() {
            None => roots.push(node),
            Some(parent) => {
                if !effective.contains_key(parent) && own_ids.contains(id) {
                    roots.push(node);
                }
            }
        }
    }

    // 6. Build the tree. A hidden node is skipped, and so is its
    //    subtree. The visited set makes a cycle through `parent`
    //    terminate instead of recursing forever.
    let mut visited: HashSet<&str> = HashSet::new();
    let mut built: Vec<MergedNode> = roots
        .into_iter()
        .filter_map(|root| build(root, &children, &mut visited))
        .collect();

    sort_by_name(&mut built);
    MergedTree { roots: built }
}

/// Build one merged node and its subtree.
fn build<'a>(
    node: &'a Node,
    children: &HashMap<&'a str, Vec<&'a Node>>,
    visited: &mut HashSet<&'a str>,
) -> Option<MergedNode> {
    if node.hidden == Some(true) {
        return None;
    }
    if !visited.insert(node.node_id.as_str()) {
        return None;
    }
    let mut kids: Vec<MergedNode> = Vec::new();
    if let Some(list) = children.get(node.node_id.as_str()) {
        for kid in list {
            if let Some(merged) = build(kid, children, visited) {
                kids.push(merged);
            }
        }
    }
    sort_by_name(&mut kids);
    Some(MergedNode {
        node: node.clone(),
        children: kids,
    })
}

/// Sort by name, then by node_id, so the order is deterministic even
/// when two nodes share a name.
fn sort_by_name(nodes: &mut [MergedNode]) {
    nodes.sort_by(|a, b| {
        a.node
            .entry_name
            .cmp(&b.node.entry_name)
            .then_with(|| a.node.node_id.cmp(&b.node.node_id))
    });
}
