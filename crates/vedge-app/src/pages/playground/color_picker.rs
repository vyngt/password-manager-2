use leptos::prelude::*;
use vedge_ui::components::form::color_picker::{ColorFormat, ColorPicker, SwatchItem, TriggerMode};
use vedge_ui::primitives::tokens::Size;

#[component]
pub fn ColorPickerPage() -> impl IntoView {
    let (color_val, set_color_val) = signal("#2563EB".to_owned());
    let (last_change_end, set_last_change_end) = signal(String::new());

    let preset_swatches = vec![
        SwatchItem {
            value: "#EF4444".into(),
            label: Some("Red".into()),
        },
        SwatchItem {
            value: "#F97316".into(),
            label: Some("Orange".into()),
        },
        SwatchItem {
            value: "#EAB308".into(),
            label: Some("Yellow".into()),
        },
        SwatchItem {
            value: "#22C55E".into(),
            label: Some("Green".into()),
        },
        SwatchItem {
            value: "#3B82F6".into(),
            label: Some("Blue".into()),
        },
        SwatchItem {
            value: "#8B5CF6".into(),
            label: Some("Purple".into()),
        },
        SwatchItem {
            value: "#EC4899".into(),
            label: Some("Pink".into()),
        },
        SwatchItem {
            value: "#000000".into(),
            label: Some("Black".into()),
        },
        SwatchItem {
            value: "#FFFFFF".into(),
            label: Some("White".into()),
        },
        SwatchItem {
            value: "#6B7280".into(),
            label: Some("Gray".into()),
        },
    ];
    let swatches_1 = preset_swatches.clone();
    let swatches_2 = preset_swatches;

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"ColorPicker"</h1>

            // Default
            <Section title="Default">
                <div class="max-w-xs">
                    <ColorPicker default_value="#2563EB" />
                </div>
            </Section>

            // Sizes
            <Section title="Sizes">
                <div class="space-y-3 max-w-xs">
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"Small"</p>
                        <ColorPicker default_value="#EF4444" size=Size::Sm />
                    </div>
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"Medium (default)"</p>
                        <ColorPicker default_value="#3B82F6" />
                    </div>
                </div>
            </Section>

            // Trigger modes
            <Section title="Trigger Modes">
                <div class="flex items-start gap-4">
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"Swatch + Input (default)"</p>
                        <div class="max-w-xs">
                            <ColorPicker default_value="#8B5CF6" />
                        </div>
                    </div>
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"Swatch Only"</p>
                        <ColorPicker default_value="#8B5CF6" trigger_mode=TriggerMode::SwatchOnly />
                    </div>
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"Swatch Only (sm)"</p>
                        <ColorPicker
                            default_value="#22C55E"
                            trigger_mode=TriggerMode::SwatchOnly
                            size=Size::Sm
                        />
                    </div>
                </div>
            </Section>

            // With alpha
            <Section title="With Alpha">
                <div class="max-w-xs">
                    <ColorPicker default_value="#2563EBCC" alpha=true />
                </div>
            </Section>

            // Formats
            <Section title="Output Formats">
                <div class="space-y-3 max-w-xs">
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"Hex (default)"</p>
                        <ColorPicker default_value="#2563EB" format=ColorFormat::Hex />
                    </div>
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"RGB"</p>
                        <ColorPicker default_value="#2563EB" format=ColorFormat::Rgb />
                    </div>
                    <div class="space-y-1">
                        <p class="text-xs text-text-tertiary">"HSL"</p>
                        <ColorPicker default_value="#2563EB" format=ColorFormat::Hsl />
                    </div>
                </div>
            </Section>

            // With swatches
            {
                let sw = swatches_1;
                view! {
                    <Section title="Preset Swatches">
                        <div class="max-w-xs">
                            <ColorPicker default_value="#3B82F6" swatches=sw />
                        </div>
                    </Section>
                }
            }

            // Full featured
            {
                let sw = swatches_2;
                view! {
                    <Section title="Full Featured (alpha + swatches + RGB)">
                        <div class="max-w-xs">
                            <ColorPicker
                                default_value="#8B5CF680"
                                alpha=true
                                format=ColorFormat::Rgb
                                swatches=sw
                            />
                        </div>
                    </Section>
                }
            }

            // Disabled
            <Section title="Disabled">
                <div class="flex items-start gap-4">
                    <div class="max-w-xs flex-1">
                        <ColorPicker default_value="#6B7280" disabled=true />
                    </div>
                    <ColorPicker
                        default_value="#6B7280"
                        disabled=true
                        trigger_mode=TriggerMode::SwatchOnly
                    />
                </div>
            </Section>

            // Interactive / controlled
            <Section title="Interactive (Controlled)">
                <div class="space-y-3 max-w-xs">
                    <ColorPicker
                        value=Signal::derive(move || color_val.get())
                        on_change=Callback::new(move |v: String| set_color_val.set(v))
                        on_change_end=Callback::new(move |v: String| set_last_change_end.set(v))
                    />
                    <div class="space-y-1 text-xs text-text-tertiary">
                        <p>
                            "Value: "
                            <span class="font-mono text-text-primary">
                                {move || color_val.get()}
                            </span>
                        </p>
                        <p>
                            "Last onChangeEnd: "
                            <span class="font-mono text-text-primary">
                                {move || {
                                    let v = last_change_end.get();
                                    if v.is_empty() { "(none)".to_owned() } else { v }
                                }}
                            </span>
                        </p>
                    </div>
                    <div
                        class="h-12 rounded-lg border border-border"
                        style=move || format!("background-color: {}", color_val.get())
                    />
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
