use leptos::prelude::*;
use ui::components::badge::Badge;
use ui::components::button::Button;
use ui::components::icon_button::IconButton;
use ui::components::separator::Separator;
use ui::components::spinner::Spinner;
use ui::components::Tooltip;
use ui::primitives::tokens::{
    BadgeAppearance, BadgeShape, BadgeSize, BadgeVariant, Orientation, Placement, Size, Variant,
};

use icondata as i;
use leptos_icons::Icon;

#[component]
pub fn AtomsPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Badge"</h1>

            // Variants — solid
            <Section title="Variants (solid)">
                <div class="flex flex-wrap gap-2">
                    <Badge>"Default"</Badge>
                    <Badge variant=BadgeVariant::Info>"Info"</Badge>
                    <Badge variant=BadgeVariant::Success>"Success"</Badge>
                    <Badge variant=BadgeVariant::Warning>"Warning"</Badge>
                    <Badge variant=BadgeVariant::Danger>"Danger"</Badge>
                </div>
            </Section>

            // Variants — outline
            <Section title="Variants (outline)">
                <div class="flex flex-wrap gap-2">
                    <Badge appearance=BadgeAppearance::Outline>"Default"</Badge>
                    <Badge variant=BadgeVariant::Info appearance=BadgeAppearance::Outline>"Info"</Badge>
                    <Badge variant=BadgeVariant::Success appearance=BadgeAppearance::Outline>"Success"</Badge>
                    <Badge variant=BadgeVariant::Warning appearance=BadgeAppearance::Outline>"Warning"</Badge>
                    <Badge variant=BadgeVariant::Danger appearance=BadgeAppearance::Outline>"Danger"</Badge>
                </div>
            </Section>

            // Sizes
            <Section title="Sizes">
                <div class="flex items-center gap-2">
                    <Badge variant=BadgeVariant::Info size=BadgeSize::Sm>"Small"</Badge>
                    <Badge variant=BadgeVariant::Info size=BadgeSize::Md>"Medium"</Badge>
                </div>
            </Section>

            // Shapes
            <Section title="Shapes">
                <div class="flex items-center gap-2">
                    <Badge variant=BadgeVariant::Info shape=BadgeShape::Pill>"Pill"</Badge>
                    <Badge variant=BadgeVariant::Info shape=BadgeShape::Square>"12"</Badge>
                    <Badge variant=BadgeVariant::Danger shape=BadgeShape::Dot />
                    <Badge variant=BadgeVariant::Success shape=BadgeShape::Dot />
                    <Badge variant=BadgeVariant::Warning shape=BadgeShape::Dot />
                </div>
            </Section>

            <h1 class="text-xl font-semibold text-text-primary pt-4">"Separator"</h1>

            // Horizontal
            <Section title="Horizontal">
                <div class="space-y-3">
                    <p class="text-sm text-text-secondary">"Content above"</p>
                    <Separator />
                    <p class="text-sm text-text-secondary">"Content below"</p>
                </div>
            </Section>

            // Horizontal strong
            <Section title="Horizontal (strong)">
                <div class="space-y-3">
                    <p class="text-sm text-text-secondary">"Content above"</p>
                    <Separator strong=true />
                    <p class="text-sm text-text-secondary">"Content below"</p>
                </div>
            </Section>

            // Labeled
            <Section title="Labeled">
                <div class="space-y-3">
                    <Separator label="General" />
                    <p class="text-sm text-text-secondary">"Section content"</p>
                    <Separator label="Advanced" />
                    <p class="text-sm text-text-secondary">"Section content"</p>
                    <Separator label="Danger Zone" />
                    <p class="text-sm text-text-secondary">"Section content"</p>
                </div>
            </Section>

            // Vertical
            <Section title="Vertical">
                <div class="flex items-center gap-3 h-8">
                    <span class="text-sm text-text-secondary">"Left"</span>
                    <Separator orientation=Orientation::Vertical />
                    <span class="text-sm text-text-secondary">"Right"</span>
                </div>
            </Section>

            <h1 class="text-xl font-semibold text-text-primary pt-4">"Spinner"</h1>

            // Sizes
            <Section title="Sizes">
                <div class="flex items-center gap-4">
                    <Spinner size=Size::Sm />
                    <Spinner size=Size::Md />
                    <Spinner size=Size::Lg />
                </div>
            </Section>

            // Color inheritance
            <Section title="Color Inheritance">
                <div class="flex items-center gap-4">
                    <span class="text-text-secondary"><Spinner /></span>
                    <span class="text-primary"><Spinner /></span>
                    <span class="text-danger"><Spinner /></span>
                    <span class="text-success"><Spinner /></span>
                    <span class="text-warning"><Spinner /></span>
                </div>
            </Section>

            <h1 class="text-xl font-semibold text-text-primary pt-4">"Tooltip"</h1>

            // Placements
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

            // With arrow
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

            // Instant (delay=0)
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

#[component]
fn Section(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-border bg-surface-1 p-4 space-y-3">
            <h3 class="text-xs font-medium text-text-tertiary uppercase tracking-wide">{title}</h3>
            {children()}
        </div>
    }
}
