//! Typestate 状态机模块

mod chunked_array;
mod page_manager;
mod session;

pub use chunked_array::{ChunkedArray, SharedChunkedArray};
pub use page_manager::{
    PageManager, PageManagerConfig, PageManagerError, SharedPageManager, Viewport,
};
pub use session::{
    FileMetadata, IndexEntry, Indexed, IndexedState, Mapped, MappedState, Session, SessionError,
    Unmapped, UnmappedState,
};
