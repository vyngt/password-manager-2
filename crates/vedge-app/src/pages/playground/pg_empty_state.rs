use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::button::Button;
use vedge_ui::components::empty_state::EmptyState;
use vedge_ui::primitives::tokens::{Size, Variant};

use super::common::Section;

#[component]
pub fn EmptyStatePage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Empty State"</h1>

            <Section title="Title only">
                <div class="rounded-lg border border-border">
                    <EmptyState icon=i::FaMagnifyingGlassSolid title="No results" />
                </div>
            </Section>

            <Section title="Icon + title + description">
                <div class="rounded-lg border border-border">
                    <EmptyState
                        icon=i::FaInboxSolid
                        title="No entries yet"
                        description="Items you add to this vault will show up here."
                    />
                </div>
            </Section>

            <Section title="With a call to action">
                <div class="rounded-lg border border-border">
                    <EmptyState
                        icon=i::FaInboxSolid
                        title="No entries yet"
                        description="Create your first entry to get started."
                    >
                        <Button variant=Variant::Primary size=Size::Sm>
                            <span class="inline-flex items-center gap-1.5">
                                <Icon icon=i::FaPlusSolid attr:aria-hidden="true" />
                                "New item"
                            </span>
                        </Button>
                    </EmptyState>
                </div>
            </Section>
        </div>
    }
}
