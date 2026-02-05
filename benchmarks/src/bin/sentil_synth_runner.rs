use sentil_benchmarks::synthesis;

fn main() {
    for record in synthesis::run() {
        match serde_json::to_string(&record) {
            Ok(line) => println!("{line}"),
            Err(err) => {
                eprintln!("failed to serialize a record: {err}");
                std::process::exit(1);
            }
        }
    }
}