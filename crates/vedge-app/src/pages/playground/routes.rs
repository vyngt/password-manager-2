use super::color_picker::ColorPickerPage;
use super::layout::PlaygroundLayout;
use super::pg_accordion::AccordionPage;
use super::pg_avatar::AvatarPage;
use super::pg_badge::BadgePage;
use super::pg_button::ButtonPage;
use super::pg_checkbox::CheckboxPage;
use super::pg_context_menu::ContextMenuPage;
use super::pg_copy_button::CopyButtonPage;
use super::pg_data_table::DataTablePage;
use super::pg_date_picker::DatePickerPage;
use super::pg_date_time_picker::DateTimePickerPage;
use super::pg_dialog::DialogPage;
use super::pg_dropdown_menu::DropdownMenuPage;
use super::pg_empty_state::EmptyStatePage;
use super::pg_file_upload::FileUploadPage;
use super::pg_form_field::FormFieldPage;
use super::pg_helper_text::HelperTextPage;
use super::pg_icon_button::IconButtonPage;
use super::pg_input::InputPage;
use super::pg_kbd::KbdPage;
use super::pg_label::LabelPage;
use super::pg_number_input::NumberInputPage;
use super::pg_pagination::PaginationPage;
use super::pg_password_strength_meter::PasswordStrengthMeterPage;
use super::pg_popover::PopoverPage;
use super::pg_progress_bar::ProgressBarPage;
use super::pg_qrcode::QRCodePage;
use super::pg_radio_group::RadioGroupPage;
use super::pg_segmented_control::SegmentedControlPage;
use super::pg_select::SelectPage;
use super::pg_separator::SeparatorPage;
use super::pg_sidebar_item::SidebarItemPage;
use super::pg_slider::SliderPage;
use super::pg_spinner::SpinnerPage;
use super::pg_step_indicator::StepIndicatorPage;
use super::pg_tabs::TabsPage;
use super::pg_tag_input::TagInputPage;
use super::pg_textarea::TextareaPage;
use super::pg_time_picker::TimePickerPage;
use super::pg_toast::ToastPage;
use super::pg_toggle::TogglePage;
use super::pg_tooltip::TooltipPage;
use super::pg_tooltip_icon_button::TooltipIconButtonPage;
use super::theme::ThemePage;
use leptos::prelude::*;
use leptos_router::{
    MatchNestedRoutes,
    components::{ParentRoute, Route},
    path,
};

#[component(transparent)]
pub fn PlayGroundRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/playground") view=PlaygroundLayout>
            <Route path=path!("/") view=ThemePage />
            // Foundation
            <Route path=path!("/accordion") view=AccordionPage />
            <Route path=path!("/avatar") view=AvatarPage />
            <Route path=path!("/badge") view=BadgePage />
            <Route path=path!("/button") view=ButtonPage />
            <Route path=path!("/copy-button") view=CopyButtonPage />
            <Route path=path!("/empty-state") view=EmptyStatePage />
            <Route path=path!("/icon-button") view=IconButtonPage />
            <Route path=path!("/kbd") view=KbdPage />
            <Route path=path!("/progress-bar") view=ProgressBarPage />
            <Route path=path!("/qrcode") view=QRCodePage />
            <Route path=path!("/separator") view=SeparatorPage />
            <Route path=path!("/sidebar-item") view=SidebarItemPage />
            <Route path=path!("/spinner") view=SpinnerPage />
            <Route path=path!("/step-indicator") view=StepIndicatorPage />
            <Route path=path!("/tabs") view=TabsPage />
            <Route path=path!("/tooltip-icon-button") view=TooltipIconButtonPage />
            // Form
            <Route path=path!("/checkbox") view=CheckboxPage />
            <Route path=path!("/color-picker") view=ColorPickerPage />
            <Route path=path!("/date-picker") view=DatePickerPage />
            <Route path=path!("/date-time-picker") view=DateTimePickerPage />
            <Route path=path!("/file-upload") view=FileUploadPage />
            <Route path=path!("/form-field") view=FormFieldPage />
            <Route path=path!("/helper-text") view=HelperTextPage />
            <Route path=path!("/input") view=InputPage />
            <Route path=path!("/label") view=LabelPage />
            <Route path=path!("/number-input") view=NumberInputPage />
            <Route path=path!("/password-strength-meter") view=PasswordStrengthMeterPage />
            <Route path=path!("/radio-group") view=RadioGroupPage />
            <Route path=path!("/segmented-control") view=SegmentedControlPage />
            <Route path=path!("/select") view=SelectPage />
            <Route path=path!("/slider") view=SliderPage />
            <Route path=path!("/tag-input") view=TagInputPage />
            <Route path=path!("/textarea") view=TextareaPage />
            <Route path=path!("/time-picker") view=TimePickerPage />
            <Route path=path!("/toggle") view=TogglePage />
            // Data Display
            <Route path=path!("/data-table") view=DataTablePage />
            <Route path=path!("/pagination") view=PaginationPage />
            // Feedback
            <Route path=path!("/context-menu") view=ContextMenuPage />
            <Route path=path!("/dialog") view=DialogPage />
            <Route path=path!("/dropdown-menu") view=DropdownMenuPage />
            <Route path=path!("/popover") view=PopoverPage />
            <Route path=path!("/toast") view=ToastPage />
            <Route path=path!("/tooltip") view=TooltipPage />
        </ParentRoute>
    }
    .into_inner()
}
