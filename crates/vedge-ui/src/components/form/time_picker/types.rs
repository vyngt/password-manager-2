#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeValue {
    pub hours: u8,
    pub minutes: u8,
}

impl TimeValue {
    pub fn new(hours: u8, minutes: u8) -> Self {
        Self {
            hours: hours.min(23),
            minutes: minutes.min(59),
        }
    }

    pub fn with_hours(self, hours: u8) -> Self {
        Self {
            hours: hours.min(23),
            minutes: self.minutes,
        }
    }

    pub fn with_minutes(self, minutes: u8) -> Self {
        Self {
            hours: self.hours,
            minutes: minutes.min(59),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TimeFormat {
    #[default]
    H24,
    H12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Hours,
    Minutes,
    AmPm,
}

pub fn to_12h(hours: u8) -> (u8, bool) {
    let is_pm = hours >= 12;
    let h12 = match hours % 12 {
        0 => 12,
        h => h,
    };
    (h12, is_pm)
}

pub fn from_12h(hours_12: u8, is_pm: bool) -> u8 {
    let base = if hours_12 == 12 { 0 } else { hours_12 };
    if is_pm { base + 12 } else { base }
}

pub fn is_before(a: TimeValue, b: TimeValue) -> bool {
    let lhs = u16::from(a.hours) * 60 + u16::from(a.minutes);
    let rhs = u16::from(b.hours) * 60 + u16::from(b.minutes);
    lhs < rhs
}

pub enum DigitOutcome {
    Wait(u8),
    Commit(u8),
}

pub fn h24_hours_first_digit(d: u8) -> DigitOutcome {
    match d {
        0..=2 => DigitOutcome::Wait(d),
        3..=9 => DigitOutcome::Commit(d),
        _ => DigitOutcome::Commit(0),
    }
}

pub fn h24_hours_second_digit(first: u8, d: u8) -> u8 {
    let combined = first * 10 + d;
    if combined <= 23 { combined } else { d }
}

pub fn h12_hours_first_digit(d: u8) -> DigitOutcome {
    match d {
        0..=1 => DigitOutcome::Wait(d),
        2..=9 => DigitOutcome::Commit(d),
        _ => DigitOutcome::Commit(1),
    }
}

pub fn h12_hours_second_digit(first: u8, d: u8) -> u8 {
    let combined = first * 10 + d;
    if (1..=12).contains(&combined) {
        combined
    } else {
        // Second digit forms no valid 1–12 hour (including the leading-zero case) →
        // treat the digit as the hour on its own.
        d
    }
}

pub fn minutes_first_digit(d: u8) -> DigitOutcome {
    match d {
        0..=5 => DigitOutcome::Wait(d),
        6..=9 => DigitOutcome::Commit(d),
        _ => DigitOutcome::Commit(0),
    }
}

pub fn minutes_second_digit(first: u8, d: u8) -> u8 {
    let combined = first * 10 + d;
    if combined <= 59 { combined } else { d }
}
