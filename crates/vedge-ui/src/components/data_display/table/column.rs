use leptos::prelude::*;

#[component]
pub fn TableColumn(
    children: Children,
    #[prop(attrs, default = "")] class: &'static str,
) -> impl IntoView {
    view! {
        <th role="column-header" class=class>
            {children()}
        </th>
    }
}
