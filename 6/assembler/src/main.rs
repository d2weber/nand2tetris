use std::{env, fs, path::Path};

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn add() {
        test_assemble("../add/Add.asm");
    }

    #[test]
    fn max_l() {
        test_assemble("../max/MaxL.asm");
    }

    #[test]
    fn pong_l() {
        test_assemble("../pong/PongL.asm");
    }

    /// Takes the path to an `*.asm*` file relative to the MANIFEST_DIR and
    /// tests the generated assembly against a `*.hack` file next to it.
    fn test_assemble(asm_file: &str) {
        let asm_file = {
            let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            p.push(asm_file);
            p
        };
        assert_eq!(asm_file.extension().unwrap(), "asm");
        let expected = fs::read_to_string(asm_file.with_extension("hack"))
            .expect("Could not read reference file.");

        let result = assemble(asm_file.as_path());
        assert_eq!(result, expected);
    }
}

fn strip_comment(s: &str) -> &str {
    if let Some((content, _comment)) = s.split_once("//") {
        content
    } else {
        s
    }
}

fn assemble(asm_file: &Path) -> String {
    let mut result = fs::read_to_string(asm_file)
        .expect(&format!("Couldn't read {}.", asm_file.display()))
        .lines()
        .map(|l| strip_comment(l).trim())
        .filter(|l| !l.is_empty())
        .map(|l| {
            if let Some(a_expr) = l.strip_prefix("@") {
                let value = a_expr
                    .parse::<i16>()
                    .expect(&format!("Couldn't parse A-instruction `{l}`"));
                format!("0{value:015b}")
            } else {
                // C-instruction

                let (dest, l) = if let Some((dest, rest)) = l.split_once('=') {
                    let dest = match dest {
                        "M" => "001",
                        "D" => "010",
                        "DM" | "MD" => "011", // MD is used in PongL.asm
                        "A" => "100",
                        "AM" => "101",
                        "AD" => "110",
                        "ADM" => "111",
                        _ => panic! {"Invalid dest: `{dest}`"},
                    };
                    (dest, rest)
                } else {
                    ("000", l)
                };
                let (comp, jump) = if let Some((comp, jump)) = l.split_once(';') {
                    let jump = match jump {
                        "JGT" => "001",
                        "JEQ" => "010",
                        "JGE" => "011",
                        "JLT" => "100",
                        "JNE" => "101",
                        "JLE" => "110",
                        "JMP" => "111",
                        _ => panic! {"Invalid jump: `{jump}`"},
                    };
                    (comp, jump)
                } else {
                    (l, "000")
                };
                let comp = match comp {
                    "0" => "0101010",
                    "1" => "0111111",
                    "-1" => "0111010",
                    "D" => "0001100",
                    "A" => "0110000",
                    "!D" => "0001101",
                    "!A" => "0110001",
                    "-D" => "0001111",
                    "-A" => "0110011",
                    "D+1" => "0011111",
                    "A+1" => "0110111",
                    "D-1" => "0001110",
                    "A-1" => "0110010",
                    "D+A" => "0000010",
                    "D-A" => "0010011",
                    "A-D" => "0000111",
                    "D&A" => "0000000",
                    "D|A" => "0010101",
                    "M" => "1110000",
                    "!M" => "1110001",
                    "-M" => "1110011",
                    "M+1" => "1110111",
                    "M-1" => "1110010",
                    "D+M" => "1000010",
                    "D-M" => "1010011",
                    "M-D" => "1000111",
                    "D&M" => "1000000",
                    "D|M" => "1010101",
                    _ => panic! {"Invalid comp: `{comp}`"},
                };

                format! {"111{comp}{dest}{jump}"}
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
    result.push('\n');
    result
}

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        panic!("Expect single parameter to `*.asm` file.");
    }
    let filename = args.next_back().unwrap();
    let result = assemble(&Path::new(&filename));
    print!("{result}");
}
