use crate::features::vault::vault_launch::VaultLaunch;
use leptos::prelude::*;

/// App entry route (`/`) — the returning-user launch screen.
#[component]
pub fn Page() -> impl IntoView {
    view! { <VaultLaunch /> }
}
