use leptos::prelude::*;
use std::cmp::Ordering;
use vedge_ui::components::data_display::{
    CellValue, ColumnDef, ColumnType, ColumnWidth, DataTable, SortState, cell_fn, string_fn,
};
use vedge_ui::components::foundation::badge::Badge;
use vedge_ui::components::foundation::button::Button;
use vedge_ui::components::foundation::icon_button::IconButton;
use vedge_ui::primitives::tokens::{Align, BadgeVariant, Size, SortDirection, Variant};

use icondata as i;
use leptos_icons::Icon;

use super::common::Section;

#[derive(Clone, PartialEq)]
struct Entry {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    category_variant: BadgeVariant,
    modified: &'static str,
    modified_order: u32,
    fingerprint: &'static str,
}

fn sample_entries() -> Vec<Entry> {
    vec![
        Entry {
            id: "1",
            name: "GitHub",
            category: "Login",
            category_variant: BadgeVariant::Info,
            modified: "2d ago",
            modified_order: 2,
            fingerprint: "SHA256:a1b2c3d4e5f6",
        },
        Entry {
            id: "2",
            name: "AWS Console",
            category: "Login",
            category_variant: BadgeVariant::Info,
            modified: "1w ago",
            modified_order: 7,
            fingerprint: "SHA256:112233445566",
        },
        Entry {
            id: "3",
            name: "Stripe API",
            category: "API Key",
            category_variant: BadgeVariant::Warning,
            modified: "3d ago",
            modified_order: 3,
            fingerprint: "SHA256:778899aabbcc",
        },
        Entry {
            id: "4",
            name: "Production DB",
            category: "Secret",
            category_variant: BadgeVariant::Danger,
            modified: "Today",
            modified_order: 0,
            fingerprint: "SHA256:ddeeff001122",
        },
        Entry {
            id: "5",
            name: "ssh-rsa-dev",
            category: "SSH Key",
            category_variant: BadgeVariant::Default,
            modified: "5d ago",
            modified_order: 5,
            fingerprint: "SHA256:334455667788",
        },
    ]
}

#[component]
pub fn DataTablePage() -> impl IntoView {
    view! {
        <div class="p-6 max-w-5xl mx-auto space-y-6">
            <h1 class="text-xl font-semibold text-text-primary">"Data Table"</h1>
            <p class="text-sm text-text-secondary">
                "Column-based view for homogeneous data. Headless on data — consumer owns sort, selection, and filtering."
            </p>

            <BasicSection />
            <SortableSection />
            <ColumnTypesSection />
            <SelectableSection />
            <RowClickVsSelectionSection />
            <LoadingSection />
            <EmptySection />
            <KitchenSinkSection />
        </div>
    }
}

// ---- Basic ----
#[component]
fn BasicSection() -> impl IntoView {
    let rows = Signal::stored(sample_entries());
    let columns = vec![
        ColumnDef {
            id: "name",
            header: "Name".into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
        },
        ColumnDef {
            id: "category",
            header: "Category".into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Fixed(140),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.category.to_owned())),
        },
        ColumnDef {
            id: "modified",
            header: "Modified".into(),
            col_type: ColumnType::Date,
            sortable: false,
            width: ColumnWidth::Fixed(120),
            align: Align::End,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.modified.to_owned())),
        },
    ];

    view! {
        <Section title="Basic">
            <p class="text-xs text-text-tertiary">"Three text columns, no selection, no sort."</p>
            <DataTable
                columns=columns
                rows=rows
                row_key=string_fn(|e: &Entry| e.id.to_owned())
                empty_message="No entries"
            />
        </Section>
    }
}

// ---- Sortable ----
#[component]
fn SortableSection() -> impl IntoView {
    let sort = RwSignal::new(Some(SortState {
        column_id: "name".into(),
        direction: SortDirection::Asc,
    }));

    let rows = Memo::new(move |_| {
        let mut v = sample_entries();
        if let Some(s) = sort.get() {
            v.sort_by(|a, b| match s.column_id.as_str() {
                "name" => a.name.cmp(b.name),
                "modified" => a.modified_order.cmp(&b.modified_order),
                _ => Ordering::Equal,
            });
            if s.direction == SortDirection::Desc {
                v.reverse();
            }
        }
        v
    });

    let columns = vec![
        ColumnDef {
            id: "name",
            header: "Name".into(),
            col_type: ColumnType::Text,
            sortable: true,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
        },
        ColumnDef {
            id: "category",
            header: "Category".into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Fixed(140),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.category.to_owned())),
        },
        ColumnDef {
            id: "modified",
            header: "Modified".into(),
            col_type: ColumnType::Date,
            sortable: true,
            width: ColumnWidth::Fixed(120),
            align: Align::End,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.modified.to_owned())),
        },
    ];

    view! {
        <Section title="Sortable">
            <p class="text-xs text-text-tertiary">
                "Click 'Name' or 'Modified' to cycle none → asc → desc → none."
            </p>
            <div class="text-xs text-text-secondary">
                "Current sort: "
                {move || match sort.get() {
                    None => "none".to_owned(),
                    Some(s) => {
                        format!(
                            "{} {}",
                            s.column_id,
                            match s.direction {
                                SortDirection::Asc => "↑",
                                SortDirection::Desc => "↓",
                            },
                        )
                    }
                }}
            </div>
            <DataTable
                columns=columns
                rows=Signal::derive(move || rows.get())
                row_key=string_fn(|e: &Entry| e.id.to_owned())
                sort=Signal::derive(move || sort.get())
                on_sort_change=Callback::new(move |s| sort.set(s))
                empty_message="No entries"
            />
        </Section>
    }
}

// ---- Column types ----
#[component]
fn ColumnTypesSection() -> impl IntoView {
    let rows = Signal::stored(sample_entries());

    let columns = vec![
        ColumnDef {
            id: "name",
            header: "Text".into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
        },
        ColumnDef {
            id: "category",
            header: "Badge".into(),
            col_type: ColumnType::Badge,
            sortable: false,
            width: ColumnWidth::Fixed(120),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Badge {
                label: e.category.to_owned(),
                variant: e.category_variant,
            }),
        },
        ColumnDef {
            id: "modified",
            header: "Date".into(),
            col_type: ColumnType::Date,
            sortable: false,
            width: ColumnWidth::Fixed(100),
            align: Align::End,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.modified.to_owned())),
        },
        ColumnDef {
            id: "fingerprint",
            header: "Mono".into(),
            col_type: ColumnType::Mono,
            sortable: false,
            width: ColumnWidth::MinMax(160, 260),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.fingerprint.to_owned())),
        },
        ColumnDef {
            id: "custom",
            header: "Custom".into(),
            col_type: ColumnType::Custom,
            sortable: false,
            width: ColumnWidth::Fixed(140),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| {
                let n = e.name.to_owned();
                CellValue::View(
                    view! {
                        <span class="inline-flex items-center gap-2">
                            <span class="inline-block w-2 h-2 rounded-full bg-primary"></span>
                            <span class="font-medium">{n}</span>
                        </span>
                    }
                    .into_any(),
                )
            }),
        },
        ColumnDef {
            id: "action",
            header: "".into(),
            col_type: ColumnType::Action,
            sortable: false,
            width: ColumnWidth::Fixed(44),
            align: Align::End,
            cell: cell_fn(|_e: &Entry| {
                CellValue::View(
                    view! {
                        <IconButton aria_label="Copy" size=Size::Sm>
                            <Icon icon=i::BiCopyRegular />
                        </IconButton>
                    }
                    .into_any(),
                )
            }),
        },
    ];

    view! {
        <Section title="Column types">
            <p class="text-xs text-text-tertiary">
                "Text, Badge, Date, Mono, Custom, and Action columns in one table."
            </p>
            <DataTable
                columns=columns
                rows=rows
                row_key=string_fn(|e: &Entry| e.id.to_owned())
                empty_message="No entries"
            />
        </Section>
    }
}

// ---- Selectable ----
#[component]
fn SelectableSection() -> impl IntoView {
    let selected = RwSignal::new(Vec::<String>::new());
    let rows = Signal::stored(sample_entries());

    let columns = vec![
        ColumnDef {
            id: "name",
            header: "Name".into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
        },
        ColumnDef {
            id: "category",
            header: "Category".into(),
            col_type: ColumnType::Text,
            sortable: false,
            width: ColumnWidth::Fixed(140),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.category.to_owned())),
        },
    ];

    view! {
        <Section title="Selectable">
            <div class="flex items-center gap-3 text-xs text-text-secondary">
                <span>
                    "Selected: "
                    <span class="font-medium text-text-primary">
                        {move || selected.get().len().to_string()}
                    </span>
                </span>
                <Button
                    variant=Variant::Secondary
                    size=Size::Sm
                    on:click=move |_| selected.set(vec![])
                >
                    "Clear"
                </Button>
                <span class="text-text-tertiary">
                    "Try: click a row, then Shift+click another row for range selection."
                </span>
            </div>
            <DataTable
                columns=columns
                rows=rows
                row_key=string_fn(|e: &Entry| e.id.to_owned())
                selectable=true
                selected_rows=Signal::derive(move || selected.get())
                on_selection_change=Callback::new(move |keys| selected.set(keys))
                select_all_label="Select all entries"
                deselect_all_label="Deselect all entries"
                row_select_label=string_fn(|e: &Entry| format!("Select {}", e.name))
                empty_message="No entries"
            />
        </Section>
    }
}

// ---- Row click vs. selection ----
#[component]
fn RowClickVsSelectionSection() -> impl IntoView {
    let last_clicked = RwSignal::new(String::new());
    let selected = RwSignal::new(Vec::<String>::new());
    let rows = Signal::stored(sample_entries());

    let columns_click = vec![ColumnDef {
        id: "name",
        header: "Name".into(),
        col_type: ColumnType::Text,
        sortable: false,
        width: ColumnWidth::Flexible,
        align: Align::Start,
        cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
    }];
    let columns_select = columns_click.clone();

    view! {
        <Section title="Row click vs. selection (mutually exclusive)">
            <p class="text-xs text-text-tertiary">
                "Left table has on_row_click — clicking a row logs the click (no toggle). Right table has selectable=true — clicking a row toggles selection."
            </p>
            <div class="grid grid-cols-2 gap-4">
                <div class="space-y-2">
                    <div class="text-xs text-text-secondary">
                        "Last clicked: "
                        <span class="font-medium text-text-primary">
                            {move || {
                                let v = last_clicked.get();
                                if v.is_empty() { "(none)".into() } else { v }
                            }}
                        </span>
                    </div>
                    <DataTable
                        columns=columns_click
                        rows=rows
                        row_key=string_fn(|e: &Entry| e.id.to_owned())
                        on_row_click=Callback::new(move |e: Entry| {
                            last_clicked.set(e.name.to_owned());
                        })
                        empty_message="No entries"
                    />
                </div>
                <div class="space-y-2">
                    <div class="text-xs text-text-secondary">
                        "Selected: "
                        <span class="font-medium text-text-primary">
                            {move || selected.get().len().to_string()}
                        </span>
                    </div>
                    <DataTable
                        columns=columns_select
                        rows=rows
                        row_key=string_fn(|e: &Entry| e.id.to_owned())
                        selectable=true
                        selected_rows=Signal::derive(move || selected.get())
                        on_selection_change=Callback::new(move |keys| selected.set(keys))
                        select_all_label="Select all"
                        deselect_all_label="Deselect all"
                        row_select_label=string_fn(|e: &Entry| format!("Select {}", e.name))
                        empty_message="No entries"
                    />
                </div>
            </div>
        </Section>
    }
}

// ---- Loading ----
#[component]
fn LoadingSection() -> impl IntoView {
    let loading_with = RwSignal::new(true);
    let loading_empty = RwSignal::new(true);
    let rows = Signal::stored(sample_entries());
    let empty_rows = Signal::stored(Vec::<Entry>::new());

    let columns = || {
        vec![
            ColumnDef {
                id: "name",
                header: "Name".into(),
                col_type: ColumnType::Text,
                sortable: false,
                width: ColumnWidth::Flexible,
                align: Align::Start,
                cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
            },
            ColumnDef {
                id: "category",
                header: "Category".into(),
                col_type: ColumnType::Text,
                sortable: false,
                width: ColumnWidth::Fixed(140),
                align: Align::Start,
                cell: cell_fn(|e: &Entry| CellValue::Text(e.category.to_owned())),
            },
        ]
    };

    view! {
        <Section title="Loading">
            <div class="grid grid-cols-2 gap-4">
                <div class="space-y-2">
                    <div class="flex items-center gap-2">
                        <span class="text-xs text-text-secondary">"With rows:"</span>
                        <Button
                            variant=Variant::Secondary
                            size=Size::Sm
                            on:click=move |_| loading_with.update(|v| *v = !*v)
                        >
                            {move || if loading_with.get() { "Stop" } else { "Start" }}
                        </Button>
                    </div>
                    <DataTable
                        columns=columns()
                        rows=rows
                        row_key=string_fn(|e: &Entry| e.id.to_owned())
                        loading=Signal::derive(move || loading_with.get())
                        empty_message="No entries"
                    />
                </div>
                <div class="space-y-2">
                    <div class="flex items-center gap-2">
                        <span class="text-xs text-text-secondary">"Empty:"</span>
                        <Button
                            variant=Variant::Secondary
                            size=Size::Sm
                            on:click=move |_| loading_empty.update(|v| *v = !*v)
                        >
                            {move || if loading_empty.get() { "Stop" } else { "Start" }}
                        </Button>
                    </div>
                    <DataTable
                        columns=columns()
                        rows=empty_rows
                        row_key=string_fn(|e: &Entry| e.id.to_owned())
                        loading=Signal::derive(move || loading_empty.get())
                        empty_message="No entries"
                    />
                </div>
            </div>
        </Section>
    }
}

// ---- Empty ----
#[component]
fn EmptySection() -> impl IntoView {
    let rows = Signal::stored(Vec::<Entry>::new());

    let columns = || {
        vec![
            ColumnDef {
                id: "name",
                header: "Name".into(),
                col_type: ColumnType::Text,
                sortable: false,
                width: ColumnWidth::Flexible,
                align: Align::Start,
                cell: cell_fn(|e: &Entry| CellValue::Text(e.name.to_owned())),
            },
            ColumnDef {
                id: "category",
                header: "Category".into(),
                col_type: ColumnType::Text,
                sortable: false,
                width: ColumnWidth::Fixed(140),
                align: Align::Start,
                cell: cell_fn(|e: &Entry| CellValue::Text(e.category.to_owned())),
            },
        ]
    };

    view! {
        <Section title="Empty state">
            <div class="grid grid-cols-2 gap-4">
                <div class="space-y-2">
                    <span class="text-xs text-text-secondary">"Message only"</span>
                    <DataTable
                        columns=columns()
                        rows=rows
                        row_key=string_fn(|e: &Entry| e.id.to_owned())
                        empty_message="No audit events recorded"
                    />
                </div>
                <div class="space-y-2">
                    <span class="text-xs text-text-secondary">"With action"</span>
                    <DataTable
                        columns=columns()
                        rows=rows
                        row_key=string_fn(|e: &Entry| e.id.to_owned())
                        empty_message="No entries in this vault"
                        empty_action=move || {
                            view! {
                                <Button variant=Variant::Primary size=Size::Sm>
                                    "Add entry"
                                </Button>
                            }
                        }
                    />
                </div>
            </div>
        </Section>
    }
}

// ---- Kitchen sink ----
#[component]
fn KitchenSinkSection() -> impl IntoView {
    let selected = RwSignal::new(Vec::<String>::new());
    let last_navigated = RwSignal::new(String::new());
    let sort = RwSignal::new(Some(SortState {
        column_id: "modified".into(),
        direction: SortDirection::Asc,
    }));

    let rows = Memo::new(move |_| {
        let mut v = sample_entries();
        if let Some(s) = sort.get() {
            v.sort_by(|a, b| match s.column_id.as_str() {
                "name" => a.name.cmp(b.name),
                "modified" => a.modified_order.cmp(&b.modified_order),
                _ => Ordering::Equal,
            });
            if s.direction == SortDirection::Desc {
                v.reverse();
            }
        }
        v
    });

    let columns = vec![
        ColumnDef {
            id: "name",
            header: "Name".into(),
            col_type: ColumnType::Custom,
            sortable: true,
            width: ColumnWidth::Flexible,
            align: Align::Start,
            cell: cell_fn(|e: &Entry| {
                let first = e
                    .name
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string();
                let name = e.name.to_owned();
                CellValue::View(
                    view! {
                        <span class="inline-flex items-center gap-2">
                            <span class="inline-flex items-center justify-center w-6 h-6 rounded-full bg-surface-3 text-text-primary text-xs font-medium">
                                {first}
                            </span>
                            <span>{name}</span>
                        </span>
                    }
                    .into_any(),
                )
            }),
        },
        ColumnDef {
            id: "category",
            header: "Category".into(),
            col_type: ColumnType::Badge,
            sortable: false,
            width: ColumnWidth::Fixed(120),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Badge {
                label: e.category.to_owned(),
                variant: e.category_variant,
            }),
        },
        ColumnDef {
            id: "fingerprint",
            header: "Fingerprint".into(),
            col_type: ColumnType::Mono,
            sortable: false,
            width: ColumnWidth::MinMax(160, 240),
            align: Align::Start,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.fingerprint.to_owned())),
        },
        ColumnDef {
            id: "modified",
            header: "Modified".into(),
            col_type: ColumnType::Date,
            sortable: true,
            width: ColumnWidth::Fixed(100),
            align: Align::End,
            cell: cell_fn(|e: &Entry| CellValue::Text(e.modified.to_owned())),
        },
        ColumnDef {
            id: "action",
            header: "".into(),
            col_type: ColumnType::Action,
            sortable: false,
            width: ColumnWidth::Fixed(44),
            align: Align::End,
            cell: cell_fn(|_e: &Entry| {
                CellValue::View(
                    view! {
                        <IconButton aria_label="Copy password" size=Size::Sm>
                            <Icon icon=i::BiCopyRegular />
                        </IconButton>
                    }
                    .into_any(),
                )
            }),
        },
    ];

    view! {
        <Section title="Kitchen sink">
            <p class="text-xs text-text-tertiary">
                "Sortable headers + multi-select checkboxes + row click (navigate) + badge column + action column. Keyboard: Tab in, Arrow ↑/↓ to move, Space to toggle, Shift+Space for range, Enter to navigate, Escape to clear."
            </p>
            <div class="flex items-center gap-4 text-xs text-text-secondary">
                <span>
                    "Selected: "
                    <span class="font-medium text-text-primary">
                        {move || selected.get().len().to_string()}
                    </span>
                </span>
                <span>
                    "Last navigated: "
                    <span class="font-medium text-text-primary">
                        {move || {
                            let v = last_navigated.get();
                            if v.is_empty() { "(none)".into() } else { v }
                        }}
                    </span>
                </span>
                <Show when=move || !selected.get().is_empty()>
                    <div class="flex items-center gap-2 ml-auto">
                        <Badge variant=BadgeVariant::Info>
                            {move || format!("{} selected", selected.get().len())}
                        </Badge>
                        <Button
                            variant=Variant::Secondary
                            size=Size::Sm
                            on:click=move |_| selected.set(vec![])
                        >
                            "Clear"
                        </Button>
                    </div>
                </Show>
            </div>
            <DataTable
                columns=columns
                rows=Signal::derive(move || rows.get())
                row_key=string_fn(|e: &Entry| e.id.to_owned())
                selectable=true
                selected_rows=Signal::derive(move || selected.get())
                on_selection_change=Callback::new(move |keys| selected.set(keys))
                sort=Signal::derive(move || sort.get())
                on_sort_change=Callback::new(move |s| sort.set(s))
                on_row_click=Callback::new(move |e: Entry| last_navigated.set(e.name.to_owned()))
                select_all_label="Select all entries"
                deselect_all_label="Deselect all entries"
                row_select_label=string_fn(|e: &Entry| format!("Select {}", e.name))
                empty_message="No entries"
                empty_action=move || {
                    view! {
                        <Button variant=Variant::Primary size=Size::Sm>
                            "Add entry"
                        </Button>
                    }
                }
            />
        </Section>
    }
}
