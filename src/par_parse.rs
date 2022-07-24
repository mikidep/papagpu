use emu_core::compile_impls::*;
use emu_core::prelude::*;

use crate::parse_error::ParseError;
use crate::stack_sym::StackSym;

use crate::gpu_grammar::GPUGrammar;

pub struct ParseConfig {
    pub alpha: Vec<u32>,
    pub stack: Vec<StackSym>,
    pub head: u32,
    pub end: u32,
}

pub struct ParseResult {
    pub stack: Vec<StackSym>,
    pub error: ParseError,
}

pub fn par_parse(
    configs: impl IntoIterator<Item = ParseConfig>,
    gpu_grammar: GPUGrammar,
) -> Result<Vec<ParseResult>, Box<dyn std::error::Error>> {
    let (joined_alphas, heads, ends, n_threads, stack, stack_base, stack_ptr) = {
        let mut joined_alphas: Vec<u32> = Vec::new();
        let mut heads = Vec::new();
        let mut ends = Vec::new();
        let mut alpha_offset = 0usize;
        let mut n_threads = 0usize;

        let mut stack = Vec::new();
        let mut stack_base = Vec::new();
        let mut stack_ptr = Vec::new();
        let mut stack_offset = 0usize;

        for conf in configs.into_iter() {
            let alpha_len = conf.alpha.len();
            let stack_len = conf.stack.len();

            joined_alphas.extend(conf.alpha);
            heads.push(alpha_offset as u32 + conf.head);
            ends.push(alpha_offset as u32 + conf.end);
            alpha_offset += alpha_len;
            n_threads += 1;

            stack.extend(conf.stack);
            stack.extend(std::iter::repeat(StackSym { sym: 0, prec: 0 }).take(alpha_len));
            stack_base.push(stack_offset as u32);
            stack_offset += stack_len;
            stack_ptr.push(stack_offset as u32);
            stack_offset += alpha_len;
        }
        (
            joined_alphas,
            heads,
            ends,
            n_threads,
            stack,
            stack_base,
            stack_ptr,
        )
    };

    let alpha_db: DeviceBox<[u32]> = joined_alphas.as_device_boxed_mut()?;
    let heads_db: DeviceBox<[u32]> = heads.as_device_boxed()?;
    let ends_db: DeviceBox<[u32]> = ends.as_device_boxed()?;

    let stack_db: DeviceBox<[StackSym]> = stack.as_device_boxed_mut()?;
    let stack_base_db: DeviceBox<[u32]> = stack_base.as_device_boxed()?;
    let stack_ptr_db: DeviceBox<[u32]> = stack_ptr.as_device_boxed_mut()?;
    let gives_stack_db: DeviceBox<[u32]> = vec![0; stack.len()].as_device_boxed_mut()?;

    let prec_mat_db: DeviceBox<[u32]> = gpu_grammar.prec_mat.as_device_boxed()?;
    let rules_db: DeviceBox<[u32]> = gpu_grammar.rules.as_device_boxed()?;

    let errors_db: DeviceBox<[ParseError]> =
        vec![ParseError::no_error(); n_threads].as_device_boxed_mut()?;

    let glsl = Glsl::new()
        .set_entry_point_name("main")
        .add_param_mut::<[u32]>() // alpha
        .add_param::<[u32]>() // heads
        .add_param::<[u32]>() // ends
        .add_param_mut::<[StackSym]>() // stack
        .add_param::<[u32]>() // stack_base
        .add_param_mut::<[u32]>() // stack_ptr
        .add_param_mut::<[u32]>() // gives_stack
        .add_param::<[u32]>() // prec_mat
        .add_param::<[u32]>() // rules
        .add_param::<u32>() // term_thresh
        .add_param_mut::<[ParseError]>() // error
        .set_code_with_glsl(include_str!("../shaders/par_parse.comp"));
    let c = compile::<Glsl, GlslCompile, _, GlobalCache>(glsl)?.finish()?;

    unsafe {
        spawn(n_threads as u32).launch(call!(
            c,
            &alpha_db,
            &heads_db,
            &ends_db,
            &stack_db,
            &stack_base_db,
            &stack_ptr_db,
            &gives_stack_db,
            &prec_mat_db,
            &rules_db,
            &DeviceBox::new(gpu_grammar.term_thresh)?,
            &errors_db
        ))?;
    }

    let result_stack = futures::executor::block_on(stack_db.get())?;
    let result_stack_ptr = futures::executor::block_on(stack_ptr_db.get())?;
    let result_errors = futures::executor::block_on(errors_db.get())?;

    let results = stack_base
        .iter()
        .zip(result_stack_ptr.iter())
        .zip(result_errors.iter())
        .map(|((base, ptr), err)| ParseResult {
            stack: result_stack[*base as usize..*ptr as usize].to_vec(),
            error: *err,
        })
        .collect();

    Ok(results)
}
