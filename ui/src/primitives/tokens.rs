#[derive(Clone, Copy)]
pub enum Effect {
    Ripple,
}

pub enum Size {
    Small,
    Medium,
    Large,
}

pub enum Variant {
    Filled,
    Outlined,
    Text,
}

pub enum Shape {
    Sharp,   // No border radius
    Rounded, // Default rounded
    Pill,    // Fully rounded (pill/circle)
}
