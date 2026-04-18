use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::Orientation;
use icondata as i;
use leptos::prelude::*;
use leptos_icons::Icon;

fn text_or(prop: TextProp, fallback: &str) -> String {
    let v = prop.get();
    if v.is_empty() { fallback.to_string() } else { v }
}

// -------------------------------------------------------------------------
// Public types
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub label: String,
    pub description: Option<String>,
}

impl Step {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
        }
    }

    pub fn with_description(label: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: Some(description.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepState {
    Completed,
    Current,
    Upcoming,
}

// -------------------------------------------------------------------------
// Component
// -------------------------------------------------------------------------

#[component]
pub fn StepIndicator(
    #[prop(into)] steps: Signal<Vec<Step>>,
    #[prop(into)] current_step: Signal<usize>,
    #[prop(optional)] variant: Orientation,
    // i18n-ready labels. Defaults fall back to English at render.
    // `completed_label` / `current_label` are the state suffix in the accessible
    // per-step aria-label: `"Step {n}: {label} — {state}"` (completed/current).
    // `step_prefix` is the "Step" word before the number.
    #[prop(into, default = TextProp::default())] aria_label: TextProp,
    #[prop(into, default = TextProp::default())] step_prefix: TextProp,
    #[prop(into, default = TextProp::default())] completed_label: TextProp,
    #[prop(into, default = TextProp::default())] current_label: TextProp,
    #[prop(optional, default = "")] class: &'static str,
) -> impl IntoView {
    #[cfg(debug_assertions)]
    {
        let len = steps.with_untracked(|v| v.len());
        if len < 2 {
            web_sys::console::error_1(
                &format!(
                    "StepIndicator: `steps` must contain at least 2 items (got {}).",
                    len
                )
                .into(),
            );
        }
        if len > 7 {
            web_sys::console::error_1(
                &format!(
                    "StepIndicator: `steps` has {} items; the practical maximum is 7. \
                    Consider grouping steps or restructuring the flow.",
                    len
                )
                .into(),
            );
        }
    }

    let is_vertical = variant == Orientation::Vertical;

    let root_cls = [
        "step-indicator",
        variant.step_indicator_class(),
        class,
    ]
    .join(" ");

    view! {
        <ol class=root_cls aria-label=move || text_or(aria_label, "Progress")>
            {move || {
                let all = steps.get();
                let len = all.len();
                let cur = if len == 0 { 0 } else { current_step.get().min(len - 1) };

                all.into_iter().enumerate().map(|(idx, step)| {
                    let state = if idx < cur {
                        StepState::Completed
                    } else if idx == cur {
                        StepState::Current
                    } else {
                        StepState::Upcoming
                    };
                    let is_last = idx + 1 == len;
                    let step_number = idx + 1;

                    view! {
                        <StepItem
                            step=step
                            step_number=step_number
                            state=state
                            is_last=is_last
                            is_vertical=is_vertical
                            step_prefix=step_prefix
                            completed_label=completed_label
                            current_label=current_label
                        />
                    }
                }).collect_view()
            }}
        </ol>
    }
}

// -------------------------------------------------------------------------
// Step item (internal)
// -------------------------------------------------------------------------

#[component]
fn StepItem(
    step: Step,
    step_number: usize,
    state: StepState,
    is_last: bool,
    is_vertical: bool,
    step_prefix: TextProp,
    completed_label: TextProp,
    current_label: TextProp,
) -> impl IntoView {
    let node_cls = match state {
        StepState::Completed => "step-indicator__node step-indicator__node--completed",
        StepState::Current => "step-indicator__node step-indicator__node--current",
        StepState::Upcoming => "step-indicator__node step-indicator__node--upcoming",
    };

    let connector_cls = match state {
        StepState::Completed => "step-indicator__connector step-indicator__connector--completed",
        StepState::Current | StepState::Upcoming => {
            "step-indicator__connector step-indicator__connector--upcoming"
        }
    };

    let label_cls = match state {
        StepState::Completed => "step-indicator__label step-indicator__label--done",
        StepState::Current => "step-indicator__label step-indicator__label--active",
        StepState::Upcoming => "step-indicator__label",
    };

    let aria_current = if state == StepState::Current {
        Some("step")
    } else {
        None
    };

    let prefix = text_or(step_prefix, "Step");
    let node_aria = match state {
        StepState::Completed => format!(
            "{} {}: {} \u{2014} {}",
            prefix,
            step_number,
            step.label,
            text_or(completed_label, "completed"),
        ),
        StepState::Current => format!(
            "{} {}: {} \u{2014} {}",
            prefix,
            step_number,
            step.label,
            text_or(current_label, "current"),
        ),
        StepState::Upcoming => format!("{} {}: {}", prefix, step_number, step.label),
    };

    let show_description = is_vertical && step.description.is_some();
    let description_text = step.description.clone().unwrap_or_default();

    let node_content = match state {
        StepState::Completed => view! {
            <span class="step-indicator__check" aria-hidden="true">
                <Icon icon=i::FaCheckSolid />
            </span>
        }
        .into_any(),
        StepState::Current | StepState::Upcoming => view! {
            <span aria-hidden="true">{step_number}</span>
        }
        .into_any(),
    };

    // Horizontal: body = node + content below (label). Connector is sibling of body.
    // Vertical:   body = node + connector below. Content is sibling of body (label + description).
    if is_vertical {
        view! {
            <li class="step-indicator__step" aria-current=aria_current>
                <div class="step-indicator__body">
                    <span class=node_cls aria-label=node_aria role="img">
                        {node_content}
                    </span>
                    {(!is_last).then(|| view! {
                        <span class=connector_cls aria-hidden="true"></span>
                    })}
                </div>
                <div class="step-indicator__content">
                    <span class=label_cls>{step.label.clone()}</span>
                    {show_description.then(|| view! {
                        <span class="step-indicator__description">{description_text}</span>
                    })}
                </div>
            </li>
        }
        .into_any()
    } else {
        view! {
            <li class="step-indicator__step" aria-current=aria_current>
                <div class="step-indicator__body">
                    <span class=node_cls aria-label=node_aria role="img">
                        {node_content}
                    </span>
                    <div class="step-indicator__content">
                        <span class=label_cls>{step.label.clone()}</span>
                    </div>
                </div>
                {(!is_last).then(|| view! {
                    <span class=connector_cls aria-hidden="true"></span>
                })}
            </li>
        }
        .into_any()
    }
}
