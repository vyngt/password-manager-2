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

use crate::utils;

#[component]
pub fn Table(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    let base_cls = "min-w-full h-auto table-auto w-full";
    let cls = utils::string::merge_classnames(vec![base_cls, class]);
    view! { <table class=cls>{children()}</table> }
}
