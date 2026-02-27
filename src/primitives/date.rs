use std::fmt;

#[derive(Debug)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub min: u8,
    pub sec: u8,
}

impl Date {
    fn is_leap(year: u16) -> bool {
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
    }

    fn days_in_month(year: u16, month: u32) -> u32 {
        debug_assert!(month >= 1 && month <= 12);
        match month {
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap(year) {
                    29
                } else {
                    28
                }
            }
            _ => 31,
        }
    }

    pub fn from_unix_secs(secs: u64) -> Date {
        let sec = (secs % 60) as u8;
        let mut secs = secs / 60;

        let min = (secs % 60) as u8;
        secs /= 60;

        let hour = (secs % 24) as u8;
        let mut days = (secs / 24) as u32;

        let mut year: u16 = 1970;
        loop {
            let diy = if Self::is_leap(year) { 366 } else { 365 };
            if days >= diy {
                days -= diy;
                year += 1;
            } else {
                break;
            }
        }

        let mut month: u32 = 1;
        loop {
            let dim = Self::days_in_month(year, month);
            if days >= dim {
                days -= dim;
                month += 1;
            } else {
                break;
            }
        }

        Date {
            year,
            month: month as u8,
            day: (days + 1) as u8,
            hour,
            min,
            sec,
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{} {}:{}",
            self.day, self.month, self.year, self.hour, self.min
        )
    }
}
