# Migration Plan: Current → Generic Enterprise Structure

This document provides a step-by-step migration plan to transition from your current UI structure to the comprehensive generic enterprise-level organization.

## 🎯 Migration Overview

**Current State**: Flat component structure with basic organization
**Target State**: Hierarchical enterprise structure with 200+ generic components
**Migration Strategy**: Incremental, backward-compatible, zero-downtime

## 📋 Migration Phases

### Phase 1: Foundation Setup (Week 1)

**Goal**: Establish new structure without breaking existing code

#### Step 1.1: Create New Directory Structure

```bash
# Create new directories
mkdir -p src/system/{theme,provider,utils}
mkdir -p src/primitives/{color,typography,spacing,layout}
mkdir -p src/components/{foundation,layout,navigation,form,data_display,feedback,advanced,mobile,utilities}
mkdir -p src/hooks
mkdir -p src/types
mkdir -p tests/{integration,unit,visual}
mkdir -p benchmarks/{components,rendering}
```

#### Step 1.2: Create System Module Files

```rust
// src/system/mod.rs
pub mod theme;
pub mod provider;
pub mod utils;

// src/primitives/mod.rs
pub mod color;
pub mod typography;
pub mod spacing;
pub mod layout;

// src/components/mod.rs
pub mod foundation;
pub mod layout;
pub mod navigation;
pub mod form;
pub mod data_display;
pub mod feedback;
pub mod advanced;
pub mod mobile;
pub mod utilities;
```

#### Step 1.3: Update Main Library File

```rust
// src/lib.rs
pub mod system;
pub mod primitives;
pub mod components;
pub mod hooks;
pub mod styles;
pub mod types;

// Maintain backward compatibility
pub use components::foundation::button::Button;
pub use components::foundation::icon_button::IconButton;
pub use components::foundation::icon::Icon;
pub use components::form::input::Input;
pub use components::feedback::modal::Modal;
pub use components::feedback::toast::Toast;
pub use components::feedback::tooltip::Tooltip;
// ... other existing components
```

### Phase 2: Move Existing Components (Week 2)

**Goal**: Migrate existing components to new structure

#### Step 2.1: Move Foundation Components

```bash
# Move existing components to foundation
mv src/components/button src/components/foundation/button/
mv src/components/icon_button src/components/foundation/icon_button/
mv src/components/icon src/components/foundation/icon/
mv src/components/checkbox src/components/foundation/checkbox/
mv src/components/chip src/components/foundation/chip/
```

#### Step 2.2: Move Form Components

```bash
# Move form components
mv src/components/input src/components/form/input/
```

#### Step 2.3: Move Feedback Components

```bash
# Move feedback components
mv src/components/modal src/components/feedback/modal/
mv src/components/toast src/components/feedback/toast/
mv src/components/tooltip src/components/feedback/tooltip/
```

#### Step 2.4: Move Utility Components

```bash
# Move utility components
mv src/components/ripple src/components/utilities/ripple/
```

#### Step 2.5: Update Component Module Files

```rust
// src/components/foundation/mod.rs
pub mod button;
pub mod icon_button;
pub mod icon;
pub mod checkbox;
pub mod chip;

// src/components/form/mod.rs
pub mod input;

// src/components/feedback/mod.rs
pub mod modal;
pub mod toast;
pub mod tooltip;

// src/components/utilities/mod.rs
pub mod ripple;
```

### Phase 3: Enhance Primitives (Week 3)

**Goal**: Expand primitive system for better theming

#### Step 3.1: Enhance Color System

```rust
// src/primitives/color/mod.rs
pub mod rgb;        // Existing
pub mod hsl;        // New
pub mod palette;    // New
pub mod contrast;   // New

// src/primitives/color/hsl.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HslColor {
    pub h: f32, // Hue (0-360)
    pub s: f32, // Saturation (0-100)
    pub l: f32, // Lightness (0-100)
}

impl HslColor {
    pub fn to_rgb(&self) -> RgbColor {
        // HSL to RGB conversion
    }

    pub fn to_css(&self) -> String {
        format!("hsl({}, {}%, {}%)", self.h, self.s, self.l)
    }
}
```

#### Step 3.2: Add Typography System

```rust
// src/primitives/typography/mod.rs
pub mod font_family;
pub mod font_size;
pub mod font_weight;
pub mod line_height;

// src/primitives/typography/font_size.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontSize {
    Xs,    // 12px
    Sm,    // 14px
    Base,  // 16px
    Lg,    // 18px
    Xl,    // 20px
    Xxl,   // 24px
    Xxxl,  // 30px
    Xxxxl, // 36px
}

impl FontSize {
    pub fn to_css(&self) -> &'static str {
        match self {
            FontSize::Xs => "12px",
            FontSize::Sm => "14px",
            FontSize::Base => "16px",
            FontSize::Lg => "18px",
            FontSize::Xl => "20px",
            FontSize::Xxl => "24px",
            FontSize::Xxxl => "30px",
            FontSize::Xxxxl => "36px",
        }
    }
}
```

#### Step 3.3: Add Spacing System

```rust
// src/primitives/spacing/mod.rs
pub mod scale;
pub mod utilities;

// src/primitives/spacing/scale.rs
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Spacing {
    Xs,    // 4px
    Sm,    // 8px
    Md,    // 16px
    Lg,    // 24px
    Xl,    // 32px
    Xxl,   // 48px
    Xxxl,  // 64px
    Xxxxl, // 96px
}

impl Spacing {
    pub fn to_css(&self) -> &'static str {
        match self {
            Spacing::Xs => "4px",
            Spacing::Sm => "8px",
            Spacing::Md => "16px",
            Spacing::Lg => "24px",
            Spacing::Xl => "32px",
            Spacing::Xxl => "48px",
            Spacing::Xxxl => "64px",
            Spacing::Xxxxl => "96px",
        }
    }
}
```

### Phase 4: Add System Theme (Week 4)

**Goal**: Implement comprehensive theming

#### Step 4.1: Create Theme Provider

```rust
// src/system/theme/mod.rs
pub mod provider;
pub mod context;
pub mod tokens;
pub mod colors;
pub mod typography;
pub mod spacing;
pub mod breakpoints;
pub mod animations;

// src/system/theme/provider.rs
use leptos::prelude::*;
use crate::primitives::color::RgbColor;

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub colors: ColorTokens,
    pub typography: TypographyTokens,
    pub spacing: SpacingTokens,
    pub breakpoints: BreakpointTokens,
    pub animations: AnimationTokens,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorTokens {
    pub primary: RgbColor,
    pub secondary: RgbColor,
    pub success: RgbColor,
    pub warning: RgbColor,
    pub error: RgbColor,
    pub background: RgbColor,
    pub surface: RgbColor,
    pub text: RgbColor,
}

#[component]
pub fn ThemeProvider(
    theme: Theme,
    children: Children,
) -> impl IntoView {
    provide_context(theme);
    children()
}
```

#### Step 4.2: Create Theme Context Hook

```rust
// src/hooks/use_theme.rs
use leptos::prelude::*;
use crate::system::theme::Theme;

pub fn use_theme() -> ReadSignal<Theme> {
    use_context::<Theme>()
        .expect("use_theme must be used within a ThemeProvider")
}
```

### Phase 5: Add Layout Components (Week 5)

**Goal**: Implement layout system

#### Step 5.1: Create Container Component

```rust
// src/components/layout/container/mod.rs
pub mod container;

// src/components/layout/container/container.rs
use leptos::prelude::*;
use crate::primitives::spacing::Spacing;

#[component]
pub fn Container(
    children: Children,
    #[prop(optional)] max_width: Option<&'static str>,
    #[prop(optional)] padding: Option<Spacing>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let base_class = "container";
    let max_width_class = max_width.map(|w| format!("max-w-{}", w)).unwrap_or_default();
    let padding_class = padding.map(|p| format!("p-{}", p.to_css())).unwrap_or_default();
    let custom_class = class.unwrap_or("");

    let class = format!("{} {} {} {}", base_class, max_width_class, padding_class, custom_class);

    view! {
        <div class=class>
            {children()}
        </div>
    }
}
```

#### Step 5.2: Create Grid System

```rust
// src/components/layout/grid/mod.rs
pub mod grid;
pub mod grid_item;

// src/components/layout/grid/grid.rs
use leptos::prelude::*;
use crate::primitives::spacing::Spacing;

#[component]
pub fn Grid(
    children: Children,
    #[prop(optional)] columns: Option<u8>,
    #[prop(optional)] gap: Option<Spacing>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let base_class = "grid";
    let columns_class = columns.map(|c| format!("grid-cols-{}", c)).unwrap_or_default();
    let gap_class = gap.map(|g| format!("gap-{}", g.to_css())).unwrap_or_default();
    let custom_class = class.unwrap_or("");

    let class = format!("{} {} {} {}", base_class, columns_class, gap_class, custom_class);

    view! {
        <div class=class>
            {children()}
        </div>
    }
}
```

### Phase 6: Add Form Components (Week 6)

**Goal**: Expand form component library

#### Step 6.1: Create Form Wrapper

```rust
// src/components/form/form/mod.rs
pub mod form;
pub mod form_item;

// src/components/form/form/form.rs
use leptos::prelude::*;

#[component]
pub fn Form(
    children: Children,
    #[prop(optional)] on_submit: Option<Box<dyn Fn() + 'static>>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let base_class = "form";
    let custom_class = class.unwrap_or("");
    let class = format!("{} {}", base_class, custom_class);

    let handle_submit = move |ev: web_sys::Event| {
        ev.prevent_default();
        if let Some(on_submit) = &on_submit {
            on_submit();
        }
    };

    view! {
        <form class=class on:submit=handle_submit>
            {children()}
        </form>
    }
}
```

#### Step 6.2: Create Select Component

```rust
// src/components/form/select/mod.rs
pub mod select;
pub mod option;

// src/components/form/select/select.rs
use leptos::prelude::*;

#[component]
pub fn Select(
    children: Children,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let base_class = "select";
    let custom_class = class.unwrap_or("");
    let class = format!("{} {}", base_class, custom_class);

    view! {
        <select class=class>
            {placeholder.map(|p| view! { <option value="" disabled selected>{p}</option> })}
            {children()}
        </select>
    }
}
```

### Phase 7: Add Specialized Components (Week 7)

**Goal**: Implement specialized reusable components

#### Step 7.1: Create Password Input Component

```rust
// src/components/form/password_input/mod.rs
pub mod password_input;

// src/components/form/password_input/password_input.rs
use leptos::prelude::*;

#[component]
pub fn PasswordInput(
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] show_strength: Option<bool>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let base_class = "password-input";
    let custom_class = class.unwrap_or("");
    let class = format!("{} {}", base_class, custom_class);

    let (show_password, set_show_password) = create_signal(false);

    view! {
        <div class=class>
            <input
                type=move || if show_password.get() { "text" } else { "password" }
                placeholder=placeholder
            />
            <button
                type="button"
                on:click=move |_| set_show_password.update(|s| *s = !*s)
            >
                {move || if show_password.get() { "Hide" } else { "Show" }}
            </button>
        </div>
    }
}
```

#### Step 7.2: Create Data Card Component

```rust
// src/components/data_display/card/mod.rs
pub mod card;

// src/components/data_display/card/card.rs
use leptos::prelude::*;

#[component]
pub fn DataCard(
    children: Children,
    #[prop(optional)] title: Option<&'static str>,
    #[prop(optional)] subtitle: Option<&'static str>,
    #[prop(optional)] on_click: Option<Box<dyn Fn() + 'static>>,
    #[prop(optional)] class: Option<&'static str>,
) -> impl IntoView {
    let base_class = "data-card";
    let custom_class = class.unwrap_or("");
    let class = format!("{} {}", base_class, custom_class);

    let handle_click = move || {
        if let Some(on_click) = &on_click {
            on_click();
        }
    };

    view! {
        <div class=class on:click=handle_click>
            {title.map(|title| view! { <h3 class="card-title">{title}</h3> })}
            {subtitle.map(|subtitle| view! { <p class="card-subtitle">{subtitle}</p> })}
            <div class="card-content">
                {children()}
            </div>
        </div>
    }
}
```

### Phase 8: Add Hooks and Utilities (Week 8)

**Goal**: Implement reusable hooks and utilities

#### Step 8.1: Create Common Hooks

```rust
// src/hooks/mod.rs
pub mod use_theme;
pub mod use_media_query;
pub mod use_local_storage;
pub mod use_click_outside;
pub mod use_keyboard;
pub mod use_toggle;

// src/hooks/use_toggle.rs
use leptos::prelude::*;

pub fn use_toggle(initial: bool) -> (ReadSignal<bool>, impl Fn() + Clone) {
    let (state, set_state) = create_signal(initial);
    let toggle = move || set_state.update(|s| *s = !*s);
    (state, toggle)
}

// src/hooks/use_click_outside.rs
use leptos::prelude::*;
use web_sys::Event;

pub fn use_click_outside<F>(callback: F) -> impl Fn(web_sys::Element) + Clone
where
    F: Fn() + 'static,
{
    let callback = Rc::new(callback);

    move |element: web_sys::Element| {
        let callback = callback.clone();
        let element = element.clone();

        let handle_click = move |event: Event| {
            if let Some(target) = event.target() {
                if !element.contains(Some(&target)) {
                    callback();
                }
            }
        };

        // Add event listener logic here
    }
}
```

## 🔄 Backward Compatibility Strategy

### 1. Maintain Existing Exports

```rust
// src/lib.rs - Keep existing exports working
pub use components::foundation::button::Button;
pub use components::foundation::icon_button::IconButton;
// ... all existing components
```

### 2. Gradual Migration

- Keep old imports working during migration
- Add deprecation warnings for old imports
- Provide migration guides for new structure

### 3. Version Strategy

- Current: v0.1.x (existing structure)
- Migration: v0.2.x (new structure with backward compatibility)
- Future: v1.0.x (clean new structure)

## 📊 Migration Checklist

### Week 1: Foundation

- [ ] Create new directory structure
- [ ] Set up system module files
- [ ] Update main library file
- [ ] Test existing components still work

### Week 2: Component Migration

- [ ] Move foundation components
- [ ] Move form components
- [ ] Move feedback components
- [ ] Move utility components
- [ ] Update module files
- [ ] Test all moved components

### Week 3: Primitives Enhancement

- [ ] Enhance color system
- [ ] Add typography system
- [ ] Add spacing system
- [ ] Test primitive systems

### Week 4: Theme System

- [ ] Create theme provider
- [ ] Create theme context
- [ ] Add theme tokens
- [ ] Test theming system

### Week 5: Layout Components

- [ ] Create container component
- [ ] Create grid system
- [ ] Create flex components
- [ ] Test layout system

### Week 6: Form Components

- [ ] Create form wrapper
- [ ] Create select component
- [ ] Create other form components
- [ ] Test form system

### Week 7: Specialized Components

- [ ] Create password input component
- [ ] Create data card component
- [ ] Create other specialized components
- [ ] Test specialized components

### Week 8: Hooks and Utilities

- [ ] Create common hooks
- [ ] Create utility functions
- [ ] Test hooks and utilities
- [ ] Final integration testing

## 🚀 Post-Migration Tasks

1. **Documentation**: Update all documentation to reflect new structure
2. **Examples**: Create examples for new components
3. **Testing**: Add comprehensive tests for all components
4. **Performance**: Optimize bundle size and performance
5. **Accessibility**: Ensure all components meet accessibility standards
6. **TypeScript**: Add proper type definitions
7. **Storybook**: Create component stories for development

## 🎯 Success Metrics

- [ ] All existing components work in new structure
- [ ] New components follow consistent patterns
- [ ] Theme system works across all components
- [ ] Bundle size remains reasonable
- [ ] Performance is maintained or improved
- [ ] Documentation is comprehensive
- [ ] Tests cover all components
- [ ] Accessibility standards are met
- [ ] Components are generic and reusable

This migration plan ensures a smooth transition to the generic enterprise-level structure while maintaining backward compatibility and following best practices.
