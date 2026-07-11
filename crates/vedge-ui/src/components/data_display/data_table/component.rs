use super::logic::{HeaderCheckState, cycle_header, cycle_sort, header_state, toggle_row};
use super::types::{CellValue, ColumnDef, ColumnType, ColumnWidth, SortState, StringFn};
use crate::components::form::checkbox::Checkbox;
use crate::components::foundation::badge::Badge;
use crate::components::foundation::spinner::Spinner;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Size, SortDirection};
use leptos::prelude::*;

#[component]
pub fn DataTable<T>(
    columns: Vec<ColumnDef<T>>,
    #[prop(into)] rows: Signal<Vec<T>>,
    #[prop(into)] row_key: StringFn<T>,
    #[prop(optional)] selectable: bool,
    #[prop(into, default = Signal::stored(Vec::<String>::new()))] selected_rows: Signal<
        Vec<String>,
    >,
    #[prop(into, default = None)] on_selection_change: Option<Callback<Vec<String>>>,
    #[prop(into, default = Signal::stored(None))] sort: Signal<Option<SortState>>,
    #[prop(into, default = None)] on_sort_change: Option<Callback<Option<SortState>>>,
    #[prop(into, default = Signal::stored(false))] loading: Signal<bool>,
    #[prop(into)] empty_message: TextProp,
    #[prop(into, default = TextProp::default())] select_all_label: TextProp,
    #[prop(into, default = TextProp::default())] deselect_all_label: TextProp,
    #[prop(into, default = None)] row_select_label: Option<StringFn<T>>,
    #[prop(optional, into)] empty_action: ViewFn,
    #[prop(into, default = None)] on_row_click: Option<Callback<T>>,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView
where
    T: Clone + Send + Sync + 'static,
{
    let anchor_idx = RwSignal::new(None::<usize>);
    let focus_idx = RwSignal::new(None::<usize>);

    let columns_sv = StoredValue::new(columns);
    let columns_len = columns_sv.with_value(std::vec::Vec::len);
    let row_key_sv = StoredValue::new(row_key);
    let row_select_label_sv = StoredValue::new(row_select_label);
    let empty_action_sv = StoredValue::new(empty_action);

    let total_cols: u32 = (columns_len + usize::from(selectable)) as u32;

    // Reactive: per-row keys in visible order.
    let row_keys = Memo::new(move |_| {
        row_key_sv.with_value(|f| rows.get().iter().map(|r| f(r)).collect::<Vec<String>>())
    });

    // Clamp focus if rows shrink.
    Effect::new(move |_| {
        let len = row_keys.with(std::vec::Vec::len);
        focus_idx.update(|f| {
            if matches!(*f, Some(idx) if idx >= len) {
                *f = None;
            }
        });
    });

    let fire_selection = move |new_sel: Vec<String>| {
        if let Some(cb) = on_selection_change {
            cb.run(new_sel);
        }
    };

    let fire_sort = move |next: Option<SortState>| {
        if let Some(cb) = on_sort_change {
            cb.run(next);
        }
    };

    // Header tri-state signals driven by selection vs. total.
    let header_chk_checked = Signal::derive(move || {
        matches!(
            header_state(
                selected_rows.with(std::vec::Vec::len),
                row_keys.with(std::vec::Vec::len)
            ),
            HeaderCheckState::Checked
        )
    });
    let header_chk_indeterminate = Signal::derive(move || {
        matches!(
            header_state(
                selected_rows.with(std::vec::Vec::len),
                row_keys.with(std::vec::Vec::len)
            ),
            HeaderCheckState::Indeterminate
        )
    });
    let header_chk_label = Signal::derive(move || {
        if matches!(
            header_state(
                selected_rows.with(std::vec::Vec::len),
                row_keys.with(std::vec::Vec::len)
            ),
            HeaderCheckState::Checked
        ) {
            deselect_all_label.get()
        } else {
            select_all_label.get()
        }
    });

    let on_header_toggle = Callback::new(move |_: bool| {
        let new_sel = row_keys.with(|keys| cycle_header(&selected_rows.get(), keys));
        anchor_idx.set(None);
        fire_selection(new_sel);
    });

    // Centralized selection dispatch — reads current rows at invocation time.
    let dispatch_toggle = move |idx: usize, shift: bool| {
        let keys = row_keys.get();
        if idx >= keys.len() {
            return;
        }
        let (new_sel, new_anchor) =
            toggle_row(&selected_rows.get(), &keys, idx, anchor_idx.get(), shift);
        anchor_idx.set(new_anchor);
        fire_selection(new_sel);
    };

    // ---- Render <colgroup> ----
    let colgroup_view = move || {
        let mut pieces: Vec<AnyView> = Vec::with_capacity(columns_len + 1);
        if selectable {
            pieces.push(view! { <col style="width:44px" /> }.into_any());
        }
        columns_sv.with_value(|cols| {
            for c in cols {
                let style = match c.width {
                    ColumnWidth::Fixed(px) => format!("width:{px}px"),
                    ColumnWidth::Flexible => String::new(),
                    ColumnWidth::MinMax(mi, ma) => {
                        format!("min-width:{mi}px;max-width:{ma}px")
                    }
                };
                if style.is_empty() {
                    pieces.push(view! { <col /> }.into_any());
                } else {
                    pieces.push(view! { <col style=style /> }.into_any());
                }
            }
        });
        pieces.collect_view()
    };

    // ---- Render header row ----
    let header_checkbox_view = move || {
        if !selectable {
            return ().into_any();
        }
        view! {
            <th class="data-table__th data-table__th--checkbox" scope="col">
                <Checkbox
                    checked=header_chk_checked
                    indeterminate=header_chk_indeterminate
                    aria_label=header_chk_label
                    on_change=on_header_toggle
                />
            </th>
        }
        .into_any()
    };

    let header_cells_view = move || {
        columns_sv.with_value(|cols| {
            cols.iter()
                .map(|c| render_header_cell(c, sort, fire_sort))
                .collect::<Vec<AnyView>>()
        })
    };

    // ---- Render body rows ----
    let has_row_click = on_row_click.is_some();

    let body_view = move || {
        let rows_vec = rows.get();
        if rows_vec.is_empty() {
            return ().into_any();
        }
        rows_vec
            .into_iter()
            .enumerate()
            .map(|(idx, row)| {
                let row_key_str = row_key_sv.with_value(|f| f(&row));
                let key_for_sel = row_key_str.clone();
                let is_selected = Signal::derive(move || {
                    selected_rows.with(|sel| sel.iter().any(|s| s == &key_for_sel))
                });

                let tr_class = Signal::derive(move || {
                    let mut c = String::from("data-table__row");
                    if has_row_click {
                        c.push_str(" data-table__row--clickable");
                    }
                    c
                });

                let tr_tabindex = Signal::derive(move || {
                    if focus_idx.get() == Some(idx) {
                        "0"
                    } else {
                        "-1"
                    }
                });

                // Sync native focus when focus_idx transitions to this row.
                let tr_ref = NodeRef::<leptos::html::Tr>::new();
                Effect::new(move |_| {
                    if focus_idx.get() == Some(idx) {
                        if let Some(el) = tr_ref.get() {
                            let html: &web_sys::HtmlElement = &el;
                            let _ = html.focus();
                        }
                    }
                });

                let row_for_click = row.clone();
                let row_for_enter = row.clone();
                let row_for_checkbox_label = row.clone();

                let checkbox_cell = {
                    let label = row_select_label_sv
                        .with_value(|opt| opt.as_ref().map(|f| f(&row_for_checkbox_label)))
                        .unwrap_or_default();
                    let key_for_checkbox = row_key_str;
                    move || {
                        if !selectable {
                            return ().into_any();
                        }
                        let key_for_memo = key_for_checkbox.clone();
                        let chk_signal = Signal::derive(move || {
                            selected_rows.with(|sel| sel.iter().any(|s| s == &key_for_memo))
                        });
                        view! {
                            <td
                                class="data-table__td data-table__td--checkbox"
                                on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                            >
                                <Checkbox
                                    checked=chk_signal
                                    aria_label=label.clone()
                                    on_change=Callback::new(move |_: bool| {
                                        dispatch_toggle(idx, false);
                                    })
                                />
                            </td>
                        }
                        .into_any()
                    }
                };

                let cells_view = columns_sv.with_value(|cols| {
                    cols.iter()
                        .map(|c| render_cell(c, &row))
                        .collect::<Vec<AnyView>>()
                });

                let on_click = move |ev: web_sys::MouseEvent| {
                    let shift = ev.shift_key();
                    focus_idx.set(Some(idx));
                    if let Some(cb) = on_row_click {
                        if !shift {
                            cb.run(row_for_click.clone());
                            return;
                        }
                    }
                    if selectable {
                        dispatch_toggle(idx, shift);
                    }
                };

                let on_keydown = move |ev: web_sys::KeyboardEvent| {
                    let key = ev.key();
                    let len = row_keys.with(std::vec::Vec::len);
                    match key.as_str() {
                        "ArrowDown" => {
                            ev.prevent_default();
                            if len > 0 {
                                let next = (idx + 1).min(len - 1);
                                focus_idx.set(Some(next));
                            }
                        }
                        "ArrowUp" => {
                            ev.prevent_default();
                            if idx > 0 {
                                focus_idx.set(Some(idx - 1));
                            }
                        }
                        "Home" => {
                            ev.prevent_default();
                            if len > 0 {
                                focus_idx.set(Some(0));
                            }
                        }
                        "End" => {
                            ev.prevent_default();
                            if len > 0 {
                                focus_idx.set(Some(len - 1));
                            }
                        }
                        " " | "Spacebar" => {
                            ev.prevent_default();
                            if selectable {
                                dispatch_toggle(idx, ev.shift_key());
                            }
                        }
                        "Enter" => {
                            ev.prevent_default();
                            if let Some(cb) = on_row_click {
                                cb.run(row_for_enter.clone());
                            }
                        }
                        "Escape" => {
                            ev.prevent_default();
                            anchor_idx.set(None);
                            fire_selection(Vec::new());
                        }
                        _ => {}
                    }
                };

                view! {
                    <tr
                        node_ref=tr_ref
                        class=tr_class
                        aria-selected=move || is_selected.get().then_some("true")
                        aria-rowindex=(idx + 1).to_string()
                        tabindex=tr_tabindex
                        on:click=on_click
                        on:keydown=on_keydown
                    >
                        {checkbox_cell}
                        {cells_view}
                    </tr>
                }
                .into_any()
            })
            .collect_view()
            .into_any()
    };

    let empty_view = move || {
        view! {
            <tr>
                <td class="data-table__empty" colspan=total_cols role="status" aria-live="polite">
                    <div class="data-table__empty-content">
                        <span class="data-table__empty-icon" aria-hidden="true">
                            <svg
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                width="28"
                                height="28"
                            >
                                <rect x="3" y="5" width="18" height="14" rx="2" />
                                <line x1="3" y1="10" x2="21" y2="10" />
                                <line x1="9" y1="15" x2="15" y2="15" />
                            </svg>
                        </span>
                        <span>{move || empty_message.get()}</span>
                        {move || empty_action_sv.with_value(leptos::prelude::ViewFn::run)}
                    </div>
                </td>
            </tr>
        }
    };

    let wrapper_cls = move || {
        let mut base = String::from("data-table-wrapper");
        if !class.is_empty() {
            base.push(' ');
            base.push_str(class);
        }
        if loading.get() {
            base.push_str(" data-table-wrapper--loading");
        }
        base
    };

    view! {
        <div class=wrapper_cls>
            <table
                class="data-table"
                role="grid"
                aria-busy=move || loading.get().then_some("true")
                aria-rowcount=move || row_keys.with(std::vec::Vec::len).to_string()
            >
                <colgroup>{colgroup_view}</colgroup>
                <thead>
                    <tr>{header_checkbox_view} {header_cells_view}</tr>
                </thead>
                <tbody>
                    <Show when=move || !rows.with(std::vec::Vec::is_empty) fallback=empty_view>
                        {body_view}
                    </Show>
                </tbody>
            </table>
            <Show when=move || loading.get() fallback=|| ()>
                <div class="data-table__loading-overlay">
                    <Spinner size=Size::Md />
                </div>
            </Show>
        </div>
    }
}

fn render_header_cell<T: 'static>(
    col: &ColumnDef<T>,
    sort: Signal<Option<SortState>>,
    fire_sort: impl Fn(Option<SortState>) + Copy + Send + Sync + 'static,
) -> AnyView {
    let col_id = col.id;
    let header = col.header;
    let sortable = col.sortable;
    let col_type = col.col_type;
    let align_cls = col.align.data_table_th_class();

    let aria_sort = Signal::derive(move || {
        if !sortable {
            return None;
        }
        sort.with(|s| match s {
            Some(st) if st.column_id == col_id => Some(st.direction.as_aria()),
            _ => Some("none"),
        })
    });

    let sort_icon = move || {
        if !sortable {
            return ().into_any();
        }
        sort.with(|s| match s {
            Some(st) if st.column_id == col_id => match st.direction {
                SortDirection::Asc => {
                    view! { <span class="data-table__sort-icon">"↑"</span> }.into_any()
                }
                SortDirection::Desc => {
                    view! { <span class="data-table__sort-icon">"↓"</span> }.into_any()
                }
            },
            _ => view! { <span class="data-table__sort-icon data-table__sort-icon--inactive">"↕"</span> }
                .into_any(),
        })
    };

    let col_type_cls = match col_type {
        ColumnType::Mono => " data-table__th--mono",
        ColumnType::Action => " data-table__th--action",
        _ => "",
    };
    let full_cls = format!("data-table__th {align_cls}{col_type_cls}");

    if sortable {
        let on_click = move |ev: web_sys::MouseEvent| {
            ev.stop_propagation();
            let next = sort.with_untracked(|cur| cycle_sort(cur.as_ref(), col_id));
            fire_sort(next);
        };
        view! {
            <th class=full_cls scope="col" aria-sort=aria_sort>
                <button type="button" class="data-table__sort-btn" on:click=on_click>
                    <span>{move || header.get()}</span>
                    {sort_icon}
                </button>
            </th>
        }
        .into_any()
    } else {
        view! {
            <th class=full_cls scope="col">
                {move || header.get()}
            </th>
        }
        .into_any()
    }
}

fn render_cell<T: 'static>(col: &ColumnDef<T>, row: &T) -> AnyView {
    let value = (col.cell)(row);
    let align_cls = col.align.data_table_td_class();
    let (type_cls, stop_click) = match col.col_type {
        ColumnType::Date => ("data-table__td--date", false),
        ColumnType::Mono => ("data-table__td--mono", false),
        ColumnType::Action => ("data-table__td--action", true),
        ColumnType::Text | ColumnType::Badge | ColumnType::Custom => ("", false),
    };
    let cls = format!("data-table__td {type_cls} {align_cls}");

    match value {
        CellValue::Text(s) => {
            if stop_click {
                view! {
                    <td class=cls on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()>
                        {s}
                    </td>
                }
                .into_any()
            } else {
                view! { <td class=cls>{s}</td> }.into_any()
            }
        }
        CellValue::Badge { label, variant } => view! {
            <td class=cls>
                <Badge variant=variant>{label}</Badge>
            </td>
        }
        .into_any(),
        CellValue::View(v) => {
            if stop_click {
                view! {
                    <td class=cls on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()>
                        {v}
                    </td>
                }
                .into_any()
            } else {
                view! { <td class=cls>{v}</td> }.into_any()
            }
        }
    }
}
