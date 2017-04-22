
use job::*;
use libc;
use std::prelude::*;
use std::borrow;
use std::rc::*;
use std::cell::*;

pub struct JobList {
    forground_job: Option<Rc<RefCell<Job>>>,
    list: Vec<Rc<RefCell<Job>>>,
}

fn rc_ptr_eq(this: &Rc<RefCell<Job>>, other: &Rc<RefCell<Job>>) -> bool {
    unsafe {
        (**this).as_ptr() as *const Job == (**other).as_ptr() as *const Job
    }
}

impl JobList {
    pub fn new() -> JobList {
        JobList {
            list: Vec::new(),
            forground_job: None,
        }
    }

    pub fn make_job_forground(&mut self) {
        match self.forground_job {
            Some(ref job_ref) => {
                let mut job = job_ref.borrow_mut();

                unsafe { libc::tcsetpgrp(libc::STDIN_FILENO, job.pgrp); }
                job.cont();
            },
            _ => {

            }
        }
    }

    pub fn add_job(&mut self, mut job: Job) -> Rc<RefCell<Job>> {
        self.list.push(Rc::new(RefCell::new((job))));
        {
            let mut new_job = self.list.last_mut().unwrap();

            {
                let mut borrow = new_job.borrow_mut();
                borrow.start();
            }

            if !new_job.borrow().is_background {
                self.forground_job = Some(new_job.clone());
            }
        }

        self.list.last_mut().unwrap().clone()
    }

    pub fn remove_job(&mut self, job_index: usize) {
        self.list.remove(job_index);
    }

    pub fn is_empty(&mut self) -> bool {
        self.list.is_empty()
    }

    pub fn find_pgrp(&mut self, pgrp: libc::pid_t) -> (Option<Rc<RefCell<Job>>>, usize) {
        for i in 0..self.list.len() {
            if self.list[i].borrow().pgrp == pgrp {
                return (Some(self.list[i].clone()), i);
            }
        }

        (None, 0)
    }

    pub fn find_pid(&mut self, pid: libc::pid_t, prog_index: &mut usize) -> (Option<Rc<RefCell<Job>>>, usize) {
        for i in 0..self.list.len() {
            let mut j = self.list[i].borrow_mut();

            println!("Checking job: {:?}", j);
            for k in 0..j.progs.len() {
                if j.progs[k].pid == pid {
                    *prog_index = k;
                    return (Some(self.list[i].clone()), i);
                }
            }
        }

        (None, 0)
    }

    fn update_job_list(&mut self) {
        let mut waitpid_flags: libc::c_int = 0;
        self.make_job_forground();

        if self.forground_job.is_none() {
            waitpid_flags |= libc::WNOHANG;
        }

        loop {
            let mut pid: libc::pid_t;
            let mut wstatus: libc::c_int = 0;

            unsafe { pid = libc::waitpid(-1, &mut wstatus as *mut libc::c_int, waitpid_flags); }

            if pid == -1 || pid == 0 {
                break;
            }

            println!("Waitpid returned: {}", pid);

            let mut prog_index: usize = 0;
            let (mut job, index) = self.find_pid(pid, &mut prog_index);

            println!("Found pid: {} -> {}", index, prog_index);

            match job {
                Some(ref j) => {
                    let mut job = j.borrow_mut();

                    unsafe {
                        let w_exited = libc::WIFEXITED(wstatus);
                        let w_signaled = libc::WIFSIGNALED(wstatus);
                        let w_stopped = libc::WIFSTOPPED(wstatus);
                        let w_continued = libc::WIFCONTINUED(wstatus);

                        println!("Exited: {}, Signaled: {}, Stopped: {}, Continued: {}", w_exited, w_signaled, w_stopped, w_continued);

                        if w_exited || w_signaled {
                            job.progs[prog_index].pid = -1;

                            if job.has_exited() {
                                self.list.remove(index);
                                if self.forground_job.is_some() && rc_ptr_eq(&j, &self.forground_job.as_mut().unwrap()) {
                                    println!("Setting shell to process group leader\n");
                                    libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid());
                                    return ;
                                } else {
                                    if w_exited {
                                        println!("[{}] Finished: {}", index, libc::WEXITSTATUS(wstatus));
                                    } else {
                                        println!("[{}] Killed by signal {}", index, libc::WTERMSIG(wstatus));
                                    }
                                }
                            }
                        } else if w_stopped {
                            if job.state == JobState::Stopped {
                                continue;
                            }

                            job.stop();

                            if self.forground_job.is_some() {
                                libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid());
                            }

                            println!("[{}] Stopped", index);

                            if rc_ptr_eq(&j, &self.forground_job.as_mut().unwrap()) {
                                return ;
                            }

                            self.make_job_forground();
                        } else if w_continued {
                            job.start();
                        }
                    }
                },
                None => {
                    panic!("Unknown Pid");
                }
            }
        }

    }

    pub fn make_forground(&mut self, job: &mut Rc<RefCell<Job>>) {
        self.forground_job = Some(job.clone());
        self.update_job_list();
    }

    pub fn update_background(&mut self) {
        self.forground_job = None;
        self.update_job_list();
    }
}

