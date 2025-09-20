pub struct GeneratePasswordOptions {
    pub len: i32,
    pub upper: bool,
    pub lower: bool,
    pub digits: bool,
    pub special: bool,
}

#[async_trait::async_trait]
pub trait UtilitiesService: Send + Sync {
    async fn generate_password(&self, opt: GeneratePasswordOptions) -> String;
}
