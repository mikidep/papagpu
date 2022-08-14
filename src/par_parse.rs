use std::iter::{Fuse, Peekable};

use crate::grammar::{OPGrammar, Prec};
use crate::par_parse_gpu::GPUParseConfig;
use crate::stack_sym::StackSym;

struct InitialConfigs<AlphaIt: Iterator<Item = u32>> {
    alpha_enc: Peekable<Fuse<AlphaIt>>,
    chunk_size: usize,
    lookbehind: u32,
}

impl<AlphaIt: Iterator<Item = u32>> InitialConfigs<AlphaIt> {
    fn new(alpha_enc: AlphaIt, chunk_size: usize) -> Self {
        assert_ne!(chunk_size, 0, "`chunk_size` must be non-zero.");
        Self {
            alpha_enc: alpha_enc.fuse().peekable(),
            chunk_size,
            lookbehind: 0,
        }
    }
}

impl<AlphaIt: Iterator<Item = u32>> Iterator for InitialConfigs<AlphaIt> {
    type Item = GPUParseConfig;

    fn next(&mut self) -> Option<Self::Item> {
        let mut res = vec![self.lookbehind];
        let chunk = self
            .alpha_enc
            .by_ref()
            .take(self.chunk_size)
            .collect::<Vec<_>>();
        if chunk.len() == 0 {
            None
        } else {
            res.extend_from_slice(&chunk);
            let old_last_sym = self.lookbehind;
            self.lookbehind = *res.last().unwrap();
            res.push(self.alpha_enc.peek().map_or(0, |x| *x));
            let res_len = res.len();
            Some(GPUParseConfig {
                alpha: res,
                stack: vec![StackSym {
                    sym: old_last_sym,
                    prec: Prec::Undef.encode(),
                }],
                head: 1,
                end: res_len as u32 - 1,
            })
        }
    }
}

pub fn encode_initial_configs<'a, TSym, NTSym>(
    alpha: impl IntoIterator<Item = TSym> + 'a,
    grammar: &'a OPGrammar<TSym, NTSym>,
    chunk_size: usize,
) -> impl Iterator<Item = GPUParseConfig> + 'a
where
    TSym: Eq + std::hash::Hash + Clone,
    NTSym: Eq + Clone,
{
    assert_ne!(chunk_size, 0, "`chunk_size` must be non-zero.");

    let alpha_enc = grammar.encode_iterator(alpha);
    InitialConfigs::new(alpha_enc, chunk_size)
}
