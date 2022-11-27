use std::ops::{Deref, DerefMut};
use std::fmt;
use aragog::{query::QueryResult, DatabaseRecord, EdgeRecord, Record};
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Document<T>(pub DatabaseRecord<T>);

impl<T> fmt::Debug for Document<T> where T: Serialize + Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(s) = serde_json::to_string(self) {
            f.write_str(&s)
        } else {
            f.debug_struct(<T as Record>::COLLECTION_NAME).finish_non_exhaustive()
        }
    }
}

impl<T> Deref for Document<T> {
    type Target = DatabaseRecord<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Document<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<DatabaseRecord<T>> for Document<T> {
    fn from(r: DatabaseRecord<T>) -> Self {
        Document(r)
    }
}

impl<T> From<Document<T>> for DatabaseRecord<T> {
    fn from(d: Document<T>) -> Self {
        d.0
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Edge<T>(pub DatabaseRecord<EdgeRecord<T>>);

impl<T> Deref for Edge<T> {
    type Target = DatabaseRecord<EdgeRecord<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Edge<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> From<DatabaseRecord<EdgeRecord<T>>> for Edge<T> {
    fn from(r: DatabaseRecord<EdgeRecord<T>>) -> Self {
        Edge(r)
    }
}

impl<T> From<Edge<T>> for DatabaseRecord<EdgeRecord<T>> {
    fn from(d: Edge<T>) -> Self {
        d.0
    }
}

impl<T> fmt::Debug for Edge<T> where T: Serialize + Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(s) = serde_json::to_string(self) {
            f.write_str(&s)
        } else {
            f.debug_struct(<T as Record>::COLLECTION_NAME).finish_non_exhaustive()
        }
    }
}

/// A helper trait for adding the right wrapper to an Aragog type.
pub trait Wrap<W> {
    fn wrap(self) -> W;
}

impl<T> Wrap<Document<T>> for DatabaseRecord<T> {
    fn wrap(self) -> Document<T> {
        self.into()
    }
}

impl<T> Wrap<Edge<T>> for DatabaseRecord<EdgeRecord<T>> {
    fn wrap(self) -> Edge<T> {
        self.into()
    }
}

impl<T> Wrap<Vec<Document<T>>> for QueryResult<T> {
    fn wrap(self) -> Vec<Document<T>> {
        self.0.into_iter().map(|x| x.into()).collect()
    }
}

impl<T> Wrap<Vec<Edge<T>>> for QueryResult<EdgeRecord<T>> {
    fn wrap(self) -> Vec<Edge<T>> {
        self.0.into_iter().map(|x| x.into()).collect()
    }
}

impl<A, B> Wrap<Option<A>> for Option<B> where B: Wrap<A> {
    fn wrap(self) -> Option<A> {
        self.map(|o| o.wrap())
    }
}

impl<A, B, E> Wrap<Result<A, E>> for Result<B, E> where B: Wrap<A> {
    fn wrap(self) -> Result<A, E> {
        self.map(|o| o.wrap())
    }
}

/// A trait for getting the type that a wrapper wraps.
pub trait Wrapper: DerefMut<Target=Self::For> + Into<Self::For> {
    type For: Wrap<Self>;

    fn into_inner(self) -> Self::For;
}

impl<T> Wrapper for Document<T> {
    type For = DatabaseRecord<T>;

    fn into_inner(self) -> Self::For { self.0 }
}

impl<T> Wrapper for Edge<T> {
    type For = DatabaseRecord<EdgeRecord<T>>;

    fn into_inner(self) -> Self::For { self.0 }
}
