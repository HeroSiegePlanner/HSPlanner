fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let paths: Vec<String> = if args.is_empty() {
        (1..=5)
            .map(|i| format!("tests/fixtures/tooltips/tooltip{i}.png"))
            .collect()
    } else {
        args
    };
    for path in paths {
        let bytes = std::fs::read(&path).unwrap_or_else(|e| {
            panic!("cannot read {path}: {e}\n(run from the engine/ directory)")
        });
        println!("=== {path} ===");
        match app_lib::ocr::ocr_image_lines(&bytes) {
            Ok(lines) => {
                for line in lines {
                    let [r, g, b] = line.color;
                    println!("#{r:02x}{g:02x}{b:02x}  {}", line.text);
                }
            }
            Err(e) => println!("!! OCR error: {e}"),
        }
        println!();
    }
}
