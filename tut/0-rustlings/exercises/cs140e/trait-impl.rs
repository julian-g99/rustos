// FIXME: Make me pass! Diff budget: 25 lines.


enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16)
}

<<<<<<< HEAD
impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        let self_time = match self {
            Duration::MilliSeconds(ms) => *ms,
            Duration::Seconds(s) => *s as u64 * 1000,
            Duration::Minutes(m) => *m as u64 * 1000 * 60
        };
        let other_time = match other {
            Duration::MilliSeconds(ms) => *ms,
            Duration::Seconds(s) => *s as u64 * 1000,
            Duration::Minutes(m) => *m as u64 * 1000 * 60
        };
        self_time == other_time
    }
}

// What traits does `Duration` need to implement?

=======
>>>>>>> skeleton/lab2
#[test]
fn traits() {
    assert_eq!(Seconds(120), Minutes(2));
    assert_eq!(Seconds(420), Minutes(7));
    assert_eq!(MilliSeconds(420000), Minutes(7));
    assert_eq!(MilliSeconds(43000), Seconds(43));
}
