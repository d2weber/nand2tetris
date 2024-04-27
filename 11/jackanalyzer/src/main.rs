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
    use std::{fs, io::BufWriter, path::Path};

    use super::*;

    #[test]
    fn seven() {
        snapshot_directory("../Seven");
    }

    #[test]
    fn convert_to_bin() {
        snapshot_directory("../ConvertToBin");
    }

    #[test]
    fn square() {
        snapshot_directory("../Square");
    }

    #[test]
    fn average() {
        snapshot_directory("../Average");
    }

    #[test]
    fn pong() {
        snapshot_directory("../Pong");
    }

    #[test]
    fn complex_array() {
        snapshot_directory("../ComplexArrays");
    }

    #[test]
    fn snake() {
        snapshot_directory("../../9/Snake");
    }

    fn snapshot_directory(s: &str) {
        let path = Path::new(s);
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);

        for file in files_with_extension(&path, "jack") {
            let snapshot_name = file.strip_prefix(cargo_root).unwrap().display().to_string();
            snapshot_compiled(&snapshot_name, &file);
        }
    }

    fn snapshot_compiled(snapshot_name: &str, file_name: &Path) {
        let mut out = BufWriter::new(Vec::new());
        compilation_engine::compile_file(file_name, &mut out);
        let vm_code = String::from_utf8(out.into_inner().unwrap()).unwrap();
        insta::assert_snapshot!(snapshot_name, vm_code);
    }

    fn files_with_extension<'a>(
        path: &'a Path,
        extension: &'a str,
    ) -> impl Iterator<Item = PathBuf> + 'a {
        fs::read_dir(path).unwrap().filter_map(move |dir_entry| {
            let file = dir_entry.unwrap().path();
            file.extension()
                .is_some_and(|e| e == extension)
                .then_some(file)
        })
    }
}
