use leptos::prelude::*;
use vedge_ui::components::button::Button;
use vedge_ui::primitives::tokens::{Shape, Size, Variant};

use super::common::Section;

#[component]
pub fn ButtonPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Button"</h1>

            <Section title="Variants">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Primary>"Primary"</Button>
                    <Button variant=Variant::Secondary>"Secondary"</Button>
                    <Button variant=Variant::Ghost>"Ghost"</Button>
                    <Button variant=Variant::Danger>"Danger"</Button>
                    <Button variant=Variant::Warning>"Warning"</Button>
                </div>
            </Section>

            <Section title="Sizes">
                <div class="flex items-end gap-2">
                    <Button variant=Variant::Primary size=Size::Sm>
                        "Small"
                    </Button>
                    <Button variant=Variant::Primary size=Size::Md>
                        "Medium"
                    </Button>
                    <Button variant=Variant::Primary size=Size::Lg>
                        "Large"
                    </Button>
                </div>
            </Section>

            <Section title="Shapes">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Primary shape=Shape::Square>
                        "Square"
                    </Button>
                    <Button variant=Variant::Primary shape=Shape::RoundedSm>
                        "Rounded SM"
                    </Button>
                    <Button variant=Variant::Primary shape=Shape::Rounded>
                        "Rounded"
                    </Button>
                    <Button variant=Variant::Primary shape=Shape::Pill>
                        "Pill"
                    </Button>
                </div>
            </Section>

            <Section title="States">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Primary>"Default"</Button>
                    <Button variant=Variant::Primary disabled=true>
                        "Disabled"
                    </Button>
                    <Button variant=Variant::Primary loading=true>
                        "Loading"
                    </Button>
                </div>
            </Section>

            <Section title="Full Width">
                <Button variant=Variant::Primary full_width=true>
                    "Full Width Button"
                </Button>
            </Section>
        </div>
    }
}
