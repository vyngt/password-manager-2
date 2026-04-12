use leptos::prelude::*;

#[component]
pub fn PasswordStrengthMeter(
    #[prop(into)] score: Signal<u8>,
    #[prop(optional, default = true)] show_label: bool,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    let label_text = move || match score.get() {
        0 => "",
        1 => "Weak",
        2 => "Fair",
        3 => "Strong",
        _ => "Very strong",
    };

    let aria_valuetext = move || match score.get() {
        0 => "".to_string(),
        1 => "Weak".to_string(),
        2 => "Fair".to_string(),
        3 => "Strong".to_string(),
        _ => "Very strong".to_string(),
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
            aria-label="Password strength"
            data-score=move || score.get().to_string()
        >
            <div class="strength-meter__track">
                {(1..=4u8)
                    .map(|idx| {
                        let filled = move || {
                            if score.get() >= idx { "true" } else { "false" }
                        };
                        view! {
                            <div
                                class="strength-meter__segment"
                                data-filled=filled
                            />
                        }
                    })
                    .collect_view()}
            </div>
            {show_label.then(|| view! {
                <span class="strength-meter__label">{label_text}</span>
            })}
        </div>
    }
}
