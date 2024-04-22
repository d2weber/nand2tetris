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
    use std::process::{Command, Stdio};

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

    out.write_all(b"").expect("Failed to write output file");
}

fn trimmed_lines(s: &str) -> impl Iterator<Item = &str> {
    s.lines()
        .map(|l| strip_comment(l).trim())
        .filter(|l| !l.is_empty())
}

fn strip_comment(s: &str) -> &str {
    if let Some((content, _comment)) = s.split_once("//") {
        content
    } else {
        s
    }
}
