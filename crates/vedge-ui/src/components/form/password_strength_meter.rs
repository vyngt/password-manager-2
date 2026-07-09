use crate::primitives::text_prop::TextProp;
use leptos::prelude::*;

#[component]
pub fn PasswordStrengthMeter(
    #[prop(into)] score: Signal<u8>,
    #[prop(optional, default = true)] show_label: bool,
    #[prop(into, default = TextProp::default())] strength_label: TextProp,
    #[prop(into, default = Signal::stored(<[String; 4]>::default()))] level_labels: Signal<
        [String; 4],
    >,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    if strength_label.get_untracked().is_empty() {
        web_sys::console::error_1(
            &"PasswordStrengthMeter: `strength_label` is required for i18n.".into(),
        );
    }

    #[cfg(debug_assertions)]
    if level_labels.with_untracked(|ll| ll.iter().any(std::string::String::is_empty)) {
        web_sys::console::error_1(
            &"PasswordStrengthMeter: all entries in `level_labels` are required for i18n.".into(),
        );
    }

    let label_text = move || match score.get() {
        0 => String::new(),
        n => level_labels.with(|ll| ll.get((n as usize - 1).min(3)).cloned().unwrap_or_default()),
    };

    let aria_valuetext = move || match score.get() {
        0 => String::new(),
        n => level_labels.with(|ll| ll.get((n as usize - 1).min(3)).cloned().unwrap_or_default()),
    };

    let root_cls = ["strength-meter", class].join(" ");

    view! {
        <div
            class=root_cls
            role="meter"
            aria-valuenow=move || score.get().to_string()
            aria-valuemin="0"
            aria-valuemax="4"
            aria-valuetext=aria_valuetext
            aria-label=move || strength_label.get()
            data-score=move || score.get().to_string()
        >
            <div class="strength-meter__track">
                {(1..=4u8)
                    .map(|idx| {
                        let filled = move || { if score.get() >= idx { "true" } else { "false" } };
                        view! { <div class="strength-meter__segment" data-filled=filled /> }
                    })
                    .collect_view()}
            </div>
            {show_label.then(|| view! { <span class="strength-meter__label">{label_text}</span> })}
        </div>
    }
}
