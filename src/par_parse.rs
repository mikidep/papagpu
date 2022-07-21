use emu_core::compile_impls::*;
use emu_core::prelude::*;

use crate::stack_sym::StackSym;
use crate::parse_error::ParseError;

const N_THREADS: usize = 4;

use crate::gpu_grammar::GPUGrammar;

pub struct ParParseResult {
    pub stacks: Vec<Vec<StackSym>>,
    pub errors: Vec<ParseError>,
}

pub fn par_parse(alpha: &[u32], gpu_grammar: GPUGrammar, chunk_size: usize) -> Result<ParParseResult, Box<dyn std::error::Error>>  {  
    let length = alpha.len();

    let alpha_db: DeviceBox<[u32]> = alpha.as_device_boxed_mut()?;
    let stack_db: DeviceBox<[StackSym]> =
        vec![StackSym { sym: 0, prec: 0 }; length].as_device_boxed_mut()?;
    let stack_ptr_db: DeviceBox<[u32]> = vec![0; N_THREADS].as_device_boxed_mut()?;
    let gives_stack_db: DeviceBox<[u32]> = vec![0; length].as_device_boxed_mut()?;
    let prec_mat_db: DeviceBox<[u32]> = gpu_grammar.prec_mat.as_device_boxed()?;
    let rules_db: DeviceBox<[u32]> = gpu_grammar.rules.as_device_boxed()?;

    let error: DeviceBox<[ParseError]> =
        vec![ParseError::no_error(); N_THREADS].as_device_boxed_mut()?;

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
        .set_code_with_glsl(include_str!("../shaders/par_parse.comp"));
    let c = compile::<Glsl, GlslCompile, _, GlobalCache>(glsl)?.finish()?;

    unsafe {
        spawn(N_THREADS as u32).launch(call!(
            c,
            &alpha_db,
            &stack_db,
            &stack_ptr_db,
            &gives_stack_db,
            &prec_mat_db,
            &rules_db,
            &DeviceBox::new(length as u32)?,
            &DeviceBox::new(chunk_size as u32)?,
            &DeviceBox::new(gpu_grammar.term_thresh)?,
            &error
        ))?;
    }

    let result_stack = futures::executor::block_on(stack_db.get())?;
    let result_stack_ptr = futures::executor::block_on(stack_ptr_db.get())?;
    let result_error = futures::executor::block_on(error.get())?;

    let stacks = result_stack
        .chunks(chunk_size as usize)
        .enumerate()
        .zip(result_stack_ptr.iter())
        .map(|((idx, st), stp)|
            st.split_at(*stp as usize - idx * chunk_size as usize).0)
        .map(|sl| Vec::from(sl))
        .collect::<Vec<_>>();

    let errors = Vec::from(result_error.split_at(stacks.len()).0);

    Ok(ParParseResult {
        stacks,
        errors
    })
}