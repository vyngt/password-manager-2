mod body;
mod cell;
mod column;
mod header;
mod row;

use leptos::prelude::*;

pub use body::TableBody;
pub use cell::TableCell;
pub use column::TableColumn;
pub use header::TableHeader;
pub use row::TableRow;

#[component]
pub fn Table(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = "w-full";
    let cls = vec![base_cls, class].join(" ");
    view! { <table class=cls>{children()}</table> }
}
