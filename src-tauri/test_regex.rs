use regex::Regex;
fn main() {
    let re = Regex::new(r"v?(\d+\.\d+\.\d+(?:-[\w.]+)?)").unwrap();
    let output = "2.0.37 (Claude Code)";
    println!("Testing regex on: {}", output);
    if let Some(captures) = re.captures(output) {
        if let Some(version) = captures.get(1) {
            println!("✅ Found version: {}", version.as_str());
        } else {
            println!("❌ No version group found");
        }
    } else {
        println!("❌ No match found");
    }
}
