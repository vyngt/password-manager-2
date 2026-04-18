use leptos::prelude::*;
use vedge_ui::components::spinner::Spinner;
use vedge_ui::primitives::tokens::Size;

use super::common::Section;

#[component]
pub fn SpinnerPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Spinner"</h1>

            <Section title="Sizes">
                <div class="flex items-center gap-4">
                    <Spinner size=Size::Sm />
                    <Spinner size=Size::Md />
                    <Spinner size=Size::Lg />
                </div>
            </Section>

            <Section title="Color Inheritance">
                <div class="flex items-center gap-4">
                    <span class="text-text-secondary">
                        <Spinner />
                    </span>
                    <span class="text-primary">
                        <Spinner />
                    </span>
                    <span class="text-danger">
                        <Spinner />
                    </span>
                    <span class="text-success">
                        <Spinner />
                    </span>
                    <span class="text-warning">
                        <Spinner />
                    </span>
                </div>
            </Section>
        </div>
    }
}
