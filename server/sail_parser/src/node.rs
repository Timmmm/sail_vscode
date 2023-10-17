use crate::{Span, cst::Id, Spanned};

pub trait Node {
    fn span(&self) -> Span;
    fn child_at_pos(&self, pos: usize) -> Option<&dyn Node>;
}

impl Node for Spanned<Id> {
    fn span(&self) -> Span {
        todo!()
    }

    fn child_at_pos(&self, pos: usize) -> Option<&dyn Node> {
        Some(self)
    }
}
