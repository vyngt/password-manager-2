# Leptos CSR Migration Plan

## From React.js to Leptos - Password Manager UI

### Current React.js Structure Analysis

**Main Features:**

- Authentication (Master Password Entry)
- Vault Management (CRUD operations for password items)
- Password Generator (configurable password creation)
- Profile Management (password change)
- Color Scheme Management (theming)
- Backup Functionality
- Desktop Window Controls (Tauri integration)

**UI Components:**

- Material Tailwind-based components
- Complex TreeView with pagination/search
- Form system with validation
- Sidebar navigation
- Theme system with CSS variables

---

## Phase 1: Foundation Setup

### 1.1 Theme System & CSS Foundation

**Goal**: Establish the theming foundation that matches current app

**Tasks:**

- [ ] Setup CSS custom properties matching current theme
- [ ] Create theme context/signal for Leptos
- [ ] Implement dark/light mode switching
- [ ] Setup Tailwind CSS integration

**Dependencies**: Update `Cargo.toml` with styling dependencies

### 1.2 Basic UI Components Library

**Goal**: Build foundational components to replace Material Tailwind

**Priority Order:**

1. **Button Component** (`components/button.rs`)
   - Variants: filled, outlined, text
   - Sizes: sm, md, lg
   - Icon support
2. **Input Component** (`components/input.rs`)
   - Text, password, email types
   - Placeholder, validation states
   - Icon integration
3. **Typography Component** (`components/typography.rs`)
   - Heading variants (h1-h6)
   - Text variants (body, caption, etc.)
   - Color and weight props
4. **IconButton Component** (`components/icon_button.rs`)
   - Size variants
   - Hover/active states

---

## Phase 2: Layout Infrastructure

### 2.1 Window & Desktop Integration

**Goal**: Desktop app shell with native window controls

**Components:**

- `components/window/title_bar.rs` - Custom title bar with app branding
- `components/window/mod.rs` - Window management utilities
- Tauri integration for window controls (minimize, maximize, close)

### 2.2 Navigation System

**Goal**: Sidebar navigation matching current structure

**Components:**

- `components/sidebar/sidebar.rs` - Main navigation sidebar
- `components/sidebar/sidebar_item.rs` - Individual nav items
- `components/panel/panel_layout.rs` - Main layout wrapper
- Navigation state management

**Menu Structure:**

```
Main Menu:
├── Vault (faVault)
├── Password Generator (faUnlockKeyhole)
├── Manage (faGear)
│   ├── Profile (faUserShield)
│   ├── Backup (faDownload)
│   └── Color Scheme (faPalette)
└── Experiment (faFlask)
```

---

## Phase 3: Advanced UI Systems

### 3.1 Form System

**Goal**: Comprehensive form handling system

**Components:**

- `components/form/form_view.rs` - Main form container
- `components/form/form_field.rs` - Individual form fields
- `components/form/form_header.rs` - Form headers with actions
- `components/form/form_actions.rs` - Save, delete, back actions

**Features:**

- Form validation
- Error handling
- Auto-save functionality
- URL hash parameter integration

### 3.2 TreeView System

**Goal**: Data table with advanced features

**Components:**

- `components/tree_view/tree_view.rs` - Main container
- `components/tree_view/table.rs` - Data table component
- `components/tree_view/header.rs` - Table header with controls
- `components/tree_view/pagination.rs` - Pagination controls
- `components/tree_view/search_filter.rs` - Search functionality

**Features:**

- Pagination
- Search/filtering
- Row selection
- Action buttons per row
- Sorting capabilities

---

## Phase 4: Core Features Implementation

### 4.1 Authentication System

**Goal**: Master password entry page

**Components:**

- `features/auth/login.rs` - Password entry form
- `features/auth/auth_context.rs` - Authentication state management

**Features:**

- Password input with toggle visibility
- Enter key submission
- Tauri backend integration
- Route protection

### 4.2 Vault Management

**Goal**: Core password management functionality

**Components:**

- `features/vault/vault_list.rs` - Password items listing
- `features/vault/vault_form.rs` - Add/edit password form
- `features/vault/vault_item.rs` - Individual vault item component
- `features/vault/copy_button.rs` - Copy to clipboard functionality

**Features:**

- CRUD operations for password items
- Search and filtering
- Clipboard integration
- Form validation

### 4.3 Password Generator

**Goal**: Configurable password generation

**Components:**

- `features/password_generator/generator.rs` - Main generator component
- `features/password_generator/config.rs` - Configuration options

**Features:**

- Length configuration
- Character set options (upper, lower, digits, special)
- Real-time generation
- Copy to clipboard

---

## Phase 5: Advanced Features

### 5.1 Profile Management

**Goal**: User profile and password management

**Components:**

- `features/profile/change_password.rs` - Password change form
- `features/profile/profile_section.rs` - Profile sections

### 5.2 Theming System

**Goal**: Color scheme management

**Components:**

- `features/theme/color_scheme.rs` - Color scheme selector
- `features/theme/theme_provider.rs` - Theme context provider
- `features/theme/color_input.rs` - Color picker input

### 5.3 Backup System

**Goal**: Data backup functionality

**Components:**

- `features/backup/backup_page.rs` - Backup interface
- `features/backup/export.rs` - Export functionality

---

## Phase 6: Integration & Polish

### 6.1 Routing Setup

**Goal**: Complete routing structure

**Structure:**

```
/ (redirect to /main/vault)
/main/
  ├── vault/
  │   ├── / (list view)
  │   └── form (add/edit)
  ├── password-generator/
  ├── manage/
  │   ├── profile/
  │   ├── backup/
  │   └── color-scheme/
  └── experiment/
```

### 6.2 State Management

**Goal**: Global state management system

**Features:**

- Reactive signals for global state
- Context providers for feature-specific state
- Local storage integration
- Tauri state synchronization

### 6.3 Tauri Integration

**Goal**: Complete desktop app integration

**APIs to integrate:**

- File system operations
- Clipboard management
- Window management
- Notifications
- Global shortcuts

---

## Implementation Priority

### High Priority (Core Functionality)

1. Theme system setup
2. Basic components (Button, Input, Typography)
3. Authentication system
4. Vault list and form
5. Basic routing

### Medium Priority (Enhanced UX)

1. TreeView system
2. Password generator
3. Profile management
4. Advanced form system

### Low Priority (Advanced Features)

1. Color scheme management
2. Backup functionality
3. Experiment page
4. Advanced Tauri integration

---

## Technical Considerations

### Dependencies to Add

```toml
[dependencies]
leptos = { version = "0.8", features = ["csr"] }
leptos_router = "0.8"
leptos_icons = "0.6"
leptos_use = "0.13"  # For clipboard, storage, etc.
wasm-bindgen = "0.2"
web-sys = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### State Management Strategy

- Use Leptos signals for reactive state
- Context providers for feature-specific state
- Local storage for persistence
- Tauri invoke for backend communication

### Styling Approach

- Tailwind CSS for utility classes
- CSS custom properties for theming
- Component-specific CSS modules when needed
- Responsive design principles

---

## Migration Timeline Estimate

- **Phase 1-2**: 1-2 weeks (Foundation & Layout)
- **Phase 3**: 1 week (Advanced UI Systems)
- **Phase 4**: 2-3 weeks (Core Features)
- **Phase 5**: 1-2 weeks (Advanced Features)
- **Phase 6**: 1 week (Integration & Polish)

**Total Estimated Time**: 6-9 weeks for complete migration

---

## Next Steps

1. Start with Phase 1.1 - Theme System setup
2. Create basic component library
3. Implement authentication flow
4. Build vault management incrementally
5. Add remaining features based on priority

Would you like me to start implementing any specific phase or component?
