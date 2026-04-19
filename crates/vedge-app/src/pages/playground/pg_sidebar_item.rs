use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;
use vedge_ui::components::badge::Badge;
use vedge_ui::components::sidebar_item::SidebarItem;
use vedge_ui::primitives::tokens::{BadgeShape, BadgeSize, BadgeVariant};

use super::common::Section;

#[component]
pub fn SidebarItemPage() -> impl IntoView {
    let (active, set_active) = signal("vault");

    view! {
        <div class="p-6 max-w-4xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Sidebar Item"</h1>

            <Section title="States">
                <div class="w-[220px] p-2 border border-border rounded-md bg-background">
                    <SidebarItem
                        label="Default"
                        icon=Box::new(|| view! { <Icon icon=i::FaFolderSolid /> }.into_any())
                        on_click=Callback::new(|_: ()| {})
                    />
                    <SidebarItem
                        label="Selected"
                        icon=Box::new(|| view! { <Icon icon=i::FaShieldSolid /> }.into_any())
                        selected=Signal::stored(true)
                        on_click=Callback::new(|_: ()| {})
                    />
                    <SidebarItem
                        label="Disabled"
                        icon=Box::new(|| view! { <Icon icon=i::FaLockSolid /> }.into_any())
                        disabled=true
                        on_click=Callback::new(|_: ()| {})
                    />
                </div>
            </Section>

            <Section title="With Badge">
                <div class="w-[220px] p-2 border border-border rounded-md bg-background">
                    <SidebarItem
                        label="Inbox"
                        icon=Box::new(|| view! { <Icon icon=i::FaInboxSolid /> }.into_any())
                        badge=Box::new(|| {
                            view! { <Badge size=BadgeSize::Sm>"12"</Badge> }.into_any()
                        })
                        on_click=Callback::new(|_: ()| {})
                    />
                    <SidebarItem
                        label="Breach Monitor"
                        icon=Box::new(|| {
                            view! { <Icon icon=i::FaTriangleExclamationSolid /> }.into_any()
                        })
                        badge=Box::new(|| {
                            view! {
                                <Badge variant=BadgeVariant::Danger size=BadgeSize::Sm>
                                    "3"
                                </Badge>
                            }
                                .into_any()
                        })
                        on_click=Callback::new(|_: ()| {})
                    />
                    <SidebarItem
                        label="Notifications"
                        icon=Box::new(|| view! { <Icon icon=i::FaBellSolid /> }.into_any())
                        badge=Box::new(|| {
                            view! {
                                <Badge
                                    variant=BadgeVariant::Danger
                                    shape=BadgeShape::Dot
                                    size=BadgeSize::Sm
                                />
                            }
                                .into_any()
                        })
                        selected=Signal::stored(true)
                        on_click=Callback::new(|_: ()| {})
                    />
                </div>
            </Section>

            <Section title="Indentation">
                <div class="w-[220px] p-2 border border-border rounded-md bg-background">
                    <SidebarItem
                        label="PKI"
                        icon=Box::new(|| view! { <Icon icon=i::FaKeySolid /> }.into_any())
                        on_click=Callback::new(|_: ()| {})
                    />
                    <SidebarItem label="SSH Keys" indent=1 on_click=Callback::new(|_: ()| {}) />
                    <SidebarItem
                        label="GPG Keys"
                        indent=1
                        selected=Signal::stored(true)
                        on_click=Callback::new(|_: ()| {})
                    />
                    <SidebarItem label="Ed25519" indent=2 on_click=Callback::new(|_: ()| {}) />
                    <SidebarItem label="RSA 4096" indent=2 on_click=Callback::new(|_: ()| {}) />
                </div>
            </Section>

            <Section title="No icon / truncation">
                <div class="w-[220px] p-2 border border-border rounded-md bg-background">
                    <SidebarItem label="Settings" on_click=Callback::new(|_: ()| {}) />
                    <SidebarItem
                        label="An extraordinarily long label that should truncate with ellipsis"
                        icon=Box::new(|| view! { <Icon icon=i::FaEllipsisSolid /> }.into_any())
                        on_click=Callback::new(|_: ()| {})
                    />
                </div>
            </Section>

            <Section title="Link variant (href)">
                <div class="w-[220px] p-2 border border-border rounded-md bg-background">
                    <SidebarItem
                        label="Open documentation"
                        icon=Box::new(|| {
                            view! { <Icon icon=i::FaArrowUpRightFromSquareSolid /> }.into_any()
                        })
                        href="https://example.com"
                    />
                    <SidebarItem
                        label="Disabled link"
                        icon=Box::new(|| view! { <Icon icon=i::FaLinkSolid /> }.into_any())
                        href="https://example.com"
                        disabled=true
                    />
                </div>
            </Section>

            <Section title="Interactive — controlled selection">
                <div class="flex gap-6">
                    <div class="w-[220px] p-2 border border-border rounded-md bg-background">
                        <SidebarItem
                            label="Vault"
                            icon=Box::new(|| view! { <Icon icon=i::FaShieldSolid /> }.into_any())
                            selected=Signal::derive(move || active.get() == "vault")
                            on_click=Callback::new(move |_: ()| set_active.set("vault"))
                        />
                        <SidebarItem
                            label="Generator"
                            icon=Box::new(|| {
                                view! { <Icon icon=i::FaWandMagicSparklesSolid /> }.into_any()
                            })
                            selected=Signal::derive(move || active.get() == "generator")
                            on_click=Callback::new(move |_: ()| set_active.set("generator"))
                        />
                        <SidebarItem
                            label="Breach Monitor"
                            icon=Box::new(|| {
                                view! { <Icon icon=i::FaTriangleExclamationSolid /> }.into_any()
                            })
                            selected=Signal::derive(move || active.get() == "breach")
                            badge=Box::new(|| {
                                view! {
                                    <Badge variant=BadgeVariant::Danger size=BadgeSize::Sm>
                                        "3"
                                    </Badge>
                                }
                                    .into_any()
                            })
                            on_click=Callback::new(move |_: ()| set_active.set("breach"))
                        />
                        <SidebarItem
                            label="Settings"
                            icon=Box::new(|| view! { <Icon icon=i::FaGearSolid /> }.into_any())
                            selected=Signal::derive(move || active.get() == "settings")
                            on_click=Callback::new(move |_: ()| set_active.set("settings"))
                        />
                    </div>
                    <div class="text-sm text-text-secondary self-start">
                        "Active route: "
                        <code class="text-text-primary">{move || active.get()}</code>
                    </div>
                </div>
            </Section>
        </div>
    }
}
