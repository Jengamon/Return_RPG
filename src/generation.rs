use std::collections::VecDeque;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GenerationalIndex {
    index: usize,
    generation: u64,
}

impl GenerationalIndex {
    pub fn index(&self) -> usize { self.index }
    pub fn generation(&self) -> u64 { self.generation }
}

#[derive(Debug)]
struct AllocatorEntry {
    is_live: bool,
    generation: u64
}

#[derive(Debug)]
pub struct GenerationalIndexAllocator {
    entries: Vec<AllocatorEntry>,
    free: VecDeque<usize>,
}

impl GenerationalIndexAllocator {
    pub fn new() -> GenerationalIndexAllocator {
        GenerationalIndexAllocator {
            free: VecDeque::new(),
            entries: vec![]
        }
    }

    pub fn allocate(&mut self) -> GenerationalIndex {
        // Check the free array for any free indicies
        if self.free.len() > 0 {
            let index = self.free.pop_front().unwrap(); // Safe to do this
            assert!(!self.entries[index].is_live);
            self.entries[index].generation += 1;
            self.entries[index].is_live = true;
            GenerationalIndex {
                index,
                generation: self.entries[index].generation
            }
        } else {
            // Create a new index and use it
            let index = self.entries.len();
            self.entries.push(AllocatorEntry {
                is_live: true,
                generation: 0
            });
            GenerationalIndex {
                index,
                generation: 0
            }
        }
    }

    pub fn deallocate(&mut self, index: GenerationalIndex) -> bool {
        let is_live = self.is_live(index);
        if is_live {
            // We can deallocate the index
            self.entries[index.index].is_live = false;
            self.free.push_back(index.index);
        }
        is_live
    }

    pub fn is_live(&self, index: GenerationalIndex) -> bool {
        let entry = self.entries.get(index.index);
        if let Some(entry) = entry {
            entry.generation == index.generation && entry.is_live
        } else {
            false
        }
    }
}

#[derive(Debug)]
struct ArrayEntry<T> {
    value: T,
    generation: u64,
}

#[derive(Debug)]
pub struct GenerationalIndexArray<T>(Vec<Option<ArrayEntry<T>>>);

impl<T> GenerationalIndexArray<T> {
    pub fn new() -> GenerationalIndexArray<T> {
        GenerationalIndexArray(vec![])
    }

    // Sets the value at index, overwriting previous generation values
    pub fn set(&mut self, index: GenerationalIndex, value: T) {
        let entry = self.0.get_mut(index.index);
        if let Some(Some(entry)) = entry {
            if index.generation >= entry.generation {
                entry.value = value;
            }
        } else {
            // Create enough stuff for this index to exist
            while self.0.len() <= index.index {
                self.0.push(None)
            }
            self.0[index.index] = Some(ArrayEntry{
                value,
                generation: index.generation
            });
        }
    }

    pub fn get(&self, index: GenerationalIndex) -> Option<&T> {
        let item = self.0.get(index.index);
        if let Some(Some(item)) = item {
            if index.generation == item.generation {
                Some(&item.value)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: GenerationalIndex) -> Option<&mut T> {
        let item = self.0.get_mut(index.index);
        if let Some(Some(item)) = item {
            if index.generation == item.generation {
                Some(&mut item.value)
            } else {
                None
            }
        } else {
            None
        }
    }
}