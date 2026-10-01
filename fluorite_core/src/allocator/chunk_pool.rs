use crate::allocator::arena::{AllocError, ArenaAllocator};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// A thread-safe pool of `ArenaAllocator`s designed for managing memory chunks
/// during asynchronous world streaming.
///
/// Instead of allocating and deallocating memory on the OS heap continuously
/// (which causes fragmentation and battery drain on Android), this pool
/// allows O(1) acquisition and recycling of fixed-capacity arenas.
pub struct ChunkPoolAllocator {
    chunk_capacity: usize,
    available_chunks: Mutex<VecDeque<Arc<ArenaAllocator>>>,
}

impl ChunkPoolAllocator {
    /// Creates a new `ChunkPoolAllocator` with a specified capacity per chunk,
    /// and pre-allocates `initial_pool_size` chunks.
    pub fn new(chunk_capacity: usize, initial_pool_size: usize) -> Result<Self, AllocError> {
        let mut available = VecDeque::with_capacity(initial_pool_size);
        for _ in 0..initial_pool_size {
            available.push_back(Arc::new(ArenaAllocator::new(chunk_capacity)?));
        }

        Ok(Self {
            chunk_capacity,
            available_chunks: Mutex::new(available),
        })
    }

    /// Acquires an `ArenaAllocator` from the pool.
    /// If the pool is empty, a new `ArenaAllocator` is created on the fly.
    pub fn acquire_chunk(&self) -> Result<Arc<ArenaAllocator>, AllocError> {
        let mut pool = self.available_chunks.lock().unwrap();
        if let Some(chunk) = pool.pop_front() {
            // Ensure the chunk is clean before handing it out
            chunk.reset();
            Ok(chunk)
        } else {
            // Pool empty, allocate a new one to satisfy demand
            Ok(Arc::new(ArenaAllocator::new(self.chunk_capacity)?))
        }
    }

    /// Recycles an `ArenaAllocator` back into the pool for future use.
    pub fn recycle_chunk(&self, chunk: Arc<ArenaAllocator>) {
        // Reset the chunk immediately to clear memory offset
        chunk.reset();
        let mut pool = self.available_chunks.lock().unwrap();
        pool.push_back(chunk);
    }

    /// Returns the number of currently available chunks in the pool.
    pub fn available_count(&self) -> usize {
        self.available_chunks.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_pool_allocation() {
        let pool = ChunkPoolAllocator::new(1024 * 1024, 2).unwrap();

        assert_eq!(pool.available_count(), 2);

        let chunk1 = pool.acquire_chunk().unwrap();
        assert_eq!(pool.available_count(), 1);

        let chunk2 = pool.acquire_chunk().unwrap();
        assert_eq!(pool.available_count(), 0);

        let _chunk3 = pool.acquire_chunk().unwrap();
        assert_eq!(pool.available_count(), 0); // dynamically created

        pool.recycle_chunk(chunk1);
        assert_eq!(pool.available_count(), 1);

        pool.recycle_chunk(chunk2);
        assert_eq!(pool.available_count(), 2);
    }
}
