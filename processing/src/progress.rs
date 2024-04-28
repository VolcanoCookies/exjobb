use std::fmt::{Display, Write};

use console::style;
use indicatif::{ProgressBar, ProgressState, ProgressStyle};

pub fn eta_bar(len: usize) -> ProgressBar {
    let pb = ProgressBar::new(len as u64);

    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] [{per_sec}] [{pos:.cyan}/{len:.blue}] ({eta_precise})")
    .unwrap()
    .with_key("per_sec", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}/s", state.per_sec()).unwrap())
    .progress_chars("█▒░"));

    pb
}

struct Step {
    step: i32,
    progress_bar: ProgressBar,
    start_time: std::time::Instant,
}

pub struct Progress {
    current_step_index: i32,
    current_step: Option<Step>,
}

impl Progress {
    pub fn new() -> Self {
        Self {
            current_step_index: 0,
            current_step: None,
        }
    }

    pub fn finish<T: Display>(&mut self, message: T) {
        let Step {
            step,
            progress_bar,
            start_time,
        } = self.current_step.take().unwrap();
        progress_bar.finish_and_clear();
        println!(
            "{:?} {}",
            style(start_time.elapsed()).bold().yellow(),
            message,
        );
    }

    fn new_step<T: Display>(&mut self, pb: ProgressBar, message: T) {
        if self.current_step.is_some() {
            panic!("Current step is not finished");
        }
        self.current_step_index += 1;

        let step = Step {
            step: self.current_step_index,
            progress_bar: pb,
            start_time: std::time::Instant::now(),
        };

        let step_idx = step.step;
        self.current_step = Some(step);
        println!("{} {}", style(step_idx).bold().green(), message);
    }

    pub fn step_unsized<T: Display>(&mut self, message: T) {
        self.new_step(ProgressBar::new_spinner(), message)
    }

    pub fn step_sized<T: Display>(&mut self, length: usize, message: T) {
        self.new_step(eta_bar(length), message)
    }

    pub fn step_single<T: Display>(&mut self, message: T) {
        self.current_step_index += 1;
        println!(
            "{} {}",
            style(self.current_step_index).bold().green(),
            message
        );
    }

    pub fn tick(&mut self) {
        if let Some(step) = &mut self.current_step {
            step.progress_bar.inc(1);
        } else {
            panic!("No current step");
        }
    }

    pub fn ticks(&mut self, ticks: u64) {
        if let Some(step) = &mut self.current_step {
            step.progress_bar.inc(ticks);
        } else {
            panic!("No current step");
        }
    }

    pub fn get_pb(&mut self) -> &mut ProgressBar {
        if let Some(step) = &mut self.current_step {
            &mut step.progress_bar
        } else {
            panic!("No current step");
        }
    }
}
