use lnk;
use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::env;

const HELP: &str =
"

Normal Usage:
lnkx /path/to/file.lnk : Tries to open the resource pointed at by the file

CLI Usage:
lnkx create /path/to/file.lnk /path/to/program.exe [arg1] [...] [argn] : Creates a new .lnk file
lnkx convert /path/to/symlink /path/to/file.lnk : Converts an existing Symlink to a .lnk file while keeping the original file
lnkx convert /path/to/file.lnk /path/to/symlink : Converts an existing .lnk file to a symlink while keeping the original file
lnkx help : Shows this message
lnkx (no arg) : Shows this message";

#[derive(Debug, Default)]
pub struct Drive{
    // path: String,
    mountpoint: String,
    label: String
}

fn lines_from_bytes(mut data: Vec<u8>) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();

    while let Some(pos) = data[0..].iter().position(|&b| b == b'\n') {
        let end = pos;
        lines.push(data.drain(0..=end).collect());
    }
    if !data.is_empty() {
        lines.push(data.drain(..).collect());
    }
    lines
}
pub fn get_drives()->Vec<Drive>{
    let mut drives = Vec::new();

    // let mut drives_path = Vec::new();
    // let lsblk = std::process::Command::new("lsblk")
    //     .args(&["-l", "-n", "-o", "PATH"])
    //     .output()
    //     .expect("lsblk failed");
    // let lines = lines_from_bytes(lsblk.stdout);
    // for i in 0..lines.len(){
    //     let p = &lines[i][0..lines[i].len()-1];
    //     drives_path.push(String::from_utf8(p.to_vec()).unwrap());
    // }

    let mut drives_mountpoint = Vec::new();
    let lsblk = std::process::Command::new("lsblk")
        .args(&["-l", "-n", "-o", "MOUNTPOINT"])
        .output()
        .expect("lsblk failed");
    let lines = lines_from_bytes(lsblk.stdout);
    for i in 0..lines.len(){
        let m = &lines[i][0..lines[i].len()-1];
        drives_mountpoint.push(String::from_utf8(m.to_vec()).unwrap());
    }

    let mut drives_label = Vec::new();
    let lsblk = std::process::Command::new("lsblk")
        .args(&["-l", "-n", "-o", "LABEL"])
        .output()
        .expect("lsblk failed");
    let lines = lines_from_bytes(lsblk.stdout);
    for i in 0..lines.len(){
        let l = &lines[i][0..lines[i].len()-1];
        drives_label.push(String::from_utf8(l.to_vec()).unwrap());
    }

    for i in 0..drives_mountpoint.len(){
        drives.push(Drive { // path: drives_path[i].clone(),
            mountpoint: drives_mountpoint[i].clone(), label: drives_label[i].clone() })
    }
    drives
}

#[derive(Debug,Default)]
pub struct Lnk {
    /// string_data -> relative_path
    relative_path: Option<String>,
    /// string_data -> working_dir
    working_dir: Option<String>,
    /// string_data -> command_line_arguments
    command_line_arguments: Option<String>,
    // Link info
    /// link_info -> volume_id -> volume_label
    volume_label: Option<String>,
    /// link_info -> local_base_path
    local_base_path: Option<String>,
    absolute_path: Option<String>,
}

impl Lnk {
    fn absolute_path(self: &mut Self, base: String){
        let mut mountpoint: Option<String> = None;
        if self.working_dir.is_some(){
            let drives = get_drives();
            for d in drives{
                if self.volume_label.is_none(){
                    // Add relative working dir
                    let r = self.working_dir.clone();
                    let relative = r.unwrap().clone().replace('\\', "/");
                    let dir = Path::new(&base).parent().unwrap_or_else(|| Path::new("."));
                    let absolute = std::fs::canonicalize(dir.join(relative));
                    if absolute.is_ok(){
                        match absolute.unwrap().to_str(){
                            Some(a) => {self.working_dir = Some(a.to_string());},
                            None => {self.working_dir = None},
                        }
                    }
                } else if d.label == self.volume_label.clone().unwrap(){
                    mountpoint = Some(d.mountpoint.clone());
                    let w = self.working_dir.clone();
                    let unix = w.unwrap().clone().replace('\\', "/");
                    let unix = mountpoint.clone().unwrap() + &unix[2..unix.len()];
                    if Path::new(&unix).exists(){
                        self.working_dir = Some(unix);
                    }else{
                        self.working_dir = None;
                    }
                }
            }
        }

        if self.relative_path.is_some(){
            let r = self.relative_path.clone();
            let relative = r.unwrap().clone().replace('\\', "/");
            let dir = Path::new(&base).parent().unwrap_or_else(|| Path::new("."));
            let absolute = std::fs::canonicalize(dir.join(relative));
            if absolute.is_ok(){
                match absolute.unwrap().to_str(){
                    Some(a) => {self.absolute_path = Some(a.to_string());},
                    None => {},
                }
            }
        }
        match self.absolute_path{
            Some(_) => return,
            None => {
                // todo!("So basically the problem is that if the relative is broken I shall use the LOCAL BASE PATH and I have to cross check the volume label with the current one using lsblk ");
                // Find from the drives which has the same label
                if mountpoint.is_none(){
                    let drives = get_drives();
                    for d in drives{
                        if d.label == self.volume_label.clone().unwrap(){
                            mountpoint = Some(d.mountpoint.clone());
                        }
                    }
                }
                let w = self.local_base_path.clone();
                let unix = w.unwrap().clone().replace('\\', "/");
                let unix = mountpoint.clone().unwrap() + &unix[2..unix.len()];
                if Path::new(&unix).exists(){
                    self.absolute_path = Some(unix);
                }else{
                    self.absolute_path = None;
                }
            },
        }
    }

}

fn split_args(s: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_arg = false;
    let mut quote: Option<char> = None;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else if c == '\\' && q == '"' && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    current.push(c);
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                    in_arg = true;
                } else if c.is_whitespace() {
                    if in_arg {
                        args.push(std::mem::take(&mut current));
                        in_arg = false;
                    }
                } else {
                    current.push(c);
                    in_arg = true;
                }
            }
        }
    }

    if quote.is_some() {
        return None;
    }
    if in_arg {
        args.push(current);
    }
    Some(args)
}

fn open_smart(target: &str, command_line_arguments: Option<String>, working_dir: Option<String>) -> io::Result<()> {
    let path = Path::new(target);
    let is_exe = path
        .extension()
        .and_then(OsStr::to_str)
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false);

    let mut cmd;
    if is_exe {
        let abs = std::path::absolute(path)?;
        let dir: PathBuf = match working_dir.as_deref().filter(|d| !d.trim().is_empty()) {
            Some(d) => PathBuf::from(d),
            None => abs
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| ".".into()),
        };

        let extra_args: Vec<String> = match command_line_arguments.as_deref() {
            Some(s) if !s.trim().is_empty() => split_args(s).ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidInput, "unbalanced quotes in arguments")
            })?,
            _ => Vec::new(),
        };

        cmd = Command::new("wine");
        cmd.arg(&abs).args(extra_args).current_dir(dir);
    } else {
        cmd = Command::new("xdg-open");
        cmd.arg(target);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    Ok(())
}

fn parse_url_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let text = text.trim_start_matches('\u{feff}');

    text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("URL")
            .then(|| value.trim().to_string())
    })
}

fn main() {
    let mut i = 0;
    let mut args = Vec::new();
    for argument in env::args() {
        println!("{argument}");
        args.push(argument.clone());
        i += 1;
    }
    if i == 1{
        println!("{HELP}");
    } else if i == 2{
        if args[1] == String::from("help"){
            println!("{HELP}");
        }
        // else if args[1] == String::from(keyword1) { ... }
        else {
            let path = Path::new(&args[1]);
            match parse_url_file(&path) {
                Some(target) => {
                    if let Err(e) = open_smart(&target, None, None)  {
                        eprintln!("failed to open {target}: {e}");
                    }
                }
                None => eprintln!("no URL= line found in file"),
            }
            assert!(args[1].ends_with(".lnk"));
            if path.exists(){
                let link = lnk::ShellLink::open(path, lnk::encoding::WINDOWS_1252);
                if link.is_err(){
                    let _ = dbg!(link);
                }else{
                    let mut lnk = Lnk::default();
                    let link = link.unwrap();
                    // dbg!(&link);
                    let string_data = link.string_data();
                    lnk.relative_path = string_data.relative_path().clone();
                    lnk.working_dir = string_data.working_dir().clone();
                    lnk.command_line_arguments = string_data.command_line_arguments().clone();
                    match link.link_info(){
                        Some(a) => {
                            match a.local_base_path(){
                                Some(b) => {
                                    lnk.local_base_path = Some(b.to_owned());
                                },
                                None => {}
                            }
                            match a.volume_id(){
                                Some(b) => {
                                    lnk.volume_label = Some(b.volume_label().to_owned());
                                },
                                None => {}
                            }
                        },
                        None => {}
                    }
                    dbg!(&lnk);
                    // Do stuff with the .lnk
                    lnk.absolute_path(args[1].clone());
                    dbg!(&lnk);

                    match lnk.absolute_path{
                        Some(a) => {
                            dbg!(&a);
                            let res = open_smart(&a.to_string(), lnk.command_line_arguments, lnk.working_dir);
                            dbg!(&res);
                        },
                        None => {},
                    }
                }
            }
        }

    }
}
