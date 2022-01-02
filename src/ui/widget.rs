use crate::shapes::Extent;

pub trait Widget {
    fn id(&self) -> u64;

    fn compute_size(&self, min: Extent, max: Extent) -> Extent;
}

pub struct Button {
    pub id: u64,
    pub min_size: Extent,
    pub max_size: Extent,
}

impl Widget for Button {
    fn id(&self) -> u64 {
        self.id
    }

    /// Minimize height while maximizing width.
    fn compute_size(&self, min: Extent, max: Extent) -> Extent {
        assert!(
            self.max_size >= min,
            "widget's max size is smaller than required space"
        );
        let min_size = Extent::new(
            max.width.min(self.max_size.width),
            min.height.max(self.min_size.height),
        );
        assert!(
            min_size <= max,
            "widget's minimum size is larger than available space"
        );
        min_size
    }
}
