
extern crate libc;

mod lexer;
mod job;
mod prog;
mod job_list;

use std::prelude::*;
use std::io::Read;
use std::io::BufRead;
use std::io;
use lexer::*;
use job::*;
use prog::*;
use job_list::*;
use std::mem;

fn main() {
    let mut new_job;
    let inp = io::stdin();
    let mut job_list: JobList = JobList::new();

    for line in inp.lock().lines() {
        let s = line.unwrap();

        if s == "" {
            job_list.update_background();
            continue ;
        }

        let mut lexer = InputLexer::new(&s);
        match Job::parseJob(&mut lexer) {
            Some(job) => {
                println!("Parsed Job!");
                println!("Job: {:?}", job);

                new_job = job_list.add_job(job);

                println!("Job started!");

                if !new_job.borrow().is_background {
                    job_list.make_forground(&mut new_job);
                } else {
                    job_list.update_background();
                }
            },
            None => {
                println!("Unable to parse job");
            }
        }
    }
}
