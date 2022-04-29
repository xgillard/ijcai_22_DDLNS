use std::{fs::File};

use anyhow::Result;
use structopt::StructOpt;
use rand_core::SeedableRng;
use tsptw_feasibility::{data::{Tsptw, Before}, Random};

#[derive(StructOpt)]
enum Args {
    Find {
        #[structopt(short, long)]
        fname: String,
        #[structopt(short, long, default_value="20211215")]
        seed : u64,
        #[structopt(short, long, default_value="8")]
        level_max : usize,
    },
    Check {
        #[structopt(short, long)]
        fname: String,
        #[structopt(short, long)]
        solution: String,
        #[structopt(short, long)]
        verbose: bool
    }
}

fn inst_name(fname: &str) -> &str {
    fname.split_terminator(std::path::MAIN_SEPARATOR)
        .last()
        .unwrap_or("-- no name -- ")
}

fn find(fname: &str, seed: u64, level_max: usize) -> Result<()> {
    let mut rng= Random::seed_from_u64(seed);
    let inst   = Tsptw::try_from(File::open(fname)?)?;
    let before = Before::new(&inst);

    let sol = inst.find_feasible(&mut rng, &before, level_max);
    let eval= inst.evaluate(&sol, false);

    let mut solution_txt = String::new();
    for x in sol {
        solution_txt.push_str(&x.to_string());
        solution_txt.push(' ');
    }

    // format: instname cost CV perm
    println!("{:<20}    {:>10.2}  {:>10}  {}", 
        inst_name(fname), 
        (eval.cost as f32) / 10000.00, 
        eval.violation,
        solution_txt);
    
    Ok(())
}

fn check(fname: &str, solution: &str, verbose: bool) -> Result<()> {
    let inst     = Tsptw::try_from(File::open(fname)?)?;
    let solution = solution.split_ascii_whitespace().map(|x| x.parse().unwrap()).collect::<Vec<_>>();
    let eval     = inst.evaluate(&solution, verbose);

    println!("cost {:>10.2}  violations {:>10.2}", 
        (eval.cost      as f32) / 10000.0, 
        (eval.violation as f32) / 10000.0);

    Ok(())
}

fn main() -> Result<()> {
    let args   = Args::from_args();
    match args {
        Args::Find{fname, seed, level_max} =>
            find(&fname, seed, level_max)?,
        Args::Check{fname, solution, verbose} => 
            check(&fname, &solution, verbose)?
    }
    
    Ok(())
}
