use leptos::prelude::*;
use ui::components::button::Button;
use ui::components::icon_button::IconButton;
use ui::primitives::tokens::{Shape, Size, Variant};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn ButtonsPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Buttons"</h1>

            // Variants
            <Section title="Variants">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Primary>"Primary"</Button>
                    <Button variant=Variant::Secondary>"Secondary"</Button>
                    <Button variant=Variant::Ghost>"Ghost"</Button>
                    <Button variant=Variant::Danger>"Danger"</Button>
                    <Button variant=Variant::Warning>"Warning"</Button>
                </div>
            </Section>

            // Sizes
            <Section title="Sizes">
                <div class="flex items-end gap-2">
                    <Button variant=Variant::Primary size=Size::Sm>"Small"</Button>
                    <Button variant=Variant::Primary size=Size::Md>"Medium"</Button>
                    <Button variant=Variant::Primary size=Size::Lg>"Large"</Button>
                </div>
            </Section>

            // Shapes
            <Section title="Shapes">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Primary shape=Shape::Square>"Square"</Button>
                    <Button variant=Variant::Primary shape=Shape::RoundedSm>"Rounded SM"</Button>
                    <Button variant=Variant::Primary shape=Shape::Rounded>"Rounded"</Button>
                    <Button variant=Variant::Primary shape=Shape::Pill>"Pill"</Button>
                </div>
            </Section>

            // States
            <Section title="States">
                <div class="flex flex-wrap gap-2">
                    <Button variant=Variant::Primary>"Default"</Button>
                    <Button variant=Variant::Primary disabled=true>"Disabled"</Button>
                    <Button variant=Variant::Primary loading=true>"Loading"</Button>
                </div>
            </Section>

            // Full width
            <Section title="Full Width">
                <Button variant=Variant::Primary full_width=true>"Full Width Button"</Button>
            </Section>

            <h1 class="text-xl font-semibold text-text-primary pt-4">"Icon Buttons"</h1>

            // Icon button variants
            <Section title="Variants">
                <div class="flex flex-wrap gap-2">
                    <IconButton aria_label="Primary" variant=Variant::Primary>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Secondary" variant=Variant::Secondary>
                        <Icon icon=i::FaGearSolid />
                    </IconButton>
                    <IconButton aria_label="Ghost">
                        <Icon icon=i::FaMagnifyingGlassSolid />
                    </IconButton>
                    <IconButton aria_label="Danger" variant=Variant::Danger>
                        <Icon icon=i::FaTrashSolid />
                    </IconButton>
                    <IconButton aria_label="Warning" variant=Variant::Warning>
                        <Icon icon=i::FaTriangleExclamationSolid />
                    </IconButton>
                </div>
            </Section>

            // Icon button sizes
            <Section title="Sizes">
                <div class="flex items-end gap-2">
                    <IconButton aria_label="Small" variant=Variant::Primary size=Size::Sm>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Medium" variant=Variant::Primary size=Size::Md>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Large" variant=Variant::Primary size=Size::Lg>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                </div>
            </Section>

            // Icon button shapes
            <Section title="Shapes">
                <div class="flex flex-wrap gap-2">
                    <IconButton aria_label="Square" variant=Variant::Primary shape=Shape::Square>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Rounded SM" variant=Variant::Primary shape=Shape::RoundedSm>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Rounded" variant=Variant::Primary shape=Shape::Rounded>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Pill" variant=Variant::Primary shape=Shape::Pill>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                </div>
            </Section>

            // Icon button states
            <Section title="States">
                <div class="flex flex-wrap gap-2">
                    <IconButton aria_label="Default" variant=Variant::Primary>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Disabled" variant=Variant::Primary disabled=true>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                    <IconButton aria_label="Loading" variant=Variant::Primary loading=true>
                        <Icon icon=i::FaStarSolid />
                    </IconButton>
                </div>
            </Section>
        </div>
    }
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
