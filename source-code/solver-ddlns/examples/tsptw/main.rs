//! LNS solver for the TSPTW
use std::{
    alloc::System,
    fs::File,
    sync::{atomic::AtomicBool, Arc},
    time::Instant, num::ParseIntError,
};

use anyhow::Result;
use libc::{SIGALRM, SIGINT};
use papier_lns::{MddLnsBuilder, ResolutionOutcome, SigLimitAllocator, Solution, Problem, SimpleMddBuilder, Var, Decision};
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use structopt::StructOpt;
use tsptw::{LeftToRight, RandomizedMinLP, Tsptw};

#[global_allocator]
static ALLOC: SigLimitAllocator<System> = SigLimitAllocator::new(System, usize::MAX);

/* */
pub type BitSet256 = smallbitset::MutSet256;
mod before;
mod tsptw;

/* */
#[derive(StructOpt)]
pub enum Args {
    Solve {
        #[structopt(short, long)]
        fname: String,
        /// Output the header
        #[structopt(short = "H", long)]
        header: bool,
        #[structopt(short, long, default_value = "100")]
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
        /// optional initial solution to kickstart the solver
        #[structopt(short, long)]
        solution: Option<String>
    },
    Check {
        #[structopt(short, long)]
        fname: String,
        #[structopt(short, long)]
        solution: String
    }, 
    Detail {
        #[structopt(short, long)]
        fname: String,
        #[structopt(short, long)]
        solution: String
    }
}

fn main() -> Result<()> {
    let args = Args::from_args();
    match args {
        Args::Solve{fname, header, width, seed, proba, ram_limit, time_limit, solution} => 
            solve(fname, header, width, seed, proba, ram_limit, time_limit, solution),
        Args::Check{fname, solution} => 
            check(fname, solution),
        Args::Detail{fname, solution} => 
            detail(fname, solution)
    }
}

fn detail(fname: String, solution: String) -> Result<()> {
    let inst     = Tsptw::try_from(File::open(&fname)?)?;
    let solution = try_solution_from_std_tour(&solution)?;
    let var_ord  = LeftToRight(inst.n_cities);
    inst.details(&var_ord, &solution);
    Ok(())
}

fn check(fname: String, solution: String) -> Result<()> {
    let inst     = Tsptw::try_from(File::open(&fname)?)?;
    let solution = try_solution_from_std_tour(&solution)?;
    let var_ord  = LeftToRight(inst.n_cities);
    let cost     = inst.evaluate(&var_ord, &solution);
    println!("cost: {:.4}", cost as f64 / 10000.0);
    
    inst.check(&var_ord, &solution);

    Ok(())
}

fn solve(fname: String, header: bool, width: usize, seed: u64, proba: f64, ram_limit: Option<f64>, time_limit: Option<u32>, solution: Option<String>) -> Result<()> {
    let kill_switch = Arc::new(AtomicBool::new(false));
    // ctrl + c   : interrupt program
    signal_hook::flag::register(SIGINT, Arc::clone(&kill_switch))?;
    // sigalrm for the timeout
    signal_hook::flag::register(SIGALRM, Arc::clone(&kill_switch))?;

    let inst = Tsptw::try_from(File::open(&fname)?)?;
    let n = inst.n_cities;

    if let Some(seconds) = time_limit {
        unsafe {
            libc::alarm(seconds);
        };
    }
    if let Some(gigabytes) = ram_limit {
        ALLOC.set_limit_gb(gigabytes);
    }

    let start_tm = Instant::now();
    let instname = instance_name(&fname);
    let init_sol = solution.map(|s| try_solution_from_std_tour(&s).expect("Cannot parse solution"));
    let init_val = init_sol.as_ref().map(|s| inst.evaluate(&LeftToRight(n), s));

    // there is no good method to find an initial solution with this problem
    let mdd = SimpleMddBuilder::default()
        .problem(&inst)
        .var_ordering(LeftToRight(n))
        .node_selection(RandomizedMinLP::new(&inst))
        .rng(Xoshiro256Plus::seed_from_u64(seed))
        .proba(proba)
        .kill_switch(Arc::clone(&kill_switch))
        .build()?;
    
    let mut solver = MddLnsBuilder::default()
        .mdd(mdd)
        .nb_var(inst.nb_vars())
        .width(width)
        .initial_sol(init_sol)
        .initial_val(init_val)
        .start(start_tm)
        .kill_switch(kill_switch)
        .build()?;
    //
    //let outcome = solver.minimize_with_cond(|o| {println!("{}", o); false});
    let outcome = solver.minimize();

    // ////////////////////////////////////////////////////////////////////////
    // Print the output
    // ////////////////////////////////////////////////////////////////////////
    if header {
        print_header();
    }
    let ram = ALLOC.get_peak_gb();
    print_result(&instname, ram, outcome);

    Ok(())
}


fn instance_name(fname: &str) -> String {
    let it = fname
        .split_terminator(std::path::MAIN_SEPARATOR)
        .rev()
        .take(2);
    
    let mut out = String::new();
    for (i, x) in it.enumerate() {
        if i == 0 {
            out = x.to_owned();
        } else {
            out = format!("{}/{}", x, out);
        }
    }
    if out.is_empty() {
        out.push_str("-- no name --");
    }
    out
}

fn print_header() {
    // instance | method | status | value | ram in gb | time to best | time to proved | solution
    println!(
        "{:>20} | {:>10} | {:>15} | {:>10} | {:>8} | {:>10} | {:>10} | {:<80}",
        "Instance", "Method", "Status", "Value", "RAM", "Best (s)", "Proved (s)", "Solution"
    );
}

fn print_result(instance: &str, ram: f64, outcome: ResolutionOutcome) {
    // instance | method | status | value | ram in gb | time to best | time to proved | solution
    println!(
        "{:>20} | {:>10} | {:>15} | {:>10} | {:>8.2} | {:>10} | {:>10} | {:<80}",
        instance,
        "lns",
        outcome.status.to_str(),
        outcome
            .best_value
            .map(|v| format!("{:>10.2}", v as f32 / 10000.0))
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
            .map(|sol| solution_as_std_tour(&sol))
            .unwrap_or_else(|| "-- no solution --".to_string())
    );
}

fn solution_as_std_tour(sol: &Solution) -> String {
    let mut out = String::new();
    for d in sol.iter() {
        if d.val == 0 {
            continue;
        } else {
            out.push_str(&d.val.to_string());
            out.push(' ');
        }
    }
    out
}

fn try_solution_from_std_tour(txt: &str) -> Result<Solution, ParseIntError> {
    let mut data = vec![];
    for token in txt.split_ascii_whitespace() {
        let decision  = token.parse::<isize>()?;
        if decision != 0 { // skip initial depot
            let var = Var::new(data.len());
            let decision = Decision::new(var, decision);
            data.push(decision);
        }
    }
    // append final depot
    let var = Var::new(data.len());
    let decision = Decision::new(var, 0);
    data.push(decision);
    
    let sol = Solution::from(data.iter().copied());

    Ok(sol)
}