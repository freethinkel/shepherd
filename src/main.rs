fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, cmd, pane] = args.as_slice() {
        if cmd == "attach" {
            if let Err(err) = shepherd::attach::run(pane) {
                eprintln!("shepherd attach: {err:#}");
                std::process::exit(1);
            }
            return;
        }
    }
}
