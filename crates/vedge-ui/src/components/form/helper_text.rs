use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Status;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn HelperText(
    id: &'static str,
    #[prop(optional)] status: Status,
    #[prop(into)] message: Signal<String>,
    #[prop(into, default = TextProp::default())] error_label: TextProp,
    #[prop(into, default = TextProp::default())] success_label: TextProp,
    #[prop(into, default = TextProp::default())] warning_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    {
        let missing = match status {
            Status::Error if error_label.get_untracked().is_empty() => Some("error_label"),
            Status::Success if success_label.get_untracked().is_empty() => Some("success_label"),
            Status::Warning if warning_label.get_untracked().is_empty() => Some("warning_label"),
            _ => None,
        };
        if let Some(prop) = missing {
            web_sys::console::error_1(
                &format!("HelperText: `{}` is required for i18n when status is non-Default.", prop).into(),
            );
        }
    }

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

    let label_signal = match status {
        Status::Default => None,
        Status::Error => Some(error_label),
        Status::Success => Some(success_label),
        Status::Warning => Some(warning_label),
    };

    view! {
        <p id=id class=cls role=role>
            {icon.map(|icon_data| {
                view! {
                    <span
                        class="helper-text__icon"
                        aria-label=move || label_signal.map(|s| s.get()).filter(|s| !s.is_empty())
                    >
                        <Icon icon=icon_data />
                    </span>
                }
            })}
            <span>{move || message.get()}</span>
        </p>
    }
}
