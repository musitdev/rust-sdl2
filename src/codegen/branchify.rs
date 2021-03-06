#[macro_escape];

use std::io::buffered::BufferedWriter;
use std::io::{File, Writer};
use std::str::CharIterator;

struct ParseBranch {
    matches: ~[u8],
    result: Option<~str>,
    children: ~[ParseBranch],
}

impl ParseBranch {
    fn new() -> ParseBranch {
        ParseBranch {
            matches: ~[],
            result: None,
            children: ~[],
        }
    }
}

pub fn branchify(options: &[(&str, &str)], case_sensitive: bool) -> ~[ParseBranch] {
    let mut root = ParseBranch::new();

    fn go_down_moses(branch: &mut ParseBranch, mut chariter: CharIterator, result: &str, case_sensitive: bool) {
        match chariter.next() {
            Some(c) => {
                let first_case = if case_sensitive { c as u8 } else { c.to_ascii().to_upper().to_byte() };
                for next_branch in branch.children.mut_iter() {
                    if next_branch.matches[0] == first_case {
                        go_down_moses(next_branch, chariter, result, case_sensitive);
                        return;
                    }
                }
                let mut subbranch = ParseBranch::new();
                subbranch.matches.push(first_case);
                if !case_sensitive {
                    let second_case = c.to_ascii().to_lower().to_byte();
                    if first_case != second_case {
                        subbranch.matches.push(second_case);
                    }
                }
                branch.children.push(subbranch);
                go_down_moses(&mut branch.children[branch.children.len() - 1], chariter, result, case_sensitive);
            },
            None => {
                assert!(branch.result.is_none());
                branch.result = Some(result.to_owned());
            },
        }
    };

    for &(key, result) in options.iter() {
        go_down_moses(&mut root, key.chars(), result, case_sensitive);
    }

    root.children
}

macro_rules! branchify(
    (case sensitive, $($key:expr => $value:ident),*) => (
        ::branchify::branchify([$(($key, stringify!($value))),*], true)
    );
    (case insensitive, $($key:expr => $value:ident),*) => (
        branchify([$(($key, stringify!($value))),*], false)
    );
)

/// Prints the contents to stdout.
///
/// :param branches: the branches to search through
/// :param indent: the level of indentation (each level representing four leading spaces)
/// :param read_call: the function call to read a byte
/// :param end: the byte which marks the end of the sequence
/// :param max_len: the maximum length a value may be before giving up and returning ``None``
/// :param valid: the function call to if a byte ``b`` is valid
/// :param unknown: the expression to call for an unknown value; in this string, ``{}`` will be
///         replaced with an expression (literal or non-literal) evaluating to a ``~str`` (it is
///         ``{}`` only, not arbitrary format strings)
pub fn generate_branchified_method(
        writer: &mut BufferedWriter<File>,
        branches: &[ParseBranch],
        indent: uint,
        read_call: &str,
        end: &str,
        max_len: &str,
        valid: &str,
        unknown: &str) {

    // Write Formatted
    macro_rules! wf(($($x:tt)*) => ({
        let indentstr = " ".repeat(indent * 4);
        let s = format!($($x)*);
        writer.write(indentstr.as_bytes());
        writer.write(s.as_bytes());
        writer.write(bytes!("\n"));
    }))

    fn r(writer: &mut BufferedWriter<File>, branch: &ParseBranch, prefix: &str, indent: uint, read_call: &str,
            end: &str, max_len: &str, valid: &str, unknown: &str) {
        for &c in branch.matches.iter() {
            let next_prefix = format!("{}{}", prefix, c as char);
            wf!("Some(b) if b == '{}' as u8 => match {} \\{", c as char, read_call);
            for b in branch.children.iter() {
                r(writer, b, next_prefix, indent + 1, read_call, end, max_len, valid, unknown);
            }
            match branch.result {
                Some(ref result) => wf!("    Some(b) if b == SP => return Some({}),", *result),
                None => wf!("    Some(b) if b == SP => return Some({}),",
                                unknown.replace("{}", format!("~\"{}\"", next_prefix))),
            }
            wf!("    Some(b) if {} => (\"{}\", b),", valid, next_prefix);
            wf!("    _ => return None,");
            wf!("\\},");
        }
    }
    wf!("let (s, next_byte) = match {} \\{", read_call);
    for b in branches.iter() {
        r(writer, b, "", indent + 1, read_call, end, max_len, valid, unknown);
    }
    wf!("    Some(b) if {} => (\"\", b),", valid);
    wf!("    _ => return None,");
    wf!("\\};");
    wf!("// OK, that didn't pan out. Let's read the rest and see what we get.");
    wf!("let mut s = s.to_owned();");
    wf!("s.push_char(next_byte as char);");
    wf!("loop \\{");
    wf!("    match {} \\{", read_call);
    wf!("        Some(b) if b == {} => return Some({}),", end, unknown.replace("{}", "s"));
    wf!("        Some(b) if {} => \\{", valid);
    wf!("            if s.len() == {} \\{", max_len);
    wf!("                // Too long; bad request");
    wf!("                return None;");
    wf!("            \\}");
    wf!("            s.push_char(b as char);");
    wf!("        \\},");
    wf!("        _ => return None,");
    wf!("    \\}");
    wf!("\\}");
}
