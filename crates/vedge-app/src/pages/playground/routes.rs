use super::layout::PlaygroundLayout;
use super::pg_avatar::AvatarPage;
use super::pg_badge::BadgePage;
use super::pg_button::ButtonPage;
use super::pg_checkbox::CheckboxPage;
use super::pg_data_table::DataTablePage;
use super::pg_date_picker::DatePickerPage;
use super::pg_dialog::DialogPage;
use super::pg_helper_text::HelperTextPage;
use super::pg_icon_button::IconButtonPage;
use super::pg_input::InputPage;
use super::pg_label::LabelPage;
use super::pg_number_input::NumberInputPage;
use super::pg_password_strength_meter::PasswordStrengthMeterPage;
use super::pg_radio_group::RadioGroupPage;
use super::pg_segmented_control::SegmentedControlPage;
use super::pg_select::SelectPage;
use super::pg_separator::SeparatorPage;
use super::pg_textarea::TextareaPage;
use super::pg_time_picker::TimePickerPage;
use super::pg_toast::ToastPage;
use super::pg_spinner::SpinnerPage;
use super::pg_toggle::TogglePage;
use super::pg_tooltip::TooltipPage;
use super::theme::ThemePage;
use super::color_picker::ColorPickerPage;
use leptos::prelude::*;
use leptos_router::{MatchNestedRoutes, components::*, path};

#[component(transparent)]
pub fn PlayGroundRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/playground") view=PlaygroundLayout>
            <Route path=path!("/") view=ThemePage />
            // Foundation
            <Route path=path!("/avatar") view=AvatarPage />
            <Route path=path!("/badge") view=BadgePage />
            <Route path=path!("/button") view=ButtonPage />
            <Route path=path!("/icon-button") view=IconButtonPage />
            <Route path=path!("/separator") view=SeparatorPage />
            <Route path=path!("/spinner") view=SpinnerPage />
            // Form
            <Route path=path!("/checkbox") view=CheckboxPage />
            <Route path=path!("/color-picker") view=ColorPickerPage />
            <Route path=path!("/date-picker") view=DatePickerPage />
            <Route path=path!("/helper-text") view=HelperTextPage />
            <Route path=path!("/input") view=InputPage />
            <Route path=path!("/label") view=LabelPage />
            <Route path=path!("/number-input") view=NumberInputPage />
            <Route path=path!("/password-strength-meter") view=PasswordStrengthMeterPage />
            <Route path=path!("/radio-group") view=RadioGroupPage />
            <Route path=path!("/segmented-control") view=SegmentedControlPage />
            <Route path=path!("/select") view=SelectPage />
            <Route path=path!("/textarea") view=TextareaPage />
            <Route path=path!("/time-picker") view=TimePickerPage />
            <Route path=path!("/toggle") view=TogglePage />
            // Data Display
            <Route path=path!("/data-table") view=DataTablePage />
            // Feedback
            <Route path=path!("/dialog") view=DialogPage />
            <Route path=path!("/toast") view=ToastPage />
            <Route path=path!("/tooltip") view=TooltipPage />
        </ParentRoute>
    }
    .into_inner()
}
