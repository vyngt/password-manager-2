use leptos::prelude::*;
use vedge_ui::components::segmented_control::{SegmentOption, SegmentedControl};
use vedge_ui::primitives::tokens::Size;

use icondata as i;

use super::common::Section;

#[component]
pub fn SegmentedControlPage() -> impl IntoView {
    let (seg_val, set_seg_val) = signal("list".to_string());
    let (seg_icon_val, set_seg_icon_val) = signal("left".to_string());

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"SegmentedControl"</h1>

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
        </div>
    }
}
