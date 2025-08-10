pub enum Effect {
    Ripple,
    None,
}

pub enum Size {
    Small,
    Medium,
    Large,
}

pub enum Variant {
    Filled,
    Outlined,
}

pub enum Shape {
    Sharp,   // No border radius
    Rounded, // Default rounded
    Pill,    // Fully rounded (pill/circle)
}
