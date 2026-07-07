use dirs::home_dir;
use std::path::PathBuf;
fn main() {
    let mut path = home_dir().expect("Could not find home directory");
    path.push(".jira-time-reporter.json");
    println!("{:?}", path);
}
