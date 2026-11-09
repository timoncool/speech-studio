//! Live's benchmark from the command line, the same code as the `live_bench` tool: a run of `latency`, `floor` or `asr`
//! written as a JSON file in the studio's `bench` folder.
//!   cargo run -p speech-server --example live-bench -- <studio folder> <latency|floor|asr> [rounds]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(root), Some(kind)) = (args.first(), args.get(1)) else {
        eprintln!("[ERROR] usage: live-bench <studio folder> <latency|floor|asr> [rounds]");
        std::process::exit(2);
    };
    let rounds = args.get(2).and_then(|r| r.parse().ok()).unwrap_or(0);
    match speech_server::live_bench(std::path::Path::new(root), kind, rounds) {
        Ok(run) => println!("{}", serde_json::to_string_pretty(&run).unwrap_or_default()),
        Err(e) => {
            eprintln!("[ERROR] {e}");
            std::process::exit(1);
        }
    }
}
