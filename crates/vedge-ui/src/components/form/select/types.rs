/// A single selectable option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }
}

/// A labeled group of options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectGroup {
    pub label: String,
    pub options: Vec<SelectOption>,
}

/// Either a standalone option or a group of options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectItem {
    Option(SelectOption),
    Group(SelectGroup),
}

impl SelectItem {
    /// Convenience for a flat option.
    pub fn option(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Option(SelectOption::new(value, label))
    }

    /// Convenience for a group.
    pub fn group(label: impl Into<String>, options: Vec<SelectOption>) -> Self {
        Self::Group(SelectGroup {
            label: label.into(),
            options,
        })
    }
}

/// Flatten items into a list of (option, `is_disabled`) for keyboard navigation indexing.
pub fn flatten_options(items: &[SelectItem]) -> Vec<SelectOption> {
    let mut flat = Vec::new();
    for item in items {
        match item {
            SelectItem::Option(opt) => flat.push(opt.clone()),
            SelectItem::Group(group) => {
                for opt in &group.options {
                    flat.push(opt.clone());
                }
            }
        }
    }
    flat
}
