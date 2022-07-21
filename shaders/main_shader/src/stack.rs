#[cfg(not(target_arch = "spirv"))]
use zerocopy::*;

#[repr(C)]
#[derive(Copy, Clone)]
#[cfg_attr(not(target_arch = "spirv"), derive(AsBytes, FromBytes, Debug))]
pub struct StackSym {
    pub sym: u32,
    pub prec: u32,
}

#[cfg(target_arch = "spirv")]
use crate::grammar;

#[cfg(not(target_arch = "spirv"))]
#[path = "grammar.rs"]
mod grammar;

use grammar::Prec;

pub struct Stack<'a> {
    stack: &'a mut [StackSym],
    stack_ptr: usize,
    stack_base: usize,
    term_thresh: u32, // Any symbol greater than or equal to this threshold will be considered a non-terminal
    top_term: usize,
    gives_stack: &'a mut [u32],
    pub gives_nr: usize,
}

impl<'a> Stack<'a> {
    pub fn new(
        stack: &'a mut [StackSym],
        stack_base: usize,
        term_thresh: u32,
        gives_stack: &'a mut [u32],
    ) -> Self {
        Stack {
            stack,
            stack_ptr: stack_base,
            stack_base: stack_base,
            term_thresh,
            top_term: 0,
            gives_stack,
            gives_nr: 0,
        }
    }

    pub fn push(&mut self, sym: StackSym) {
        self.stack[self.stack_ptr] = sym;
        if !self.is_nt(sym.sym) {
            self.top_term = self.stack_ptr
        }
        match Prec::from(sym.prec) {
            Prec::Gives => {
                self.gives_stack[self.stack_base + self.gives_nr] = self.stack_ptr as u32;
                self.gives_nr += 1;
            },
            _ => {}            
        }
        self.stack_ptr += 1;
    }

    pub fn is_nt(&self, sym: u32) -> bool {
        sym >= self.term_thresh
    }

    pub fn handle_head(&self) -> usize {
        let top_gives = self.gives_stack[self.stack_base + self.gives_nr - 1] as usize;
        if self.is_nt(self.stack[top_gives - 1].sym) {
            top_gives - 1
        } else {
            top_gives
        }
    }

    pub fn handle_matches(&self, rules: &[u32], rule_offset: usize, rule_length: usize) -> bool {
        let handle_head = self.handle_head();
        if self.stack_ptr - self.handle_head() != rule_length {
            return false;
        }
        for i in 0..rule_length {
            if self.stack[handle_head + i].sym != rules[rule_offset + i] {
                return false;
            }
        }
        true
    }

    pub fn pop_handle(&mut self) {
        let handle_head = self.handle_head();
        self.stack_ptr = handle_head;
        self.gives_nr -= 1;
        self.top_term = handle_head - 1;
    }

    // Calling this before a terminal symbol is pushed will return an undefined value.
    pub fn peek_top_term(&self) -> u32 {
        self.stack[self.top_term].sym
    }
}
