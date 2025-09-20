mod password_generator;

use async_trait::async_trait;

use crate::business::domain::services::{GeneratePasswordOptions, UtilitiesService};

pub struct UtilitiesServiceImpl;

#[async_trait]
impl UtilitiesService for UtilitiesServiceImpl {
    async fn generate_password(&self, opt: GeneratePasswordOptions) -> String {
        password_generator::generate_password(opt)
    }
}
