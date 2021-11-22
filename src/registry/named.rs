use std::{
    any::Any,
    collections::hash_map::{DefaultHasher, Entry, HashMap},
    hash::{Hash, Hasher},
};

use super::slot;
use super::indexed::{Ops, Error as IndexedError};
pub use super::{
    indexed::{self, Type},
    slot::Id,
};

#[derive(thiserror::Error, Debug)]
pub enum Error<'a> {
    #[error("The provided ID is invalid, the item having either been deleted or never existed.")]
    InvalidId,
    #[error("Too many objects have been allocated. No Ids are available.")]
    TooManyObjects,
    #[error("The provided item name does not refer to an item in the registry.")]
    NameNotFound(&'a str),
    #[error("The name already exists, and a new value cannot be inserted")]
    NameAlreadyExists(&'a str),
    #[error("The type of the stored item ({actual:?}) is not hte same as the expected type ({expected:?})")]
    TypeMismatch { expected: Type, actual: Type },
}

impl From<IndexedError> for Error<'static> {
    fn from(err: IndexedError) -> Self {
        match err {
            IndexedError::TooManyObjects => Self::TooManyObjects,
            IndexedError::InvalidId => Self::InvalidId,
            IndexedError::TypeMismatch { expected, actual } => {
                Self::TypeMismatch { expected, actual }
            }
        }
    }
}

pub struct Registry {
    indexed: indexed::Registry,
    map: HashMap<u64, slot::Id>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            indexed: indexed::Registry::new(),
            map: HashMap::new(),
        }
    }

    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        let id = self.map.remove(&hash_name(name)).ok_or(Error::InvalidId)?;
        self.indexed.remove(id)?;
        Ok(())
    }

    pub fn remove_id(&mut self, id: Id) -> Result<(), Error> {
        self.indexed.remove(id).map_err(|e| Error::from(e))
    }
}

pub trait StrOps<T> {
    fn set(&mut self, name: &str, value: T) -> Result<Id, Error>;
    fn insert<'a>(&mut self, name: &'a str, value: T) -> Result<Id, Error<'a>>;

    fn get<'a>(&self, name: &'a str) -> Result<&T, Error<'a>>;
    fn get_mut<'a>(&mut self, name: &'a str) -> Result<&mut T, Error<'a>>;
}

pub trait IdOps<T> {
    fn get_id(&self, id: Id) -> Result<&T, Error>;
    fn get_mut_id(&mut self, id: Id) -> Result<&mut T, Error>;
}

macro_rules! impl_ops {
    ($($t:ty),+) => {
        $(
            impl StrOps<$t> for Registry {
                fn set(&mut self, name: &str, value: $t) -> Result<Id, Error> {
                    let id = self.indexed.insert(value)?.get();
                    if let Some(old_id) = self.map.insert(hash_name(name), id) {
                        let _ = self.indexed.remove(old_id);
                    }
                    Ok(id)
                }

                fn insert<'a>(&mut self, name: &'a str, value: $t) -> Result<Id, Error<'a>> {
                    if let Entry::Vacant(entry) = self.map.entry(hash_name(name)) {
                        Ok(*entry.insert(self.indexed.insert(value)?.get()))
                    } else {
                        Err(Error::NameAlreadyExists(name))
                    }
                }

                fn get<'a>(&self, name: &'a str) -> Result<&$t, Error<'a>> {
                    let id = *self.map.get(&hash_name(name)).ok_or(Error::NameNotFound(name))?;
                    Ok(self.indexed.get(id)?)
                }

                fn get_mut<'a>(&mut self, name: &'a str) -> Result<&mut $t, Error<'a>> {
                    let id = *self.map.get(&hash_name(name)).ok_or(Error::NameNotFound(name))?;
                    Ok(self.indexed.get_mut(id)?)
                }
            }

            impl IdOps<$t> for Registry {
                fn get_id(&self, id: Id) -> Result<&$t, Error> {
                    self.indexed.get(id).map_err(|e| Error::from(e))
                }

                fn get_mut_id(&mut self, id: Id) -> Result<&mut $t, Error> {
                    self.indexed.get_mut(id).map_err(|e| Error::from(e))
                }
            }
        )+
    };
}

impl_ops!(
    u128,
    i128,
    Box<dyn Any>,
    &'static str,
    u64,
    i64,
    f64,
    u32,
    i32,
    f32,
    char
);

fn hash_name(name: &str) -> u64 {
    let mut state = DefaultHasher::new();
    name.hash(&mut state);
    state.finish()
}
