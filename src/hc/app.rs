//! The Herdr app's layout, `herdr-app`, a document the clients of a core share: it rereads it when the core says it
//! changed, and two writers are settled by its version. Its spaces and tabs are [`Doc`]'s workspaces and tabs, under
//! their own ids. What Shepherd did not change goes back as it came, with whatever Shepherd cannot draw in it. A pane
//! whose terminal is gone stays in the document, which is the app's to tidy: a list that came back short must not cost
//! a layout. Focus and zoom are each client's own, as in the app.
use super::tree::{Doc, Node, OTHERS, Tab, Workspace};
use anyhow::bail;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::hash::BuildHasher;

pub const DOC: &str = "herdr-app";
/// The app's document as it was before Shepherd first wrote it.
pub const BACKUP: &str = "shepherd-herdr-app-backup";

/// A tree's live terminals as splits; the app's rows hold any number, each with its weight. `None`: it has none.
fn tree(node: &Value, live: &impl Fn(&str) -> bool) -> Option<Node> {
    if node["type"] == "block" {
        return node["content"]["terminal"].as_str().filter(|t| live(t)).map(|t| Node::Pane(t.to_string()));
    }
    let weight = |child: &Value| child["size"]["weight"].as_f64().filter(|w| *w > 0.).unwrap_or(1.) as f32;
    let right = node["type"] != "column";
    let children: Vec<(Node, f32)> = node["children"].as_array()?.iter().filter_map(|c| Some((tree(c, live)?, weight(c)))).collect();
    let mut children = children.into_iter().rev();
    let (mut folded, mut held) = children.next()?;
    for (node, weight) in children {
        folded = Node::Split { right, ratio: weight / (weight + held), first: Box::new(node), second: Box::new(folded) };
        held += weight;
    }
    Some(folded)
}

/// Whether a tree is only what [`tree`] reads, so that nothing is lost by writing it anew.
fn plain(node: &Value) -> bool {
    match node["type"].as_str() {
        Some("block") => node["content"]["terminal"].is_string(),
        Some("row" | "column") => node["floating"].as_array().is_none_or(Vec::is_empty) && node["children"].as_array().is_some_and(|c| c.iter().all(plain)),
        _ => false,
    }
}

/// Each terminal's pane id in a tree: the app keeps a pane's view under it.
fn pane_ids(node: &Value, ids: &mut HashMap<String, String>) {
    if let (Some(terminal), Some(id)) = (node["content"]["terminal"].as_str(), node["id"].as_str()) {
        ids.insert(terminal.to_string(), id.to_string());
    }
    node["children"].as_array().into_iter().flatten().for_each(|child| pane_ids(child, ids));
}

/// An id as the app makes them: random, so that no two clients pick the same.
fn fresh(kind: &str) -> String {
    let random = || std::collections::hash_map::RandomState::new().hash_one(kind);
    format!("{kind}{:016x}{:016x}", random(), random())
}

/// `node` in the app's words, under `id`.
fn node_value(node: &Node, id: String, ids: &HashMap<String, String>) -> Value {
    match node {
        Node::Pane(t) => json!({"type": "block", "id": ids.get(t).cloned().unwrap_or(id), "content": {"terminal": t}}),
        Node::Split { right, ratio, first, second } => {
            let side = |child: &Node, weight: f32| {
                let mut value = node_value(child, fresh("p"), ids);
                value["size"] = json!({"weight": weight});
                value
            };
            json!({"type": if *right { "row" } else { "column" }, "id": id, "children": [side(first, *ratio), side(second, 1. - *ratio)]})
        }
    }
}

/// A tab's root: always a container with the tab's id, as the app writes them.
fn root_value(root: &Node, tab: &str, ids: &HashMap<String, String>) -> Value {
    match root {
        Node::Pane(_) => json!({"type": "row", "id": tab, "children": [node_value(root, fresh("p"), ids)]}),
        split => node_value(split, tab.to_string(), ids),
    }
}

fn spaces(app: &Value) -> &[Value] {
    app["spaces"].as_array().map_or(&[], Vec::as_slice)
}

fn tabs(space: &Value) -> &[Value] {
    space["tabs"].as_array().map_or(&[], Vec::as_slice)
}

/// Puts the app's spaces in `doc`, keeping what is each client's own. Shepherd's workspaces not written yet stay, and go
/// after them, the terminals no client groups last.
pub fn absorb(doc: &mut Doc, app: &Value, live: &impl Fn(&str) -> bool) {
    let old = std::mem::take(&mut doc.workspaces);
    let tab_of = |id: &str| old.iter().flat_map(|w| &w.tabs).find(|t| t.id == id);
    let mut shown = Vec::new();
    for space in spaces(app) {
        let id = space["id"].as_str().unwrap_or_default();
        let tabs: Vec<Tab> = tabs(space)
            .iter()
            .filter_map(|tab| {
                let root = tree(&tab["root"], live)?;
                let id = tab["id"].as_str().unwrap_or_default();
                let terminals = root.terminals();
                let view = tab_of(id).filter(|t| terminals.contains(&t.focused));
                let focused = view.map_or_else(|| terminals[0].clone(), |t| t.focused.clone());
                let label = tab["name"].as_str().unwrap_or_default().to_string();
                Some(Tab { id: id.to_string(), label, focused, zoomed: view.is_some_and(|t| t.zoomed), root, origin: Some(id.to_string()) })
            })
            .collect();
        if tabs.is_empty() {
            continue;
        }
        let before = old.iter().find(|w| w.id == id);
        let active = before.map(|w| w.active_tab.clone()).filter(|a| tabs.iter().any(|t| t.id == *a));
        shown.push(Workspace {
            id: id.to_string(),
            label: space["name"].as_str().unwrap_or_default().to_string(),
            active_tab: active.unwrap_or_else(|| tabs[0].id.clone()),
            tabs,
            worktree: before.and_then(|w| w.worktree.clone()),
            origin: Some(id.to_string()),
        });
    }
    let (others, own): (Vec<_>, Vec<_>) = old.iter().filter(|w| w.origin.is_none() || w.origin.as_deref() == Some(OTHERS)).cloned().partition(|w| w.origin.is_some());
    doc.adopted.retain(|t| live(t));
    for terminal in shown.iter().flat_map(|w| &w.tabs).flat_map(|t| t.root.terminals()) {
        if !doc.adopted.contains(&terminal) {
            doc.adopted.push(terminal);
        }
    }
    let own = own.into_iter().filter(|w| !shown.iter().any(|s| s.id == w.id)).collect::<Vec<_>>();
    doc.workspaces = shown.into_iter().chain(own).chain(others).collect();
    doc.settle();
}

/// `shown` with what Shepherd does not show of `was` back at its place. Elsewhere, every write would reorder the app's
/// list, and the two would rewrite it in turn.
fn with_unseen(mut shown: Vec<Value>, was: &[Value], unseen: impl Fn(&Value) -> bool) -> Vec<Value> {
    for (at, item) in was.iter().enumerate().filter(|(_, item)| unseen(item)) {
        shown.insert(at.min(shown.len()), item.clone());
    }
    shown
}

/// The app's document with `doc`'s workspaces in it, which are now the app's too. `cwd`: where a terminal is, for a new
/// space's folder. Refuses to redraw a tab holding what [`tree`] does not read.
pub fn written(doc: &mut Doc, app: &Value, live: &impl Fn(&str) -> bool, cwd: &impl Fn(&str) -> Option<String>) -> anyhow::Result<Value> {
    let mut ids = HashMap::new();
    spaces(app).iter().flat_map(tabs).for_each(|tab| pane_ids(&tab["root"], &mut ids));
    let tab_was = |id: &str| spaces(app).iter().flat_map(tabs).find(|t| t["id"] == id);
    let mut out = Vec::new();
    for w in doc.workspaces.iter_mut().filter(|w| w.origin.as_deref() != Some(OTHERS)) {
        let was = spaces(app).iter().find(|s| s["id"] == w.id.as_str());
        let mut space = was.cloned().unwrap_or_else(|| {
            let first = w.tabs.first().map(|t| t.focused.as_str()).and_then(cwd);
            json!({"id": w.id, "folder": first.unwrap_or_default()})
        });
        if space["name"].as_str().unwrap_or_default() != w.label {
            space["name"] = json!(w.label);
        }
        let mut new_tabs = Vec::new();
        for tab in &mut w.tabs {
            let old = tab_was(&tab.id);
            let same = old.is_some_and(|o| tree(&o["root"], live).as_ref() == Some(&tab.root));
            let mut value = old.cloned().unwrap_or_else(|| json!({"id": tab.id}));
            if !same {
                if old.is_some_and(|o| !plain(&o["root"])) {
                    bail!("this tab holds a mod or a stack: change its layout in the Herdr app");
                }
                value["root"] = root_value(&tab.root, &tab.id, &ids);
            }
            if value["name"].as_str().unwrap_or_default() != tab.label {
                value["name"] = json!(tab.label);
            }
            tab.origin = Some(tab.id.clone());
            new_tabs.push(value);
        }
        let unseen = |t: &Value| tree(&t["root"], live).is_none();
        space["tabs"] = json!(with_unseen(new_tabs, was.map_or(&[][..], tabs), unseen));
        w.origin = Some(w.id.clone());
        out.push(space);
    }
    let unseen = |s: &Value| !doc.workspaces.iter().any(|w| w.id == s["id"].as_str().unwrap_or_default()) && tabs(s).iter().all(|t| tree(&t["root"], live).is_none());
    let mut document = if app.is_object() { app.clone() } else { json!({}) };
    document["spaces"] = json!(with_unseen(out, spaces(app), unseen));
    Ok(document)
}

/// Terminals in a space or tab their labels don't name, with the two ids: the app places what they say.
pub fn misplaced(doc: &Doc, terminals: &[Value]) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for w in doc.workspaces.iter().filter(|w| w.origin.as_deref() != Some(OTHERS)) {
        for tab in &w.tabs {
            for address in tab.root.terminals() {
                let labels = terminals.iter().find(|t| t["address"] == address.as_str()).map(|t| &t["labels"]);
                if labels.is_some_and(|l| l["herdr.space"] != w.id.as_str() || l["herdr.tab"] != tab.id.as_str()) {
                    out.push((address, w.id.clone(), tab.id.clone()));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document as the app wrote one (`~/.config`-free: taken from a running core): one tab with a mod beside a terminal.
    fn app() -> Value {
        let block = |id: &str, t: &str| json!({"content": {"terminal": t}, "id": id, "type": "block"});
        json!({"future": 1, "spaces": [
            {"folder": "/home", "id": "sp1", "name": "", "extra": true, "tabs": [
                {"id": "tb1", "name": "", "root": {"children": [block("p1", "t1")], "id": "tb1", "type": "row"}}]},
            {"folder": "/dots", "id": "sp2", "name": "dots", "tabs": [
                {"id": "tb2", "name": "claude", "root": {"children": [
                    {"content": {"terminal": "t2"}, "id": "p2", "size": {"weight": 3.0}, "type": "block"},
                    {"children": [block("p3", "t3"), block("p4", "t4")], "id": "c1", "size": {"weight": 1.0}, "type": "column"}],
                    "id": "tb2", "type": "row"}},
                {"id": "tb3", "name": "board", "root": {"children": [
                    {"content": {"mod": "board"}, "id": "m1", "type": "block"}, block("p5", "t5")], "id": "tb3", "type": "row"}}]},
            {"folder": "/x", "id": "sp3", "name": "", "tabs": [
                {"id": "tb4", "name": "", "root": {"children": [{"content": {"mod": "notes"}, "id": "m2", "type": "block"}], "id": "tb4", "type": "row"}}]},
        ]})
    }

    fn all(_: &str) -> bool {
        true
    }

    fn nowhere(_: &str) -> Option<String> {
        None
    }

    fn loaded() -> Doc {
        let mut doc = Doc::default();
        absorb(&mut doc, &app(), &all);
        doc
    }

    #[test]
    fn spaces_tabs_and_splits_read_as_the_app_has_them() {
        let doc = loaded();
        assert_eq!(doc.workspaces.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(), ["sp1", "sp2"], "sp3 shows no terminal");
        let tab = &doc.workspaces[1].tabs[0];
        assert_eq!((tab.id.as_str(), tab.label.as_str()), ("tb2", "claude"));
        let Node::Split { right, ratio, second, .. } = &tab.root else { panic!("{:?}", tab.root) };
        assert_eq!((*right, *ratio), (true, 0.75), "the weights");
        assert!(matches!(**second, Node::Split { right: false, .. }), "the column inside it");
    }

    #[test]
    fn nothing_changed_writes_the_same_document() {
        let mut doc = loaded();
        assert_eq!(written(&mut doc, &app(), &all, &nowhere).unwrap(), app());
    }

    #[test]
    fn a_rename_touches_only_the_name() {
        let mut doc = loaded();
        doc.rename_tab("tb2", "work");
        doc.rename_workspace("sp1", "home");
        let out = written(&mut doc, &app(), &all, &nowhere).unwrap();
        assert_eq!(out["spaces"][1]["tabs"][0]["name"], "work");
        assert_eq!(out["spaces"][0]["name"], "home");
        assert_eq!(out["spaces"][1]["tabs"][0]["root"], app()["spaces"][1]["tabs"][0]["root"]);
        assert_eq!((&out["spaces"][0]["extra"], &out["future"]), (&json!(true), &json!(1)), "what the app added stays");
    }

    #[test]
    fn a_new_space_and_tab_go_in_the_apps_shape() {
        let mut doc = loaded();
        doc.add_tab("sp1", "t8".into());
        let (workspace, tab) = doc.add_workspace("t9".into());
        let out = written(&mut doc, &app(), &all, &|t| (t == "t9").then(|| "/new".to_string())).unwrap();
        let spaces = out["spaces"].as_array().unwrap();
        let space = spaces.iter().find(|s| s["id"] == workspace.as_str()).unwrap();
        assert_eq!(space["folder"], "/new");
        assert_eq!(space["tabs"][0]["root"]["id"], tab.as_str(), "a tab's root has the tab's id");
        assert_eq!(space["tabs"][0]["root"]["type"], "row");
        assert_eq!(space["tabs"][0]["root"]["children"][0]["content"]["terminal"], "t9");
        assert_eq!(spaces[0]["tabs"].as_array().unwrap().len(), 2);
        assert!(spaces.iter().any(|s| s["id"] == "sp3"), "a space Shepherd cannot show stays");
        assert_eq!(doc.workspaces.iter().find(|w| w.id == workspace).unwrap().origin.as_deref(), Some(workspace.as_str()));
        let mut again = doc.clone();
        absorb(&mut again, &out, &all);
        assert_eq!(written(&mut again, &out, &all, &nowhere).unwrap(), out, "read back, it is the same");
    }

    #[test]
    fn a_split_keeps_pane_ids_and_comes_back_the_same() {
        let mut doc = loaded();
        doc.split("t1", "t7", false, true);
        let out = written(&mut doc, &app(), &all, &nowhere).unwrap();
        let root = &out["spaces"][0]["tabs"][0]["root"];
        assert_eq!((root["id"].as_str(), root["type"].as_str()), (Some("tb1"), Some("row")));
        assert_eq!(root["children"][0]["id"], "p1", "the app keeps a pane's view under its id");
        assert_eq!(root["children"][1]["size"]["weight"], 0.5);
        let mut again = Doc::default();
        absorb(&mut again, &out, &all);
        assert_eq!(again.workspaces[0].tabs[0].root.terminals(), ["t1", "t7"]);
    }

    #[test]
    fn closing_goes_through_and_a_gone_terminal_is_left_to_the_app() {
        let mut doc = loaded();
        doc.close_tab("tb1");
        let out = written(&mut doc, &app(), &all, &nowhere).unwrap();
        assert!(!out["spaces"].as_array().unwrap().iter().any(|s| s["id"] == "sp1"));

        // a list that came back without t1: its space hides here, and stays in the document
        let live = |t: &str| t != "t1";
        let mut doc = Doc::default();
        absorb(&mut doc, &app(), &live);
        assert!(!doc.workspaces.iter().any(|w| w.id == "sp1"));
        assert_eq!(written(&mut doc, &app(), &live, &nowhere).unwrap(), app());
    }

    #[test]
    fn what_is_hidden_keeps_its_place() {
        let mut document = app();
        let hidden = document["spaces"][2].clone();
        document["spaces"].as_array_mut().unwrap().insert(0, hidden);
        document["spaces"][0]["id"] = json!("sp0");
        let tabs = document["spaces"][2]["tabs"].as_array_mut().unwrap();
        tabs.insert(0, json!({"id": "tb9", "name": "", "root": {"children": [{"content": {"mod": "x"}, "id": "m9", "type": "block"}], "id": "tb9", "type": "row"}}));
        let mut doc = Doc::default();
        absorb(&mut doc, &document, &all);
        assert_eq!(written(&mut doc, &document, &all, &nowhere).unwrap(), document, "nothing to write, so nothing to fight over");
    }

    #[test]
    fn a_tab_with_a_mod_is_not_redrawn() {
        let mut doc = loaded();
        doc.split("t5", "t7", false, true);
        assert!(written(&mut doc, &app(), &all, &nowhere).unwrap_err().to_string().contains("Herdr app"));
    }

    #[test]
    fn what_is_each_clients_own_stays_and_shepherds_own_follow() {
        let mut doc = loaded();
        doc.focus_tab("tb2");
        doc.focus_pane("t3");
        doc.toggle_zoom("t3");
        let (own, _) = doc.add_workspace("t11".into());
        absorb(&mut doc, &app(), &all);
        let tab = &doc.workspaces[1].tabs[0];
        assert_eq!((tab.focused.as_str(), tab.zoomed), ("t3", true));
        assert_eq!(doc.shown_tab().unwrap().id, "tb2");
        assert_eq!(doc.workspaces.last().unwrap().id, own, "not written yet");
    }

    #[test]
    fn labels_follow_the_layout() {
        let doc = loaded();
        let terminals = [json!({"address": "t1", "labels": {"herdr.space": "sp1", "herdr.tab": "tb1"}}), json!({"address": "t2", "labels": {"shepherd": "1"}})];
        assert_eq!(misplaced(&doc, &terminals), [("t2".to_string(), "sp2".to_string(), "tb2".to_string())]);
    }
}
