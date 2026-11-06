//! Lines shown on the HUD (`[GamePlay] bGeneralSubtitles`), from `Fallout3.exe`
//! 1.7.0.3: `HUDMainMenu`'s queue (`+0x16c`).
//!
//! * A line goes in (`00649e40`, only with the option on; `0061b770` from the
//!   say functions, `006fdb80`, `00767660`) with where it was said, for as long
//!   as its voice lasts or, with no voice, its length times
//!   `fNoticeTextTimePerCharacter`, and a priority: the player's own 0, someone
//!   else's 2 (1 with no one saying it), forced the most there is. `Say x 1` and
//!   `SayTo y x 1` force. Forcing doesn't get round the option.
//! * Every frame (`00652000`) lines past their time go, the one on show being
//!   hidden with them. With nothing on show the next is picked (`00646ab0`):
//!   nearest the player, at or above the priority of the one it beats, none
//!   further than 500 units unless it's forced.

/// How far off a line can be heard (`00646ab0`, compared squared).
pub const REACH: f32 = 500.0;
/// A forced line's priority (`00649e40`).
pub const FORCED: i32 = i32::MAX;
/// `fNoticeTextTimePerCharacter`'s default (`00f63318`).
pub const SECONDS_PER_CHARACTER: f32 = 0.06;
/// The longest text kept (`00649e40`: a 0x104-byte buffer, `strcpy_s`).
pub const LONGEST: usize = 0x103;

/// Who's saying a line, for its priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Said {
    ByPlayer,
    BySomeone,
    ByNoOne,
}

#[derive(Debug, Clone, PartialEq)]
struct Line {
    id: u64,
    text: String,
    at_ms: f64,
    seconds: f32,
    position: [f32; 3],
    priority: i32,
}

/// The HUD's lines.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Subtitles {
    lines: Vec<Line>,
    next_id: u64,
    showing: Option<u64>,
}

/// A line's time: its voice's, else by its length (`00649e40`).
pub fn seconds_of(text: &str, voice_seconds: Option<f32>, per_character: f32) -> f32 {
    match voice_seconds.filter(|s| *s > 0.0) {
        Some(s) => s,
        None => text.len() as f32 * per_character,
    }
}

impl Subtitles {
    /// A line said at `now_ms` (`00649e40`). Nothing for no text.
    pub fn add(
        &mut self,
        text: &str,
        seconds: f32,
        position: [f32; 3],
        said: Said,
        forced: bool,
        now_ms: f64,
    ) {
        if text.is_empty() {
            return;
        }
        let priority = if forced {
            FORCED
        } else {
            match said {
                Said::ByPlayer => 0,
                Said::BySomeone => 2,
                Said::ByNoOne => 1,
            }
        };
        let mut end = text.len().min(LONGEST);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        self.next_id += 1;
        self.lines.push(Line {
            id: self.next_id,
            text: text[..end].to_string(),
            at_ms: now_ms,
            seconds,
            position,
            priority,
        });
    }

    /// `00646ab0`: which line to show to someone at `player`.
    fn pick(&self, player: [f32; 3]) -> Option<u64> {
        let mut best: Option<&Line> = None;
        let mut nearest = f32::MAX;
        for line in &self.lines {
            let d: f32 = (0..3).map(|k| (line.position[k] - player[k]).powi(2)).sum();
            let takes = match best {
                None => d <= REACH * REACH,
                Some(b) => nearest > d && line.priority >= b.priority,
            };
            if takes || line.priority == FORCED {
                best = Some(line);
                nearest = d;
            }
        }
        best.map(|l| l.id)
    }

    /// Every frame (`00652000`): what to show now.
    pub fn update(&mut self, now_ms: f64, player: [f32; 3]) -> Option<&str> {
        self.lines
            .retain(|line| now_ms - line.at_ms < f64::from(line.seconds) * 1000.0);
        if self.showing.is_some_and(|id| !self.lines.iter().any(|l| l.id == id)) {
            self.showing = None;
        }
        if self.showing.is_none() {
            self.showing = self.pick(player);
        }
        self.current()
    }

    /// The line on show.
    pub fn current(&self) -> Option<&str> {
        let id = self.showing?;
        self.lines
            .iter()
            .find(|l| l.id == id)
            .map(|l| l.text.as_str())
    }

    /// Everything gone (a load).
    pub fn clear(&mut self) {
        self.lines.clear();
        self.showing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERE: [f32; 3] = [0.0; 3];

    #[test]
    fn a_line_lasts_as_long_as_its_voice_or_its_length() {
        assert_eq!(seconds_of("abcd", Some(2.5), 0.06), 2.5);
        assert_eq!(seconds_of("abcd", None, 0.5), 2.0);
        assert_eq!(seconds_of("abcd", Some(0.0), 0.5), 2.0);
    }

    #[test]
    fn a_line_shows_for_its_time_then_goes() {
        let mut s = Subtitles::default();
        s.add("Hello", 1.0, HERE, Said::BySomeone, false, 0.0);
        assert_eq!(s.update(500.0, HERE), Some("Hello"));
        assert_eq!(s.update(999.0, HERE), Some("Hello"));
        assert_eq!(s.update(1000.0, HERE), None);
    }

    #[test]
    fn a_line_too_far_off_isnt_shown_unless_forced() {
        let far = [REACH + 1.0, 0.0, 0.0];
        let mut s = Subtitles::default();
        s.add("Far", 5.0, far, Said::BySomeone, false, 0.0);
        assert_eq!(s.update(1.0, HERE), None);
        s.add("Far but forced", 5.0, far, Said::BySomeone, true, 1.0);
        assert_eq!(s.update(2.0, HERE), Some("Far but forced"));
    }

    #[test]
    fn the_nearest_wins_but_not_over_a_higher_priority() {
        let mut s = Subtitles::default();
        s.add("Farther", 5.0, [300.0, 0.0, 0.0], Said::BySomeone, false, 0.0);
        s.add("Nearer", 5.0, [100.0, 0.0, 0.0], Said::BySomeone, false, 0.0);
        assert_eq!(s.update(1.0, HERE), Some("Nearer"));

        let mut s = Subtitles::default();
        s.add("Someone", 5.0, [300.0, 0.0, 0.0], Said::BySomeone, false, 0.0);
        s.add("Me", 5.0, HERE, Said::ByPlayer, false, 0.0);
        assert_eq!(s.update(1.0, HERE), Some("Someone"));
    }

    #[test]
    fn the_line_on_show_stays_until_its_time_is_up() {
        let mut s = Subtitles::default();
        s.add("First", 2.0, [100.0, 0.0, 0.0], Said::BySomeone, false, 0.0);
        assert_eq!(s.update(1.0, HERE), Some("First"));
        s.add("Nearer", 5.0, HERE, Said::BySomeone, false, 1.0);
        assert_eq!(s.update(1000.0, HERE), Some("First"));
        assert_eq!(s.update(2000.0, HERE), Some("Nearer"));
    }

    #[test]
    fn text_is_cut_to_the_games_buffer_and_clear_empties_it() {
        let mut s = Subtitles::default();
        s.add(&"x".repeat(400), 5.0, HERE, Said::BySomeone, false, 0.0);
        assert_eq!(s.update(1.0, HERE).map(str::len), Some(LONGEST));
        s.clear();
        assert_eq!(s.current(), None);
        s.add("", 5.0, HERE, Said::BySomeone, false, 0.0);
        assert_eq!(s.update(1.0, HERE), None);
    }
}
