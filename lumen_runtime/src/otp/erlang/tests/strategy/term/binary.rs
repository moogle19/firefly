use std::sync::Arc;

use proptest::arbitrary::any;
use proptest::prop_oneof;
use proptest::strategy::{BoxedStrategy, Just, Strategy};

use crate::otp::erlang::tests::strategy::size_range;
use crate::otp::erlang::tests::strategy::term::binary::sub::{
    bit_count, bit_offset, byte_count, byte_offset,
};
use crate::process::Process;
use crate::term::Term;

pub mod heap;
pub mod sub;

pub fn containing_bytes(byte_vec: Vec<u8>, arc_process: Arc<Process>) -> BoxedStrategy<Term> {
    prop_oneof![
        Just(Term::slice_to_binary(&byte_vec, &arc_process)),
        sub::containing_bytes(byte_vec, arc_process.clone())
    ]
    .boxed()
}

pub fn heap(arc_process: Arc<Process>) -> BoxedStrategy<Term> {
    heap::with_size_range(size_range(), arc_process)
}

pub fn is_utf8(arc_process: Arc<Process>) -> BoxedStrategy<Term> {
    any::<String>()
        .prop_flat_map(move |string| {
            containing_bytes(string.as_bytes().to_owned(), arc_process.clone())
        })
        .boxed()
}

pub fn sub(arc_process: Arc<Process>) -> BoxedStrategy<Term> {
    sub::with_size_range(
        byte_offset(),
        bit_offset(),
        byte_count(),
        bit_count(),
        arc_process,
    )
}