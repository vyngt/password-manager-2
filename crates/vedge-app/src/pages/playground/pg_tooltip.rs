use leptos::prelude::*;
use vedge_ui::components::Tooltip;
use vedge_ui::components::button::Button;
use vedge_ui::components::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Placement, Variant};

use icondata as i;
use leptos_icons::Icon;

use super::common::Section;

#[component]
pub fn TooltipPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Tooltip"</h1>

            <Section title="Placements">
                <div class="flex flex-wrap items-center gap-4 py-8 justify-center">
                    <Tooltip content="Appears above" placement=Placement::Top>
                        <Button variant=Variant::Secondary>"Top"</Button>
                    </Tooltip>
                    <Tooltip content="Appears below" placement=Placement::Bottom>
                        <Button variant=Variant::Secondary>"Bottom"</Button>
                    </Tooltip>
                    <Tooltip content="Appears left" placement=Placement::Left>
                        <Button variant=Variant::Secondary>"Left"</Button>
                    </Tooltip>
                    <Tooltip content="Appears right" placement=Placement::Right>
                        <Button variant=Variant::Secondary>"Right"</Button>
                    </Tooltip>
                </div>
            </Section>

            <Section title="With Arrow">
                <div class="flex flex-wrap items-center gap-4 py-8 justify-center">
                    <Tooltip content="Lock vault" arrow=true>
                        <IconButton aria_label="Lock" variant=Variant::Primary>
                            <Icon icon=i::FaStarSolid />
                        </IconButton>
                    </Tooltip>
                    <Tooltip content="Settings" arrow=true placement=Placement::Bottom>
                        <IconButton aria_label="Settings" variant=Variant::Secondary>
                            <Icon icon=i::FaGearSolid />
                        </IconButton>
                    </Tooltip>
                    <Tooltip content="Delete item" arrow=true placement=Placement::Right>
                        <IconButton aria_label="Delete" variant=Variant::Danger>
                            <Icon icon=i::FaTrashSolid />
                        </IconButton>
                    </Tooltip>
                </div>
            </Section>

            <Section title="Instant (delay=0)">
                <div class="flex items-center gap-4 py-4">
                    <Tooltip content="No delay" delay=0>
                        <Button variant=Variant::Primary>"Hover me"</Button>
                    </Tooltip>
                </div>
            </Section>
        </div>
    }
}
