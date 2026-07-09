use crate::components::form::helper_text::HelperText;
use crate::components::form::label::Label;
use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Status;
use crate::utils::text::text_or;
use leptos::prelude::*;

/// Structural wrapper that composes a Label, a control slot, and HelperText.
///
/// The consumer must pass a matching `id` to the control inside the slot so that
/// the Label's `for` attribute resolves correctly. The consumer is also
/// responsible for wiring `aria-describedby="{id}-helper"` and, when applicable,
/// `aria-invalid="true"` and `required` on the control — FormField cannot inject
/// into the slot.
///
/// Do not use for Checkbox, RadioGroup, Toggle, or FileUpload — those have
/// different label relationships (see Form Field spec).
#[component]
pub fn FormField(
    #[prop(into)] label: TextProp,
    id: &'static str,
    children: Children,
    #[prop(optional)] status: Status,
    #[prop(into, default = TextProp::default())] hint: TextProp,
    #[prop(into, default = TextProp::default())] error: TextProp,
    #[prop(into, default = TextProp::default())] success_message: TextProp,
    #[prop(into, default = TextProp::default())] warning: TextProp,
    #[prop(into, default = TextProp::default())] error_label: TextProp,
    #[prop(into, default = TextProp::default())] success_label: TextProp,
    #[prop(into, default = TextProp::default())] warning_label: TextProp,
    #[prop(optional)] required: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    // Resolve a stable `{id}-helper` string for the HelperText element. We
    // leak it once because HelperText's `id` prop is `&'static str`; the
    // allocation is bounded to one-per-FormField-mount.
    let helper_id: &'static str = Box::leak(format!("{id}-helper").into_boxed_str());

    let status_cls = match status {
        Status::Error => "form-field--error",
        Status::Warning => "form-field--warning",
        Status::Success => "form-field--success",
        Status::Default => "",
    };

    let root_cls = ["form-field", status_cls, class].join(" ");

    // Priority: error > warning > success > hint. Only one message shows at a time.
    let message = Signal::derive(move || match status {
        Status::Error => text_or(error, ""),
        Status::Warning => text_or(warning, ""),
        Status::Success => text_or(success_message, ""),
        Status::Default => text_or(hint, ""),
    });

    let show_helper = Signal::derive(move || !message.get().is_empty());

    view! {
        <div class=root_cls aria-required=required.then_some("true")>
            <Label html_for=id required=required status=status disabled=disabled>
                {move || text_or(label, "")}
            </Label>
            <div class="form-field__control">{children()}</div>
            <Show when=move || show_helper.get()>
                <div class="form-field__helper">
                    <HelperText
                        id=helper_id
                        status=status
                        message=message
                        error_label=error_label
                        success_label=success_label
                        warning_label=warning_label
                    />
                </div>
            </Show>
        </div>
    }
}
