use leptos::prelude::*;

#[component]
pub fn VorpalIcon() -> impl IntoView {
    view! {
        <svg
            class="h-full w-full"
            viewBox="0 0 200 200"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
        >
            <circle
                cx="100"
                cy="100"
                r="95"
                transform="rotate(90 100 100)"
                stroke="currentColor"
                stroke-width="10"
            />
            <circle
                cx="99.5"
                cy="100.5"
                r="77.5"
                stroke="currentColor"
                stroke-width="10"
            />
            <path d="M49.5 181L155.5 130.5L182 50" stroke="currentColor" stroke-width="10" />
            <path d="M63 13.3032L144.295 117.501" stroke="currentColor" stroke-width="10" />
            <path
                d="M51.3329 79.0373L63.7841 110.269L147.5 119"
                stroke="currentColor"
                stroke-width="10"
            />
            <path d="M22 42.3032L53.0003 81.5362" stroke="currentColor" stroke-width="10" />
            <path d="M55.5 80.5L91 54" stroke="currentColor" stroke-width="10" />
            <path d="M35 141L153 130" stroke="currentColor" stroke-width="5" />
            <path d="M153 128L157 45" stroke="currentColor" stroke-width="5" />
            <path d="M47 158L135 140" stroke="currentColor" stroke-width="5" />
            <path d="M25 44L32 35.5L45 25.5L62 17L68 25.5L32 53.5L25 44Z" fill="currentColor" />
            <path
                d="M69.5 176.5L93.5 181.5L130.5 173L160.5 153.5L176.5 128L180 100L176.5 65.5L189.636 62.0859L194 98L189.636 136L162.5 173L133 188L86.5 193.5L57.5 181.5L69.5 176.5Z"
                fill="currentColor"
            />
            <path
                d="M72.9103 44.7188L67.2108 48.9027L65.7125 40.8877L72.9103 44.7188Z"
                fill="currentColor"
                stroke="currentColor"
            />
            <path
                d="M70.9103 58.7188L65.2108 62.9027L63.7125 54.8877L70.9103 58.7188Z"
                fill="currentColor"
                stroke="currentColor"
            />
            <path
                d="M55.9103 56.7188L50.2108 60.9027L48.7125 52.8877L55.9103 56.7188Z"
                fill="currentColor"
                stroke="currentColor"
            />
        </svg>
    }
}
