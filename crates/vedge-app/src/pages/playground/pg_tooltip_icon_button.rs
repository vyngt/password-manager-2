use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::tooltip_icon_button::TooltipIconButton;
use vedge_ui::primitives::tokens::{Placement, Shape, Size, Variant};

use super::common::Section;

#[component]
pub fn TooltipIconButtonPage() -> impl IntoView {
    let (count, set_count) = signal(0_u32);
    let bump = Callback::new(move |_: ()| set_count.update(|n| *n += 1));

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Tooltip Icon Button"</h1>

            <Section title="Variants">
                <div class="flex flex-wrap items-center gap-2">
                    <TooltipIconButton label="Copy" variant=Variant::Ghost>
                        <Icon icon=i::FaCopySolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Save" variant=Variant::Secondary>
                        <Icon icon=i::FaFloppyDiskSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Create" variant=Variant::Primary>
                        <Icon icon=i::FaPlusSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Delete entry" variant=Variant::Danger>
                        <Icon icon=i::FaTrashSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Archive" variant=Variant::Warning>
                        <Icon icon=i::FaBoxArchiveSolid />
                    </TooltipIconButton>
                </div>
            </Section>

            <Section title="Sizes">
                <div class="flex items-center gap-2">
                    <TooltipIconButton label="Small" size=Size::Sm>
                        <Icon icon=i::FaGearSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Medium" size=Size::Md>
                        <Icon icon=i::FaGearSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Large" size=Size::Lg>
                        <Icon icon=i::FaGearSolid />
                    </TooltipIconButton>
                </div>
            </Section>

            <Section title="Shapes">
                <div class="flex items-center gap-2">
                    <TooltipIconButton label="Square" shape=Shape::Square>
                        <Icon icon=i::FaSquareSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Rounded" shape=Shape::Rounded>
                        <Icon icon=i::FaSquareSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Pill" shape=Shape::Pill>
                        <Icon icon=i::FaCircleSolid />
                    </TooltipIconButton>
                </div>
            </Section>

            <Section title="States">
                <div class="flex items-center gap-2">
                    <TooltipIconButton label="Default">
                        <Icon icon=i::FaEyeSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Disabled — tooltip suppressed" disabled=true>
                        <Icon icon=i::FaEyeSlashSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Loading" loading=true>
                        <Icon icon=i::FaArrowsRotateSolid />
                    </TooltipIconButton>
                </div>
            </Section>

            <Section title="Placements">
                <div class="grid grid-cols-4 gap-6 max-w-xl">
                    <TooltipIconButton label="Tooltip on top" tooltip_placement=Placement::Top>
                        <Icon icon=i::FaArrowUpSolid />
                    </TooltipIconButton>
                    <TooltipIconButton
                        label="Tooltip on right"
                        tooltip_placement=Placement::Right
                    >
                        <Icon icon=i::FaArrowRightSolid />
                    </TooltipIconButton>
                    <TooltipIconButton
                        label="Tooltip on bottom"
                        tooltip_placement=Placement::Bottom
                    >
                        <Icon icon=i::FaArrowDownSolid />
                    </TooltipIconButton>
                    <TooltipIconButton label="Tooltip on left" tooltip_placement=Placement::Left>
                        <Icon icon=i::FaArrowLeftSolid />
                    </TooltipIconButton>
                </div>
            </Section>

            <Section title="Interactive — click counter">
                <div class="flex items-center gap-3">
                    <TooltipIconButton
                        label="Increment"
                        variant=Variant::Primary
                        on_click=bump
                    >
                        <Icon icon=i::FaPlusSolid />
                    </TooltipIconButton>
                    <span class="text-sm text-text-secondary">
                        "Clicks: "
                        <code class="text-text-primary">{move || count.get().to_string()}</code>
                    </span>
                </div>
            </Section>

            <div class="text-xs text-text-tertiary">
                "Hover: tooltip appears after 500ms. Focus (Tab): tooltip appears immediately. "
                "Disabled: tooltip never shows."
            </div>
        </div>
    }
}
