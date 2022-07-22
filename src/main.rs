use std::vec::Vec;

use emu_core::prelude::*;

mod stack_sym;
use stack_sym::StackSym;

mod grammar;
use grammar::{Prec, OPGrammar, MixedSym, MixedSymOrBorder};

mod parse_error;

pub mod gpu_grammar;
use gpu_grammar::GPUGrammar;

mod par_parse;
use par_parse::{par_parse, ParseConfig, ParseResult};

fn print_gpu_results<'a, TSym, NTSym>(
    opg: &OPGrammar<TSym, NTSym>,
    results: &[ParseResult]
) where
    TSym: Eq + std::hash::Hash + Clone + std::fmt::Display,
    NTSym: Eq + Clone + std::fmt::Display,
{
    for (idx, res) in results.iter().enumerate() {
        println!("Stack {idx}:");
        for ssym in res.stack.iter() {
            let sym_fmt = match opg.decode_mixed_symbol(ssym.sym) {
                MixedSymOrBorder::MixedSym(MixedSym::Term(sym)) => format!("{sym}"),
                MixedSymOrBorder::MixedSym(MixedSym::NonTerm(sym)) => format!("{sym}"),
                MixedSymOrBorder::Border => format!("#"),
            };
            print!(
                "[{}, {}]",
                sym_fmt,
                Prec::decode(ssym.prec)
            );
        }
        println!();
        println!("Error: {}", res.error);
        println!();
    }
}

fn parse_rule(
    alphabet: &[char],
    nt_alphabet: &[char],
    rule: &str
) -> Vec<MixedSym<char, char>> {
    rule.chars()
        .map(|c| {
            if alphabet.contains(&c) {
                MixedSym::Term(c)
            } else if nt_alphabet.contains(&c) {
                MixedSym::NonTerm(c)
            } else {
                panic!("Symbol '{c}' is neither a terminal nor a non-terminal!");
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    futures::executor::block_on(assert_device_pool_initialized());

    select(|_, info| {
        if let Some(info) = info {
            info.name().to_ascii_lowercase().contains("nvidia")  // Please select the desired GPU
        } else {
            false
        }
    })?;

    println!("Selected GPU: {}.", info()?.info.map_or("device info not available".to_string(), |i| i.name()));
    println!();

    let alphabet = "()".chars().collect::<Vec<_>>();
    let nt_alphabet = "S".chars().collect::<Vec<_>>();
    let rules = vec![('S', "()"), ('S', "(S)"), ('S', "S()"), ('S', "S(S)")].iter()
        .map(|(lhs, rhs)| (*lhs, parse_rule(&alphabet, &nt_alphabet, *rhs))).collect::<Vec<_>>();

    let opg = OPGrammar::new_with_prec_function(
        alphabet,
        nt_alphabet,
        rules,
        |sym_i, sym_j| match (sym_i, sym_j) {
            ('#', '#') => Prec::Equals,
            ('#', _) => Prec::Gives,
            (_, '#') => Prec::Takes,
            ('(', '(') => Prec::Gives,
            ('(', ')') => Prec::Equals,
            (')', '(') => Prec::Takes,
            (')', ')') => Prec::Takes,
            _ => Prec::Undef,
        }
    );

    let gpu_gramm = GPUGrammar {
        term_thresh: opg.term_thresh,
        prec_mat: &opg.encode_op_matrix(),
        rules: &opg.encode_rules(),
    };

    let alpha = "()(()(()()))";
    let alpha_gpu = opg.encode_string_with_border(&alpha.chars().collect::<Vec<_>>());
    let split = 6;
    let alpha_split_1 = &alpha_gpu[0..split + 1];
    let alpha_split_2 = &alpha_gpu[split..];

    let parse_results = par_parse(&vec![
        ParseConfig {
            alpha: alpha_split_1,
            stack: &vec![StackSym { sym: alpha_split_1[0], prec: Prec::Undef.encode() }],
            head: 1,
            end: alpha_split_1.len() as u32 - 1
        },
        ParseConfig {
            alpha: alpha_split_2,
            stack: &vec![StackSym { sym: alpha_split_2[0], prec: Prec::Undef.encode() }],
            head: 1,
            end: alpha_split_2.len() as u32 - 1
        }
    ], gpu_gramm)?;

    print_gpu_results(
        &opg,
        &parse_results
    );

    Ok(())
}
