use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub created: Instant,
    pub kind: ToastKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Error,
}
