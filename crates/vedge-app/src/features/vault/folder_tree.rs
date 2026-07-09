//! Folder navigation — a tree built from `Folder`-type entries plus the pure,
//! host-testable helpers that back it.
//!
//! A folder is just an `EntryType::Folder` entry; other entries (and sub-folders)
//! reference their parent via `meta.folder_id`, so the whole hierarchy is derived
//! client-side from the loaded `Vec<IndexEntryDto>` — no per-navigation IPC. The
//! tree feeds three surfaces: [`FolderTree`] (the sidebar), [`FolderBreadcrumb`]
//! (the path row), and the move/assign pickers in `folder_move`. Selecting a node
//! sets a [`FolderScope`] the page's `filter_and_sort` narrows on.
//!
//! `build_folder_tree` is **cycle-safe**: a folder whose parent chain loops (data
//! corruption) is grafted to the root instead of recursing forever, and every
//! downstream helper walks the resulting *acyclic* tree.

use crate::i18n::*;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use vedge_ipc::{EntryTypeDto, IndexEntryDto};
use vedge_ui::components::{Button, IconButton, Input, SidebarItem};
use vedge_ui::primitives::tokens::{Size, Variant};

/// A node in the folder hierarchy (acyclic — see module docs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderNode {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    /// Presentation, projected from the folder's index entry (`meta.color/icon`).
    pub color: Option<String>,
    pub icon: Option<String>,
    pub children: Vec<FolderNode>,
}

/// A tree node flattened for indented rendering / `Select` option labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatFolder {
    pub id: String,
    pub name: String,
    pub depth: usize,
    pub has_children: bool,
    pub color: Option<String>,
    pub icon: Option<String>,
}

/// The curated folder icon set. The stored `meta.icon` key maps back to an
/// `icondata` icon here; an unknown / absent key falls back to the plain folder.
/// Keep this list and the picker (`folder_customize`) in sync.
pub const FOLDER_ICONS: &[(&str, icondata::Icon)] = &[
    ("folder", i::FaFolderSolid),
    ("briefcase", i::FaBriefcaseSolid),
    ("house", i::FaHouseSolid),
    ("user", i::FaUserSolid),
    ("star", i::FaStarSolid),
    ("heart", i::FaHeartSolid),
    ("key", i::FaKeySolid),
    ("lock", i::FaLockSolid),
    ("globe", i::FaGlobeSolid),
    ("code", i::FaCodeSolid),
    ("credit-card", i::FaCreditCardSolid),
    ("tag", i::FaTagSolid),
    ("bookmark", i::FaBookmarkSolid),
    ("flag", i::FaFlagSolid),
    ("bell", i::FaBellSolid),
    ("gear", i::FaGearSolid),
];

/// Resolve a stored icon key to its `icondata` icon (default folder on miss).
#[must_use]
pub fn folder_icon_from_key(key: &str) -> icondata::Icon {
    FOLDER_ICONS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(i::FaFolderSolid, |(_, ic)| *ic)
}

/// The `(color, icon_key)` presentation of folder `id`, if it exists.
#[must_use]
pub fn folder_style(nodes: &[FolderNode], id: &str) -> (Option<String>, Option<String>) {
    find_node(nodes, id).map_or((None, None), |n| (n.color.clone(), n.icon.clone()))
}

/// Which slice of the index the list shows. `All` is the cleared/default view;
/// `Unfiled` is the dedicated root-only node (`folder_id == None`); `Folder(id)`
/// is one folder's direct contents.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FolderScope {
    #[default]
    All,
    Unfiled,
    Folder(String),
}

/// Whether `entry` belongs to `scope` (`All` matches everything).
#[must_use]
pub fn scope_matches(scope: &FolderScope, entry: &IndexEntryDto) -> bool {
    match scope {
        FolderScope::All => true,
        FolderScope::Unfiled => entry.folder_id.is_none(),
        FolderScope::Folder(id) => entry.folder_id.as_deref() == Some(id.as_str()),
    }
}

/// Build the folder hierarchy from all `Folder`-type entries. A parent that is
/// missing, non-folder, or self roots the node; folders trapped in a cycle are
/// grafted to the root. Siblings (and roots) are name-sorted for a stable order.
#[must_use]
pub fn build_folder_tree(items: &[IndexEntryDto]) -> Vec<FolderNode> {
    let folder_ids: HashSet<&str> = items
        .iter()
        .filter(|e| e.entry_type == EntryTypeDto::Folder)
        .map(|e| e.id.as_str())
        .collect();

    // Per-folder presentation, keyed by id: (name, color, icon).
    type Attrs = (String, Option<String>, Option<String>);
    let mut attrs: HashMap<String, Attrs> = HashMap::new();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    let mut root_ids: Vec<String> = Vec::new();
    for e in items
        .iter()
        .filter(|e| e.entry_type == EntryTypeDto::Folder)
    {
        attrs.insert(
            e.id.clone(),
            (e.name.clone(), e.color.clone(), e.icon.clone()),
        );
        // Normalize the parent: it must reference an existing *folder* and not be
        // the node itself; anything else roots the node.
        let parent = e
            .folder_id
            .as_deref()
            .filter(|p| *p != e.id && folder_ids.contains(p));
        match parent {
            Some(p) => children.entry(p.to_owned()).or_default().push(e.id.clone()),
            None => root_ids.push(e.id.clone()),
        }
    }

    let by_name = |a: &String, b: &String| {
        let na = attrs
            .get(a)
            .map(|(n, ..)| n.to_lowercase())
            .unwrap_or_default();
        let nb = attrs
            .get(b)
            .map(|(n, ..)| n.to_lowercase())
            .unwrap_or_default();
        na.cmp(&nb).then_with(|| a.cmp(b))
    };
    root_ids.sort_by(by_name);
    for v in children.values_mut() {
        v.sort_by(by_name);
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut roots: Vec<FolderNode> = Vec::new();
    for id in &root_ids {
        if !visited.contains(id) {
            roots.push(build_node(id, None, &children, &attrs, &mut visited));
        }
    }
    // Cycle-trapped folders (never reached from a real root) → graft to root so
    // nothing is lost and rendering can't loop.
    for e in items
        .iter()
        .filter(|e| e.entry_type == EntryTypeDto::Folder)
    {
        if !visited.contains(&e.id) {
            roots.push(build_node(&e.id, None, &children, &attrs, &mut visited));
        }
    }
    roots.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    roots
}

fn build_node(
    id: &str,
    parent: Option<&str>,
    children: &HashMap<String, Vec<String>>,
    attrs: &HashMap<String, (String, Option<String>, Option<String>)>,
    visited: &mut HashSet<String>,
) -> FolderNode {
    visited.insert(id.to_owned());
    let mut child_nodes = Vec::new();
    if let Some(kids) = children.get(id) {
        for k in kids {
            if !visited.contains(k) {
                child_nodes.push(build_node(k, Some(id), children, attrs, visited));
            }
        }
    }
    let (name, color, icon) = attrs.get(id).cloned().unwrap_or_default();
    FolderNode {
        id: id.to_owned(),
        name,
        parent: parent.map(str::to_owned),
        color,
        icon,
        children: child_nodes,
    }
}

/// Depth-first flatten. A node whose id is in `collapsed` omits its subtree; pass
/// an empty set to flatten the whole tree (used for `Select` option lists).
#[must_use]
pub fn flatten_tree(nodes: &[FolderNode], collapsed: &HashSet<String>) -> Vec<FlatFolder> {
    let mut out = Vec::new();
    flatten_into(nodes, 0, collapsed, &mut out);
    out
}

fn flatten_into(
    nodes: &[FolderNode],
    depth: usize,
    collapsed: &HashSet<String>,
    out: &mut Vec<FlatFolder>,
) {
    for n in nodes {
        out.push(FlatFolder {
            id: n.id.clone(),
            name: n.name.clone(),
            depth,
            has_children: !n.children.is_empty(),
            color: n.color.clone(),
            icon: n.icon.clone(),
        });
        if !collapsed.contains(&n.id) {
            flatten_into(&n.children, depth.saturating_add(1), collapsed, out);
        }
    }
}

/// The breadcrumb chain root→…→`id` as `(id, name)` pairs (empty if `id` isn't a
/// known folder).
#[must_use]
pub fn folder_path(nodes: &[FolderNode], id: &str) -> Vec<(String, String)> {
    let mut idx: HashMap<String, (String, Option<String>)> = HashMap::new();
    index_nodes(nodes, &mut idx);
    let mut chain: Vec<(String, String)> = Vec::new();
    let mut cur = Some(id.to_owned());
    while let Some(c) = cur {
        let Some((name, parent)) = idx.get(&c) else {
            break;
        };
        chain.push((c.clone(), name.clone()));
        cur = parent.clone();
    }
    chain.reverse();
    chain
}

fn index_nodes(nodes: &[FolderNode], idx: &mut HashMap<String, (String, Option<String>)>) {
    for n in nodes {
        idx.insert(n.id.clone(), (n.name.clone(), n.parent.clone()));
        index_nodes(&n.children, idx);
    }
}

/// The ids of every folder beneath `id` (self excluded).
#[must_use]
pub fn descendants_of(nodes: &[FolderNode], id: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(node) = find_node(nodes, id) {
        collect_ids(&node.children, &mut out);
    }
    out
}

/// The display name of folder `id`, if it exists (for the detail "Folder" row).
#[must_use]
pub fn folder_display_name(nodes: &[FolderNode], id: &str) -> Option<String> {
    find_node(nodes, id).map(|n| n.name.clone())
}

fn find_node<'a>(nodes: &'a [FolderNode], id: &str) -> Option<&'a FolderNode> {
    for n in nodes {
        if n.id == id {
            return Some(n);
        }
        if let Some(found) = find_node(&n.children, id) {
            return Some(found);
        }
    }
    None
}

fn collect_ids(nodes: &[FolderNode], out: &mut HashSet<String>) {
    for n in nodes {
        out.insert(n.id.clone());
        collect_ids(&n.children, out);
    }
}

/// Ids of everything *inside* `root_id` — all entries whose `folder_id` is
/// `root_id` or any descendant folder (which sweeps in the descendant sub-folders
/// themselves) — with `root_id` excluded. Backs the recursive "Empty folder".
#[must_use]
pub fn subtree_contents(
    items: &[IndexEntryDto],
    nodes: &[FolderNode],
    root_id: &str,
) -> Vec<String> {
    let mut target: HashSet<String> = descendants_of(nodes, root_id);
    target.insert(root_id.to_owned());
    items
        .iter()
        .filter(|e| e.id != root_id && e.folder_id.as_deref().is_some_and(|f| target.contains(f)))
        .map(|e| e.id.clone())
        .collect()
}

/// Folders an entry may be moved into: every folder except the entry itself and
/// its descendants (a folder can't move inside its own subtree). For a non-folder
/// entry nothing is excluded.
#[must_use]
pub fn available_move_targets(nodes: &[FolderNode], moving_entry_id: &str) -> Vec<FlatFolder> {
    let blocked = descendants_of(nodes, moving_entry_id);
    flatten_tree(nodes, &HashSet::new())
        .into_iter()
        .filter(|f| f.id != moving_entry_id && !blocked.contains(&f.id))
        .collect()
}

/// Per-scope entry counts computed once: `(total, unfiled, folder_id -> direct
/// count)`. Folder-type entries are excluded — they're navigated in the tree, not
/// listed, so the badges match what selecting a scope actually shows.
fn folder_counts(items: &[IndexEntryDto]) -> (usize, usize, HashMap<String, usize>) {
    let mut per_folder: HashMap<String, usize> = HashMap::new();
    let mut unfiled = 0usize;
    let mut total = 0usize;
    for e in items {
        if e.entry_type == EntryTypeDto::Folder {
            continue;
        }
        total = total.saturating_add(1);
        match &e.folder_id {
            Some(f) => *per_folder.entry(f.clone()).or_default() += 1,
            None => unfiled = unfiled.saturating_add(1),
        }
    }
    (total, unfiled, per_folder)
}

/// Sentinel keys for the two root drop targets (folder rows use their own id).
const DROP_ALL: &str = "\u{0}all";
const DROP_UNFILED: &str = "\u{0}unfiled";

/// The folder-tree sidebar: an "All items" node, an "Unfiled" node, the nested
/// folders (indented, collapsible), and an inline "+ New folder" input. Selecting
/// a node sets `scope`; creating a folder fires `on_new_folder((parent, name))`
/// with the current folder as the parent. Rows are also **drop targets**: a table
/// row dragged onto a folder moves there (onto "All"/"Unfiled" → root), guarded so
/// a folder can't be dropped into its own subtree.
#[component]
pub fn FolderTree(
    #[prop(into)] items: Signal<Vec<IndexEntryDto>>,
    #[prop(into)] folders: Signal<Vec<FolderNode>>,
    scope: RwSignal<FolderScope>,
    on_new_folder: Callback<(Option<String>, String)>,
    /// `(entry_id, destination)` — persist a drag-and-drop move.
    on_move: Callback<(String, Option<String>)>,
    /// Delete a folder (routes through the page's empty-first logic).
    on_delete: Callback<String>,
    /// `(folder_id, new_name)` — persist an inline rename.
    on_rename: Callback<(String, String)>,
    /// Open the color/icon customize dialog for a folder id.
    on_customize: Callback<String>,
    /// Extra sidebar content rendered below the folder list (the smart folders).
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let i18n = use_i18n();
    let collapsed = RwSignal::new(HashSet::<String>::new());
    let new_name = RwSignal::new(String::new());
    // Which drop target is currently under the pointer (for the highlight ring).
    let drag_over = RwSignal::new(String::new());
    // The folder id currently being renamed inline (+ its edit buffer).
    let renaming = RwSignal::new(Option::<String>::None);
    let rename_value = RwSignal::new(String::new());
    // Whether the compact "new folder" input is revealed.
    let show_new = RwSignal::new(false);
    // Resizable width as a fraction of the viewport, clamped 20–40%.
    let panel_frac = RwSignal::new(0.22_f64);
    let resizing = RwSignal::new(false);
    let aside_ref = NodeRef::<leptos::html::Aside>::new();
    let handle_ref = NodeRef::<leptos::html::Div>::new();
    let counts = Memo::new(move |_| folder_counts(&items.get()));

    let rows = move || flatten_tree(&folders.get(), &collapsed.get());

    // Read the dragged entry id off a drop event's dataTransfer.
    let dragged_id = |ev: &web_sys::DragEvent| -> Option<String> {
        ev.data_transfer()
            .and_then(|dt| dt.get_data("text/plain").ok())
            .filter(|s| !s.is_empty())
    };

    // Drop onto a folder `dest`: allowed only if `dest` is a valid target for the
    // dragged entry (not itself / a descendant).
    let drop_on_folder = move |ev: web_sys::DragEvent, dest: String| {
        ev.prevent_default();
        drag_over.set(String::new());
        if let Some(id) = dragged_id(&ev) {
            let ok = available_move_targets(&folders.get_untracked(), &id)
                .iter()
                .any(|f| f.id == dest);
            if ok {
                on_move.run((id, Some(dest)));
            }
        }
    };
    // Drop onto All / Unfiled: move to root (always valid).
    let drop_on_root = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        drag_over.set(String::new());
        if let Some(id) = dragged_id(&ev) {
            on_move.run((id, None));
        }
    };

    // Pointer-capture resize: the handle keeps receiving move/up even when the
    // pointer leaves it, so no window-level listeners are needed.
    let start_resize = move |ev: web_sys::PointerEvent| {
        ev.prevent_default();
        if let Some(el) = handle_ref.get() {
            let _ = el.set_pointer_capture(ev.pointer_id());
        }
        resizing.set(true);
    };
    let on_resize = move |ev: web_sys::PointerEvent| {
        if !resizing.get_untracked() {
            return;
        }
        let win_w = window()
            .inner_width()
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(1280.0);
        let left = aside_ref
            .get()
            .map_or(0.0, |el| el.get_bounding_client_rect().left());
        let frac = ((f64::from(ev.client_x()) - left) / win_w).clamp(0.20, 0.40);
        panel_frac.set(frac);
    };
    let end_resize = move |ev: web_sys::PointerEvent| {
        resizing.set(false);
        if let Some(el) = handle_ref.get() {
            let _ = el.release_pointer_capture(ev.pointer_id());
        }
    };

    let create = move || {
        let name = new_name.get().trim().to_owned();
        if name.is_empty() {
            return;
        }
        let parent = match scope.get() {
            FolderScope::Folder(id) => Some(id),
            _ => None,
        };
        on_new_folder.run((parent, name));
        new_name.set(String::new());
        show_new.set(false);
    };

    view! {
        <aside
            node_ref=aside_ref
            class="relative shrink-0 flex flex-col border border-border rounded-lg bg-primary-muted"
            style:width=move || format!("{}vw", panel_frac.get() * 100.0)
        >
            <div class="flex-1 overflow-auto flex flex-col gap-0.5 p-2">
                <div class="flex items-center justify-between px-1 py-0.5">
                    <span class="text-foreground/50 text-xs uppercase tracking-wider">
                        {move || t!(i18n, vault.folders_label)}
                    </span>
                    <IconButton
                        variant=Variant::Ghost
                        size=Size::Xs
                        class="text-foreground/40"
                        aria_label=Signal::derive(move || t_string!(i18n, vault.folder_new).to_string())
                        on:click=move |_: web_sys::MouseEvent| show_new.update(|v| *v = !*v)
                    >
                        <Icon attr:aria-hidden="true" icon=i::FaPlusSolid width="12" height="12" />
                    </IconButton>
                </div>

                // Compact new-folder input, revealed by the header "+".
                <Show when=move || show_new.get()>
                    <Input
                        id="folder-new"
                        class="mb-1"
                        autofocus=true
                        placeholder=Signal::derive(move || {
                            t_string!(i18n, vault.folder_new_placeholder).to_string()
                        })
                        value=Signal::derive(move || new_name.get())
                        on_input=Callback::new(move |v: String| new_name.set(v))
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            match ev.key().as_str() {
                                "Enter" => {
                                    ev.prevent_default();
                                    create();
                                }
                                "Escape" => {
                                    ev.prevent_default();
                                    new_name.set(String::new());
                                    show_new.set(false);
                                }
                                _ => {}
                            }
                        }
                    />
                </Show>

                // All items — the cleared scope (also a "move to root" drop target).
            // Drop-target highlight rides the reactive `class`; drag handlers spread
            // onto SidebarItem's root button (same element the click nav uses).
            <SidebarItem
                label=Signal::derive(move || t_string!(i18n, vault.folder_root).to_string())
                icon=Box::new(|| {
                    view! { <Icon icon=i::FaLayerGroupSolid width="14" height="14" /> }.into_any()
                })
                badge=Box::new(move || view! { {move || counts.get().0} }.into_any())
                selected=Signal::derive(move || matches!(scope.get(), FolderScope::All))
                on_click=Callback::new(move |_: ()| scope.set(FolderScope::All))
                class=Signal::derive(move || {
                    if drag_over.get() == DROP_ALL {
                        "ring-1 ring-primary bg-primary/15".to_string()
                    } else {
                        String::new()
                    }
                })
                on:dragover=move |ev: web_sys::DragEvent| ev.prevent_default()
                on:dragenter=move |_: web_sys::DragEvent| drag_over.set(DROP_ALL.to_owned())
                on:dragleave=move |_: web_sys::DragEvent| drag_over.set(String::new())
                on:drop=drop_on_root
            />

            // Unfiled — entries with no folder (drop here = move to root).
            <SidebarItem
                label=Signal::derive(move || t_string!(i18n, vault.folder_unfiled).to_string())
                icon=Box::new(|| {
                    view! { <Icon icon=i::FaInboxSolid width="14" height="14" /> }.into_any()
                })
                badge=Box::new(move || view! { {move || counts.get().1} }.into_any())
                selected=Signal::derive(move || matches!(scope.get(), FolderScope::Unfiled))
                on_click=Callback::new(move |_: ()| scope.set(FolderScope::Unfiled))
                class=Signal::derive(move || {
                    if drag_over.get() == DROP_UNFILED {
                        "ring-1 ring-primary bg-primary/15".to_string()
                    } else {
                        String::new()
                    }
                })
                on:dragover=move |ev: web_sys::DragEvent| ev.prevent_default()
                on:dragenter=move |_: web_sys::DragEvent| drag_over.set(DROP_UNFILED.to_owned())
                on:dragleave=move |_: web_sys::DragEvent| drag_over.set(String::new())
                on:drop=drop_on_root
            />

            <For
                each=rows
                key=|f| (
                    f.id.clone(),
                    f.name.clone(),
                    f.depth,
                    f.has_children,
                    f.color.clone(),
                    f.icon.clone(),
                )
                children=move |f| {
                    let sel_id_a = f.id.clone();
                    let sel_id_b = f.id.clone();
                    let click_id = f.id.clone();
                    let chev_id = f.id.clone();
                    let col_id = f.id.clone();
                    let cnt_id = f.id.clone();
                    let over_id = f.id.clone();
                    // Three clones — one per single-token drag-over ring toggle below
                    // (`class=(...)` takes one token; multi-token strings break at runtime).
                    let over_id2 = f.id.clone();
                    let over_id3 = f.id.clone();
                    let enter_id = f.id.clone();
                    let drop_id = f.id.clone();
                    let drag_check_id = f.id.clone();
                    let dragstart_id = f.id.clone();
                    let rn_check_id = f.id.clone();
                    let start_id = f.id.clone();
                    let start_nm = f.name.clone();
                    let cust_id = f.id.clone();
                    let del_id = f.id.clone();
                    let name = f.name.clone();
                    let depth = f.depth;
                    let has_children = f.has_children;
                    let icon_data = folder_icon_from_key(f.icon.as_deref().unwrap_or_default());
                    let icon_color = f.color.clone();
                    let indent = format!("padding-left:{}rem", 0.25 + depth as f64 * 0.85);
                    // Commit an inline rename (shared by Enter + blur). Untracked
                    // reads — invoked from event handlers, no-op once cleared.
                    let commit = move || {
                        if let Some(id) = renaming.get_untracked() {
                            on_rename.run((id, rename_value.get_untracked()));
                        }
                        renaming.set(None);
                    };
                    view! {
                        <div
                            class="group flex items-center gap-1 w-full px-1 py-1.5 rounded text-sm cursor-pointer hover:bg-primary/5"
                            class=("bg-primary/10", move || matches!(scope.get(), FolderScope::Folder(ref s) if *s == sel_id_a))
                            class=("text-primary", move || matches!(scope.get(), FolderScope::Folder(ref s) if *s == sel_id_b))
                            class=("ring-1", move || drag_over.get() == over_id)
                            class=("ring-primary", move || drag_over.get() == over_id2)
                            class=("bg-primary/15", move || drag_over.get() == over_id3)
                            style=indent
                            // Draggable as a move source (to reparent under another
                            // folder / root) — but not while its rename input is open,
                            // or the parent drag would hijack text selection.
                            draggable=move || {
                                (renaming.get().as_deref() != Some(drag_check_id.as_str()))
                                    .then_some("true")
                            }
                            on:dragstart=move |ev: web_sys::DragEvent| {
                                if let Some(dt) = ev.data_transfer() {
                                    let _ = dt.set_data("text/plain", &dragstart_id);
                                }
                            }
                            on:click=move |_: web_sys::MouseEvent| {
                                scope.set(FolderScope::Folder(click_id.clone()));
                            }
                            on:dragover=move |ev: web_sys::DragEvent| ev.prevent_default()
                            on:dragenter=move |_: web_sys::DragEvent| drag_over.set(enter_id.clone())
                            on:dragleave=move |_: web_sys::DragEvent| drag_over.set(String::new())
                            on:drop=move |ev: web_sys::DragEvent| drop_on_folder(ev, drop_id.clone())
                        >
                            {if has_children {
                                leptos::either::Either::Left(
                                    view! {
                                        <IconButton
                                            variant=Variant::Ghost
                                            size=Size::Xs
                                            class="text-foreground/40"
                                            aria_label=Signal::derive(move || t_string!(i18n, vault.folder_toggle).to_string())
                                            on:click=move |ev: web_sys::MouseEvent| {
                                                ev.stop_propagation();
                                                collapsed
                                                    .update(|c| {
                                                        if !c.remove(&chev_id) {
                                                            c.insert(chev_id.clone());
                                                        }
                                                    });
                                            }
                                        >
                                            {move || {
                                                if collapsed.get().contains(&col_id) {
                                                    leptos::either::Either::Left(
                                                        view! { <Icon attr:aria-hidden="true" icon=i::FaChevronRightSolid width="10" height="10" /> },
                                                    )
                                                } else {
                                                    leptos::either::Either::Right(
                                                        view! { <Icon attr:aria-hidden="true" icon=i::FaChevronDownSolid width="10" height="10" /> },
                                                    )
                                                }
                                            }}
                                        </IconButton>
                                    },
                                )
                            } else {
                                leptos::either::Either::Right(view! { <span class="w-4 shrink-0"></span> })
                            }}
                            <span
                                class="flex shrink-0 text-foreground/50"
                                style:color=move || icon_color.clone().unwrap_or_default()
                            >
                                <Icon attr:aria-hidden="true" icon=icon_data width="14" height="14" />
                            </span>
                            {move || {
                                if renaming.get().as_deref() == Some(rn_check_id.as_str()) {
                                    leptos::either::Either::Left(
                                        view! {
                                            <Input
                                                id="folder-rename"
                                                class="flex-1 min-w-0"
                                                autofocus=true
                                                value=Signal::derive(move || rename_value.get())
                                                on_input=Callback::new(move |v: String| rename_value.set(v))
                                                on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()
                                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                    match ev.key().as_str() {
                                                        "Enter" => {
                                                            ev.prevent_default();
                                                            commit();
                                                        }
                                                        "Escape" => {
                                                            ev.prevent_default();
                                                            renaming.set(None);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                                on:focusout=move |_: web_sys::FocusEvent| commit()
                                            />
                                        },
                                    )
                                } else {
                                    let name = name.clone();
                                    let name_title = name.clone();
                                    leptos::either::Either::Right(
                                        view! {
                                            <span class="flex-1 truncate" title=name_title>
                                                {name}
                                            </span>
                                        },
                                    )
                                }
                            }}
                            <span class="shrink-0 text-[10px] text-foreground/40">
                                {move || counts.get().2.get(&cnt_id).copied().unwrap_or(0)}
                            </span>
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Xs
                                class="shrink-0 opacity-0 group-hover:opacity-100 text-foreground/40"
                                aria_label=Signal::derive(move || t_string!(i18n, vault.folder_customize).to_string())
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    on_customize.run(cust_id.clone());
                                }
                            >
                                <Icon attr:aria-hidden="true" icon=i::FaPaletteSolid width="10" height="10" />
                            </IconButton>
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Xs
                                class="shrink-0 opacity-0 group-hover:opacity-100 text-foreground/40"
                                aria_label=Signal::derive(move || t_string!(i18n, vault.folder_rename).to_string())
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    renaming.set(Some(start_id.clone()));
                                    rename_value.set(start_nm.clone());
                                }
                            >
                                <Icon attr:aria-hidden="true" icon=i::FaPenSolid width="10" height="10" />
                            </IconButton>
                            <IconButton
                                variant=Variant::Ghost
                                size=Size::Xs
                                class="shrink-0 opacity-0 group-hover:opacity-100 text-foreground/40 hover:text-danger"
                                aria_label=Signal::derive(move || t_string!(i18n, vault.folder_delete).to_string())
                                on:click=move |ev: web_sys::MouseEvent| {
                                    ev.stop_propagation();
                                    on_delete.run(del_id.clone());
                                }
                            >
                                <Icon attr:aria-hidden="true" icon=i::BiTrashRegular width="10" height="10" />
                            </IconButton>
                        </div>
                    }
                }
            />

            // Optional extra sidebar content (e.g. the smart-folders section),
            // rendered below the folder list inside the same scrollable aside.
            {children.map(|c| c())}
            </div>

            // Drag handle — pointer-capture resize (20–40vw).
            <div
                node_ref=handle_ref
                class="absolute top-0 right-0 h-full w-1.5 cursor-col-resize hover:bg-primary/40 rounded-r-lg touch-none"
                class=("bg-primary/40", move || resizing.get())
                aria-hidden="true"
                on:pointerdown=start_resize
                on:pointermove=on_resize
                on:pointerup=end_resize
            ></div>
        </aside>
    }
}

/// The current-folder path: a leading "All items" plus each ancestor segment, all
/// clickable to navigate. Renders `All › Unfiled` or `All › a › b › current`.
#[component]
pub fn FolderBreadcrumb(
    #[prop(into)] folders: Signal<Vec<FolderNode>>,
    scope: RwSignal<FolderScope>,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="flex items-center gap-1 text-sm text-foreground/60 flex-wrap">
            <Button
                variant=Variant::Link
                class="text-foreground/60"
                on:click=move |_: web_sys::MouseEvent| scope.set(FolderScope::All)
            >
                {move || t!(i18n, vault.folder_root)}
            </Button>
            {move || match scope.get() {
                FolderScope::All => ().into_any(),
                FolderScope::Unfiled => {
                    view! {
                        <span class="text-foreground/30">"/"</span>
                        <span class="text-text-primary">{move || t!(i18n, vault.folder_unfiled)}</span>
                    }
                        .into_any()
                }
                FolderScope::Folder(id) => {
                    let nodes = folders.get();
                    folder_path(&nodes, &id)
                        .into_iter()
                        .map(|(pid, pname)| {
                            let target = pid.clone();
                            let (color, icon) = folder_style(&nodes, &pid);
                            let icon_data = folder_icon_from_key(icon.as_deref().unwrap_or_default());
                            view! {
                                <span class="text-foreground/30">"/"</span>
                                <Button
                                    variant=Variant::Link
                                    class="gap-1 text-foreground/60"
                                    on:click=move |_: web_sys::MouseEvent| {
                                        scope.set(FolderScope::Folder(target.clone()));
                                    }
                                >
                                    <span
                                        class="flex shrink-0"
                                        style:color=color.unwrap_or_default()
                                    >
                                        <Icon attr:aria-hidden="true" icon=icon_data width="11" height="11" />
                                    </span>
                                    {pname}
                                </Button>
                            }
                        })
                        .collect_view()
                        .into_any()
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vedge_ipc::{EntryTypeDto, IndexEntryDto};

    fn base(id: &str, name: &str, ty: EntryTypeDto, folder: Option<&str>) -> IndexEntryDto {
        IndexEntryDto {
            id: id.into(),
            name: name.into(),
            entry_type: ty,
            url: None,
            favicon_url: None,
            tag_ids: vec![],
            folder_id: folder.map(str::to_owned),
            is_favorite: false,
            color: None,
            icon: None,
            sort_order: 0,
            is_trashed: false,
            cipher_suite: 1,
            created_at: "2026-07-07T00:00:00.000Z".into(),
            updated_at: "2026-07-07T00:00:00.000Z".into(),
            accessed_at: None,
        }
    }
    fn folder(id: &str, name: &str, parent: Option<&str>) -> IndexEntryDto {
        base(id, name, EntryTypeDto::Folder, parent)
    }
    fn entry(id: &str, parent: Option<&str>) -> IndexEntryDto {
        base(id, id, EntryTypeDto::Login, parent)
    }

    fn flat_ids(nodes: &[FolderNode]) -> Vec<String> {
        flatten_tree(nodes, &HashSet::new())
            .into_iter()
            .map(|f| f.id)
            .collect()
    }

    #[test]
    fn build_nests_children_under_parent() {
        let items = vec![
            folder("a", "Alpha", None),
            folder("b", "Bravo", Some("a")),
            folder("c", "Charlie", Some("b")),
        ];
        let tree = build_folder_tree(&items);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, "a");
        assert_eq!(tree[0].children[0].id, "b");
        assert_eq!(tree[0].children[0].children[0].id, "c");
        assert_eq!(flat_ids(&tree), ["a", "b", "c"]);
    }

    #[test]
    fn build_roots_orphans_with_missing_or_nonfolder_parent() {
        let items = vec![
            folder("a", "Alpha", Some("ghost")), // parent doesn't exist → root
            folder("b", "Bravo", Some("login1")), // parent is a non-folder → root
            entry("login1", None),
        ];
        let tree = build_folder_tree(&items);
        let ids: Vec<&str> = tree.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]); // both rooted, name-sorted
    }

    #[test]
    fn build_is_cycle_safe() {
        // a -> b -> a : a mutual cycle must not loop and must keep both nodes.
        let items = vec![
            folder("a", "Alpha", Some("b")),
            folder("b", "Bravo", Some("a")),
        ];
        let tree = build_folder_tree(&items);
        let mut ids = flat_ids(&tree);
        ids.sort();
        assert_eq!(ids, ["a", "b"]);
    }

    #[test]
    fn folder_path_walks_to_root() {
        let items = vec![
            folder("a", "Alpha", None),
            folder("b", "Bravo", Some("a")),
            folder("c", "Charlie", Some("b")),
        ];
        let tree = build_folder_tree(&items);
        let path: Vec<String> = folder_path(&tree, "c")
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(path, ["a", "b", "c"]);
    }

    #[test]
    fn descendants_excludes_self_includes_subtree() {
        let items = vec![
            folder("a", "Alpha", None),
            folder("b", "Bravo", Some("a")),
            folder("c", "Charlie", Some("b")),
        ];
        let tree = build_folder_tree(&items);
        let d = descendants_of(&tree, "a");
        assert_eq!(d, HashSet::from(["b".to_owned(), "c".to_owned()]));
        assert!(descendants_of(&tree, "c").is_empty());
    }

    #[test]
    fn scope_matches_all_unfiled_and_folder() {
        let root = entry("root1", None);
        let in_a = entry("in_a", Some("a"));
        // `All` matches everything.
        assert!(scope_matches(&FolderScope::All, &root));
        assert!(scope_matches(&FolderScope::All, &in_a));
        // `Unfiled` matches only entries with no folder.
        assert!(scope_matches(&FolderScope::Unfiled, &root));
        assert!(!scope_matches(&FolderScope::Unfiled, &in_a));
        // `Folder(id)` matches only that folder's direct members.
        let a = FolderScope::Folder("a".into());
        assert!(scope_matches(&a, &in_a));
        assert!(!scope_matches(&a, &root));
    }

    #[test]
    fn subtree_contents_is_recursive_and_excludes_root() {
        let items = vec![
            folder("a", "Alpha", None),
            entry("e1", Some("a")),          // direct child entry
            folder("b", "Bravo", Some("a")), // sub-folder
            entry("e2", Some("b")),          // entry inside the sub-folder
            entry("outside", None),
        ];
        let tree = build_folder_tree(&items);
        let mut got = subtree_contents(&items, &tree, "a");
        got.sort();
        assert_eq!(got, ["b", "e1", "e2"]);
    }

    #[test]
    fn folder_icon_from_key_known_and_fallback() {
        assert_eq!(folder_icon_from_key("briefcase"), i::FaBriefcaseSolid);
        assert_eq!(folder_icon_from_key("star"), i::FaStarSolid);
        // Unknown / empty keys fall back to the plain folder.
        assert_eq!(folder_icon_from_key("nope"), i::FaFolderSolid);
        assert_eq!(folder_icon_from_key(""), i::FaFolderSolid);
    }

    #[test]
    fn build_tree_carries_color_and_icon() {
        let mut a = folder("a", "Alpha", None);
        a.color = Some("#ef4444".into());
        a.icon = Some("star".into());
        let tree = build_folder_tree(&[a]);
        assert_eq!(tree[0].color.as_deref(), Some("#ef4444"));
        assert_eq!(tree[0].icon.as_deref(), Some("star"));
        // …and the flattened row + style lookup expose the same.
        let flat = flatten_tree(&tree, &HashSet::new());
        assert_eq!(flat[0].icon.as_deref(), Some("star"));
        assert_eq!(
            folder_style(&tree, "a"),
            (Some("#ef4444".to_owned()), Some("star".to_owned()))
        );
    }

    #[test]
    fn move_targets_exclude_self_and_descendants() {
        let items = vec![
            folder("a", "Alpha", None),
            folder("b", "Bravo", Some("a")),
            folder("c", "Charlie", None),
        ];
        let tree = build_folder_tree(&items);
        // Moving folder `a`: cannot target `a` or its descendant `b`.
        let for_a: Vec<String> = available_move_targets(&tree, "a")
            .into_iter()
            .map(|f| f.id)
            .collect();
        assert_eq!(for_a, ["c"]);
        // Moving a plain entry: every folder is a valid target.
        let for_entry: Vec<String> = available_move_targets(&tree, "some-login")
            .into_iter()
            .map(|f| f.id)
            .collect();
        assert_eq!(for_entry, ["a", "b", "c"]);
    }
}
