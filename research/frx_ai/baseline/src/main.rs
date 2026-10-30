#[allow(dead_code)]
#[path = "../../../../src-tauri/src/fixes/reports.rs"]
mod reports;
use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let value: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let result = reports::analyze(value["text"].as_str().unwrap(), value["target"].as_str().unwrap());
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
