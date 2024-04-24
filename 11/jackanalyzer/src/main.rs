mod compilation_engine;
mod token;

use std::{env, path::PathBuf};

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
    compilation_engine::compile_path(path.as_path()).unwrap();
}

#[cfg(test)]
mod test {
    use std::{fs, io::BufWriter, io::Write, path::Path, process::Command, str::from_utf8};

    use self::token::TokenStream;

    use super::*;

    #[test]
    fn array_test() {
        check_folder("../ArrayTest");
    }

    #[test]
    fn expressionless_square() {
        check_folder("../ExpressionLessSquare");
    }

    #[test]
    fn square() {
        check_folder("../Square");
    }

    fn check_folder(s: &str) {
        let path = Path::new(s);
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);

        // Cleanup old xml files
        for file in files_with_extension(&path, "xml") {
            fs::remove_file(file).unwrap();
        }

        // Write token xml files
        for file in files_with_extension(&path, "jack") {
            write_token_xml(&file);
        }

        // Write normal xml files
        compilation_engine::compile_path(&path).unwrap();

        for file in files_with_extension(&path.join("reference"), "xml") {
            check_file(&file)
        }
    }

    fn files_with_extension<'a>(
        path: &'a Path,
        extension: &'a str,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        fs::read_dir(&path).unwrap().filter_map(move |dir_entry| {
            let file = dir_entry.unwrap().path();
            file.extension()
                .is_some_and(|e| e == extension)
                .then_some(file)
        })
    }

    fn write_token_xml(jack_file: &Path) -> PathBuf {
        let mut out_filename = jack_file.file_stem().unwrap().to_os_string();
        out_filename.push("T.xml");
        let out_filename = jack_file.with_file_name(out_filename);
        {
            let mut out = BufWriter::new(fs::File::create(out_filename.clone()).unwrap());
            let filtered =
                compilation_engine::filter_comments(&fs::read_to_string(&jack_file).unwrap());
            writeln!(out, "<tokens>").unwrap();
            TokenStream::new(&filtered).for_each(|t| t.write_xml(&mut out));
            writeln!(out, "</tokens>").unwrap();
        }
        out_filename
    }

    /// Checks that reference is equal to file in parent folder
    fn check_file(reference: &Path) {
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let reference = cargo_root.join(reference);
        let path = reference
            .ancestors()
            .nth(2)
            .unwrap()
            .join(reference.file_name().unwrap());
        let output = Command::new("bash")
            .arg("../../../tools/TextComparer.sh")
            .arg(path)
            .arg(&reference)
            .current_dir(cargo_root)
            .output()
            .expect("Failed to run TextComparer");
        assert!(
            output.status.success(),
            "TextComparer failed for reference {}: {}{}",
            reference.display(),
            from_utf8(&output.stderr).unwrap(),
            from_utf8(&output.stdout).unwrap()
        );
    }
}
