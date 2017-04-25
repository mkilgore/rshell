
use prog::*;
use std::fmt;
use std::collections::*;
use ::*;

pub type BuiltinCallback = fn(&mut Prog) -> usize;

pub struct ShellBuiltin {
    pub callback: BuiltinCallback,
}

impl Copy for ShellBuiltin { }
impl Clone for ShellBuiltin {
    fn clone(&self) -> Self { *self }
}

impl ShellBuiltin {
    pub fn new(c: BuiltinCallback) -> ShellBuiltin {
        ShellBuiltin {
            callback: c,
        }
    }
}

lazy_static! {
    static ref BUILTIN_MAP: HashMap<&'static str, ShellBuiltin> = {
        let mut m = HashMap::new();
        m.insert("pwd",  ShellBuiltin::new(builtin_pwd));
        m.insert("cd",   ShellBuiltin::new(builtin_cd));
        m.insert("jobs", ShellBuiltin::new(builtin_jobs));
        m.insert("fg",   ShellBuiltin::new(builtin_fg));
        m.insert("bg",   ShellBuiltin::new(builtin_bg));
        m.insert("echo", ShellBuiltin::new(builtin_echo));
        m
    };
}

pub fn builtin_find_callback(s: &str) -> Option<ShellBuiltin> {
    match BUILTIN_MAP.get(s) {
        Some(builtin) => Some(*builtin),
        None => None
    }
}

impl fmt::Debug for ShellBuiltin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ShellBuiltin {{ }}")
    }
}

fn fd_print(fd: libc::c_int, s: &str) -> usize {
    unsafe { libc::write(fd, s.as_ptr() as *const libc::c_void, s.len()) as usize }
}

fn builtin_jobs(prog: &mut Prog) -> usize {
    let job_list = JOB_LIST.lock().unwrap();
    let mut i: usize = 0;
    let mut ret: usize = 0;

    for job in &job_list.list {
        i += 1;

        let real_job = job.lock().unwrap();

        ret = fd_print(prog.stdout, &format!("[{}] {}: {}\n",
                                             i,
                                             if real_job.state == JobState::Running { "Running" } else { "Stopped" },
                                             real_job.name));
    }

    return ret;
}

fn builtin_pwd(prog: &mut Prog) -> usize {
    let cwd = CURRENT_DIR.lock().unwrap();

    fd_print(prog.stdout, &format!("{}\n", *cwd))
}

fn builtin_fg(prog: &mut Prog) -> usize {
    let job_id;

    if prog.args.len() < 1 {
        job_id = 1;
    } else {
        job_id = prog.args[0].parse::<usize>().unwrap();
    }

    let mut job_list = JOB_LIST.lock().unwrap();
    let real_job_id = job_id - 1;

    if real_job_id >= job_list.list.len() {
        fd_print(prog.stdout, &format!("bg: Unknown job {}\n", job_id));
        return 1;
    }

    let j = job_list.list[real_job_id].clone();

    job_list.set_forground_job(Some(j));

    0
}

fn builtin_bg(prog: &mut Prog) -> usize {
    let job_id;

    if prog.args.len() < 1 {
        job_id = 1;
    } else {
        job_id = prog.args[0].parse::<usize>().unwrap();
    }

    let job_list = JOB_LIST.lock().unwrap();
    let real_job_id = job_id - 1;

    if real_job_id >= job_list.list.len() {
        fd_print(prog.stdout, &format!("fg: Unknown job {}\n", job_id));
        return 1;
    }

    let lck = job_list.list[real_job_id].clone();
    let mut j = lck.lock().unwrap();

    j.cont();

    0
}

fn builtin_cd(prog: &mut Prog) -> usize {
    let mut new_cwd: String;
    if prog.args.len() == 0 {
        return 1;
    }

    let vec_dirs: Vec<&str> = prog.args[0].split("/").collect();
    let mut dirs: &[&str] = &vec_dirs;

    if dirs[0] == "" {
        new_cwd = "/".to_string();
        dirs = &dirs[1..];
    } else {
        new_cwd = CURRENT_DIR.lock().unwrap().clone();
    }

    for dir in dirs {
        if dir.len() == 0 {
            continue;
        }

        if *dir == ".." {
            if new_cwd.len() == 1 {
                continue;
            }

            while new_cwd.pop().unwrap() != '/' {
                ;
            }

            if new_cwd.len() == 0 {
                new_cwd += "/";
            }
        } else if *dir != "." {
            if new_cwd.len() != 1 {
                new_cwd += "/";
            }
            new_cwd += *dir;
        }
    }

    let ret;

    unsafe {
        let cs = CString::new(new_cwd.clone()).unwrap();
        ret = libc::chdir(cs.as_ptr());
    }

    if ret == 0 {
        *CURRENT_DIR.lock().unwrap() = new_cwd;
        0
    } else {
        fd_print(prog.stdout, &format!("cd {}: No such directory\n", prog.args[0]));
        1
    }
}

fn builtin_echo(prog: &mut Prog) -> usize {
    if prog.args.len() == 0 {
        return 0;
    }

    let s = prog.args.join(" ") + "\n";

    fd_print(prog.stdout, &s)
}

