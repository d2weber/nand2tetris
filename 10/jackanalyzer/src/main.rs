use std::{
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

fn main() {
    let mut args = env::args();
    let path = if args.len() == 1 {
        // Default to current directory
        env::current_dir().unwrap()
    } else if args.len() == 2 {
        let filename = args.next_back().unwrap();
        PathBuf::from(&filename)
    } else {
        panic!("Zero parameters or one parameter expected.");
    };
    compile_path(&path.as_path()).unwrap();
}

#[cfg(test)]
mod test {
    use std::process::Command;

    use super::*;

    #[test]
    fn square_main() {
        let path = Path::new("../Square/Main.jack");
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);
        compile_path(&path).unwrap();
        check_file(&path.with_extension("xml"));
    }

    /// Checks if file is equal to reference in subfolder `reference`
    fn check_file(path: &Path) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);
        let reference = path
            .parent()
            .unwrap()
            .join("reference")
            .join(path.file_name().unwrap());
        assert!(
            Command::new("bash")
                .arg("../../../tools/TextComparer.sh")
                .arg(path)
                .arg(reference)
                .current_dir(cargo_root)
                // .stdout(Stdio::null())
                .status()
                .expect("Failed to run TextComparer")
                .success(),
            "Bad status when running TextComparer"
        );
    }
}

fn compile_path(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        let mut out = BufWriter::new(File::create(path.with_extension("xml"))?);
        compile_file(path, &mut out);
        Ok(())
    } else if path.is_dir() {
        let name = path
            .file_name()
            .expect("Already checked that it's a directory");
        let out_file = path.join(name).with_extension("xml");
        let mut out = BufWriter::new(File::create(out_file)?);
        // TODO: Error when no jack file is found
        for dir_entry in fs::read_dir(path)? {
            let file = dir_entry?.path();
            if file.extension().is_some_and(|e| e == "jack") {
                compile_file(&file, &mut out)
            }
        }
        Ok(())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

fn compile_file(jack_file: &Path, out: &mut impl Write) {
    let module_id = jack_file
        .file_stem()
        .unwrap_or_else(|| panic!("Expected *.jack file, got `{}`", jack_file.display()))
        .to_str()
        .expect("Filename has to be unicode.");
    let jack_file = fs::read_to_string(jack_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", jack_file.display()));

    let filtered = filter_comments(&jack_file);
    out.write_all(filtered.as_bytes())
        .expect("Failed to write output file");
}

fn filter_comments(s: &str) -> String {
    // Remove block comments
    let mut rest = s;
    let mut block_filtered = String::new();
    while let Some((first, comment_and_rest)) = rest.split_once("/*") {
        block_filtered += first;
        let (_comment, new_rest) = comment_and_rest
            .split_once("*/")
            .expect("Missing closing block comment");
        rest = new_rest;
    }
    block_filtered += rest;

    // Remove line comments
    let mut rest = block_filtered.as_str();
    let mut filtered = String::new();
    while let Some((first, comment_and_rest)) = rest.split_once("//") {
        filtered += first;
        filtered.push('\n'); // reinsert newline
        rest = if let Some((_comment, new_rest)) = comment_and_rest.split_once('\n') {
            new_rest
        } else {
            "" // Last line in string
        }
    }
    filtered + rest
}
