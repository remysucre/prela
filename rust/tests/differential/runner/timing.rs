//! Opt-in timings for finding slow differential-test stages.

pub(crate) struct Timing {
    label: String,
    start: Option<std::time::Instant>,
}

impl Timing {
    pub(crate) fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        let start = std::env::var_os("PRELA_PROFILE").map(|_| {
            eprintln!("pbt-profile start {label}");
            std::time::Instant::now()
        });
        Self { label, start }
    }
}

impl Drop for Timing {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            eprintln!(
                "pbt-profile finish {:.6} {}",
                start.elapsed().as_secs_f64(),
                self.label
            );
        }
    }
}
