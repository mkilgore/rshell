
extern crate libc;

#[macro_use]
extern crate lazy_static;

mod lexer;
mod job;
mod builtin;
mod prog;
mod job_list;

use std::io::Write;
use std::io::BufRead;
use std::io;
use lexer::*;
use job::*;
use job_list::*;
use std::sync::*;
use std::ffi::CString;

lazy_static! {
    pub static ref JOB_LIST: Mutex<JobList> = Mutex::new(JobList::new());
    pub static ref CURRENT_DIR: Mutex<String> = Mutex::new(String::new());
}

fn main() {
    let mut new_job;
    let inp = io::stdin();
    let mut out = io::stdout();
    let mut lines = inp.lock().lines();
    let env = std::env::vars();

    match env.filter(|x| x.0 == "HOME").next() {
        Some((_, cwd)) => {
            *CURRENT_DIR.lock().unwrap() =  cwd;
        },
        _ => { *CURRENT_DIR.lock().unwrap() = "/".to_string() }
    }

    unsafe {
        let cs = CString::new(CURRENT_DIR.lock().unwrap().clone()).unwrap();
        libc::chdir(cs.as_ptr());
    }

    loop {
        print!("{}: ", *CURRENT_DIR.lock().unwrap());
        out.flush().unwrap();

        let s = lines.next().unwrap().unwrap();

        if s != "" {
            let mut lexer = InputLexer::new(&s);
            match Job::parse_job(&mut lexer) {
                Some(mut job) => {
                    job.name = s.clone();

                    if job.is_simple_bulitin() {
                        job.simple_builtin_run();
                    } else {
                        new_job = JOB_LIST.lock().unwrap().add_job(job);

                        if !new_job.lock().unwrap().is_background {
                            JOB_LIST.lock().unwrap().set_forground_job(Some(new_job));
                        } else {
                            JOB_LIST.lock().unwrap().set_forground_job(None);
                        }
                    }
                },
                None => {
                    println!("rshell: Syntax error in command");
                }
            }
        }

        JOB_LIST.lock().unwrap().update_job_list();
    }
}
