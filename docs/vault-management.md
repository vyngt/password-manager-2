# Vault Management - Feature TODO

> Route: `/v` (VPage) - The core feature of vedge.
> Manages credential items (login, SSH, env, certs) with full CRUD, multiple view modes, search/filter, and bulk operations.

---

## Data Model (Backend - already implemented)

```
VaultItem {
  id: Uuid
  title: String
  kind: VaultItemKind (Credential, ...)
  data: VaultItemData (tagged enum)
    -> Credential { identifier, password, url }
  created_at, updated_at
}

Folder {
  id: Uuid
  name: String
  parent_id: Option<Uuid>  // hierarchical
}
```

**Available Tauri Commands:**
- `list_vault_items(pagination)` -> `PaginationOutput<VaultItem>`
- `create_vault_item(input)` -> `VaultItem`
- `get_vault_item(id)` -> `VaultItem`
- `update_vault_item(id, data)` -> `VaultItem`
- `delete_vault_item(id)` -> `VaultItem`
- `export_vault_items(path)` -> `()`
- `import_vault_items(path)` -> `()`
- `generate_password(options)` -> `String`

---

## Page Layout

```
+--------------------------------------------------------------+
| Toolbar                                                       |
| [Search] [Filter v] [Sort v] | [Table][List][Tree] | [+ New] |
+--------------------------------------------------------------+
| Sidebar (folders)  |  Content Area                            |
| - All Items        |  (view changes based on mode)            |
| - Logins           |                                          |
| - SSH Keys         |                                          |
| - Environments     |                                          |
| - Certificates     |                                          |
| - > Custom Folder  |                                          |
|   - Subfolder      |                                          |
+--------------------------------------------------------------+
| Status Bar: "42 items" | Pagination                           |
+--------------------------------------------------------------+
```

---

## Screens & Tasks

### 1. Toolbar / Action Bar

- [ ] **Search bar** - real-time filter by title, identifier, url
  - Debounced input (300ms)
  - Highlight matching text in results
  - Keyboard shortcut: `Ctrl+K` or `/` to focus
- [ ] **Filter dropdown** - filter by:
  - Kind (Credential, SSH, Env, Cert)
  - Folder
  - Date range (created/updated)
- [ ] **Sort dropdown** - sort by:
  - Title (A-Z / Z-A)
  - Date created (newest/oldest)
  - Date updated (newest/oldest)
- [ ] **View mode toggle** - icon buttons to switch between Table / List / Tree
  - Persist selection in local storage
- [ ] **"+ New" button** - opens create form/modal

### 2. View Modes

#### 2a. Table View (default)

```
| Title       | Identifier      | URL              | Kind       | Updated    | Actions    |
|-------------|-----------------|------------------|------------|------------|------------|
| GitHub      | user@email.com  | github.com       | Credential | 2 days ago | [E] [C] [D]|
| AWS Prod    | admin           | console.aws...   | Credential | 1 week ago | [E] [C] [D]|
```

- [ ] Sortable column headers (click to toggle asc/desc)
- [ ] Resizable columns
- [ ] Row selection (checkbox) for bulk actions
- [ ] Inline copy buttons (copy identifier, copy password)
- [ ] Row hover -> show quick actions
- [ ] Password column: masked by default, eye icon to reveal
- [ ] Pagination controls at bottom (page size: 10/25/50/100)
- [ ] Empty state: illustration + "No items yet" + CTA button

#### 2b. List View (compact cards)

```
+------------------------------------------+
| [icon] GitHub                    [E] [D] |
|   user@email.com | github.com            |
|   Updated 2 days ago                     |
+------------------------------------------+
| [icon] AWS Prod                  [E] [D] |
|   admin | console.aws.amazon.com         |
|   Updated 1 week ago                     |
+------------------------------------------+
```

- [ ] Card-style rows with more vertical info
- [ ] Click to expand -> shows full detail inline
- [ ] Drag to reorder (optional, for custom sort)
- [ ] Same selection/bulk actions as table

#### 2c. Tree View (folder-based hierarchy)

```
v All Items (42)
  v Logins (30)
    - GitHub
    - AWS Prod
    - ...
  v SSH Keys (5)
    - Server A
    - Server B
  > Environments (4)
  > Certificates (3)
```

- [ ] Collapsible folder nodes
- [ ] Item count badges per folder
- [ ] Drag-and-drop items between folders
- [ ] Right-click context menu (rename folder, delete, move)
- [ ] Double-click item -> opens detail/edit

### 3. Create Item (Modal/Drawer)

```
+-- Create New Item --------------------------+
| Kind: [Credential v]                        |
|                                             |
| Title:       [___________________________]  |
| Identifier:  [___________________________]  |
| Password:    [________] [Generate] [Show]   |
| URL:         [___________________________]  |
| Folder:      [None v]                       |
|                                             |
|                      [Cancel] [Save]        |
+---------------------------------------------+
```

- [ ] Modal or slide-in drawer (right side)
- [ ] Form validation (title required, url format)
- [ ] Kind selector changes form fields dynamically:
  - **Credential**: identifier, password, url
  - **SSH Key** (future): public key, private key, passphrase
  - **Environment** (future): key-value pairs
  - **Certificate** (future): cert content, expiry date
- [ ] Password field:
  - Toggle visibility (eye icon)
  - "Generate" button -> calls `generate_password` command
  - Password strength indicator bar
  - Copy button
- [ ] Folder assignment dropdown (with "Create new folder" option)
- [ ] Save -> calls `create_vault_item` -> refresh list -> show success toast
- [ ] Keyboard: `Escape` to cancel, `Ctrl+Enter` to save

### 4. View Item Detail (Modal/Drawer or Inline Expand)

- [ ] Read-only view of all fields
- [ ] Password masked, click to reveal (auto-hide after 10s)
- [ ] One-click copy for each field (identifier, password, url)
- [ ] "Edit" button -> switches to edit mode
- [ ] "Delete" button -> confirmation dialog
- [ ] Show metadata: created_at, updated_at
- [ ] Keyboard: `Escape` to close

### 5. Edit Item (Reuse Create Form)

- [ ] Pre-populate form with existing data
- [ ] Same validation as create
- [ ] Save -> calls `update_vault_item` -> refresh list -> success toast
- [ ] Cancel -> discard changes (confirm if dirty)

### 6. Delete Item

- [ ] Confirmation dialog: "Delete [title]? This cannot be undone."
- [ ] Delete -> calls `delete_vault_item` -> refresh list -> success toast
- [ ] Bulk delete: "Delete N items?" confirmation

### 7. Bulk Actions

- [ ] Select all / deselect all checkbox in header
- [ ] Selection count indicator: "3 items selected"
- [ ] Bulk actions bar (appears when items selected):
  - Delete selected
  - Move to folder
  - Export selected
- [ ] `Escape` clears selection

### 8. Import / Export

- [ ] Export button in toolbar (or menu)
  - Export all or export selected
  - Calls `export_vault_items` -> file save dialog
  - JSON format
- [ ] Import button
  - File picker dialog
  - Calls `import_vault_items` -> refresh list
  - Show summary: "Imported N items"
  - Handle duplicates: skip / overwrite / rename

### 9. Folder Sidebar (for Tree View context)

- [ ] Folder list with nested hierarchy
- [ ] Click folder -> filters content area to that folder
- [ ] "All Items" at top (unfiltered)
- [ ] Create folder (inline rename)
- [ ] Rename folder (double-click or context menu)
- [ ] Delete folder (moves items to parent or "unfiled")
- [ ] Drag items to folders
- [ ] Collapsible sidebar (toggle with icon)

---

## UI Components Needed

### Already Available
- `Button`, `IconButton`
- `Table`, `TableHeader`, `TableBody`, `TableRow`, `TableCell`, `TableColumn`
- `Toast` (feedback)
- `Tooltip`
- `Input` (form)
- `Ripple` effect

### Need to Build
- [ ] **Modal / Dialog** - for create/edit/delete confirmation
- [ ] **Drawer** - slide-in panel (alternative to modal for forms)
- [ ] **Dropdown / Select** - for filters, sort, kind selector
- [ ] **Checkbox** - for row selection
- [ ] **SearchInput** - input with search icon + clear button
- [ ] **Badge** - for item counts, tags
- [ ] **ContextMenu** - right-click menu for tree view
- [ ] **TreeView** component - collapsible tree nodes
- [ ] **EmptyState** - illustration + message + CTA
- [ ] **PasswordField** - input with show/hide toggle + generate + strength bar
- [ ] **Pagination** - page controls (prev/next, page numbers, page size)
- [ ] **ConfirmDialog** - "Are you sure?" pattern
- [ ] **Toolbar** - horizontal action bar container
- [ ] **SegmentedControl** - for view mode toggle (Table/List/Tree)
- [ ] **Skeleton / Loading** - placeholder while data loads

---

## UX Recommendations

### Interaction Patterns
1. **Keyboard-first** - All actions reachable via keyboard
   - `/` or `Ctrl+K` -> focus search
   - `Ctrl+N` -> new item
   - `Enter` on row -> open detail
   - `Delete` on selected -> delete confirmation
   - Arrow keys -> navigate rows
   - `Escape` -> close modal / clear selection
2. **Clipboard integration** - One-click copy with visual feedback (toast: "Copied!")
3. **Auto-hide passwords** - Revealed passwords hide after 10 seconds
4. **Optimistic UI** - Immediate visual feedback, rollback on error
5. **Responsive sidebar** - Collapsible folder panel for more space

### Visual Design
1. **Monospace for secrets** - Use `font-jetbrains-mono` for passwords, identifiers, URLs
2. **Subtle row hover** - Light background highlight + reveal action icons
3. **Color-coded kinds** - Different accent colors per VaultItemKind
4. **Density options** - Comfortable (default) / Compact toggle in toolbar
5. **Dark-first** - Design for dark theme (matches current `bg-secondary/20` pattern)

### Priority Order (suggested implementation)
1. Table View + basic list (MVP)
2. Create/Edit modal
3. Search + pagination
4. Delete with confirmation
5. Copy to clipboard
6. List View
7. Filter + Sort
8. Import/Export UI
9. Folder sidebar + Tree View
10. Bulk actions
11. Keyboard shortcuts
12. Password generator integration

---

## State Management

```
VaultPageState {
  // Data
  items: Vec<VaultItem>
  folders: Vec<Folder>
  pagination: PaginationMetadata
  
  // UI State
  view_mode: ViewMode (Table | List | Tree)
  search_query: String
  active_filters: Filters
  sort_by: SortField
  sort_order: SortOrder (Asc | Desc)
  selected_items: HashSet<Uuid>
  selected_folder: Option<Uuid>
  
  // Modal State
  modal: Option<ModalState> (None | Create | Edit(Uuid) | Delete(Uuid) | Detail(Uuid))
  
  // Loading
  is_loading: bool
}
```

Use Leptos signals (`RwSignal`, `ReadSignal`, `WriteSignal`) with context providers for shared state across child components.
