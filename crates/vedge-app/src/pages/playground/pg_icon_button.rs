use leptos::prelude::*;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Shape, Size, Variant};

use icondata as i;
use leptos_icons::Icon;

use super::common::Section;

#[component]
pub fn IconButtonPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Icon Button"</h1>

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
