use crate::primitives::tokens::Status;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn HelperText(
    id: &'static str,
    #[prop(optional)] status: Status,
    #[prop(into)] message: Signal<String>,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let cls = [
        "helper-text",
        status.helper_text_class(),
        class,
    ]
    .join(" ");

    let role = if status == Status::Error {
        Some("alert")
    } else {
        None
    };

    let icon = match status {
        Status::Default => None,
        Status::Error => Some(i::FaCircleExclamationSolid),
        Status::Success => Some(i::FaCircleCheckSolid),
        Status::Warning => Some(i::FaTriangleExclamationSolid),
    };

    let aria_label = match status {
        Status::Default => None,
        Status::Error => Some("Error:"),
        Status::Success => Some("Success:"),
        Status::Warning => Some("Warning:"),
    };

    view! {
        <p id=id class=cls role=role>
            {icon.map(|icon_data| {
                view! {
                    <span class="helper-text__icon" aria-label=aria_label>
                        <Icon icon=icon_data />
                    </span>
                }
            })}
            <span>{move || message.get()}</span>
        </p>
    }
}
