use std::{
    collections::{hash_map::Entry, HashMap},
    env, fs,
    path::Path,
};

fn main() {
    let mut args = env::args();
    if args.len() != 2 {
        panic!("Expect single parameter to `*.asm` file.");
    }
    let filename = args.next_back().unwrap();
    let result = assemble(Path::new(&filename));

    // I prefer to print to stdout, users can easily pipe to a file
    print!("{result}");
}

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

    #[test]
    fn max() {
        test_assemble("../max/Max.asm");
    }

    #[test]
    fn pong() {
        test_assemble("../pong/Pong.asm");
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

fn assemble(asm_file: &Path) -> String {
    let asm_file = fs::read_to_string(asm_file)
        .unwrap_or_else(|_| panic!("Couldn't read {}.", asm_file.display()));

    let mut symbols = first_pass(&asm_file);
    let mut last_symbol_address = 15;
    let mut result = trimmed_lines(&asm_file)
        .filter(|l| !l.starts_with('('))
        .map(|l| {
            if let Some(a_expr) = l.strip_prefix('@') {
                let value = if is_symbol(a_expr) {
                    *symbols.entry(a_expr).or_insert_with(|| {
                        last_symbol_address += 1;
                        last_symbol_address
                    })
                } else {
                    a_expr
                        .parse::<i32>()
                        .unwrap_or_else(|_| panic!("Couldn't parse A-instruction `{l}`"))
                };
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

#[must_use]
fn first_pass(asm_file: &str) -> HashMap<&str, i32> {
    let mut symbols = HashMap::from([
        ("R0", 0),
        ("R1", 1),
        ("R2", 2),
        ("R3", 3),
        ("R4", 4),
        ("R5", 5),
        ("R6", 6),
        ("R7", 7),
        ("R8", 8),
        ("R9", 9),
        ("R10", 10),
        ("R11", 11),
        ("R12", 12),
        ("R13", 13),
        ("R14", 14),
        ("R15", 15),
        ("SP", 0),
        ("LCL", 1),
        ("ARG", 2),
        ("THIS", 3),
        ("THAT", 4),
        ("SCREEN", 16384),
        ("KBD", 24576),
        ("LOOP", 4),
        ("STOP", 18),
        ("i", 16),
        ("sum", 17),
    ]);
    let mut byte_offset = 0;
    trimmed_lines(asm_file).for_each(|l| {
        if let Some(sym_name) = l.strip_prefix('(') {
            let sym_name = sym_name
                .strip_suffix(')')
                .expect("Missing closing bracket for symbol.");
            symbols.entry(sym_name).or_insert(byte_offset);
        } else {
            byte_offset += 1;
        }
    });
    symbols
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

fn is_symbol(s: &str) -> bool {
    !s.chars()
        .next()
        .expect("Identifier expected")
        .is_ascii_digit()
}
