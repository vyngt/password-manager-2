use icondata as i;
use leptos::prelude::*;
use vedge_ui::components::dropdown_menu::{
    DropdownMenu, MenuEntry, MenuItem, MenuItemVariant, MenuSection,
};
use vedge_ui::components::feedback::popover::PopoverPlacement;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::components::foundation::tooltip_icon_button::TooltipIconButton;
use vedge_ui::primitives::tokens::Variant;

use super::common::Section;

fn basic_items() -> Vec<MenuSection> {
    vec![MenuSection {
        label: None,
        items: vec![
            MenuEntry::Item(MenuItem {
                id: "edit".into(),
                label: "Edit".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaPenSolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "duplicate".into(),
                label: "Duplicate".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaCloneSolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
            MenuEntry::Separator,
            MenuEntry::Item(MenuItem {
                id: "delete".into(),
                label: "Delete".into(),
                variant: MenuItemVariant::Danger,
                icon: Some(i::FaTrashSolid),
                shortcut: None,
                on_click: None,
                href: None,
            }),
        ],
    }]
}

#[component]
pub fn DropdownMenuPage() -> impl IntoView {
    let (last_action, set_last_action) = signal(String::new());
    let make_action =
        move |name: &'static str| Callback::new(move |_: ()| set_last_action.set(name.to_string()));

    let (open_count, set_open_count) = signal(0_u32);
    let (close_count, set_close_count) = signal(0_u32);

    let tracked_items = vec![MenuSection {
        label: None,
        items: vec![
            MenuEntry::Item(MenuItem {
                id: "a".into(),
                label: "Action A".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaCircleCheckSolid),
                shortcut: None,
                on_click: Some(make_action("A")),
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "b".into(),
                label: "Action B".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaCircleCheckSolid),
                shortcut: None,
                on_click: Some(make_action("B")),
                href: None,
            }),
        ],
    }];

    let grouped_items = vec![MenuSection {
        label: Some("Sort by".into()),
        items: vec![
            MenuEntry::Item(MenuItem {
                id: "sort-name".into(),
                label: "Name".into(),
                variant: MenuItemVariant::Default,
                icon: None,
                shortcut: Some("N".into()),
                on_click: Some(make_action("sort:name")),
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "sort-modified".into(),
                label: "Last modified".into(),
                variant: MenuItemVariant::Default,
                icon: None,
                shortcut: Some("M".into()),
                on_click: Some(make_action("sort:modified")),
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "sort-size".into(),
                label: "Size".into(),
                variant: MenuItemVariant::Default,
                icon: None,
                shortcut: Some("S".into()),
                on_click: Some(make_action("sort:size")),
                href: None,
            }),
        ],
    }];

    let with_href = vec![MenuSection {
        label: None,
        items: vec![
            MenuEntry::Item(MenuItem {
                id: "settings".into(),
                label: "Settings".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaGearSolid),
                shortcut: None,
                on_click: Some(make_action("settings")),
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "docs".into(),
                label: "Open documentation".into(),
                variant: MenuItemVariant::Default,
                icon: Some(i::FaArrowUpRightFromSquareSolid),
                shortcut: None,
                on_click: None,
                href: Some("https://example.com".into()),
            }),
        ],
    }];

    let with_disabled = vec![MenuSection {
        label: None,
        items: vec![
            MenuEntry::Item(MenuItem {
                id: "one".into(),
                label: "First".into(),
                variant: MenuItemVariant::Default,
                icon: None,
                shortcut: None,
                on_click: Some(make_action("first")),
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "two".into(),
                label: "Unavailable (disabled)".into(),
                variant: MenuItemVariant::Disabled,
                icon: None,
                shortcut: None,
                on_click: None,
                href: None,
            }),
            MenuEntry::Item(MenuItem {
                id: "three".into(),
                label: "Last".into(),
                variant: MenuItemVariant::Default,
                icon: None,
                shortcut: None,
                on_click: Some(make_action("last")),
                href: None,
            }),
        ],
    }];

    let typeahead_items = vec![MenuSection {
        label: None,
        items: (b'a'..=b'j')
            .map(|c| {
                let letter = (c as char).to_ascii_uppercase();
                MenuEntry::Item(MenuItem {
                    id: format!("letter-{letter}"),
                    label: format!("Item {letter}"),
                    variant: MenuItemVariant::Default,
                    icon: None,
                    shortcut: None,
                    on_click: Some(make_action("typeahead")),
                    href: None,
                })
            })
            .collect(),
    }];

    view! {
        <div class="p-6 max-w-3xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Dropdown Menu"</h1>

            <Section title="Basic">
                <div class="flex items-center gap-3">
                    <DropdownMenu
                        trigger=Box::new(|| {
                            view! {
                                <TooltipIconButton label="More actions">
                                    <Icon icon=i::FaEllipsisSolid />
                                </TooltipIconButton>
                            }
                                .into_any()
                        })
                        items=basic_items()
                    />
                    <span class="text-xs text-text-tertiary">
                        "Default items, separator, danger delete."
                    </span>
                </div>
            </Section>

            <Section title="Grouped with keyboard shortcuts">
                <DropdownMenu
                    trigger=Box::new(|| {
                        view! { <Button variant=Variant::Secondary>"Sort by"</Button> }.into_any()
                    })
                    items=grouped_items
                />
            </Section>

            <Section title="Link items (href)">
                <DropdownMenu
                    trigger=Box::new(|| {
                        view! { <Button variant=Variant::Secondary>"Open…"</Button> }.into_any()
                    })
                    items=with_href
                />
            </Section>

            <Section title="Disabled items — arrow nav + typeahead skip them">
                <DropdownMenu
                    trigger=Box::new(|| {
                        view! { <Button variant=Variant::Secondary>"Menu with disabled"</Button> }
                            .into_any()
                    })
                    items=with_disabled
                />
            </Section>

            <Section title="Typeahead">
                <div class="flex items-center gap-3">
                    <DropdownMenu
                        trigger=Box::new(|| {
                            view! { <Button variant=Variant::Secondary>"A–J"</Button> }.into_any()
                        })
                        items=typeahead_items
                    />
                    <span class="text-xs text-text-tertiary">
                        "Open the menu and press a letter (A-J) — focus jumps to that item. Buffer clears after 500ms."
                    </span>
                </div>
            </Section>

            <Section title="Disabled trigger">
                <DropdownMenu
                    disabled=true
                    trigger=Box::new(|| {
                        view! {
                            <Button variant=Variant::Secondary disabled=true>
                                "Can't open"
                            </Button>
                        }
                            .into_any()
                    })
                    items=basic_items()
                />
            </Section>

            <Section title="Placement — top-end">
                <div class="pt-32">
                    <DropdownMenu
                        placement=PopoverPlacement::TopEnd
                        trigger=Box::new(|| {
                            view! { <Button variant=Variant::Secondary>"Menu (top-end)"</Button> }
                                .into_any()
                        })
                        items=basic_items()
                    />
                </div>
            </Section>

            <Section title="on_open / on_close counters">
                <div class="flex items-center gap-3">
                    <DropdownMenu
                        trigger=Box::new(|| {
                            view! { <Button variant=Variant::Secondary>"Tracked"</Button> }
                                .into_any()
                        })
                        items=tracked_items
                        on_open=Callback::new(move |_: ()| set_open_count.update(|n| *n += 1))
                        on_close=Callback::new(move |_: ()| set_close_count.update(|n| *n += 1))
                    />
                    <div class="text-xs text-text-secondary">
                        "Opens: "
                        <code class="text-text-primary">
                            {move || open_count.get().to_string()}
                        </code> " · Closes: "
                        <code class="text-text-primary">
                            {move || close_count.get().to_string()}
                        </code>
                    </div>
                </div>
            </Section>

            <div class="text-xs text-text-tertiary">
                "Last action: "
                <code class="text-text-primary">
                    {move || {
                        let s = last_action.get();
                        if s.is_empty() { "(none)".to_string() } else { s }
                    }}
                </code>
            </div>

            <div class="text-xs text-text-tertiary">
                "Keyboard: Enter/Space/ArrowDown on trigger opens menu. Inside: Arrow Up/Down navigate (wraps, skips disabled). "
                "Home/End jump to first/last. Letters typeahead. Enter activates. Escape closes (focus returns to trigger). "
                "Tab closes and continues focus in document."
            </div>
        </div>
    }
}

use leptos_icons::Icon;
