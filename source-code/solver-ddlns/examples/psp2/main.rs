mod psp;

use anyhow::Result;
use libc::SIGALRM;
use papier_lns::{
    SimpleMddBuilder, MddLnsBuilder, ResolutionOutcome,
    SigLimitAllocator, Problem, Solution,
};
use psp::{Psp, RandomizedMinLP};
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use signal_hook::consts::SIGINT;
use std::{
    alloc::System,
    fs::File,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};
use structopt::StructOpt;

use crate::psp::LeftToRight;

#[global_allocator]
static ALLOC: SigLimitAllocator<System> = SigLimitAllocator::new(System, usize::MAX);

/// This program lets you solve a PSP instance with various methods
#[derive(Debug, StructOpt)]
enum Args {
    /// Just print the header
    Header,
    Greedy {
        #[structopt(short, long)]
        /// Path to the problem instance we want to solve
        fname: String,
    },
    /// Check the feasibility of a given solution
    Check {
        #[structopt(short, long)]
        /// Path to the problem instance we want to solve
        fname: String,
        #[structopt(short, long)]
        /// The solution to check
        solution: String,
    },
    /// Solve an instance with lns+dd
    Solve {
        #[structopt(short, long)]
        /// Path to the problem instance we want to solve
        fname: String,
        /// Output the header
        #[structopt(short = "H", long)]
        header: bool,
        #[structopt(short, long, default_value = "10000")]
        width: usize,
        #[structopt(short, long, default_value = "20211105")]
        seed: u64,
        #[structopt(short, long, default_value = "0.1")]
        proba: f64,
        /// optional memory limit in gigabytes
        #[structopt(short, long)]
        ram_limit: Option<f64>,
        /// optional time limit in seconds
        #[structopt(short, long)]
        time_limit: Option<u32>,
    }
}

fn main() -> anyhow::Result<()> {
    let args  = Args::from_args();

    match args {
        Args::Header => { print_header(); Ok(())},
        Args::Greedy { fname } => greedy(&fname),
        Args::Check  { fname, solution } => check(&fname, &solution),
        Args::Solve  { fname, header, width, seed, proba, ram_limit, time_limit } => 
            solve(&fname, header, width, seed, proba, time_limit, ram_limit)
    }
}

fn check(fname: &str, solution: &str) -> Result<()> {
    let instance = Psp::try_from(File::open(fname)?)?;
    let solution = Solution::from_str(solution)?;
    let cost     = instance.evaluate(&LeftToRight, &solution);
    instance.check(&LeftToRight, &solution);
    println!("cost {}", cost);
    Ok(())
}

fn greedy(fname: &str) -> Result<()> {
    let instance = Psp::try_from(File::open(fname)?)?;
    let greedy   = instance.greedy();
    let init_val = greedy.0;
    let init_sol = greedy.1;
    let init_sol = init_sol
        .map(|sol| format!("{}", sol))
        .unwrap_or_else(|| "-- no solution --".to_string());

    println!("cost {} -- {}", init_val, init_sol);
    Ok(())
}

fn solve(fname: &str, header: bool, width: usize, seed: u64, proba: f64, time_limit: Option<u32>, ram_limit: Option<f64>) -> Result<()> {
    let kill_switch = setup_kill_switch(time_limit, ram_limit)?;
    let instance = Psp::try_from(File::open(fname)?)?;
    let instname = instance_name(fname);
    let start_tm = Instant::now();
    //
    let greedy = instance.greedy();
    let init_val = Some(greedy.0);
    let init_sol = greedy.1;

    let mdd = SimpleMddBuilder::default()
        .problem(&instance)
        .var_ordering(LeftToRight)
        .node_selection(RandomizedMinLP::default())
        .rng(Xoshiro256Plus::seed_from_u64(seed))
        .proba(proba)
        .kill_switch(Arc::clone(&kill_switch))
        .build()?;

    let mut solver = MddLnsBuilder::default()
        .mdd(mdd)
        .nb_var(instance.nb_vars())
        .width(width)
        .initial_sol(init_sol)
        .initial_val(init_val)
        .start(start_tm)
        .kill_switch(kill_switch)
        .build()?;
    //
    let outcome = solver.minimize();
    
    // ////////////////////////////////////////////////////////////////////////
    // Print the output
    // ////////////////////////////////////////////////////////////////////////
    if header {
        print_header();
    }
    let ram = ALLOC.get_peak_gb();
    print_result(instname, ram, outcome);

    Ok(())
} 

fn setup_kill_switch(time_limit: Option<u32>, ram_limit: Option<f64>) -> Result<Arc<AtomicBool>> {
    let kill_switch = Arc::new(AtomicBool::new(false));
    // ctrl + c   : interrupt program
    signal_hook::flag::register(SIGINT, Arc::clone(&kill_switch))?;
    // sigalrm for the timeout
    signal_hook::flag::register(SIGALRM, Arc::clone(&kill_switch))?;

    if let Some(seconds) = time_limit {
        unsafe {
            libc::alarm(seconds);
        };
    }
    if let Some(gigabytes) = ram_limit {
        ALLOC.set_limit_gb(gigabytes);
    }

    Ok(kill_switch)
}

fn instance_name(fname: &str) -> &str {
    fname
        .split_terminator(std::path::MAIN_SEPARATOR)
        .last()
        .unwrap_or("-- no name --")
}

fn print_header() {
    // instance | method | status | value | ram in gb | time to best | time to proved | solution
    println!(
        "{:>20} | {:>10} | {:>20} | {:>10} | {:>8} | {:>10} | {:>10} | {:<80}",
        "Instance", "Method", "Status", "Value", "RAM", "Best (s)", "Proved (s)", "Solution"
    );
}

fn print_result(instance: &str, ram: f64, outcome: ResolutionOutcome) {
    // instance | method | status | value | ram in gb | time to best | time to proved | solution
    println!(
        "{:>20} | {:>10} | {:>20} | {:>10} | {:>8.2} | {:>10} | {:>10} | {:<80}",
        instance,
        "lns",
        outcome.status.to_str(),
        outcome
            .best_value
            .map(|v| format!("{}", v))
            .unwrap_or_else(|| "N.A.".to_string()),
        ram,
        outcome
            .time_to_best
            .map(|d| format!("{:.2}", d.as_secs_f32()))
            .unwrap_or_else(|| "N.A.".to_string()),
        outcome
            .time_to_prove
            .map(|d| format!("{:.2}", d.as_secs_f32()))
            .unwrap_or_else(|| "N.A.".to_string()),
        outcome
            .best_sol
            .map(|sol| format!("{}", sol))
            .unwrap_or_else(|| "-- no solution --".to_string())
    );
}
