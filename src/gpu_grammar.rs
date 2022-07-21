pub struct GPUGrammar<'a> {
    pub term_thresh: u32,
    pub prec_mat: &'a [u32],
    pub rules: &'a [u32]
}