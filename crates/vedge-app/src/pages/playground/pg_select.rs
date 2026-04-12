use leptos::prelude::*;
use vedge_ui::components::select::{Select, SelectItem, SelectOption};
use vedge_ui::primitives::tokens::{Size, Status};

use super::common::Section;

fn simple_options() -> Vec<SelectItem> {
    vec![
        SelectItem::option("option-1", "Option 1"),
        SelectItem::option("option-2", "Option 2"),
        SelectItem::option("option-3", "Option 3"),
    ]
}

#[component]
pub fn SelectPage() -> impl IntoView {
    let (select_val, set_select_val) = signal(String::new());
    let (select_grouped_val, set_select_grouped_val) = signal(String::new());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Select"</h1>

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
                    <Select options=simple_options() default_value="option-1" size=Size::Sm placeholder="Small" />
                    <Select options=simple_options() default_value="option-1" placeholder="Medium (default)" />
                    <Select options=simple_options() default_value="option-1" size=Size::Lg placeholder="Large" />
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
                    <Select options=simple_options() default_value="option-1" disabled=true />
                </div>
            </Section>
        </div>
    }
}
