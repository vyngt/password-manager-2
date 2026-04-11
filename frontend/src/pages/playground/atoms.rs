use leptos::prelude::*;
use ui::components::badge::Badge;
use ui::components::separator::Separator;
use ui::components::spinner::Spinner;
use ui::primitives::tokens::{
    BadgeAppearance, BadgeShape, BadgeSize, BadgeVariant, Orientation, Size,
};

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
