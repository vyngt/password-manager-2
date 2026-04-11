use leptos::prelude::*;
use ui::components::checkbox::Checkbox;
use ui::components::radio_group::{RadioGroup, RadioOption};
use ui::components::segmented_control::{SegmentOption, SegmentedControl};
use ui::components::select::{Select, SelectItem, SelectOption};
use ui::components::toggle::Toggle;
use ui::primitives::tokens::{CheckboxSize, Orientation, Size, Status};

use icondata as i;

#[component]
pub fn ControlsPage() -> impl IntoView {
    // ---- Checkbox signals ----
    let (cb_checked, set_cb_checked) = signal(false);
    let (cb_indeterminate, _set_cb_indeterminate) = signal(true);

    // ---- Toggle signals ----
    let (toggle_checked, set_toggle_checked) = signal(false);

    // ---- RadioGroup signals ----
    let (radio_val, set_radio_val) = signal("option-1".to_string());

    // ---- SegmentedControl signals ----
    let (seg_val, set_seg_val) = signal("list".to_string());
    let (seg_icon_val, set_seg_icon_val) = signal("left".to_string());

    // ---- Select signals ----
    let (select_val, set_select_val) = signal(String::new());
    let (select_grouped_val, set_select_grouped_val) = signal(String::new());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">

            // =================================================================
            // Checkbox
            // =================================================================
            <h1 class="text-xl font-semibold text-text-primary">"Checkbox"</h1>

            <Section title="Sizes">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Checkbox size=CheckboxSize::Sm aria_label="Small checkbox" />
                        <span class="text-sm text-text-secondary">"Small"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox aria_label="Medium checkbox" />
                        <span class="text-sm text-text-secondary">"Medium"</span>
                    </div>
                </div>
            </Section>

            <Section title="States">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Checkbox aria_label="Unchecked" />
                        <span class="text-sm text-text-secondary">"Unchecked"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox default_checked=true aria_label="Checked" />
                        <span class="text-sm text-text-secondary">"Checked"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox indeterminate=Signal::derive(move || cb_indeterminate.get()) aria_label="Indeterminate" />
                        <span class="text-sm text-text-secondary">"Indeterminate"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox disabled=true aria_label="Disabled" />
                        <span class="text-sm text-text-secondary">"Disabled"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Checkbox disabled=true default_checked=true aria_label="Disabled checked" />
                        <span class="text-sm text-text-secondary">"Disabled checked"</span>
                    </div>
                </div>
            </Section>

            <Section title="Interactive">
                <div class="flex items-center gap-2">
                    <Checkbox
                        checked=Signal::derive(move || cb_checked.get())
                        on_change=Callback::new(move |v: bool| set_cb_checked.set(v))
                        aria_label="Interactive checkbox"
                    />
                    <span class="text-sm text-text-secondary">
                        {move || if cb_checked.get() { "Checked" } else { "Unchecked" }}
                    </span>
                </div>
            </Section>

            // =================================================================
            // Toggle
            // =================================================================
            <h1 class="text-xl font-semibold text-text-primary pt-4">"Toggle"</h1>

            <Section title="Sizes">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Toggle size=CheckboxSize::Sm aria_label="Small toggle" />
                        <span class="text-sm text-text-secondary">"Small"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle aria_label="Medium toggle" />
                        <span class="text-sm text-text-secondary">"Medium"</span>
                    </div>
                </div>
            </Section>

            <Section title="States">
                <div class="flex flex-wrap items-center gap-6">
                    <div class="flex items-center gap-2">
                        <Toggle aria_label="Off" />
                        <span class="text-sm text-text-secondary">"Off"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle default_checked=true aria_label="On" />
                        <span class="text-sm text-text-secondary">"On"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle disabled=true aria_label="Disabled off" />
                        <span class="text-sm text-text-secondary">"Disabled off"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Toggle disabled=true default_checked=true aria_label="Disabled on" />
                        <span class="text-sm text-text-secondary">"Disabled on"</span>
                    </div>
                </div>
            </Section>

            <Section title="Interactive">
                <div class="flex items-center gap-2">
                    <Toggle
                        checked=Signal::derive(move || toggle_checked.get())
                        on_change=Callback::new(move |v: bool| set_toggle_checked.set(v))
                        aria_label="Interactive toggle"
                    />
                    <span class="text-sm text-text-secondary">
                        {move || if toggle_checked.get() { "On" } else { "Off" }}
                    </span>
                </div>
            </Section>

            // =================================================================
            // RadioGroup
            // =================================================================
            <h1 class="text-xl font-semibold text-text-primary pt-4">"RadioGroup"</h1>

            <Section title="Vertical (default)">
                <RadioGroup
                    options=vec![
                        RadioOption::new("option-1", "Option 1"),
                        RadioOption::new("option-2", "Option 2"),
                        RadioOption::new("option-3", "Option 3"),
                    ]
                    name="demo-vertical"
                    value=Signal::derive(move || radio_val.get())
                    on_change=Callback::new(move |v: String| set_radio_val.set(v))
                    aria_label="Vertical radio group"
                />
                <p class="text-xs text-text-tertiary pt-1">
                    "Selected: " {move || radio_val.get()}
                </p>
            </Section>

            <Section title="Horizontal">
                <RadioGroup
                    options=vec![
                        RadioOption::new("a", "Alpha"),
                        RadioOption::new("b", "Beta"),
                        RadioOption::new("c", "Gamma"),
                    ]
                    name="demo-horizontal"
                    default_value="a"
                    orientation=Orientation::Horizontal
                    aria_label="Horizontal radio group"
                />
            </Section>

            <Section title="Disabled">
                <RadioGroup
                    options=vec![
                        RadioOption::new("x", "First"),
                        RadioOption::new("y", "Second"),
                        RadioOption::new("z", "Third"),
                    ]
                    name="demo-disabled"
                    default_value="x"
                    disabled=true
                    aria_label="Disabled radio group"
                />
            </Section>

            // =================================================================
            // SegmentedControl
            // =================================================================
            <h1 class="text-xl font-semibold text-text-primary pt-4">"SegmentedControl"</h1>

            <Section title="Text segments">
                <SegmentedControl
                    options=vec![
                        SegmentOption::text("list", "List"),
                        SegmentOption::text("grid", "Grid"),
                        SegmentOption::text("board", "Board"),
                    ]
                    value=Signal::derive(move || seg_val.get())
                    on_change=Callback::new(move |v: String| set_seg_val.set(v))
                    aria_label="View mode"
                />
                <p class="text-xs text-text-tertiary pt-1">
                    "Active: " {move || seg_val.get()}
                </p>
            </Section>

            <Section title="Icon segments">
                <SegmentedControl
                    options=vec![
                        SegmentOption::icon("left", i::FaAlignLeftSolid, "Align left"),
                        SegmentOption::icon("center", i::FaAlignCenterSolid, "Align center"),
                        SegmentOption::icon("right", i::FaAlignRightSolid, "Align right"),
                    ]
                    value=Signal::derive(move || seg_icon_val.get())
                    on_change=Callback::new(move |v: String| set_seg_icon_val.set(v))
                    aria_label="Text alignment"
                />
            </Section>

            <Section title="Sizes">
                <div class="flex flex-wrap items-center gap-4">
                    <SegmentedControl
                        options=vec![
                            SegmentOption::text("a", "One"),
                            SegmentOption::text("b", "Two"),
                        ]
                        value=Signal::stored("a".to_string())
                        size=Size::Sm
                        aria_label="Small segmented"
                    />
                    <SegmentedControl
                        options=vec![
                            SegmentOption::text("a", "One"),
                            SegmentOption::text("b", "Two"),
                        ]
                        value=Signal::stored("a".to_string())
                        aria_label="Medium segmented"
                    />
                </div>
            </Section>

            <Section title="Disabled">
                <SegmentedControl
                    options=vec![
                        SegmentOption::text("a", "Alpha"),
                        SegmentOption::text("b", "Beta"),
                        SegmentOption::text("c", "Gamma"),
                    ]
                    value=Signal::stored("a".to_string())
                    disabled=true
                    aria_label="Disabled segmented"
                />
            </Section>

            // =================================================================
            // Select
            // =================================================================
            <h1 class="text-xl font-semibold text-text-primary pt-4">"Select"</h1>

            <Section title="Basic">
                <div class="max-w-xs">
                    <Select
                        options=vec![
                            SelectItem::option("apple", "Apple"),
                            SelectItem::option("banana", "Banana"),
                            SelectItem::option("cherry", "Cherry"),
                            SelectItem::option("date", "Date"),
                            SelectItem::option("elderberry", "Elderberry"),
                        ]
                        value=Signal::derive(move || select_val.get())
                        placeholder="Choose a fruit..."
                        on_change=Callback::new(move |v: String| set_select_val.set(v))
                    />
                    <p class="text-xs text-text-tertiary pt-2">
                        "Selected: " {move || {
                            let v = select_val.get();
                            if v.is_empty() { "(none)".to_string() } else { v }
                        }}
                    </p>
                </div>
            </Section>

            <Section title="Grouped options">
                <div class="max-w-xs">
                    <Select
                        options=vec![
                            SelectItem::group("Fruits", vec![
                                SelectOption::new("apple", "Apple"),
                                SelectOption::new("banana", "Banana"),
                                SelectOption::new("cherry", "Cherry"),
                            ]),
                            SelectItem::group("Vegetables", vec![
                                SelectOption::new("carrot", "Carrot"),
                                SelectOption::new("broccoli", "Broccoli"),
                                SelectOption::new("spinach", "Spinach"),
                            ]),
                        ]
                        value=Signal::derive(move || select_grouped_val.get())
                        placeholder="Choose food..."
                        on_change=Callback::new(move |v: String| set_select_grouped_val.set(v))
                    />
                </div>
            </Section>

            <Section title="Sizes">
                <div class="space-y-3 max-w-xs">
                    <Select
                        options=simple_options()
                        default_value="option-1"
                        size=Size::Sm
                        placeholder="Small"
                    />
                    <Select
                        options=simple_options()
                        default_value="option-1"
                        placeholder="Medium (default)"
                    />
                    <Select
                        options=simple_options()
                        default_value="option-1"
                        size=Size::Lg
                        placeholder="Large"
                    />
                </div>
            </Section>

            <Section title="Status">
                <div class="space-y-3 max-w-xs">
                    <Select options=simple_options() default_value="option-1" status=Status::Error />
                    <Select options=simple_options() default_value="option-1" status=Status::Success />
                    <Select options=simple_options() default_value="option-1" status=Status::Warning />
                </div>
            </Section>

            <Section title="Disabled">
                <div class="max-w-xs">
                    <Select
                        options=simple_options()
                        default_value="option-1"
                        disabled=true
                    />
                </div>
            </Section>
        </div>
    }
}

fn simple_options() -> Vec<SelectItem> {
    vec![
        SelectItem::option("option-1", "Option 1"),
        SelectItem::option("option-2", "Option 2"),
        SelectItem::option("option-3", "Option 3"),
    ]
}

#[component]
fn Section(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-3">
            <h3 class="text-xs font-medium text-text-tertiary uppercase tracking-wide">{title}</h3>
            {children()}
        </div>
    }
}
