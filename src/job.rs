
use prog::*;
use libc;
use lexer::*;
use std::ffi::CString;
use builtin::*;

#[derive(PartialEq, Debug)]
pub enum JobState {
    Stopped,
    Running,
}

#[derive(Debug)]
pub struct Job {
    pub pgrp: libc::pid_t,
    pub name: String,
    pub state: JobState,
    pub progs: Vec<Prog>,
    pub is_background: bool,
}

impl Job {
    pub fn new() -> Job {
        Job {
            pgrp: -1,
            name: "".to_string(),
            state: JobState::Stopped,
            progs: Vec::new(),
            is_background: false,
        }
    }

    pub fn start(&mut self) {
        let mut pgrp: libc::c_int = 0;

        for prog in &mut self.progs {
            prog.pgrp = pgrp;
            prog.run();


            if pgrp == 0 {
                pgrp = prog.pid;
                prog.pgrp = prog.pid;
            }
        }

        self.pgrp = pgrp;
        self.state = JobState::Running;
    }

    pub fn kill(&mut self) {
        unsafe { libc::kill(-self.pgrp, libc::SIGKILL); }
    }

    pub fn stop(&mut self) {
        unsafe { libc::kill(-self.pgrp, libc::SIGSTOP); }
        self.state = JobState::Stopped;
    }

    pub fn cont(&mut self) {
        unsafe { libc::kill(-self.pgrp, libc::SIGCONT); }
        self.state = JobState::Running;
    }

    pub fn has_exited(&mut self) -> bool {
        for prog in &self.progs {
            if prog.pid != -1 {
                return false;
            }
        }

        return true;
    }

    pub fn add_prog(&mut self, prog: Prog) {
        self.progs.push(prog);
    }

    pub fn parse_job(mut lex: &mut InputLexer) -> Option<Job> {
        let mut job: Job = Job::new();
        let mut cur_prog: Prog = Prog::new();
        let mut opt_tok: Option<InputToken>;
        let mut job_err: bool = false;

        'lop: while let Some(tok) = lex.next() {
            match tok {
                InputToken::Comment => {
                    while { opt_tok = lex.next();
                            opt_tok.is_some() && opt_tok.unwrap() != InputToken::NewLine }
                    { ; }
                    break;
                },
                InputToken::Identifier(s) => {
                    if cur_prog.file == "" {
                        cur_prog.file = s;
                        cur_prog.builtin = builtin_find_callback(&cur_prog.file);
                    } else {
                        cur_prog.add_arg(&s);
                    }
                },
                InputToken::NewLine => {
                    break;
                },
                InputToken::RedirectOut | InputToken::RedirectAppendOut | InputToken::RedirectIn => {
                    opt_tok = lex.next();
                    match opt_tok {
                        Some(InputToken::Identifier(s)) => {
                            if tok == InputToken::RedirectOut {
                                cur_prog.stdout = unsafe { libc::open(CString::new(s).unwrap().as_ptr(),
                                                            libc::O_WRONLY | libc::O_CREAT, 0777) };
                            } else if tok == InputToken::RedirectAppendOut {
                                cur_prog.stdout = unsafe { libc::open(CString::new(s).unwrap().as_ptr(),
                                                            libc::O_WRONLY | libc::O_CREAT | libc::O_APPEND, 0777) };
                            } else if tok == InputToken::RedirectIn {
                                cur_prog.stdin = unsafe { libc::open(CString::new(s).unwrap().as_ptr(), libc::O_RDONLY) };
                            }
                        },
                        _ => {
                            println!("Error: Redirect requires filename");
                            job_err = true;
                            break 'lop;
                        }
                    }
                },
                InputToken::Pipe => {
                    let mut pipefd: [libc::c_int; 2] = [0, 2];
                    unsafe {
                        libc::pipe(&mut pipefd[0] as *mut libc::c_int);

                        libc::fcntl(pipefd[0], libc::F_SETFD, libc::fcntl(pipefd[0], libc::F_GETFD) | libc::FD_CLOEXEC);
                        libc::fcntl(pipefd[1], libc::F_SETFD, libc::fcntl(pipefd[1], libc::F_GETFD) | libc::FD_CLOEXEC);
                    }

                    cur_prog.stdout = pipefd[1];

                    job.add_prog(cur_prog);
                    cur_prog = Prog::new();

                    cur_prog.stdin = pipefd[0];
                },
                InputToken::Background => {
                    job.is_background = true;
                }
                t => {
                    println!("Error: Unexpected token {:?}", t);
                    job_err = true;
                    break 'lop;
                }
            }
        }

        if cur_prog.file != "" {
            job.add_prog(cur_prog);
        } else {
            cur_prog.close_fds();
        }

        if !job_err {
            Some(job)
        } else {
            None
        }
    }

    pub fn is_simple_bulitin(&mut self) -> bool {
        if self.progs.len() != 1 {
            return false;
        }

        if self.progs[0].builtin.is_none() {
            return false;
        }

        return true;
    }

    pub fn simple_builtin_run(&mut self) -> usize {
        self.progs[0].run_builtin()
    }
}

