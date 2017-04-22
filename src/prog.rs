
use libc;
use std::ffi::CString;
use std::fmt;

pub struct ProgBuiltin {
    pub id: &'static str,
    pub cmd: Option<fn(&mut Prog) -> bool>,
}

#[derive(Debug)]
pub struct Prog {
    pub file: String,
    pub args: Vec<String>,
    pub stdin: libc::c_int,
    pub stdout: libc::c_int,
    pub stderr: libc::c_int,
    pub pgrp: libc::pid_t,
    pub pid: libc::pid_t,
    pub is_builtin: bool,
    pub builtin: ProgBuiltin,
}

impl fmt::Debug for ProgBuiltin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ProgBuiltin {{ id: \"{}\", cmd: {}",
               self.id,
               if self.cmd.is_some() { "Some" } else { "None" })
    }
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
            is_builtin: false,
            builtin: ProgBuiltin { id: "", cmd: None },
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

        libc::execvp(CString::new(self.file.clone()).unwrap().as_ptr() as *const libc::c_char, c_char_vec.as_ptr() as *const *const libc::c_char);

        panic!("Execvp error");
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
                    if self.stdin != libc::STDIN_FILENO {
                        libc::close(self.stdin);
                    }
                    if self.stdout != libc::STDOUT_FILENO {
                        libc::close(self.stdout);
                    }
                    if self.stderr != libc::STDERR_FILENO {
                        libc::close(self.stderr);
                    }
                }

                println!("Started Child: {}", self.pid);
            }
        }
    }

    pub fn addArg(&mut self, s: &str) {
        self.args.push(s.to_string());
    }
}

