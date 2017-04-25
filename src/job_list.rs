
use job::*;
use libc;
use std::sync::*;

pub struct JobList {
    pub forground_job: Option<Arc<Mutex<Job>>>,
    pub list: Vec<Arc<Mutex<Job>>>,
}

fn rc_ptr_eq(this: &Arc<Mutex<Job>>, other: &Arc<Mutex<Job>>) -> bool {
    Arc::ptr_eq(this, other)
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
                let mut job = job_ref.lock().unwrap();

                unsafe { libc::tcsetpgrp(libc::STDIN_FILENO, (*job).pgrp); }
                job.cont();
            },
            None => {

            }
        }
    }

    pub fn add_job(&mut self, job: Job) -> Arc<Mutex<Job>> {
        self.list.push(Arc::new(Mutex::new((job))));
        {
            let new_job = self.list.last_mut().unwrap();
            let mut job_locked = new_job.lock().unwrap();
            job_locked.start();

            if !job_locked.is_background {
                self.forground_job = Some(new_job.clone());
            }
        }

        self.list.last_mut().unwrap().clone()
    }

    pub fn set_forground_job(&mut self, job: Option<Arc<Mutex<Job>>>) {
        self.forground_job = job;
    }

    pub fn remove_job(&mut self, job_index: usize) {
        self.list.remove(job_index);
    }

    pub fn is_empty(&mut self) -> bool {
        self.list.is_empty()
    }

    pub fn find_pgrp(&mut self, pgrp: libc::pid_t) -> (Option<Arc<Mutex<Job>>>, usize) {
        for i in 0..self.list.len() {
            if self.list[i].lock().unwrap().pgrp == pgrp {
                return (Some(self.list[i].clone()), i);
            }
        }

        (None, 0)
    }

    pub fn find_pid(&mut self, pid: libc::pid_t, prog_index: &mut usize) -> (Option<Arc<Mutex<Job>>>, usize) {
        for i in 0..self.list.len() {
            let j = self.list[i].lock().unwrap();

            for k in 0..j.progs.len() {
                if j.progs[k].pid == pid {
                    *prog_index = k;
                    return (Some(self.list[i].clone()), i);
                }
            }
        }

        (None, 0)
    }

    pub fn update_job_list(&mut self) {
        let mut waitpid_flags: libc::c_int = libc::WUNTRACED;
        self.make_job_forground();

        if self.forground_job.is_none() {
            waitpid_flags |= libc::WNOHANG;
        }

        loop {
            let pid: libc::pid_t;
            let mut wstatus: libc::c_int = 0;

            unsafe { pid = libc::waitpid(-1, &mut wstatus as *mut libc::c_int, waitpid_flags); }

            if pid == -1 || pid == 0 {
                break;
            }

            let mut prog_index: usize = 0;
            let (job, index) = self.find_pid(pid, &mut prog_index);

            match job {
                Some(ref j) => {
                    let mut job = j.lock().unwrap();

                    unsafe {
                        let w_exited = libc::WIFEXITED(wstatus);
                        let w_signaled = libc::WIFSIGNALED(wstatus);
                        let w_stopped = libc::WIFSTOPPED(wstatus);
                        let w_continued = libc::WIFCONTINUED(wstatus);

                        if w_exited || w_signaled {
                            job.progs[prog_index].pid = -1;

                            if job.has_exited() {
                                self.list.remove(index);
                                if self.forground_job.is_some() && rc_ptr_eq(&j, &self.forground_job.as_mut().unwrap()) {
                                    libc::tcsetpgrp(libc::STDIN_FILENO, libc::getpid());
                                    return ;
                                } else {
                                    if w_exited {
                                        println!("[{}] Finished: {}", index + 1, libc::WEXITSTATUS(wstatus));
                                    } else {
                                        println!("[{}] Killed by signal {}", index + 1, libc::WTERMSIG(wstatus));
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

                            println!("[{}] Stopped", index + 1);

                            if rc_ptr_eq(&j, &self.forground_job.as_mut().unwrap()) {
                                self.forground_job = None;
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
}

