use leptos::prelude::*;
use vedge_ui::components::badge::Badge;
use vedge_ui::primitives::tokens::{BadgeAppearance, BadgeShape, BadgeSize, BadgeVariant};

use super::common::Section;

#[component]
pub fn BadgePage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Badge"</h1>

            <Section title="Variants (solid)">
                <div class="flex flex-wrap gap-2">
                    <Badge>"Default"</Badge>
                    <Badge variant=BadgeVariant::Info>"Info"</Badge>
                    <Badge variant=BadgeVariant::Success>"Success"</Badge>
                    <Badge variant=BadgeVariant::Warning>"Warning"</Badge>
                    <Badge variant=BadgeVariant::Danger>"Danger"</Badge>
                </div>
            </Section>

            <Section title="Variants (outline)">
                <div class="flex flex-wrap gap-2">
                    <Badge appearance=BadgeAppearance::Outline>"Default"</Badge>
                    <Badge variant=BadgeVariant::Info appearance=BadgeAppearance::Outline>
                        "Info"
                    </Badge>
                    <Badge variant=BadgeVariant::Success appearance=BadgeAppearance::Outline>
                        "Success"
                    </Badge>
                    <Badge variant=BadgeVariant::Warning appearance=BadgeAppearance::Outline>
                        "Warning"
                    </Badge>
                    <Badge variant=BadgeVariant::Danger appearance=BadgeAppearance::Outline>
                        "Danger"
                    </Badge>
                </div>
            </Section>

            <Section title="Sizes">
                <div class="flex items-center gap-2">
                    <Badge variant=BadgeVariant::Info size=BadgeSize::Sm>
                        "Small"
                    </Badge>
                    <Badge variant=BadgeVariant::Info size=BadgeSize::Md>
                        "Medium"
                    </Badge>
                </div>
            </Section>

            <Section title="Shapes">
                <div class="flex items-center gap-2">
                    <Badge variant=BadgeVariant::Info shape=BadgeShape::Pill>
                        "Pill"
                    </Badge>
                    <Badge variant=BadgeVariant::Info shape=BadgeShape::Square>
                        "12"
                    </Badge>
                    <Badge variant=BadgeVariant::Danger shape=BadgeShape::Dot />
                    <Badge variant=BadgeVariant::Success shape=BadgeShape::Dot />
                    <Badge variant=BadgeVariant::Warning shape=BadgeShape::Dot />
                </div>
            </Section>
        </div>
    }
}
