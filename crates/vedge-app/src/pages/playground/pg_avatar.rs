use leptos::prelude::*;
use vedge_ui::components::avatar::Avatar;
use vedge_ui::primitives::tokens::{AvatarSize, AvatarStatus};

use super::common::Section;

const PHOTO: &str = "https://images.unsplash.com/photo-1502685104226-ee32379fefbe?auto=format&fit=facearea&facepad=3&w=256&h=256&q=80";
const BROKEN: &str = "https://example.com/definitely-missing-avatar.png";

#[component]
pub fn AvatarPage() -> impl IntoView {
    let click_count = RwSignal::new(0u32);
    let increment = Callback::new(move |_: web_sys::MouseEvent| {
        click_count.update(|n| *n += 1);
    });

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Avatar"</h1>

            <Section title="Sizes">
                <div class="flex items-end gap-4">
                    <Avatar
                        src=PHOTO.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Xs
                    />
                    <Avatar
                        src=PHOTO.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Sm
                    />
                    <Avatar
                        src=PHOTO.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Md
                    />
                    <Avatar
                        src=PHOTO.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Lg
                    />
                    <Avatar
                        src=PHOTO.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Xl
                    />
                </div>
            </Section>

            <Section title="Fallback cascade">
                <div class="flex items-center gap-4">
                    <div class="flex flex-col items-center gap-1">
                        <Avatar
                            src=PHOTO.to_owned()
                            name="Ada Lovelace".to_owned()
                            size=AvatarSize::Lg
                        />
                        <span class="text-xs text-text-tertiary">"image"</span>
                    </div>
                    <div class="flex flex-col items-center gap-1">
                        <Avatar name="Grace Hopper".to_owned() size=AvatarSize::Lg />
                        <span class="text-xs text-text-tertiary">"initials"</span>
                    </div>
                    <div class="flex flex-col items-center gap-1">
                        <Avatar size=AvatarSize::Lg />
                        <span class="text-xs text-text-tertiary">"icon"</span>
                    </div>
                </div>
            </Section>

            <Section title="Initials derivation">
                <div class="flex items-center gap-4">
                    <div class="flex flex-col items-center gap-1">
                        <Avatar name="Ada Lovelace".to_owned() size=AvatarSize::Lg />
                        <span class="text-xs text-text-tertiary">"Ada Lovelace"</span>
                    </div>
                    <div class="flex flex-col items-center gap-1">
                        <Avatar name="Cher".to_owned() size=AvatarSize::Lg />
                        <span class="text-xs text-text-tertiary">"Cher"</span>
                    </div>
                    <div class="flex flex-col items-center gap-1">
                        <Avatar name="Dr. Martin Luther King Jr.".to_owned() size=AvatarSize::Lg />
                        <span class="text-xs text-text-tertiary">"Dr. Martin Luther King Jr."</span>
                    </div>
                    <div class="flex flex-col items-center gap-1">
                        <Avatar name="maria o'neill".to_owned() size=AvatarSize::Lg />
                        <span class="text-xs text-text-tertiary">"maria o\'neill"</span>
                    </div>
                </div>
            </Section>

            <Section title="Status — all statuses, all sizes">
                <div class="space-y-3">
                    {[
                        AvatarStatus::Online,
                        AvatarStatus::Away,
                        AvatarStatus::Busy,
                        AvatarStatus::Offline,
                    ]
                        .iter()
                        .map(|st| {
                            let s = *st;
                            let label = match s {
                                AvatarStatus::Online => "online",
                                AvatarStatus::Away => "away",
                                AvatarStatus::Busy => "busy",
                                AvatarStatus::Offline => "offline",
                            };
                            view! {
                                <div class="flex items-end gap-4">
                                    <span class="w-16 text-xs text-text-tertiary">{label}</span>
                                    <Avatar
                                        src=PHOTO.to_owned()
                                        name="Ada".to_owned()
                                        size=AvatarSize::Xs
                                        status=Some(s)
                                    />
                                    <Avatar
                                        src=PHOTO.to_owned()
                                        name="Ada".to_owned()
                                        size=AvatarSize::Sm
                                        status=Some(s)
                                    />
                                    <Avatar
                                        src=PHOTO.to_owned()
                                        name="Ada".to_owned()
                                        size=AvatarSize::Md
                                        status=Some(s)
                                    />
                                    <Avatar
                                        src=PHOTO.to_owned()
                                        name="Ada".to_owned()
                                        size=AvatarSize::Lg
                                        status=Some(s)
                                    />
                                    <Avatar
                                        src=PHOTO.to_owned()
                                        name="Ada".to_owned()
                                        size=AvatarSize::Xl
                                        status=Some(s)
                                    />
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            </Section>

            <Section title="Interactive">
                <div class="flex items-center gap-6">
                    <div class="flex items-center gap-3">
                        <Avatar
                            src=PHOTO.to_owned()
                            name="Ada Lovelace".to_owned()
                            size=AvatarSize::Lg
                            interactive=true
                            status=Some(AvatarStatus::Online)
                            on_click=increment
                        />
                        <span class="text-sm text-text-secondary">
                            "clicks: " {move || click_count.get()}
                        </span>
                    </div>

                    <Avatar
                        name="Grace Hopper".to_owned()
                        size=AvatarSize::Lg
                        interactive=true
                        disabled=true
                        aria_label="Grace Hopper (disabled)".to_owned()
                    />
                </div>
            </Section>

            <Section title="Broken image — falls back to initials">
                <div class="flex items-center gap-4">
                    <Avatar
                        src=BROKEN.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Lg
                    />
                    <span class="text-xs text-text-tertiary">
                        "src=404 → initials (AL) appear when the image errors"
                    </span>
                </div>
            </Section>

            <Section title="In a row (composition)">
                <div class="flex items-center gap-3">
                    <Avatar
                        src=PHOTO.to_owned()
                        name="Ada Lovelace".to_owned()
                        size=AvatarSize::Sm
                        status=Some(AvatarStatus::Online)
                    />
                    <div class="flex flex-col gap-0.5">
                        <span class="text-sm font-medium text-text-primary">"Ada Lovelace"</span>
                        <span class="text-xs text-text-secondary">"Enchantress of Numbers"</span>
                    </div>
                </div>
            </Section>
        </div>
    }
}
