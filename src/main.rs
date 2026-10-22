use lnk;

#[derive(Debug, Default)]
pub struct Drive{
    path: String,
    mountpoint: String,
    label: String
}
fn lines_from_bytes(mut data: Vec<u8>) -> Vec<Vec<u8>> {
    let mut lines = Vec::new();

    while let Some(pos) = data[0..].iter().position(|&b| b == b'\n') {
        let end = pos;
        lines.push(data.drain(0..=end).collect());
        // No need for start = end + 1; drain adjusts remaining data
    }

    // Last line
    if !data.is_empty() {
        lines.push(data.drain(..).collect());
    }

    lines
}
pub fn get_drives()->Vec<Drive>{
    let mut drives = Vec::new();

    let mut drives_path = Vec::new();
    let lsblk = std::process::Command::new("lsblk")
        .args(&["-l", "-n", "-o", "PATH"])
        .output()
        .expect("lsblk failed");
    let lines = lines_from_bytes(lsblk.stdout);
    for i in 0..lines.len(){
        let p = &lines[i][0..lines[i].len()-1];
        drives_path.push(String::from_utf8(p.to_vec()).unwrap());
    }

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

    for i in 0..drives_path.len(){
        drives.push(Drive { path: drives_path[i].clone(), mountpoint: drives_mountpoint[i].clone(), label: drives_label[i].clone() })
    }
    drives
}

fn main() {
    println!("Hello, world!");
    let d = get_drives();
    dbg!(d);
}
