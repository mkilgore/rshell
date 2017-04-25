
use libc;
use std::ffi::CString;
use std::process;
use builtin::*;

#[derive(Debug)]
pub struct Prog {
    pub file: String,
    pub args: Vec<String>,
    pub stdin: libc::c_int,
    pub stdout: libc::c_int,
    pub stderr: libc::c_int,
    pub pgrp: libc::pid_t,
    pub pid: libc::pid_t,
    pub builtin: Option<ShellBuiltin>,
}

unsafe fn dup_if_not_eq(old_fd: libc::c_int, new_fd: libc::c_int) {
    if old_fd != new_fd {
        libc::dup2(new_fd, old_fd);
        libc::close(new_fd);
    }
}

impl Prog {
    pub fn new() -> Prog {
        Prog {
            file: "".to_string(),
            args: Vec::new(),
            stdin: libc::STDIN_FILENO,
            stdout: libc::STDOUT_FILENO,
            stderr: libc::STDERR_FILENO,
            pgrp: -1,
            pid: -1,
            builtin: None,
        }
    }

    pub fn close_fds(&mut self) {
        if self.stdin != libc::STDIN_FILENO {
            unsafe { libc::close(self.stdin); }
        }

        if self.stdout != libc::STDOUT_FILENO {
            unsafe { libc::close(self.stdout); }
        }

        if self.stderr != libc::STDERR_FILENO {
            unsafe { libc::close(self.stderr); }
        }
    }

    unsafe fn start_child(&mut self) {
        libc::setpgid(0, self.pgrp);

        dup_if_not_eq(libc::STDIN_FILENO, self.stdin);
        dup_if_not_eq(libc::STDOUT_FILENO, self.stdout);
        dup_if_not_eq(libc::STDERR_FILENO, self.stderr);

        match self.builtin {
            Some(builtin) => {
                let ret = (builtin.callback) (self);
                process::exit(ret as i32);
            },
            _ => { }
        }

        let mut cstr_vec: Vec<CString> = Vec::new();

        cstr_vec.push(CString::new(self.file.clone()).unwrap());

        for s in &self.args {
            let cstr = CString::new(s.as_str()).unwrap();
            cstr_vec.push(cstr);
        }
        cstr_vec.shrink_to_fit();

        let mut c_char_vec: Vec<*const libc::c_char> = Vec::new();
        for s in &mut cstr_vec {
            c_char_vec.push(s.as_ptr());
        }
        c_char_vec.push(0 as *const libc::c_char);

        for i in 0..32 {
            libc::signal(i, libc::SIG_DFL);
        }

        libc::execvp(CString::new(self.file.clone()).unwrap().as_ptr() as *const libc::c_char, c_char_vec.as_ptr() as *const *const libc::c_char);


        println!("{}: command not found", self.file);
        process::exit(0);
    }

    pub fn run(&mut self) {
        self.pid = unsafe { libc::fork() };

        match self.pid {
            -1 => return,
            0 => unsafe { self.start_child() },
            _ => {
                /* Parent - close fd's */
                unsafe {
                    libc::setpgid(self.pid, self.pgrp);
                    self.close_fds();
                }
            }
        }
    }

    pub fn add_arg(&mut self, s: &str) {
        self.args.push(s.to_string());
    }

    pub fn run_builtin(&mut self) -> usize {
        let mut ret: usize = 0;

        match self.builtin {
            Some(builtin) => {
                ret = (builtin.callback) (self);
            },
            _ => { }
        }

        self.close_fds();

        ret
    }
}

