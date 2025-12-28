use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Id<T>(u32, PhantomData<T>);

impl<T> Copy for Id<T> {}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Id<T> {
    fn new(index: u32) -> Self {
        Self(index, PhantomData)
    }

    pub fn new_raw(index: u32) -> Self {
        Self(index, PhantomData)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone)]
pub struct Arena<T> {
    items: Vec<T>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn alloc(&mut self, item: T) -> Id<T> {
        let id = Id::new(self.items.len() as u32);
        self.items.push(item);
        id
    }

    pub fn get(&self, id: Id<T>) -> &T {
        &self.items[id.index()]
    }

    pub fn get_mut(&mut self, id: Id<T>) -> &mut T {
        &mut self.items[id.index()]
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| (Id::new(i as u32), item))
    }
}

#[derive(Debug, Clone)]
pub struct Interner<T: Eq + Hash + Clone> {
    arena: Arena<T>,
    index: HashMap<T, Id<T>>,
}

impl<T: Eq + Hash + Clone> Default for Interner<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + Hash + Clone> Interner<T> {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            index: HashMap::new(),
        }
    }

    pub fn intern(&mut self, item: T) -> Id<T> {
        if let Some(&id) = self.index.get(&item) {
            id
        } else {
            let id = self.arena.alloc(item.clone());
            self.index.insert(item, id);
            id
        }
    }

    pub fn get(&self, id: Id<T>) -> &T {
        self.arena.get(id)
    }
}

