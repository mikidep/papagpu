use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Copy)]
pub enum Prec {
    Takes,
    Equals,
    Gives,
    Undef,
}

impl Prec {
    pub fn encode(&self) -> u32 {
        match self {
            Prec::Gives => 1,
            Prec::Equals => 2,
            Prec::Takes => 3,
            Prec::Undef => 0,
        }
    }

    pub fn decode(x: u32) -> Self {
        match x {
            1 => Prec::Gives,
            2 => Prec::Equals,
            3 => Prec::Takes,
            _ => Prec::Undef,
        }
    }
}

impl std::fmt::Display for Prec {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Prec::Gives => write!(f, "⋖"),
            Prec::Equals => write!(f, "≐"),
            Prec::Takes => write!(f, "⋗"),
            Prec::Undef => write!(f, "⊥"),
        }
    }
}

pub enum MixedSym<TSym, NTSym> {
    Term(TSym),
    NonTerm(NTSym),
}

pub enum MixedSymOrBorder<TSym, NTSym> {
    MixedSym(MixedSym<TSym, NTSym>),
    Border,
}

pub struct OPGrammar<TSym, NTSym> {
    term_alphabet: Vec<TSym>,
    nonterm_alphabet: Vec<NTSym>,
    rules: Vec<(NTSym, Vec<MixedSym<TSym, NTSym>>)>,
    op_matrix: HashMap<(TSym, TSym), Prec>,
    pub term_thresh: u32,
}

impl<TSym, NTSym> OPGrammar<TSym, NTSym>
where
    TSym: Eq + Hash + Clone,
    NTSym: Eq + Clone,
{
    pub fn new(
        term_alphabet: Vec<TSym>,
        nonterm_alphabet: Vec<NTSym>,
        rules: Vec<(NTSym, Vec<MixedSym<TSym, NTSym>>)>,
        op_matrix: HashMap<(TSym, TSym), Prec>,
    ) -> Self {
        let term_thresh = term_alphabet.len() as u32 + 1;
        Self {
            term_alphabet,
            nonterm_alphabet,
            rules,
            op_matrix,
            term_thresh,
        }
    }

    pub fn new_with_prec_function(
        term_alphabet: Vec<TSym>,
        nonterm_alphabet: Vec<NTSym>,
        rules: Vec<(NTSym, Vec<MixedSym<TSym, NTSym>>)>,
        mut prec_func: impl FnMut(&TSym, &TSym) -> Prec,
    ) -> Self {
        let mut op_matrix = HashMap::new();
        for sym_i in term_alphabet.iter() {
            for sym_j in term_alphabet.iter() {
                op_matrix.insert((sym_i.clone(), sym_j.clone()), prec_func(sym_i, sym_j));
            }
        }
        Self::new(term_alphabet, nonterm_alphabet, rules, op_matrix)
    }

    fn encode_term(&self, sym: impl Borrow<TSym>) -> u32 {
        self.term_alphabet
            .iter()
            .position(|s| s == sym.borrow())
            .unwrap() as u32
            + 1
    }

    fn encode_nonterm(&self, sym: impl Borrow<NTSym>) -> u32 {
        self.nonterm_alphabet
            .iter()
            .position(|s| s == sym.borrow())
            .unwrap() as u32
            + self.term_thresh
    }

    fn encode_rule_rhs(&self, rule_rhs: &Vec<MixedSym<TSym, NTSym>>) -> Vec<u32> {
        rule_rhs
            .iter()
            .map(|sym| match sym {
                MixedSym::Term(sym) => self.encode_term(sym),
                MixedSym::NonTerm(sym) => self.encode_nonterm(sym),
            })
            .collect()
    }

    // Structure of the encoded array:
    // <number of rules>
    // <rule LHS> <rule length> <rule RHS ...>
    // <rule LHS> <rule length> <rule RHS ...>
    // ...
    pub fn encode_rules(&self) -> Vec<u32> {
        let length = self.rules.len();
        let mut res: Vec<u32> = Vec::new();
        res.push(length as u32);
        for (rule_lhs, rule_rhs) in self.rules.iter() {
            res.push(self.encode_nonterm(rule_lhs));
            res.push(rule_rhs.len() as u32);
            res.extend(self.encode_rule_rhs(rule_rhs));
        }
        res
    }

    pub fn encode_op_matrix(&self) -> Vec<u32> {
        let mut res: Vec<u32> = Vec::new();
        res.push(Prec::Equals.encode());
        for _ in 0..self.term_alphabet.len() {
            res.push(Prec::Gives.encode());
        }
        for sym_i in self.term_alphabet.iter() {
            res.push(Prec::Takes.encode());
            for sym_j in self.term_alphabet.iter() {
                res.push(
                    self.op_matrix
                        .get(&(sym_i.clone(), sym_j.clone()))
                        .map(|pref| *pref)
                        .unwrap_or(Prec::Undef)
                        .encode(),
                );
            }
        }
        res
    }

    pub fn encode_iterator_with_border<'a, B>(
        &'a self,
        s: impl IntoIterator<Item = B> + 'a,
    ) -> impl Iterator<Item = u32> + 'a
    where
        B: Borrow<TSym>,
    {
        std::iter::once(0)
            .chain(s.into_iter().map(|sym| self.encode_term(sym)))
            .chain(std::iter::once(0))
    }

    pub fn encode_string_with_border<'a>(
        &'a self,
        s: impl IntoIterator<Item = &'a TSym> + 'a,
    ) -> Vec<u32> {
        self.encode_iterator_with_border(s).collect()
    }

    pub fn encode_mixed_string<'a>(
        &'a self,
        s: impl IntoIterator<Item = &'a MixedSym<TSym, NTSym>>,
    ) -> Vec<u32> {
        s.into_iter()
            .map(|sym| match sym {
                MixedSym::Term(sym) => self.encode_term(sym),
                MixedSym::NonTerm(sym) => self.encode_nonterm(sym),
            })
            .collect()
    }

    pub fn decode_mixed_symbol(&self, sym: u32) -> MixedSymOrBorder<TSym, NTSym> {
        if sym == 0 {
            MixedSymOrBorder::Border
        } else if sym < self.term_thresh {
            MixedSymOrBorder::MixedSym(MixedSym::Term(self.term_alphabet[sym as usize - 1].clone()))
        } else {
            MixedSymOrBorder::MixedSym(MixedSym::NonTerm(
                self.nonterm_alphabet[(sym - self.term_thresh) as usize].clone(),
            ))
        }
    }

    pub fn decode_mixed_string(&self, s: &[u32]) -> Vec<MixedSymOrBorder<TSym, NTSym>> {
        s.iter().map(|sym| self.decode_mixed_symbol(*sym)).collect()
    }
}
