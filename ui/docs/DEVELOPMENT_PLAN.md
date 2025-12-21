# Enterprise UI Framework Development Plan
## 12-Week Production Roadmap (v0.1.0 → v1.0.0)

**Project**: VLock Password Manager UI Library
**Goal**: Transform from prototype (8 components) to production-ready design system
**Timeline**: 12 weeks
**Strategy**: Internal password manager use first, architected for eventual public release

**🚨 CLEAN SLATE APPROACH**: This is a complete rewrite with NO backward compatibility. Modern APIs from day one.

---

## Executive Summary

### Current State
- **Components**: 8 production-ready (Button, IconButton, Input, Toast, Tooltip, Table, 3 icons)
- **Stack**: Leptos 0.8.14 CSR, Tailwind v4, Rust 2024
- **Strengths**: Clean architecture, WCAG color contrast, provider pattern working
- **Pain Points**:
  - Verbose `Signal::derive(move || color_store.primary().get())` used 20+ times
  - No two-way binding for Input (manual sync required)
  - RgbColor-only API (no hex, CSS names, theme tokens)
  - Manual style overrides (52 instances of class prop)
  - No Modal/Dialog (blocks vault CRUD operations)

### Target State (Week 12)
- **Components**: 30+ production components across all categories
- **Theme System**: ThemeProvider with design tokens (eliminates Signal::derive boilerplate)
- **Forms**: Complete form system with validation (enables vault creation/editing)
- **Modals**: Dialog system with focus management (enables CRUD workflows)
- **DX**: Simplified APIs, two-way binding, flexible color inputs

### Key Decisions
✅ **Clean slate - NO backward compatibility** - Complete rewrite with modern APIs
✅ **Keep Tailwind hybrid** - No big CSS refactor, enhance with design tokens
✅ **Priority**: Forms & Validation → Modals & Dialogs
✅ **Pattern**: Replicate ToastProvider success → ThemeProvider → FormProvider → ModalProvider
✅ **Frontend migration**: All at once after Week 4 foundation is complete

---

## Breaking Changes Strategy

### Philosophy
**CLEAN SLATE APPROACH**: Complete rewrite with modern APIs. No backward compatibility. Frontend will be migrated all at once after Week 4.

This approach allows us to:
- Design optimal APIs without legacy constraints
- Move faster without compatibility layers
- Establish clean patterns from day one
- Simplify implementation (no deprecated code paths)

### Critical Breaking Changes

#### 1. Input Component: Two-Way Binding ⚡ BREAKING

**Current** (verbose manual sync):
```rust
// 21 instances of this across frontend
let (text, set_text) = signal(String::new());
<Input
    color=Signal::derive(move || color_store.primary().get())
    on_input_target=Callback::new(move |ev| set_text.set(ev.target().value()))
/>
```

**New** (automatic two-way binding):
```rust
let text = RwSignal::new(String::new());
<Input
    value=text           // Two-way binding
    color="primary"      // Theme token
/>
```

**Migration**: Frontend will use new API immediately after Week 4 foundation is complete. No gradual migration needed.

#### 2. Color API: Flexible Input ⚡ BREAKING

**Current** (RgbColor only):
```rust
color: Signal<RgbColor>  // Fixed type, requires Signal::derive
```

**New** (flexible, ergonomic):
```rust
color: impl Into<Color>  // Accepts: hex, CSS names, tokens, Signals

// All valid:
<Button color="primary">                    // Theme token
<Button color="#FF5733">                    // Hex string
<Button color=rgb_color>                    // RgbColor
<Button color=signal>                       // Signal (no derive!)
<Button>                                    // Uses theme.colors.primary
```

**Implementation**:
```rust
// ui/src/primitives/color/mod.rs
pub enum ColorValue {
    Rgb(RgbColor),
    Hex(String),
    ThemeToken(String),  // "primary", "danger", etc.
}

pub struct Color {
    inner: MaybeSignal<ColorValue>,
}

impl From<&str> for ColorValue {
    fn from(s: &str) -> Self {
        if s.starts_with('#') { ColorValue::Hex(s.to_string()) }
        else { ColorValue::ThemeToken(s.to_string()) }
    }
}
```

#### 3. Component Props: Consistent API Standards ⚡ BREAKING

**Standardize ALL components**:
```rust
#[component]
pub fn Component(
    children: Children,

    // Colors: ALWAYS MaybeSignal + Into<Color>, optional defaults to theme
    #[prop(into, optional)] color: Option<MaybeSignal<ColorValue>>,

    // Sizes: ALWAYS Size enum, default Medium
    #[prop(default = Size::Medium)] size: Size,

    // Classes: ALWAYS Into<TextProp> (Leptos 0.8 pattern)
    #[prop(into, default = "".into())] class: TextProp,

    // Booleans: ALWAYS default = false
    #[prop(default = false)] disabled: bool,
)
```

---

## Phase 1: Foundation & Infrastructure (Weeks 1-4)

### Week 1: Core Infrastructure Setup

**Goal**: Fix critical API issues and create flexible color system

#### Task 1.1: Enhanced Color System (3 days) 🎨

**Files to Create**:
- `ui/src/primitives/color/mod.rs` - Restructure module
- `ui/src/primitives/color/rgb.rs` - Move existing RgbColor (from primitives/color.rs)
- `ui/src/primitives/color/parser.rs` - CSS color parsing
- `ui/src/primitives/color/convert.rs` - Conversion traits

**Implementation**:
```rust
// ui/src/primitives/color/mod.rs
pub mod rgb;
pub mod parser;
pub mod convert;

pub use rgb::RgbColor;
use leptos::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum ColorValue {
    Rgb(RgbColor),
    Hex(String),           // "#FF5733"
    ThemeToken(String),    // "primary", "danger"
}

pub struct Color {
    inner: MaybeSignal<ColorValue>,
}

impl Color {
    pub fn get(&self) -> RgbColor {
        match self.inner.get() {
            ColorValue::Rgb(rgb) => rgb,
            ColorValue::Hex(hex) => RgbColor::from_hex(&hex),
            ColorValue::ThemeToken(token) => {
                use_theme_color(&token).unwrap_or(RgbColor::new(0, 0, 0))
            }
        }
    }

    pub fn to_css_string(&self) -> String {
        let rgb = self.get();
        format!("rgb({} {} {})", rgb.r, rgb.g, rgb.b)
    }
}

// Ergonomic conversions
impl From<&str> for ColorValue { /* ... */ }
impl From<RgbColor> for ColorValue { /* ... */ }
impl From<Signal<RgbColor>> for Color { /* ... */ }
```

**Files to Modify**:
- `ui/src/primitives/color.rs` → Move to `rgb.rs`
- `ui/src/components/foundation/button.rs` - Accept new Color type
- `ui/src/components/foundation/icon_button.rs` - Accept new Color type

**Success Criteria**:
- ✓ Hex strings work: `color="#FF5733"`
- ✓ Theme tokens work: `color="primary"`
- ✓ Direct RgbColor works: `color=RgbColor::new(255, 87, 51)`
- ✓ Signal support works: `color=color_signal`
- ✓ Color parser handles edge cases correctly

#### Task 1.2: Input Two-Way Binding (2 days) 🔄

**Files to Modify**:
- `ui/src/components/form/input.rs`

**Implementation**:
```rust
// ui/src/components/form/input.rs
#[component]
pub fn Input(
    // Two-way binding with RwSignal
    #[prop(into, optional)] value: Option<RwSignal<String>>,

    // Flexible color API (defaults to theme primary)
    #[prop(into, optional)] color: Option<MaybeSignal<ColorValue>>,

    #[prop(attrs, default = "text")] input_type: &'static str,
    #[prop(attrs)] id: &'static str,
    #[prop(attrs)] placeholder: &'static str,
    #[prop(into, default = "".into())] class: TextProp,

    // Error state
    #[prop(into, optional)] error: Option<Signal<Option<String>>>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] required: bool,
) -> impl IntoView {
    // Use theme primary if no color provided
    let color = color.unwrap_or_else(|| {
        let theme_color = use_theme_color("primary");
        MaybeSignal::Dynamic(Signal::derive(move || ColorValue::Rgb(theme_color.get())))
    });

    let handle_input = move |ev: Event| {
        let input: HtmlInputElement = event_target(&ev);
        let new_value = input.value();

        // Two-way binding
        if let Some(signal) = value {
            signal.set(new_value);
        }
    };

    // ... rest of implementation
}
```

**New CSS** (ui/src/styles/input.css):
```css
.input--error {
    @apply border-red-500 focus:border-red-500;
}

.input--error-message {
    @apply text-xs text-red-500 mt-1 animate-in fade-in duration-200;
}
```

**Success Criteria**:
- ✓ RwSignal two-way binding works
- ✓ Error messages display correctly
- ✓ Required indicator shows (*)
- ✓ Theme colors apply automatically
- ✓ Disabled state prevents interaction

#### Task 1.3: ThemeProvider Foundation (4 days) 🎨

**Files to Create**:
- `ui/src/providers/mod.rs`
- `ui/src/providers/theme/mod.rs`
- `ui/src/providers/theme/provider.rs`
- `ui/src/providers/theme/context.rs`
- `ui/src/providers/theme/tokens.rs`
- `ui/src/hooks/mod.rs`
- `ui/src/hooks/use_theme.rs`

**Design Token System**:
```rust
// ui/src/providers/theme/tokens.rs
use crate::primitives::color::RgbColor;

#[derive(Debug, Clone)]
pub struct DesignTokens {
    pub colors: ColorTokens,
    pub spacing: SpacingTokens,
    pub typography: TypographyTokens,
}

#[derive(Debug, Clone)]
pub struct ColorTokens {
    // Semantic colors
    pub primary: RgbColor,
    pub secondary: RgbColor,
    pub success: RgbColor,
    pub warning: RgbColor,
    pub danger: RgbColor,

    // Surface colors
    pub background: RgbColor,
    pub foreground: RgbColor,
    pub surface: RgbColor,

    pub fn get(&self, token: &str) -> Option<RgbColor> {
        match token {
            "primary" => Some(self.primary),
            "secondary" => Some(self.secondary),
            // ...
            _ => None,
        }
    }
}

impl ColorTokens {
    pub fn light_theme() -> Self {
        Self {
            primary: RgbColor::from_hex("#6366f1"),    // Indigo-500
            secondary: RgbColor::from_hex("#8b5cf6"),  // Violet-500
            success: RgbColor::from_hex("#22c55e"),    // Green-500
            warning: RgbColor::from_hex("#f59e0b"),    // Amber-500
            danger: RgbColor::from_hex("#ef4444"),     // Red-500
            background: RgbColor::new(255, 255, 255),
            foreground: RgbColor::new(0, 0, 0),
            surface: RgbColor::new(249, 250, 251),
        }
    }

    pub fn dark_theme() -> Self {
        Self {
            primary: RgbColor::from_hex("#818cf8"),    // Indigo-400
            // ... dark variants
        }
    }
}
```

**ThemeProvider**:
```rust
// ui/src/providers/theme/provider.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeMode { Light, Dark, System }

#[component]
pub fn ThemeProvider(
    children: Children,
    #[prop(optional)] initial_mode: Option<ThemeMode>,
) -> impl IntoView {
    let mode = RwSignal::new(initial_mode.unwrap_or(ThemeMode::Light));

    let tokens = Signal::derive(move || {
        match mode.get() {
            ThemeMode::Light => DesignTokens::light(),
            ThemeMode::Dark => DesignTokens::dark(),
            ThemeMode::System => /* check window.matchMedia */ DesignTokens::light(),
        }
    });

    let context = ThemeContext { mode, tokens };
    provide_context(context);

    // Apply CSS variables to document root
    Effect::new(move || {
        let t = tokens.get();
        apply_css_variables(&t);
    });

    view! {
        <div class="theme-root" data-theme=move || format!("{:?}", mode.get()).to_lowercase()>
            {children()}
        </div>
    }
}
```

**useTheme Hook**:
```rust
// ui/src/hooks/use_theme.rs
pub fn use_theme() -> ThemeContext {
    use_context::<ThemeContext>()
        .expect("use_theme must be used within ThemeProvider")
}

pub fn use_theme_color(name: &str) -> Signal<RgbColor> {
    let theme = use_theme();
    Signal::derive(move || {
        theme.tokens.get().colors.get(name).unwrap_or(RgbColor::new(0, 0, 0))
    })
}
```

**Frontend Integration**:
```rust
// frontend/src/app.rs
use ui::providers::theme::ThemeProvider;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <ThemeProvider>
            <ToastProvider>
                <Router>
                    // ... routes
                </Router>
            </ToastProvider>
        </ThemeProvider>
    }
}
```

**Success Criteria**:
- ✓ ThemeProvider injects CSS variables
- ✓ use_theme() hook accessible
- ✓ Light/dark mode toggle works
- ✓ Components use theme colors: `<Button color="primary">`
- ✓ No Signal::derive needed for theme colors

---

### Week 2: Layout System & Utilities

**Goal**: Build layout primitives and helper hooks

#### Task 2.1: Layout Components (3 days) 📐

**Files to Create**:
- `ui/src/components/layout/mod.rs`
- `ui/src/components/layout/container.rs`
- `ui/src/components/layout/stack.rs`
- `ui/src/components/layout/grid.rs`
- `ui/src/styles/layout.css`

**Container**:
```rust
#[derive(Debug, Clone, Copy)]
pub enum ContainerSize {
    Sm,   // max-w-screen-sm (640px)
    Md,   // max-w-screen-md (768px)
    Lg,   // max-w-screen-lg (1024px)
    Xl,   // max-w-screen-xl (1280px)
    Full,
}

#[component]
pub fn Container(
    children: Children,
    #[prop(default = ContainerSize::Lg)] max_width: ContainerSize,
    #[prop(default = true)] center: bool,
    #[prop(into, default = "".into())] class: TextProp,
) -> impl IntoView
```

**Stack** (VStack/HStack pattern):
```rust
#[component]
pub fn Stack(
    children: Children,
    #[prop(default = StackDirection::Vertical)] direction: StackDirection,
    #[prop(default = "md")] gap: &'static str,  // Uses theme spacing
    #[prop(optional)] align: Option<StackAlign>,
    #[prop(optional)] justify: Option<StackJustify>,
) -> impl IntoView
```

**Grid**:
```rust
#[component]
pub fn Grid(
    children: Children,
    #[prop(default = 12)] cols: u8,
    #[prop(optional)] cols_sm: Option<u8>,  // Responsive
    #[prop(optional)] cols_md: Option<u8>,
    #[prop(default = "md")] gap: &'static str,
) -> impl IntoView
```

**Success Criteria**:
- ✓ Container responsive sizing works
- ✓ Stack direction (vertical/horizontal) correct
- ✓ Grid 12-column system functional
- ✓ All use theme spacing tokens

#### Task 2.2: Utility Hooks (2 days) 🪝

**Files to Create**:
- `ui/src/hooks/use_media_query.rs`
- `ui/src/hooks/use_click_outside.rs`
- `ui/src/hooks/use_toggle.rs`
- `ui/src/hooks/use_debounce.rs`

**useMediaQuery**:
```rust
pub fn use_media_query(query: &'static str) -> Signal<bool>;
pub fn use_is_mobile() -> Signal<bool>;  // "(max-width: 768px)"
pub fn use_is_desktop() -> Signal<bool>; // "(min-width: 1024px)"
```

**useClickOutside**:
```rust
pub fn use_click_outside<F>(node_ref: NodeRef<html::Div>, callback: F)
where F: Fn() + 'static;
```

**useToggle**:
```rust
pub fn use_toggle(initial: bool) -> (Signal<bool>, impl Fn(), impl Fn(bool));
```

**Success Criteria**:
- ✓ useMediaQuery responds to viewport changes
- ✓ useClickOutside detects external clicks (needed for Modal)
- ✓ useToggle simplifies boolean state
- ✓ All hooks work in components

#### Task 2.3: Update Existing Components (2 days) 🔄

**Files to Modify**:
- `ui/src/components/foundation/button.rs`
- `ui/src/components/foundation/icon_button.rs`
- `ui/src/components/feedback/tooltip.rs`

**Changes**:
1. Accept new Color API: `#[prop(into, optional)] color: Option<MaybeSignal<ColorValue>>`
2. Default to theme primary: `color.unwrap_or_else(|| use_theme_color("primary"))`
3. Use TextProp for class: `#[prop(into, default = "".into())] class: TextProp`

**Success Criteria**:
- ✓ All existing components use theme colors by default
- ✓ New Color API working on all components
- ✓ Components use clean, modern prop patterns

---

### Week 3: Form Components Foundation

**Goal**: Build remaining form inputs (Select, Checkbox, Radio, Textarea)

#### Task 3.1: Field Wrapper (1 day) 📋

**Files to Create**:
- `ui/src/components/form/field.rs`
- `ui/src/styles/form.css`

**Purpose**: Wrap form inputs with label, helper text, error messages

```rust
#[component]
pub fn Field(
    children: Children,
    #[prop(into, optional)] label: Option<TextProp>,
    #[prop(into, optional)] helper_text: Option<TextProp>,
    #[prop(into, optional)] error: Option<Signal<Option<String>>>,
    #[prop(default = false)] required: bool,
) -> impl IntoView
```

**Usage**:
```rust
<Field label="Email" helper_text="We'll never share your email" required=true error=email_error>
    <Input id="email" value=email color="primary" />
</Field>
```

#### Task 3.2: Select Component (2 days) 🔽

**Files to Create**:
- `ui/src/components/form/select.rs`
- `ui/src/styles/select.css`

**Implementation**:
```rust
#[component]
pub fn Select<T>(
    children: Children,
    #[prop(into)] value: RwSignal<T>,  // Two-way binding
    #[prop(into, optional)] color: Option<MaybeSignal<ColorValue>>,
    #[prop(into, optional)] placeholder: Option<TextProp>,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView
where T: Clone + PartialEq + ToString + FromStr + 'static
```

**Success Criteria**:
- ✓ Generic type support
- ✓ Two-way binding works
- ✓ Theme colors applied
- ✓ Disabled state

#### Task 3.3: Checkbox & Radio (2 days) ☑️

**Files to Create**:
- `ui/src/components/form/checkbox.rs`
- `ui/src/components/form/radio.rs`
- `ui/src/components/form/radio_group.rs`
- `ui/src/styles/checkbox.css`
- `ui/src/styles/radio.css`

**Checkbox**:
```rust
#[component]
pub fn Checkbox(
    #[prop(into)] checked: RwSignal<bool>,
    #[prop(into, optional)] label: Option<TextProp>,
    #[prop(default = false)] disabled: bool,
) -> impl IntoView
```

**Radio**:
```rust
#[component]
pub fn Radio<T>(
    value: T,
    #[prop(into)] selected: RwSignal<T>,
    #[prop(into, optional)] label: Option<TextProp>,
) -> impl IntoView
where T: Clone + PartialEq + 'static
```

**Success Criteria**:
- ✓ Checkbox two-way binding
- ✓ RadioGroup single selection
- ✓ Custom styling with theme colors

#### Task 3.4: Textarea & Switch (2 days) 📝

**Files to Create**:
- `ui/src/components/form/textarea.rs`
- `ui/src/components/form/switch.rs`
- `ui/src/styles/textarea.css`
- `ui/src/styles/switch.css`

**Textarea**: Like Input but multi-line, optional resize
**Switch**: Toggle with smooth animation (iOS-style)

---

### Week 4: Modal System

**Goal**: Implement Modal/Dialog with Portal and focus management

#### Task 4.1: Portal System (1 day) 🌀

**Files to Create**:
- `ui/src/components/utilities/portal.rs`

**Purpose**: Render components outside React tree (for modals, tooltips)

```rust
#[component]
pub fn Portal(
    children: Children,
    #[prop(default = "portal-root")] mount_id: &'static str,
) -> impl IntoView
```

#### Task 4.2: Modal Component (3 days) 🪟

**Files to Create**:
- `ui/src/components/feedback/modal/mod.rs`
- `ui/src/components/feedback/modal/modal.rs`
- `ui/src/components/feedback/modal/backdrop.rs`
- `ui/src/styles/modal.css`

**Modal**:
```rust
#[component]
pub fn Modal(
    children: Children,
    #[prop(into)] open: RwSignal<bool>,
    #[prop(default = ModalSize::Md)] size: ModalSize,
    #[prop(default = true)] close_on_backdrop: bool,
    #[prop(default = true)] close_on_escape: bool,
) -> impl IntoView
```

**Subcomponents**:
- `ModalHeader`: Title section with close button
- `ModalBody`: Content area
- `ModalFooter`: Action buttons

**Features**:
- ✓ Portal rendering
- ✓ Backdrop click closes
- ✓ Escape key closes
- ✓ Body scroll lock when open
- ✓ Focus trap (Tab cycles within modal)
- ✓ Animation on open/close

**Usage Example** (Vault entry creation):
```rust
let is_open = RwSignal::new(false);
let title = RwSignal::new(String::new());
let username = RwSignal::new(String::new());

view! {
    <Button on_click=move |_| is_open.set(true) color="primary">
        "New Entry"
    </Button>

    <Modal open=is_open size=ModalSize::Md>
        <ModalHeader>
            <h2>"Create Vault Entry"</h2>
        </ModalHeader>

        <ModalBody>
            <Field label="Title" required=true>
                <Input id="title" value=title color="primary" />
            </Field>

            <Field label="Username">
                <Input id="username" value=username color="primary" />
            </Field>
        </ModalBody>

        <ModalFooter>
            <Button variant=Variant::Outlined on_click=move |_| is_open.set(false)>
                "Cancel"
            </Button>
            <Button color="primary" on_click=move |_| {
                // Save logic
                is_open.set(false);
            }>
                "Save"
            </Button>
        </ModalFooter>
    </Modal>
}
```

**Success Criteria**:
- ✓ Modal renders in portal
- ✓ Backdrop and escape close work
- ✓ Body scroll locks
- ✓ **Unblocks vault CRUD operations** ⭐

---

## Phase 2: Forms & Validation (Weeks 5-8)

### Week 5: Advanced Form Components

#### Task 5.1: Password Input with Toggle (2 days) 🔒

**File**: `ui/src/components/form/password_input.rs`

**Features**:
- Eye icon to show/hide password
- Password strength indicator (5 levels: Very Weak → Strong)
- Score calculation: length, uppercase, lowercase, numbers, symbols

**Critical for**: Vault password creation/editing

#### Task 5.2: Date & Time Pickers (3 days) 📅

**Files**:
- `ui/src/components/form/date_input.rs`
- `ui/src/components/form/time_input.rs`

**Approach**: Native HTML5 date/time inputs styled with theme

#### Task 5.3: Form State Management (2 days) 📊

**Files**:
- `ui/src/hooks/use_form.rs`
- `ui/src/hooks/use_field.rs`

**useForm Hook**:
```rust
pub struct FormState<T> {
    pub values: RwSignal<T>,
    pub errors: RwSignal<HashMap<String, String>>,
    pub is_submitting: RwSignal<bool>,
    pub is_valid: Signal<bool>,
}

pub fn use_form<T: Default + Clone>() -> FormState<T>;
```

---

### Week 6: Form Validation System

#### Task 6.1: Validation Rules (2 days) ✅

**Files**:
- `ui/src/validation/mod.rs`
- `ui/src/validation/rules.rs`
- `ui/src/validation/validator.rs`

**Built-in Rules**:
```rust
pub fn required(message: &str) -> ValidationRule;
pub fn min_length(len: usize, message: &str) -> ValidationRule;
pub fn max_length(len: usize, message: &str) -> ValidationRule;
pub fn email(message: &str) -> ValidationRule;
pub fn url(message: &str) -> ValidationRule;
pub fn regex(pattern: &str, message: &str) -> ValidationRule;

// Password manager specific
pub fn password_strength(min_score: u8, message: &str) -> ValidationRule;
```

**Usage**:
```rust
let email_field = use_field(
    email_signal,
    vec![
        required("Email is required"),
        email("Invalid email format"),
    ]
);
```

#### Task 6.2: Field-Level Validation (3 days) 🎯

**Integration with Input/Select/Textarea**:
- Auto-validate on blur
- Show error messages
- Visual error states (red border)

#### Task 6.3: Form-Level Validation (2 days) 📋

**Coordinated validation**:
- Validate all fields on submit
- Focus first invalid field
- Disable submit while invalid

---

### Week 7: Navigation Components

**Goal**: Tabs, Breadcrumbs, Menu (for vault organization)

#### Task 7.1: Tabs Component (2 days) 📑

**Files**:
- `ui/src/components/navigation/tabs/mod.rs`
- `ui/src/components/navigation/tabs/tabs.rs`
- `ui/src/styles/tabs.css`

**Usage** (Vault settings page):
```rust
<Tabs default_value="general">
    <TabsList>
        <TabsTrigger value="general">"General"</TabsTrigger>
        <TabsTrigger value="security">"Security"</TabsTrigger>
        <TabsTrigger value="export">"Export"</TabsTrigger>
    </TabsList>

    <TabsContent value="general">
        // General settings
    </TabsContent>

    <TabsContent value="security">
        // Security settings
    </TabsContent>
</Tabs>
```

#### Task 7.2: Breadcrumbs (1 day) 🍞

**File**: `ui/src/components/navigation/breadcrumbs.rs`

**For**: Vault → Folder → Item navigation

#### Task 7.3: Menu/Dropdown (3 days) 📋

**Files**:
- `ui/src/components/navigation/menu/mod.rs`
- `ui/src/components/navigation/menu/menu.rs`

**Features**:
- Keyboard navigation (arrow keys)
- Click outside closes
- Nested menus
- Dividers

**For**: Context menus (right-click on vault items)

---

### Week 8: Data Display Components

**Goal**: Card, List, Badge (for vault item display)

#### Task 8.1: Card Component (2 days) 🃏

**File**: `ui/src/components/data_display/card/mod.rs`

**Subcomponents**: CardHeader, CardBody, CardFooter

**For**: Vault item cards in grid view

#### Task 8.2: List Component (2 days) 📝

**Files**:
- `ui/src/components/data_display/list/mod.rs`
- `ui/src/components/data_display/list_item.rs`

**For**: Vault items in list view

#### Task 8.3: Badge & Tag (1 day) 🏷️

**Files**:
- `ui/src/components/data_display/badge.rs`
- `ui/src/components/data_display/tag.rs`

**For**: Category badges, favorite tags

#### Task 8.4: Avatar & Icon Components (2 days) 👤

**Files**:
- `ui/src/components/data_display/avatar.rs`
- Enhance existing icon system

**For**: User profile, vault item icons

---

## Phase 3: Polish & Production (Weeks 9-12)

### Week 9: Frontend Migration & Testing

**Goal**: Complete frontend migration to new UI library APIs, add comprehensive tests

**Note**: Since we're using a clean slate approach, frontend was already migrated during Week 4-5 as components were being built. This week focuses on polish and testing.

#### Task 9.1: Frontend Polish & Optimization (3 days) 🔄

**Files to Review & Optimize**:
- `frontend/src/pages/page.rs` - Optimize auth page
- `frontend/src/pages/playground.rs` - Complete component showcase
- `frontend/src/pages/v/layout.rs` - Optimize vault layout
- `frontend/src/features/window_panel.rs` - Polish window controls

**Example modern API usage**:
```rust
// Clean, simple two-way binding
let text = RwSignal::new(String::new());
<Input value=text color="primary" />

// Theme colors throughout
<Button color="primary">
<Button color="danger">

// Modal for CRUD
let is_open = RwSignal::new(false);
<Modal open=is_open>
    <ModalHeader>"Create Entry"</ModalHeader>
    <ModalBody>
        <Field label="Title" required=true>
            <Input id="title" value=title />
        </Field>
    </ModalBody>
</Modal>
```

#### Task 9.2: Component Tests (3 days) 🧪

**Files to Create**:
- `ui/tests/button_test.rs`
- `ui/tests/input_test.rs`
- `ui/tests/modal_test.rs`

**Test Framework**: wasm-bindgen-test

**Example**:
```rust
#[wasm_bindgen_test]
fn test_button_click() {
    let clicked = RwSignal::new(false);
    mount_to_body(|| view! {
        <Button on_click=move |_| clicked.set(true)>
            "Click me"
        </Button>
    });

    // Simulate click
    // Assert clicked.get() == true
}
```

#### Task 9.3: Documentation (1 day) 📚

**Files to Create**:
- `ui/README.md` - Getting started with v1.0.0
- `ui/CHANGELOG.md` - v0.1.0 → v1.0.0 changes (complete rewrite notes)
- `ui/docs/MIGRATION_v0_to_v1.md` - Migration guide from v0.1.0 to v1.0.0

**Migration Guide Contents**:
- API comparison table (old vs new)
- Component-by-component migration examples
- Breaking changes summary
- New features showcase

---

### Week 10: Accessibility & Keyboard Navigation

**Goal**: WCAG 2.1 AA compliance, keyboard support

#### Task 10.1: Focus Management (2 days) ⌨️

**Files**:
- `ui/src/hooks/use_focus_trap.rs`
- `ui/src/hooks/use_focus_visible.rs`

**Apply to**: Modal, Menu, Tooltip

#### Task 10.2: ARIA Labels & Roles (2 days) ♿

**Update all components with**:
- Proper ARIA roles
- ARIA labels for icon buttons
- ARIA-expanded for dropdowns
- ARIA-invalid for errors

#### Task 10.3: Keyboard Navigation (3 days) ⌨️

**Components**:
- Modal: Tab cycles, Escape closes
- Menu: Arrow keys, Enter selects
- Tabs: Arrow keys switch tabs
- Form: Tab order, Enter submits

---

### Week 11: Performance & Bundle Size

**Goal**: Optimize for production

#### Task 11.1: Code Splitting (2 days) ✂️

**Strategy**:
- Lazy load heavy components (Modal, DatePicker)
- Tree-shaking unused components

#### Task 11.2: CSS Optimization (2 days) 🎨

**Tasks**:
- Remove unused Tailwind classes
- Minify CSS in production
- Combine critical CSS

#### Task 11.3: Performance Audit (3 days) 📊

**Metrics**:
- Component render times
- Bundle size analysis
- Runtime performance
- Memory leaks check

---

### Week 12: Demo Site & Documentation

**Goal**: Production-ready release

#### Task 12.1: Demo/Storybook Site (3 days) 🎨

**Files to Create**:
- `ui/examples/demo/` - Interactive component playground
- Show all variants, sizes, states
- Copy code snippets
- Dark mode toggle

#### Task 12.2: Component Documentation (2 days) 📖

**For each component**:
- Props table
- Usage examples
- Do's and Don'ts
- Accessibility notes

#### Task 12.3: Final Release (2 days) 🚀

**Tasks**:
1. Update Cargo.toml to v1.0.0
2. Write comprehensive CHANGELOG.md (complete rewrite notes)
3. Create GitHub release
4. Update README with examples
5. Write migration guide (old v0.1.0 → new v1.0.0)
6. Tag release: `git tag v1.0.0`

---

## Success Metrics

### Week 4 Checkpoint
- ✓ ThemeProvider eliminates Signal::derive boilerplate
- ✓ Input two-way binding working
- ✓ Layout system (Container, Stack, Grid) functional
- ✓ Modal system unblocks vault CRUD

### Week 8 Checkpoint
- ✓ Complete form system with validation
- ✓ 20+ production components
- ✓ Password manager can create/edit vault entries
- ✓ Navigation components enable vault organization

### Week 12 Final
- ✓ 30+ production components
- ✓ WCAG 2.1 AA accessible
- ✓ Comprehensive documentation
- ✓ Demo site live
- ✓ Frontend fully migrated
- ✓ Tests passing
- ✓ **Production-ready for password manager** ⭐
- ✓ **Architected for public release** 🌐

---

## Risk Mitigation

### Technical Risks

**Risk**: Frontend breaks during migration
**Mitigation**: Migrate frontend incrementally as each component is completed (Week 4-5), keep old UI functional until new UI is complete

**Risk**: Modal focus trap breaks keyboard navigation
**Mitigation**: Test extensively with keyboard-only navigation in week 10

**Risk**: Bundle size too large
**Mitigation**: Code splitting and tree-shaking in week 11

**Risk**: Color system parser edge cases
**Mitigation**: Comprehensive unit tests for all color formats (hex, CSS names, rgb(), rgba())

### Timeline Risks

**Risk**: Form validation takes longer than expected
**Buffer**: Week 6 has 7 days total, can extend to week 7 if needed

**Risk**: Accessibility audit reveals major issues
**Buffer**: Week 12 has flexibility for critical fixes

---

## Dependencies & Blockers

### Sequential Dependencies
1. **ThemeProvider MUST be done Week 1** → All components depend on it
2. **Input two-way binding Week 1** → Form validation depends on it
3. **Portal system Week 4** → Modal depends on it
4. **Modal Week 4** → Vault CRUD depends on it
5. **Form validation Week 6** → Production forms depend on it

### Parallel Work Opportunities
- Week 2: Layout components + Hooks (independent)
- Week 3: All form components can be done in parallel
- Week 7-8: Navigation + Data Display (independent)

---

## Post-v0.2.0 Roadmap (Weeks 13+)

### Future Enhancements
- **Animations**: Framer Motion-like animations
- **Advanced Data**: DataGrid, TreeView with virtualization
- **Charts**: Simple chart components for security dashboard
- **Mobile**: Responsive improvements, touch gestures
- **Themes**: Additional built-in themes beyond light/dark
- **i18n**: Internationalization support
- **SSR**: Server-side rendering support (Leptos SSR)

---

## Critical Files to Implement

### Week 1 Priority Files (Must complete)
1. `ui/src/primitives/color/mod.rs` - Flexible color API
2. `ui/src/providers/theme/tokens.rs` - Design token system
3. `ui/src/providers/theme/provider.rs` - ThemeProvider
4. `ui/src/hooks/use_theme.rs` - Theme hook
5. `ui/src/components/form/input.rs` - Two-way binding update

### Week 4 Critical File (Unblocks CRUD)
6. `ui/src/components/feedback/modal/modal.rs` - Modal component

---

## Conclusion

This 12-week plan transforms the UI library from a working prototype into a production-ready design system through a **complete rewrite** with modern APIs and no backward compatibility constraints.

**By Week 4**: Modal + Forms foundation enables vault CRUD operations
**By Week 12**: 30+ components, fully documented, accessible, tested → v1.0.0 release

**Key to Success**:
- Clean slate approach allows optimal API design
- No backward compatibility = faster development
- Focus on high-value components first (Forms, Modal)
- Migrate frontend incrementally as components are built
- Test extensively before v1.0.0 release

**Expected Outcome**: A battle-tested, accessible, well-documented UI framework with modern APIs that serves as the foundation for the VLock password manager and can be open-sourced as a reference Leptos component library.

**Version Strategy**:
- v0.1.0 (current): Prototype with 8 components
- **v1.0.0 (Week 12)**: Production-ready with 30+ components, complete rewrite
- No v0.2.x - jumping straight to v1.0.0 reflects the complete rewrite
