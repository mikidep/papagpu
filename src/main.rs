use std::vec;

use emu_core::prelude::*;

mod stack_sym;
use stack_sym::StackSym;

mod enc_dec;
use enc_dec::*;

mod grammar;
use grammar::Prec;

mod parse_error;
use parse_error::ParseError;

pub mod gpu_grammar;
use gpu_grammar::GPUGrammar;

mod par_parse;
use par_parse::par_parse;

fn print_gpu_stacks<'a, I>(
    alphabet: &[char],
    nt_alphabet: &[char],
    stacks: I,
    errors: &[ParseError],
) where
    I: IntoIterator<Item = &'a [StackSym]>
{
    for (idx, (st, err)) in stacks.into_iter().zip(errors).enumerate() {
        println!("Stack {idx}:");
        for ssym in st {
            print!(
                "[{}, {}]",
                decode_mixed_symbol(alphabet, nt_alphabet, ssym.sym),
                Prec::from(ssym.prec)
            );
        }
        println!();
        println!("Error: {}", err);
        println!();
    }
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

    let prec_mat_gpu = encode_prec_mat(&alphabet, |sym_i, sym_j| match (sym_i, sym_j) {
        ('#', '#') => Prec::Equals,
        ('#', _) => Prec::Gives,
        (_, '#') => Prec::Takes,
        ('(', '(') => Prec::Gives,
        ('(', ')') => Prec::Equals,
        (')', '(') => Prec::Takes,
        (')', ')') => Prec::Takes,
        _ => panic!("Undefined precedence for {sym_i:?} and {sym_j:?}"),
    });
    let rules_gpu = encode_rules_str(
        &alphabet,
        &nt_alphabet,
        &vec![('S', "()"), ('S', "(S)"), ('S', "S()"), ('S', "S(S)")],
    );
    let term_thresh = alphabet.len() as u32 + 1;

    let gpu_gramm = GPUGrammar {
        term_thresh,
        prec_mat: &prec_mat_gpu,
        rules: &rules_gpu,
    };

    let alpha = "#()(()(()()))#";
    let alpha_gpu = encode_string(&alphabet, alpha);

    let chunk_size = 8;
    let parse_res = par_parse(&alpha_gpu, gpu_gramm, chunk_size)?;

    print_gpu_stacks(
        &alphabet,
        &nt_alphabet,
        parse_res.stacks.iter().map(|v| v.as_slice()),
        &&parse_res.errors
    );

    Ok(())
}
