use leptos::prelude::*;
use vedge_ui::components::foundation::kbd::Kbd;

use super::common::Section;

#[component]
pub fn KbdPage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Kbd"</h1>

            <Section title="Single key">
                <div class="flex items-center gap-3 text-sm text-text-primary">
                    <Kbd>"⌘"</Kbd>
                    <Kbd>"Ctrl"</Kbd>
                    <Kbd>"Shift"</Kbd>
                    <Kbd>"Alt"</Kbd>
                    <Kbd>"Enter"</Kbd>
                    <Kbd>"Esc"</Kbd>
                    <Kbd>"Tab"</Kbd>
                    <Kbd>"↑"</Kbd>
                </div>
            </Section>

            <Section title="Shortcut (consumer composes with a separator)">
                <div class="flex flex-col gap-3 text-sm text-text-primary">
                    <div class="flex items-center gap-1">
                        <Kbd>"Ctrl"</Kbd>
                        <span>"+"</span>
                        <Kbd>"K"</Kbd>
                        <span class="ml-3 text-text-secondary">"Open Command Palette"</span>
                    </div>
                    <div class="flex items-center gap-1">
                        <Kbd>"⌘"</Kbd>
                        <span>"+"</span>
                        <Kbd>"Shift"</Kbd>
                        <span>"+"</span>
                        <Kbd>"P"</Kbd>
                        <span class="ml-3 text-text-secondary">"Run command"</span>
                    </div>
                    <div class="flex items-center gap-2">
                        <Kbd>"g"</Kbd>
                        <span class="text-text-tertiary">"then"</span>
                        <Kbd>"i"</Kbd>
                        <span class="ml-3 text-text-secondary">"Go to inbox"</span>
                    </div>
                </div>
            </Section>

            <Section title="Inherits font-size from context">
                <div class="space-y-3">
                    <p class="text-xs text-text-primary">
                        "Press "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"K"</Kbd>" (text-xs)"
                    </p>
                    <p class="text-sm text-text-primary">
                        "Press "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"K"</Kbd>" (text-sm — default)"
                    </p>
                    <p class="text-base text-text-primary">
                        "Press "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"K"</Kbd>" (text-base)"
                    </p>
                    <p class="text-lg text-text-primary">
                        "Press "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"K"</Kbd>" (text-lg)"
                    </p>
                </div>
            </Section>

            <Section title="Inline in sentence">
                <p class="text-sm text-text-primary leading-relaxed">
                    "Tip: select all entries with "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"A"</Kbd>
                    ", then copy using "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"C"</Kbd>
                    ". Paste into a new vault with "<Kbd>"Ctrl"</Kbd>" + "<Kbd>"V"</Kbd>"."
                </p>
            </Section>
        </div>
    }
}
