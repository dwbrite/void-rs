use ir_parser;
use std::fs::File;
use std::path::Path;

fn main() {
    detect_changes(Path::new("../dialogue-src"));

    // languages
    let path = Path::new("../dialogue-src/en");
    let in_file = File::open("../dialogue-src/en/intro.xml").unwrap();
    let out_path = Path::new("../dialogue-src/en/ir");
    ir_parser::compile_ir(path, in_file, out_path);
}

fn detect_changes(path: &Path) {
    for dir in std::fs::read_dir(path).unwrap() {
        let p = dir.unwrap().path();

        if p.is_dir() {
            detect_changes(&p);
        } else {
            print!("cargo:rerun-if-changed={}", p.clone().to_str().unwrap());
        }
    }

    // let walk = WalkDir::new(path);
    //
    // for p in walk.into_iter().filter_map(Result::ok) {
    //     if p.file_type().is_dir() {
    //         detect_changes(p.path());
    //     } else {
    //         print!("cargo:rerun-if-changed={}", path.clone().to_str().unwrap());
    //         // println!("{}", path.clone().to_str().unwrap());
    //     }
    // }
}
