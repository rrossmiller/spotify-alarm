use chrono::{DateTime, Datelike, Local, NaiveTime, Timelike, Weekday};

#[derive(Debug, Clone)]
pub struct Alarm {
    pub desc: String,
    pub time: NaiveTime,
    pub days: Vec<Weekday>,
    pub played: bool,
}
impl PartialEq for Alarm {
    fn eq(&self, other: &Self) -> bool {
        self.desc == other.desc && self.time == other.time && self.days == other.days
    }
}
impl Alarm {
    /// An alarm should play if it has not already been played and
    /// its time's hour and minute are the same as the current time
    pub fn should_play(&self, time: DateTime<Local>) -> bool {
        if !self.played && self.time.minute() == time.minute() && self.time.hour() == time.hour() {
            return true;
        }
        false
    }
}

pub fn get_alarms(f: &str) -> Result<Vec<Alarm>, String> {
    let alarms: Vec<Alarm> = f
        .lines()
        .filter(|e| !e.starts_with("#")) // skip commented out alarms
        .filter_map(|line| {
            let spl = line.split(' ').collect::<Vec<&str>>();

            let times = spl[0]
                .split(':')
                .filter_map(|e| e.parse::<u32>().ok())
                .collect::<Vec<u32>>();
            if times.len() < 2 {
                return None;
            }
            let time = chrono::NaiveTime::from_hms_opt(times[0], times[1], 0).unwrap();

            let days = spl[1]
                .split(',')
                .filter_map(|e| to_weekday(e))
                .collect::<Vec<Weekday>>();

            let desc = spl[2..].join(" "); // everything else is the description
            Some(Alarm {
                desc,
                time,
                days,
                played: false,
            })
        })
        .collect();

    return Ok(alarms);
}

/// Get the alarms that still need to be run for today.
pub fn get_valid_alarms(
    new_alarms: Vec<Alarm>,
    alarms: Vec<Alarm>,
    time: DateTime<Local>,
) -> Vec<Alarm> {
    let mut alarms: Vec<Alarm> = new_alarms
        .into_iter()
        .filter_map(|mut a| {
            // is the alarm valid for today?
            if !a.days.contains(&time.weekday()) {
                return None;
            }
            // is the alarm in the past?
            if a.time.hour() <= time.hour() && a.time.minute() < time.minute() {
                return None;
            }
            for alrm in alarms.iter() {
                if a == *alrm {
                    a.played = alrm.played;
                }
            }
            Some(a)
        })
        .collect();
    alarms.sort_by(|a, b| a.time.cmp(&b.time));
    alarms
}

fn to_weekday(d: &str) -> Option<Weekday> {
    match d {
        "M" => Some(Weekday::Mon),
        "T" => Some(Weekday::Tue),
        "W" => Some(Weekday::Wed),
        "Th" => Some(Weekday::Thu),
        "F" => Some(Weekday::Fri),
        "S" => Some(Weekday::Sat),
        "Su" => Some(Weekday::Sun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_alarms() {
        let alarms = "06:00 M,T,W,Th,F,S,Su first alarm
#0:asdf 
6:17 M,T,F,S,Su this is the second alarm
6:17 M,T,F,S,Su"
            .to_string();
        let alarms = get_alarms(&alarms).unwrap();
        assert_eq!(alarms.len(), 3);
    }
    #[test]
    fn alarm_equals() {
        let alarm1 = Alarm {
            desc: "Wake up".to_string(),
            time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            days: vec![Weekday::Mon, Weekday::Wed],
            played: false,
        };

        let alarm2 = Alarm {
            desc: "Wake up".to_string(),
            time: NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            days: vec![Weekday::Mon, Weekday::Wed],
            played: false,
        };

        let alarm3 = Alarm {
            desc: "Workout".to_string(),
            time: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
            days: vec![Weekday::Tue, Weekday::Thu],
            played: false,
        };

        assert_eq!(alarm1, alarm2);
        assert_ne!(alarm1, alarm3);
        assert!(vec![alarm1.clone(), alarm2].contains(&alarm1));
    }
}
