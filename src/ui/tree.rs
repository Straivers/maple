#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Could not add a new layout node to the tree.")]
    TooManyNodes,
    #[error("Attempted to add more than u16::MAX children to a node in the tree.")]
    TooManyChildren,
    #[error("A node can only have one parent.")]
    MoreThanOneParent,
}

#[derive(Clone, Copy)]
pub struct Node<Payload>
where
    Payload: Clone,
{
    payload: Payload,
    first_child: Index,
    next: Index,
}

impl<Payload> Node<Payload>
where
    Payload: Clone,
{
    fn new(payload: Payload) -> Self {
        Self {
            payload,
            first_child: Index::null(),
            next: Index::null(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Index(u16);

impl Default for Index {
    fn default() -> Self {
        Self::null()
    }
}

impl Index {
    const MAX: Index = Index(u16::MAX - 1);

    pub fn null() -> Self {
        Index(u16::MAX)
    }

    pub fn is_null(self) -> bool {
        self == Self::null()
    }
}

pub struct Tree<Payload>
where
    Payload: Clone,
{
    /// Data stored per-node in the tree.
    data: Vec<Payload>,

    /// Indices (1 per data element) pointing to a slice in `children_array` or
    /// `0` if the node does not have any children.
    children: Vec<u16>,

    /// Single vector of slices of indices into 'data'. The first index pointed
    /// to from `children` contains the length of the slice, followed by the
    /// slice's content.
    ///
    /// We store the length because we either have to store a null character or
    /// a length, and since they're the same size, length was the easy option.
    /// This limits us to 65535 children, but since the tree can only hold 65534
    /// nodes, this isn't a problem.
    children_array: Vec<u16>,
}

impl<Payload> Tree<Payload>
where
    Payload: Clone,
{
    pub fn new() -> Self {
        Self {
            data: vec![],
            children: vec![],
            children_array: vec![0],
        }
    }

    pub fn get(&self, node: Index) -> &Payload {
        &self.data[node.0 as usize]
    }

    pub fn get_mut(&mut self, node: Index) -> &mut Payload {
        &mut self.data[node.0 as usize]
    }

    pub fn children(&self, node: Index) -> &[Index] {
        if self.children[node.0 as usize] == 0 {
            &[]
        } else {
            let ptr = self.children[node.0 as usize] as usize;
            let len = self.children_array[ptr] as usize;
            let start: *const _ = &self.children_array[ptr + 1];

            unsafe { std::slice::from_raw_parts(start.cast(), len) }
        }
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.children.clear();
        self.children_array.truncate(1);
    }

    pub fn reserve(&mut self, additional: usize) {
        let actual = additional.min(Index::MAX.0 as usize - self.data.len());
        self.data.reserve(actual);
        self.children.reserve(actual);
    }

    pub fn add(&mut self, payload: Payload, children: &[Index]) -> Result<Index, Error> {
        if self.data.len() > Index::MAX.0 as usize {
            return Err(Error::TooManyNodes);
        }

        let first_child = if children.is_empty() {
            0
        } else {
            let i = self.children_array.len();

            // Reserve the requisite amount of space to store header + indices.
            self.children_array.reserve(children.len() + 1);
            // Push slice header with length of slice.
            self.children_array.push(
                children
                    .len()
                    .try_into()
                    .map_err(|_| Error::TooManyChildren)?,
            );
            // Extend array with indices of children.
            self.children_array.extend_from_slice(unsafe {
                std::slice::from_raw_parts(children.as_ptr().cast(), children.len())
            });

            i.try_into().map_err(|_| Error::TooManyNodes)?
        };

        let index = Index(
            self.data
                .len()
                .try_into()
                .map_err(|_| Error::TooManyNodes)?,
        );
        self.data.push(payload);
        self.children.push(first_child);
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization() {
        let tree = Tree::<u32>::new();
        // There are no nodes in the tree.
        assert_eq!(tree.data.len(), 0);
        assert_eq!(tree.children.len(), 0);
        // children_array[0] is reserved so that we can safely use `try_into()`
        // to convert from usize to u16. Using `Index` would have required extra
        // checks against `Index::MAX`, which is effort we don't need to do.
        //
        // Either approach means at each node can have at most 65535 children (1
        // sentinel value).
        assert_eq!(tree.children_array.len(), 1);
    }

    #[test]
    fn simple() -> Result<(), Error> {
        let mut tree = Tree::new();

        /*
        Tree:
        0
            1
                2
            3
            4
        */

        let four = tree.add(4, &[])?;
        let three = tree.add(3, &[])?;
        let two = tree.add(2, &[])?;
        let one = tree.add(1, &[two])?;
        let zero = tree.add(0, &[one, three, four])?;

        assert_eq!(*tree.get(zero), 0);
        assert_eq!(*tree.get(one), 1);
        assert_eq!(*tree.get(two), 2);
        assert_eq!(*tree.get(three), 3);
        assert_eq!(*tree.get(four), 4);

        assert_eq!(*tree.children(zero), [one, three, four]);
        assert_eq!(*tree.children(one), [two]);
        assert_eq!(*tree.children(two), []);
        assert_eq!(*tree.children(three), []);
        assert_eq!(*tree.children(two), []);

        Ok(())
    }

    #[test]
    fn capacity() -> Result<(), Error> {
        let mut tree = Tree::new();
        tree.reserve(Index::MAX.0 as usize);

        for i in 0..Index::MAX.0 + 1 {
            tree.add(i, &[])?;
        }

        let fail = tree.add(Index::MAX.0, &[]);
        assert_eq!(fail, Err(Error::TooManyNodes));
        Ok(())
    }
}
