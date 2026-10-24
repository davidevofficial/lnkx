use lnk;
use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::env;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;

const HELP: &str =
"
Normal Usage:
lnkx /path/to/file.lnk : Tries to open the resource pointed at by the file

CLI Usage:
lnkx create /path/to/file.lnk .\\relative\\windows\\path\\program.exe \"[arg1] [...] [argn]\" : Creates a new .lnk file with relative path
lnkx create /path/to/file.lnk X:\\absolute\\windows\\path\\program.exe volume_label \"[arg1] [...] [argn]\": Creates a new .lnk file with absolute path. Use lsblk to get volume_label
lnkx create /path/to/file.url https://example.com : Creates a new .url file
lnkx help : Shows this message
lnkx (no arg) : Shows this message";

/// Regular file, has an exec bit, and is an ELF binary or a #! script.
fn is_native_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
        return false;
    }
    let mut magic = [0u8; 4];
    let Ok(mut f) = std::fs::File::open(path) else { return false };
    if f.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == *b"\x7fELF" || magic[..2] == *b"#!"
}

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
    let is_native = !is_exe && is_native_executable(path);

    let mut cmd;
    if is_exe || is_native {
        let abs = std::path::absolute(path)?;
        let dir: PathBuf = match working_dir.as_deref().filter(|d| !d.trim().is_empty()) {
            Some(d) => PathBuf::from(d),
            None => abs.parent().map(Path::to_path_buf).unwrap_or_else(|| ".".into()),
        };
        let extra_args: Vec<String> = match command_line_arguments.as_deref() {
            Some(s) if !s.trim().is_empty() => split_args(s).ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidInput, "unbalanced quotes in arguments")
            })?,
            _ => Vec::new(),
        };

        cmd = if is_exe {
            let mut c = Command::new("wine");
            c.arg(&abs);
            c
        } else {
            Command::new(&abs) // absolute path, so no PATH lookup
        };
        cmd.args(extra_args).current_dir(dir);
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

/// Encode as NUL-terminated ANSI (Windows-1252, same codepage lnkx reads with).
fn to_ansi_z(s: &str) -> Result<Vec<u8>, String> {
    let (bytes, _, had_errors) = lnk::encoding::WINDOWS_1252.encode(s);
    if had_errors {
        return Err(format!("{s:?} has characters that don't exist in Windows-1252"));
    }
    let mut v = bytes.into_owned();
    v.push(0);
    Ok(v)
}

fn build_link_info(label: &str, local_base_path: &str, serial: u32) -> Result<Vec<u8>, String> {
    const HEADER_SIZE: u32 = 0x1C;
    let label_z = to_ansi_z(label)?;
    let base_z = to_ansi_z(local_base_path)?;
    let mut vol = Vec::new();
    vol.extend_from_slice(&(0x10 + label_z.len() as u32).to_le_bytes());
    vol.extend_from_slice(&3u32.to_le_bytes());
    vol.extend_from_slice(&serial.to_le_bytes());
    vol.extend_from_slice(&0x10u32.to_le_bytes());
    vol.extend_from_slice(&label_z);
    let volume_id_offset = HEADER_SIZE;
    let local_base_offset = volume_id_offset + vol.len() as u32;
    let suffix_offset = local_base_offset + base_z.len() as u32;
    let total = suffix_offset + 1;
    let mut li = Vec::with_capacity(total as usize);
    li.extend_from_slice(&total.to_le_bytes());
    li.extend_from_slice(&HEADER_SIZE.to_le_bytes());
    li.extend_from_slice(&1u32.to_le_bytes());
    li.extend_from_slice(&volume_id_offset.to_le_bytes());
    li.extend_from_slice(&local_base_offset.to_le_bytes());
    li.extend_from_slice(&0u32.to_le_bytes());
    li.extend_from_slice(&suffix_offset.to_le_bytes());
    li.extend_from_slice(&vol);
    li.extend_from_slice(&base_z);
    li.push(0);
    Ok(li)
}

fn add_link_info(lnk_path: &Path, link_info: &[u8]) -> io::Result<()> {
    let bytes = std::fs::read(lnk_path)?;
    if bytes.len() < 0x4C {
        return Err(io::Error::new(ErrorKind::InvalidData, "file shorter than a lnk header"));
    }
    let flags = u32::from_le_bytes(bytes[0x14..0x18].try_into().unwrap());
    if flags & 0x3 != 0 {
        return Err(io::Error::new(ErrorKind::InvalidData,
            "file already has an ID list or LinkInfo; refusing to splice"));
    }
    let mut out = Vec::with_capacity(bytes.len() + link_info.len());
    out.extend_from_slice(&bytes[..0x4C]);
    out.extend_from_slice(link_info);
    out.extend_from_slice(&bytes[0x4C..]);
    out[0x14..0x18].copy_from_slice(&(flags | 0x2).to_le_bytes());
    std::fs::write(lnk_path, out)
}

fn create_lnk_absolute(out: &Path, win_path: &str, label: &str, arguments: Option<&str>) -> Result<(), String> {
    let win_path = win_path.replace('/', "\\");
    let b = win_path.as_bytes();
    if !(b.len() > 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\') {
        return Err(format!("expected a path like X:\\dir\\program.exe, got {win_path}"));
    }
    if label.trim().is_empty() {
        return Err("volume label must not be empty".into());
    }
    let link_info = build_link_info(label, &win_path, 0)?;
    let (dir, _file) = win_path.rsplit_once('\\').unwrap();
    let dir = if dir.len() == 2 { format!("{dir}\\") } else { dir.to_string() };
    let mut sl = lnk::ShellLink::default();
    sl.set_working_dir(Some(dir));
    if let Some(a) = arguments.filter(|a| !a.trim().is_empty()) {
        sl.set_arguments(Some(a.to_string()));
    }
    sl.save(out).map_err(|e| format!("{e:?}"))?;

    if let Err(e) = add_link_info(out, &link_info) {
        let _ = std::fs::remove_file(out);
        return Err(e.to_string());
    }
    Ok(())
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
    } else if i >= 2{
        if args[1] == String::from("help"){
            println!("{HELP}");
        }
        else if args[1] == String::from("create") {
            println!("Creating...");

            if i < 4{
                println!("Lacks some arguments, see help below ----\\/\n");

                println!("{HELP}");
                return;
            }
            let path = Path::new(&args[2]);
            if path.extension().is_none(){
                println!("Path lacks extension (use either .url or .lnk");
                return;
            }
            if path.extension().unwrap() == "lnk"{
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                let pointing = &args[3];
                // we check if the second char is a ':' because absolute paths' first chars are A: / B: / ... / Z:
                if pointing[1..2] == *":"{
                    if i < 5{
                        println!("There is a lack of arguments (such as volume label missing), see help below ----\\/\n");

                        println!("{HELP}");
                        return;
                    }
                    match create_lnk_absolute(path, pointing, &args[4], args.get(5).map(String::as_str)) {
                        Ok(()) => println!("Created {}", path.display()),
                        Err(e) => eprintln!("create failed: {e}"),
                    }
                    return;
                }
                let mut new_lnk = lnk::ShellLink::default();
                new_lnk.set_relative_path(Some(pointing.clone()));
                if i == 5{
                    if let Some(a) = (Some(&*args[4])).filter(|a| !a.trim().is_empty()) {
                        new_lnk.set_arguments(Some(a.to_string()));
                    }
                }
                let res = new_lnk.save(path);
                dbg!(&res);
            }
            if path.extension().unwrap() == "url"{
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                let contents = format!(
                    "[InternetShortcut]\nURL={}",
                    &args[3]
                );
                let res = std::fs::write(path, contents);
                dbg!(&res);

            }

        }
        else {
            let path = Path::new(&args[1]);
            if path.extension().unwrap() == "url" {
                println!(".url found");
                match parse_url_file(&path) {
                    Some(target) => {
                        if let Err(e) = open_smart(&target, None, None)  {
                            eprintln!("failed to open {target}: {e}");
                        }
                    }
                    None => eprintln!("no URL= line found in file"),
                }
            } if path.extension().unwrap() == "lnk"{
                println!(".lnk found");
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
}
