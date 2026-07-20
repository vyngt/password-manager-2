use chrono::NaiveDate;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerVariant {
    #[default]
    Single,
    Range,
    Month,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DateRange {
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YearMonth {
    pub year: i32,
    pub month: u32,
}

impl YearMonth {
    pub fn new(year: i32, month: u32) -> Self {
        Self { year, month }
    }

    pub fn from_date(d: NaiveDate) -> Self {
        use chrono::Datelike;
        Self {
            year: d.year(),
            month: d.month(),
        }
    }

    pub fn prev(&self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    pub fn next(&self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerValue {
    Single(Option<NaiveDate>),
    Range(DateRange),
    Month(Option<YearMonth>),
}

impl DatePickerValue {
    pub fn empty_for(variant: DatePickerVariant) -> Self {
        match variant {
            DatePickerVariant::Single => DatePickerValue::Single(None),
            DatePickerVariant::Range => DatePickerValue::Range(DateRange::default()),
            DatePickerVariant::Month => DatePickerValue::Month(None),
        }
    }

    pub fn as_single(&self) -> Option<NaiveDate> {
        match self {
            DatePickerValue::Single(d) => *d,
            _ => None,
        }
    }

    pub fn as_range(&self) -> DateRange {
        match self {
            DatePickerValue::Range(r) => *r,
            _ => DateRange::default(),
        }
    }

    pub fn as_month(&self) -> Option<YearMonth> {
        match self {
            DatePickerValue::Month(m) => *m,
            _ => None,
        }
    }
}
