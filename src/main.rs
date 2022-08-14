use std::vec::Vec;

use emu_core::prelude::*;

mod stack_sym;

mod grammar;
use grammar::{MixedSym, MixedSymOrBorder, OPGrammar, Prec};

mod gpu_parse_error;

pub mod gpu_grammar;
use gpu_grammar::GPUGrammar;

mod par_parse_gpu;
use par_parse_gpu::{par_parse_gpu, GPUParseResult};

mod par_parse;
use par_parse::encode_initial_configs;

fn print_gpu_results<'a, TSym, NTSym>(opg: &OPGrammar<TSym, NTSym>, results: &[GPUParseResult])
where
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
            print!("[{}, {}]", sym_fmt, Prec::decode(ssym.prec));
        }
        println!();
        println!("Error: {}", res.error);
        println!("Bot gives: {}", res.bot_gives);
        println!();
    }
}

fn parse_rule(alphabet: &[char], nt_alphabet: &[char], rule: &str) -> Vec<MixedSym<char, char>> {
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
            info.name().to_ascii_lowercase().contains("nvidia") // Please select the desired GPU
        } else {
            false
        }
    })?;

    println!(
        "Selected GPU: {}.",
        info()?
            .info
            .map_or("device info not available".to_string(), |i| i.name())
    );
    println!();

    let alphabet = "()".chars().collect::<Vec<_>>();
    let nt_alphabet = "S".chars().collect::<Vec<_>>();
    let rules = vec![('S', "()"), ('S', "(S)"), ('S', "S()"), ('S', "S(S)")]
        .iter()
        .map(|(lhs, rhs)| (*lhs, parse_rule(&alphabet, &nt_alphabet, *rhs)))
        .collect::<Vec<_>>();

    let opg =
        OPGrammar::new_with_prec_function(alphabet, nt_alphabet, rules, |sym_i, sym_j| {
            match (sym_i, sym_j) {
                ('#', '#') => Prec::Equals,
                ('#', _) => Prec::Gives,
                (_, '#') => Prec::Takes,
                ('(', '(') => Prec::Gives,
                ('(', ')') => Prec::Equals,
                (')', '(') => Prec::Takes,
                (')', ')') => Prec::Takes,
                _ => Prec::Undef,
            }
        });

    let gpu_gramm = GPUGrammar {
        term_thresh: opg.term_thresh,
        prec_mat: &opg.encode_op_matrix(),
        rules: &opg.encode_rules(),
    };

    let alpha_file = std::fs::File::open("alpha.txt")?;
    let mut alpha_reader = utf8_read::Reader::new(std::io::BufReader::new(alpha_file));
    // let alpha = "()(()(()()))";
    let chars = alpha_reader.map(|c| c.unwrap());
    let chunk_size = 4;

    let parse_results = par_parse_gpu(
        encode_initial_configs(chars, &opg, chunk_size).inspect(|conf| {
            println!(
                "Config string: \"{}\"",
                opg.decode_mixed_string(&conf.alpha).iter().map(|msb| match msb {
                    MixedSymOrBorder::MixedSym(ms) => match ms {
                        MixedSym::Term(c) => *c,
                        MixedSym::NonTerm(c) => *c,
                    },
                    MixedSymOrBorder::Border => '#',
                }).collect::<String>()
            )
        }),
        gpu_gramm,
    )?;

    print_gpu_results(&opg, &parse_results);

    Ok(())
}
