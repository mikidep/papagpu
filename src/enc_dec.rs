use crate::grammar::Prec;

pub fn encode_char(alphabet: &[char], c: char) -> u32 {
    if c == '#' {
        0
    } else {
        alphabet.iter().position(|&x| x == c).unwrap() as u32 + 1
    }
}

pub fn decode_char(alphabet: &[char], c: u32) -> char {
    if c == 0 {
        '#'
    } else {
        alphabet[c as usize - 1]
    }
}

pub fn encode_nt(nt_alphabet: &[char], nt: char, term_thresh: u32) -> u32 {
    term_thresh + nt_alphabet.iter().position(|&x| x == nt).unwrap() as u32
}

pub fn decode_nt(nt_alphabet: &[char], nt: u32, term_thresh: u32) -> char {
    nt_alphabet[(nt - term_thresh) as usize]
}

pub fn encode_string(alphabet: &[char], s: &str) -> Vec<u32> {
    s.chars().map(|c| encode_char(alphabet, c)).collect()
}

pub fn decode_string(alphabet: &[char], s: &[u32]) -> String {
    s.iter().map(|&c| decode_char(alphabet, c)).collect()
}

pub fn encode_mixed_symbol(alphabet: &[char], nt_alphabet: &[char], c: char) -> u32 {
    if alphabet.contains(&c) {
        encode_char(alphabet, c)
    } else {
        encode_nt(nt_alphabet, c, alphabet.len() as u32 + 1)
    }
}

pub fn encode_mixed_string(alphabet: &[char], nt_alphabet: &[char], s: &str) -> Vec<u32> {
    s.chars()
        .map(|c| encode_mixed_symbol(alphabet, nt_alphabet, c))
        .collect()
}

pub fn decode_mixed_symbol(alphabet: &[char], nt_alphabet: &[char], sym: u32) -> char {
    if sym < alphabet.len() as u32 + 1 {
        decode_char(alphabet, sym)
    } else {
        decode_nt(nt_alphabet, sym, alphabet.len() as u32 + 1)
    }
}

pub fn decode_mixed_string(alphabet: &[char], nt_alphabet: &[char], s: &[u32]) -> String {
    s.iter()
        .map(|&sym| {
            decode_mixed_symbol(alphabet, nt_alphabet, sym)
        })
        .collect()
}

pub fn encode_prec_mat<F>(alphabet: &[char], prec_fn: F) -> Vec<u32>
where
    F: Fn(char, char) -> Prec,
{
    let is = 0..=alphabet.len() as u32;
    let js = 0..=alphabet.len() as u32;
    let ijs = is.flat_map(|i| js.clone().map(move |j| (i, j)));
    ijs.map(|(i, j)| prec_fn(decode_char(alphabet, i), decode_char(alphabet, j)).into())
        .collect()
}

// Structure of the encoded array:
// <number of rules>
// <rule LHS> <rule length> <rule RHS ...>
// <rule LHS> <rule length> <rule RHS ...>
// ...
pub fn encode_rules(rules: &Vec<(u32, Vec<u32>)>) -> Vec<u32> {
    let length = rules.len();
    let mut res: Vec<u32> = Vec::new();
    res.push(length as u32);
    for (rule_lhs, rule_rhs) in rules {
        res.push(*rule_lhs);
        res.push(rule_rhs.len() as u32);
        res.extend(rule_rhs);
    }
    res
}

pub fn encode_rules_str(
    alphabet: &[char],
    nt_alphabet: &[char],
    rules: &Vec<(char, &str)>,
) -> Vec<u32> {
    let rules = rules
        .iter()
        .map(|(lhs, rhs)| {
            (
                encode_mixed_symbol(alphabet, nt_alphabet, *lhs),
                encode_mixed_string(alphabet, nt_alphabet, rhs),
            )
        })
        .collect();
    encode_rules(&rules)
}
