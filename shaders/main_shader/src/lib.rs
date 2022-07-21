#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
#![deny(warnings)]
#![feature(asm_experimental_arch)]

use spirv_std::glam::UVec3;
// use spirv_std::macros::debug_printfln;
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

pub mod grammar;
pub mod parse_error;
pub mod stack;

use grammar::{Prec, PrecMatrix};
use parse_error::ParseError;
use stack::{Stack, StackSym};

fn min(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

fn advance_head(head: &mut usize, error: &mut [ParseError], thread_idx: usize) {
    *head += 1;
    error[thread_idx] = ParseError::at_location(*head as u32);
}

fn reduce_handle(stack: &mut Stack, rules: &[u32]) -> bool {
    let mut offset = 0usize;
    let rules_nr = rules[offset];
    offset += 1;
    for _ in 0..rules_nr {
        let rule_lhs = rules[offset];
        offset += 1;
        let rule_length = rules[offset];
        offset += 1;
        if stack.handle_matches(rules, offset, rule_length as usize) {
            stack.pop_handle();
            stack.push(StackSym {
                sym: rule_lhs,
                prec: Prec::Undef.into(),
            });
            return true;
        } else {
            offset += rule_length as usize;
        }
    }
    false
}

#[spirv(compute(threads(4)))]
pub fn main(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] alpha: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] stack: &mut [StackSym],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] gives_stack: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] prec_mat: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] rules: &[u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 5)] length: &u32,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 6)] chunk_size: &u32,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 7)] term_thresh: &u32,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 8)] error: &mut [ParseError],
) {
    let thread_idx = id.x as usize;

/*     unsafe {
        debug_printfln!("%u", thread_idx as u32);
    }
 */
    let mut head = thread_idx * *chunk_size as usize;
    error[thread_idx] = ParseError::at_location(head as u32);
    let prec_matrix = PrecMatrix::new(&prec_mat, *term_thresh);
    if head < *length as usize {
        let mut stack = Stack::new(stack, head, *term_thresh, gives_stack);
        let end = min(head + *chunk_size as usize, *length as usize) - 1;
        stack.push(StackSym {
            sym: alpha[head],
            prec: Prec::Undef.into(),
        });
        advance_head(&mut head, error, thread_idx);
        while head < end {
            if stack.is_nt(alpha[head]) {
                stack.push(StackSym {
                    sym: alpha[head],
                    prec: Prec::Undef.into(),
                });
                advance_head(&mut head, error, thread_idx);
            } else {
                let top_term = stack.peek_top_term();
                let prec = prec_matrix.get(top_term, alpha[head]);
                match prec {
                    Prec::Gives | Prec::Equals => {
                        stack.push(StackSym {
                            sym: alpha[head],
                            prec: prec.into(),
                        });
                        advance_head(&mut head, error, thread_idx);
                    }
                    Prec::Takes => {
                        if stack.gives_nr == 0 {
                            stack.push(StackSym {
                                sym: alpha[head],
                                prec: prec.into(),
                            });
                            advance_head(&mut head, error, thread_idx);
                        } else if !reduce_handle(&mut stack, &rules) {
                            return; // Error
                        }
                    }
                    Prec::Undef => {
                        return; // Error
                    }
                }
            }
        }
    }
    error[thread_idx] = ParseError::no_error();
}
