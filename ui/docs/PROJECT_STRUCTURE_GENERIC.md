# UI Library Project Structure - Enterprise Component Library

This document outlines the recommended project structure for the enterprise-level UI component library, building on your existing Leptos-based architecture.

## 🏗️ Current Structure Analysis

Your current structure shows excellent patterns:

- ✅ Component-based organization
- ✅ Separation of styles, variants, and logic
- ✅ Proper module organization
- ✅ CSS-in-Rust approach with separate CSS files
- ✅ Primitives layer for shared utilities

## 📁 Recommended Enterprise Structure

```
ui/
├── Cargo.toml                          # Package configuration
├── README.md                           # Library documentation
├── PROJECT_STRUCTURE.md                # This file
├── examples/                           # Usage examples and demos
│   ├── basic/                          # Basic component examples
│   ├── advanced/                       # Complex usage patterns
│   └── showcase/                       # Component showcase
├── docs/                               # Documentation
│   ├── components/                     # Individual component docs
│   ├── theming/                        # Theming guide
│   └── migration/                      # Migration guides
├── src/
│   ├── lib.rs                          # Main library entry point
│   ├── prelude.rs                      # Common imports and re-exports
│   │
│   ├── system/                         # System-level components
│   │   ├── mod.rs
│   │   ├── theme/                      # Theme system
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs             # Theme provider
│   │   │   ├── context.rs              # Theme context
│   │   │   ├── tokens.rs               # Design tokens
│   │   │   ├── colors.rs               # Color system
│   │   │   ├── typography.rs           # Typography system
│   │   │   ├── spacing.rs              # Spacing system
│   │   │   ├── breakpoints.rs          # Responsive breakpoints
│   │   │   └── animations.rs           # Animation tokens
│   │   ├── provider/                   # Global providers
│   │   │   ├── mod.rs
│   │   │   ├── toast.rs                # Toast provider
│   │   │   ├── modal.rs                # Modal provider
│   │   │   └── theme.rs                # Theme provider
│   │   └── utils/                      # System utilities
│   │       ├── mod.rs
│   │       ├── class_names.rs          # CSS class utilities
│   │       ├── dom.rs                  # DOM utilities
│   │       ├── events.rs               # Event handling
│   │       └── accessibility.rs        # A11y utilities
│   │
│   ├── primitives/                     # Design primitives
│   │   ├── mod.rs
│   │   ├── color/                      # Color system
│   │   │   ├── mod.rs
│   │   │   ├── rgb.rs                  # RGB color (existing)
│   │   │   ├── hsl.rs                  # HSL color
│   │   │   ├── palette.rs              # Color palettes
│   │   │   └── contrast.rs             # Contrast utilities
│   │   ├── typography/                 # Typography primitives
│   │   │   ├── mod.rs
│   │   │   ├── font_family.rs          # Font families
│   │   │   ├── font_size.rs            # Font sizes
│   │   │   ├── font_weight.rs          # Font weights
│   │   │   └── line_height.rs          # Line heights
│   │   ├── spacing/                    # Spacing system
│   │   │   ├── mod.rs
│   │   │   ├── scale.rs                # Spacing scale
│   │   │   └── utilities.rs            # Spacing utilities
│   │   └── layout/                     # Layout primitives
│   │       ├── mod.rs
│   │       ├── grid.rs                 # Grid system
│   │       ├── flex.rs                 # Flexbox utilities
│   │       └── position.rs             # Positioning
│   │
│   ├── components/                     # UI Components
│   │   ├── mod.rs                      # Component exports
│   │   │
│   │   ├── foundation/                 # Foundation components
│   │   │   ├── mod.rs
│   │   │   ├── button/                 # Button component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── button.rs           # Main component (existing)
│   │   │   │   ├── styles.rs           # Styles (existing)
│   │   │   │   ├── variants.rs         # Variants (existing)
│   │   │   │   └── types.rs            # Type definitions
│   │   │   ├── icon_button/            # Icon button (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── icon_button.rs
│   │   │   │   ├── styles.rs
│   │   │   │   └── variants.rs
│   │   │   ├── icon/                   # Icon system (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── icon.rs
│   │   │   │   ├── feather.rs           # Feather icons
│   │   │   │   ├── lucide.rs           # Lucide icons
│   │   │   │   └── heroicons.rs        # Heroicons
│   │   │   ├── typography/             # Typography components
│   │   │   │   ├── mod.rs
│   │   │   │   ├── heading.rs          # H1-H6 components
│   │   │   │   ├── text.rs             # Text component
│   │   │   │   ├── code.rs             # Code component
│   │   │   │   └── link.rs             # Link component
│   │   │   ├── spacer/                 # Spacing component
│   │   │   │   ├── mod.rs
│   │   │   │   └── spacer.rs
│   │   │   └── divider/                # Divider component
│   │   │       ├── mod.rs
│   │   │       └── divider.rs
│   │   │
│   │   ├── layout/                     # Layout components
│   │   │   ├── mod.rs
│   │   │   ├── container/              # Container component
│   │   │   │   ├── mod.rs
│   │   │   │   └── container.rs
│   │   │   ├── grid/                   # Grid system
│   │   │   │   ├── mod.rs
│   │   │   │   ├── grid.rs
│   │   │   │   └── grid_item.rs
│   │   │   ├── flex/                   # Flexbox component
│   │   │   │   ├── mod.rs
│   │   │   │   └── flex.rs
│   │   │   ├── stack/                  # Stack component
│   │   │   │   ├── mod.rs
│   │   │   │   └── stack.rs
│   │   │   ├── sidebar/                # Sidebar component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── sidebar.rs
│   │   │   │   └── sidebar_item.rs
│   │   │   ├── header/                 # Header component
│   │   │   │   ├── mod.rs
│   │   │   │   └── header.rs
│   │   │   ├── footer/                 # Footer component
│   │   │   │   ├── mod.rs
│   │   │   │   └── footer.rs
│   │   │   └── page/                   # Page wrapper
│   │   │       ├── mod.rs
│   │   │       └── page.rs
│   │   │
│   │   ├── navigation/                 # Navigation components
│   │   │   ├── mod.rs
│   │   │   ├── menu/                   # Menu component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── menu.rs
│   │   │   │   ├── menu_item.rs
│   │   │   │   └── submenu.rs
│   │   │   ├── breadcrumb/             # Breadcrumb component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── breadcrumb.rs
│   │   │   │   └── breadcrumb_item.rs
│   │   │   ├── pagination/             # Pagination component
│   │   │   │   ├── mod.rs
│   │   │   │   └── pagination.rs
│   │   │   ├── tabs/                   # Tabs component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── tabs.rs
│   │   │   │   ├── tab.rs
│   │   │   │   └── tab_panel.rs
│   │   │   ├── stepper/                # Stepper component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── stepper.rs
│   │   │   │   └── step.rs
│   │   │   └── back_button/            # Back button
│   │   │       ├── mod.rs
│   │   │       └── back_button.rs
│   │   │
│   │   ├── form/                       # Form components
│   │   │   ├── mod.rs
│   │   │   ├── form/                   # Form wrapper
│   │   │   │   ├── mod.rs
│   │   │   │   ├── form.rs
│   │   │   │   └── form_item.rs
│   │   │   ├── field/                  # Field wrapper (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   └── field.rs
│   │   │   ├── input/                  # Input component (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── input.rs
│   │   │   │   ├── styles.rs
│   │   │   │   └── variants.rs
│   │   │   ├── textarea/               # Textarea component
│   │   │   │   ├── mod.rs
│   │   │   │   └── textarea.rs
│   │   │   ├── select/                 # Select component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── select.rs
│   │   │   │   └── option.rs
│   │   │   ├── multi_select/           # Multi-select component
│   │   │   │   ├── mod.rs
│   │   │   │   └── multi_select.rs
│   │   │   ├── checkbox/               # Checkbox (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   └── checkbox.rs
│   │   │   ├── radio/                  # Radio component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── radio.rs
│   │   │   │   └── radio_group.rs
│   │   │   ├── switch/                 # Switch component
│   │   │   │   ├── mod.rs
│   │   │   │   └── switch.rs
│   │   │   ├── slider/                 # Slider component
│   │   │   │   ├── mod.rs
│   │   │   │   └── slider.rs
│   │   │   ├── date_picker/            # Date picker
│   │   │   │   ├── mod.rs
│   │   │   │   └── date_picker.rs
│   │   │   ├── time_picker/            # Time picker
│   │   │   │   ├── mod.rs
│   │   │   │   └── time_picker.rs
│   │   │   ├── color_picker/           # Color picker
│   │   │   │   ├── mod.rs
│   │   │   │   └── color_picker.rs
│   │   │   ├── file_upload/            # File upload
│   │   │   │   ├── mod.rs
│   │   │   │   └── file_upload.rs
│   │   │   ├── search_input/           # Search input
│   │   │   │   ├── mod.rs
│   │   │   │   └── search_input.rs
│   │   │   ├── password_input/         # Password input
│   │   │   │   ├── mod.rs
│   │   │   │   └── password_input.rs
│   │   │   ├── otp_input/              # OTP input
│   │   │   │   ├── mod.rs
│   │   │   │   └── otp_input.rs
│   │   │   └── code_input/             # Code input
│   │   │       ├── mod.rs
│   │   │       └── code_input.rs
│   │   │
│   │   ├── data_display/               # Data display components
│   │   │   ├── mod.rs
│   │   │   ├── table/                  # Table component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── table.rs
│   │   │   │   ├── table_header.rs
│   │   │   │   ├── table_body.rs
│   │   │   │   ├── table_row.rs
│   │   │   │   └── table_cell.rs
│   │   │   ├── data_grid/              # Data grid
│   │   │   │   ├── mod.rs
│   │   │   │   └── data_grid.rs
│   │   │   ├── list/                   # List component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── list.rs
│   │   │   │   └── list_item.rs
│   │   │   ├── virtual_list/           # Virtual list
│   │   │   │   ├── mod.rs
│   │   │   │   └── virtual_list.rs
│   │   │   ├── tree/                   # Tree component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── tree.rs
│   │   │   │   └── tree_node.rs
│   │   │   ├── timeline/               # Timeline
│   │   │   │   ├── mod.rs
│   │   │   │   └── timeline.rs
│   │   │   ├── card/                   # Card component
│   │   │   │   ├── mod.rs
│   │   │   │   ├── card.rs
│   │   │   │   ├── card_header.rs
│   │   │   │   ├── card_body.rs
│   │   │   │   └── card_footer.rs
│   │   │   ├── accordion/              # Accordion
│   │   │   │   ├── mod.rs
│   │   │   │   ├── accordion.rs
│   │   │   │   └── accordion_item.rs
│   │   │   ├── collapse/               # Collapse
│   │   │   │   ├── mod.rs
│   │   │   │   └── collapse.rs
│   │   │   ├── panel/                  # Panel
│   │   │   │   ├── mod.rs
│   │   │   │   └── panel.rs
│   │   │   ├── drawer/                 # Drawer
│   │   │   │   ├── mod.rs
│   │   │   │   └── drawer.rs
│   │   │   ├── avatar/                 # Avatar
│   │   │   │   ├── mod.rs
│   │   │   │   └── avatar.rs
│   │   │   ├── badge/                  # Badge
│   │   │   │   ├── mod.rs
│   │   │   │   └── badge.rs
│   │   │   ├── tag/                    # Tag
│   │   │   │   ├── mod.rs
│   │   │   │   └── tag.rs
│   │   │   ├── chip/                   # Chip (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   └── chip.rs
│   │   │   ├── image/                  # Image
│   │   │   │   ├── mod.rs
│   │   │   │   └── image.rs
│   │   │   ├── video/                  # Video
│   │   │   │   ├── mod.rs
│   │   │   │   └── video.rs
│   │   │   ├── code_block/             # Code block
│   │   │   │   ├── mod.rs
│   │   │   │   └── code_block.rs
│   │   │   └── markdown/               # Markdown
│   │   │       ├── mod.rs
│   │   │       └── markdown.rs
│   │   │
│   │   ├── feedback/                   # Feedback components
│   │   │   ├── mod.rs
│   │   │   ├── modal/                  # Modal (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── modal.rs
│   │   │   │   ├── modal_header.rs
│   │   │   │   ├── modal_body.rs
│   │   │   │   └── modal_footer.rs
│   │   │   ├── popover/                # Popover
│   │   │   │   ├── mod.rs
│   │   │   │   └── popover.rs
│   │   │   ├── tooltip/                # Tooltip (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   └── tooltip.rs
│   │   │   ├── dropdown/               # Dropdown
│   │   │   │   ├── mod.rs
│   │   │   │   └── dropdown.rs
│   │   │   ├── context_menu/           # Context menu
│   │   │   │   ├── mod.rs
│   │   │   │   └── context_menu.rs
│   │   │   ├── toast/                  # Toast (existing)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── toast.rs
│   │   │   │   ├── item.rs
│   │   │   │   ├── provider.rs
│   │   │   │   ├── styles.rs
│   │   │   │   └── types.rs
│   │   │   ├── alert/                  # Alert
│   │   │   │   ├── mod.rs
│   │   │   │   └── alert.rs
│   │   │   ├── banner/                 # Banner
│   │   │   │   ├── mod.rs
│   │   │   │   └── banner.rs
│   │   │   ├── snackbar/               # Snackbar
│   │   │   │   ├── mod.rs
│   │   │   │   └── snackbar.rs
│   │   │   ├── progress/               # Progress
│   │   │   │   ├── mod.rs
│   │   │   │   └── progress.rs
│   │   │   ├── skeleton/               # Skeleton
│   │   │   │   ├── mod.rs
│   │   │   │   └── skeleton.rs
│   │   │   ├── spinner/                # Spinner
│   │   │   │   ├── mod.rs
│   │   │   │   └── spinner.rs
│   │   │   ├── loading/                # Loading
│   │   │   │   ├── mod.rs
│   │   │   │   └── loading.rs
│   │   │   ├── empty_state/            # Empty state
│   │   │   │   ├── mod.rs
│   │   │   │   └── empty_state.rs
│   │   │   ├── error_boundary/         # Error boundary
│   │   │   │   ├── mod.rs
│   │   │   │   └── error_boundary.rs
│   │   │   └── status_indicator/       # Status indicator
│   │   │       ├── mod.rs
│   │   │       └── status_indicator.rs
│   │   │
│   │   ├── advanced/                   # Advanced components
│   │   │   ├── mod.rs
│   │   │   ├── chart/                  # Chart components
│   │   │   │   ├── mod.rs
│   │   │   │   ├── line_chart.rs
│   │   │   │   ├── bar_chart.rs
│   │   │   │   └── pie_chart.rs
│   │   │   ├── progress_bar/           # Progress bar
│   │   │   │   ├── mod.rs
│   │   │   │   └── progress_bar.rs
│   │   │   ├── meter/                  # Meter
│   │   │   │   ├── mod.rs
│   │   │   │   └── meter.rs
│   │   │   ├── calendar/               # Calendar
│   │   │   │   ├── mod.rs
│   │   │   │   └── calendar.rs
│   │   │   ├── heatmap/                # Heatmap
│   │   │   │   ├── mod.rs
│   │   │   │   └── heatmap.rs
│   │   │   ├── carousel/               # Carousel
│   │   │   │   ├── mod.rs
│   │   │   │   └── carousel.rs
│   │   │   ├── wizard/                 # Wizard
│   │   │   │   ├── mod.rs
│   │   │   │   ├── wizard.rs
│   │   │   │   └── wizard_step.rs
│   │   │   ├── tour/                   # Tour
│   │   │   │   ├── mod.rs
│   │   │   │   └── tour.rs
│   │   │   ├── command_palette/        # Command palette
│   │   │   │   ├── mod.rs
│   │   │   │   └── command_palette.rs
│   │   │   └── floating_action_button/ # FAB
│   │   │       ├── mod.rs
│   │   │       └── floating_action_button.rs
│   │   │
│   │   ├── mobile/                     # Mobile components
│   │   │   ├── mod.rs
│   │   │   ├── bottom_sheet/           # Bottom sheet
│   │   │   │   ├── mod.rs
│   │   │   │   └── bottom_sheet.rs
│   │   │   ├── swipe_actions/          # Swipe actions
│   │   │   │   ├── mod.rs
│   │   │   │   └── swipe_actions.rs
│   │   │   ├── pull_to_refresh/        # Pull to refresh
│   │   │   │   ├── mod.rs
│   │   │   │   └── pull_to_refresh.rs
│   │   │   ├── infinite_scroll/        # Infinite scroll
│   │   │   │   ├── mod.rs
│   │   │   │   └── infinite_scroll.rs
│   │   │   └── touch_feedback/         # Touch feedback
│   │   │       ├── mod.rs
│   │   │       └── touch_feedback.rs
│   │   │
│   │   └── utilities/                  # Utility components
│   │       ├── mod.rs
│   │       ├── responsive/             # Responsive utilities
│   │       │   ├── mod.rs
│   │       │   └── responsive.rs
│   │       ├── visibility/             # Visibility utilities
│   │       │   ├── mod.rs
│   │       │   └── visibility.rs
│   │       ├── position/               # Position utilities
│   │       │   ├── mod.rs
│   │       │   └── position.rs
│   │       ├── z_index/                # Z-index utilities
│   │       │   ├── mod.rs
│   │       │   └── z_index.rs
│   │       ├── focus_trap/             # Focus trap
│   │       │   ├── mod.rs
│   │       │   └── focus_trap.rs
│   │       ├── skip_link/              # Skip link
│   │       │   ├── mod.rs
│   │       │   └── skip_link.rs
│   │       ├── screen_reader/          # Screen reader
│   │       │   ├── mod.rs
│   │       │   └── screen_reader.rs
│   │       ├── high_contrast/          # High contrast
│   │       │   ├── mod.rs
│   │       │   └── high_contrast.rs
│   │       └── ripple/                 # Ripple effect (existing)
│   │           ├── mod.rs
│   │           ├── ripple.rs
│   │           └── effect.rs
│   │
│   ├── hooks/                          # Custom hooks
│   │   ├── mod.rs
│   │   ├── use_theme.rs                # Theme hook
│   │   ├── use_media_query.rs          # Media query hook
│   │   ├── use_local_storage.rs        # Local storage hook
│   │   ├── use_session_storage.rs      # Session storage hook
│   │   ├── use_click_outside.rs        # Click outside hook
│   │   ├── use_keyboard.rs             # Keyboard hook
│   │   ├── use_focus.rs                # Focus hook
│   │   ├── use_intersection.rs         # Intersection hook
│   │   ├── use_resize.rs               # Resize hook
│   │   ├── use_scroll.rs               # Scroll hook
│   │   ├── use_debounce.rs             # Debounce hook
│   │   ├── use_throttle.rs             # Throttle hook
│   │   ├── use_async.rs                # Async hook
│   │   ├── use_interval.rs             # Interval hook
│   │   ├── use_timeout.rs              # Timeout hook
│   │   └── use_toggle.rs               # Toggle hook
│   │
│   ├── styles/                         # Global styles
│   │   ├── mod.rs
│   │   ├── index.css                   # Main CSS file (existing)
│   │   ├── reset.css                   # CSS reset
│   │   ├── variables.css               # CSS variables
│   │   ├── animations.css              # Animation styles
│   │   ├── utilities.css               # Utility classes
│   │   ├── components/                 # Component-specific styles
│   │   │   ├── button.css              # Button styles (existing)
│   │   │   ├── icon-button.css         # Icon button styles (existing)
│   │   │   ├── input.css               # Input styles (existing)
│   │   │   ├── toast.css               # Toast styles (existing)
│   │   │   ├── ripple.css              # Ripple styles (existing)
│   │   │   └── ...                     # Other component styles
│   │   └── themes/                     # Theme-specific styles
│   │       ├── light.css               # Light theme
│   │       ├── dark.css                # Dark theme
│   │       └── high-contrast.css       # High contrast theme
│   │
│   └── types/                          # Type definitions
│       ├── mod.rs
│       ├── common.rs                   # Common types
│       ├── theme.rs                    # Theme types
│       ├── component.rs                # Component types
│       ├── form.rs                     # Form types
│       └── event.rs                    # Event types
│
├── tests/                              # Test files
│   ├── integration/                    # Integration tests
│   ├── unit/                           # Unit tests
│   └── visual/                         # Visual regression tests
│
└── benchmarks/                         # Performance benchmarks
    ├── components/                     # Component benchmarks
    └── rendering/                      # Rendering benchmarks
```

## 🎯 Key Organizational Principles

### 1. **Hierarchical Organization**

- **Foundation** → **Layout** → **Navigation** → **Form** → **Data Display** → **Feedback** → **Advanced**
- Each category builds upon the previous ones
- Clear dependency relationships

### 2. **Consistent Module Structure**

Each component follows this pattern:

```
component_name/
├── mod.rs              # Module exports
├── component_name.rs   # Main component
├── styles.rs           # Component styles (if complex)
├── variants.rs         # Component variants (if needed)
└── types.rs            # Component-specific types (if needed)
```

### 3. **Separation of Concerns**

- **System**: Theme system, providers, utilities
- **Primitives**: Design tokens and basic building blocks
- **Components**: UI components organized by function
- **Hooks**: Reusable logic
- **Styles**: CSS organization
- **Types**: TypeScript-like type definitions

### 4. **Generic and Reusable**

- No domain-specific components in the main library
- Components can be used in any application
- Extensible through composition and theming

## 🚀 Migration Strategy

### Phase 1: Reorganize Existing Components

1. Move existing components to new structure
2. Update imports and module declarations
3. Maintain backward compatibility

### Phase 2: Add System Infrastructure

1. Implement theme system
2. Add provider components
3. Create utility functions

### Phase 3: Expand Component Library

1. Add missing foundation components
2. Implement layout components
3. Build form components

### Phase 4: Advanced Features

1. Add advanced components
2. Implement mobile components
3. Add accessibility features

## 📝 Module Export Strategy

### `src/lib.rs`

```rust
pub mod system;
pub mod primitives;
pub mod components;
pub mod hooks;
pub mod styles;
pub mod types;

// Re-export commonly used items
pub use components::foundation::*;
pub use components::layout::*;
pub use components::form::*;
pub use system::theme::*;
```

### `src/components/mod.rs`

```rust
pub mod foundation;
pub mod layout;
pub mod navigation;
pub mod form;
pub mod data_display;
pub mod feedback;
pub mod advanced;
pub mod mobile;
pub mod utilities;

// Re-export for convenience
pub use foundation::*;
pub use layout::*;
pub use navigation::*;
pub use form::*;
pub use data_display::*;
pub use feedback::*;
```

## 🎨 Benefits of This Structure

1. **Scalability**: Easy to add new components without restructuring
2. **Maintainability**: Clear organization makes code easier to find and maintain
3. **Reusability**: Components are organized by function, making reuse obvious
4. **Team Collaboration**: Clear boundaries for different team members
5. **Documentation**: Structure itself serves as documentation
6. **Testing**: Easy to organize tests alongside components
7. **Performance**: Tree-shaking friendly with clear module boundaries
8. **Generic**: Can be used in any application, not tied to specific domains

## 🔧 Implementation Notes

- Each component should be self-contained with its own styles and types
- Use consistent naming conventions across all components
- Implement proper error handling and accessibility features
- Follow your existing patterns for styles and variants
- Maintain backward compatibility during migration
- Add comprehensive documentation for each component
- Keep components generic and composable

This structure provides a solid foundation for an enterprise-level UI component library while maintaining the excellent patterns you've already established.
