use std::vec;

use emu_core::compile_impls::*;
use emu_core::prelude::*;
// use rspirv::binary::Disassemble;

#[allow(dead_code, unused)]
#[path = "../shaders/main_shader/src/stack.rs"]
mod stack;
use stack::StackSym;

mod enc_dec;
use enc_dec::grammar::Prec;
use enc_dec::*;

#[allow(dead_code, unused)]
#[path = "../shaders/main_shader/src/parse_error.rs"]
mod parse_error;
use parse_error::ParseError;

/* #[allow(dead_code, unused)]
#[path = "../shaders/main_shader/src/lib.rs"]
mod main_shader;
 */

pub mod gpu_grammar;

fn print_stack(
    alphabet: &[char],
    nt_alphabet: &[char],
    stack: &[StackSym],
    stack_ptrs: &[u32],
    chunk_size: u32,
) {
    let stacks = stack
        .chunks(chunk_size as usize)
        .enumerate()
        .zip(stack_ptrs)
        .map(|((idx, st), stp)|
            st.split_at(*stp as usize - idx * chunk_size as usize).0);
    for (idx, st) in stacks.enumerate() {
        println!("Stack {idx}:");
        for ssym in st {
            print!(
                "[{}, {}]",
                decode_mixed_symbol(alphabet, nt_alphabet, ssym.sym),
                Prec::from(ssym.prec)
            );
        }
        println!();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    /* let code = include_bytes!(env!("main_shader.spv"));
    let mut loader = rspirv::dr::Loader::new();
    rspirv::binary::parse_bytes(code, &mut loader)?;
    let module = loader.module();

    std::fs::write("main_shader_disass.txt", module.disassemble())?; */

    futures::executor::block_on(assert_device_pool_initialized());

    select(|_, info| {
        if let Some(info) = info {
            info.name().to_ascii_lowercase().contains("amd")
        } else {
            false
        }
    })?;

    dbg!(info()?);
    let n_threads: u32 = 4;
    let alphabet = "()".chars().collect::<Vec<_>>();
    let nt_alphabet = "S".chars().collect::<Vec<_>>();
    let alpha_str = "#()(()(()()))#";
    let prec_mat_vec = encode_prec_mat(&alphabet, |sym_i, sym_j| match (sym_i, sym_j) {
        ('#', '#') => Prec::Equals,
        ('#', _) => Prec::Gives,
        (_, '#') => Prec::Takes,
        ('(', '(') => Prec::Gives,
        ('(', ')') => Prec::Equals,
        (')', '(') => Prec::Takes,
        (')', ')') => Prec::Takes,
        _ => panic!("Undefined precedence for {sym_i:?} and {sym_j:?}"),
    });
    let rules = encode_rules_str(
        &alphabet,
        &nt_alphabet,
        &vec![('S', "()"), ('S', "(S)"), ('S', "S()"), ('S', "S(S)")],
    );
    let length = alpha_str.chars().count();
    let term_thresh = alphabet.len() as u32 + 1;

    /*     main_shader::main(
           spirv_std::glam::uvec3(0, 0, 0),
           &mut encode_string(&alphabet, alpha_str),
           &mut vec![main_shader::stack::StackSym { sym: 0, prec: 0 }; length],
           &mut vec![0; length],
           &prec_mat_vec,
           &rules,
           &(length as u32),
           &8,
           &term_thresh,
           &mut vec![main_shader::parse_error::ParseError::no_error(); n_threads as usize],
       );
    */
    let alpha: DeviceBox<[u32]> = encode_string(&alphabet, alpha_str).as_device_boxed_mut()?;
    let stack: DeviceBox<[StackSym]> =
        vec![StackSym { sym: 0, prec: 0 }; length].as_device_boxed_mut()?;
    let stack_ptr: DeviceBox<[u32]> = vec![0; n_threads as usize].as_device_boxed_mut()?;
    let gives_stack: DeviceBox<[u32]> = vec![0; length].as_device_boxed_mut()?;
    let prec_mat: DeviceBox<[u32]> = prec_mat_vec.as_device_boxed()?;
    let rules_db: DeviceBox<[u32]> = rules.as_device_boxed()?;

    let error: DeviceBox<[ParseError]> =
        vec![ParseError::no_error(); n_threads as usize].as_device_boxed_mut()?;

    /*     let spirv = SpirvBuilder::new()
           .set_entry_point_name("main")
           .add_param_mut::<[u32]>() // alpha
           .add_param_mut::<[StackSym]>() // stack
           .add_param_mut::<[u32]>() // stack_ptr
           .add_param_mut::<[u32]>() // gives_stack
           .add_param::<[u32]>() // prec_mat
           .add_param::<[u32]>() // rules
           .add_param::<u32>() // length
           .add_param::<u32>() // chunk_size
           .add_param::<u32>() // term_thresh
           .add_param_mut::<[ParseError]>() // error
           .set_code_with_u8(std::io::Cursor::new(code))?
           .build();
       let c = compile::<Spirv<_>, SpirvCompile, _, GlobalCache>(spirv)?.finish()?;
    */
    let glsl = Glsl::new()
        .set_entry_point_name("main")
        .add_param_mut::<[u32]>() // alpha
        .add_param_mut::<[StackSym]>() // stack
        .add_param_mut::<[u32]>() // stack_ptr
        .add_param_mut::<[u32]>() // gives_stack
        .add_param::<[u32]>() // prec_mat
        .add_param::<[u32]>() // rules
        .add_param::<u32>() // length
        .add_param::<u32>() // chunk_size
        .add_param::<u32>() // term_thresh
        .add_param_mut::<[ParseError]>() // error
        .set_code_with_glsl(include_str!("main_shader.comp"));
    let c = compile::<Glsl, GlslCompile, _, GlobalCache>(glsl)?.finish()?;

    dbg!("I get here!");

    let chunk_size: u32 = 8;

    // we spawn 128 threads (really 128 thread blocks)
    unsafe {
        spawn(n_threads).launch(call!(
            c,
            &alpha,
            &stack,
            &stack_ptr,
            &gives_stack,
            &prec_mat,
            &rules_db,
            &DeviceBox::new(length as u32)?,
            &DeviceBox::new(chunk_size)?,
            &DeviceBox::new(term_thresh)?,
            &error
        ))?;
    }

    // this is the Future we need to block on to get stuff to happen
    // everything else is non-blocking in the API (except stuff like compilation)
    // println!("{:?}", futures::executor::block_on(stack.get())?);
    let result_stack = futures::executor::block_on(stack.get())?;
    let result_stack_ptr = futures::executor::block_on(stack_ptr.get())?;
    print_stack(
        &alphabet,
        &nt_alphabet,
        &result_stack,
        &result_stack_ptr,
        chunk_size,
    );
    dbg!(futures::executor::block_on(error.get())?);

    Ok(())
}
