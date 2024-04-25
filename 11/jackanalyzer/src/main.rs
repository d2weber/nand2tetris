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
    use std::{fs, path::Path};

    use super::*;

    #[test]
    fn seven() {
        check_compile("../Seven");
    }

    #[test]
    fn convert_to_bin() {
        check_compile("../ConvertToBin");
    }

    fn check_compile(s: &str) {
        let path = Path::new(s);
        let cargo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = cargo_root.join(path);

        // Cleanup old vm files
        for file in files_with_extension(&path, "vm") {
            fs::remove_file(file).unwrap();
        }

        // Write vm files
        compilation_engine::compile_path(&path).unwrap();
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
}
