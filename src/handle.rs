#[derive(Debug)]
pub enum Error {
    NullHandle,
    IndexOutOfRange,
    GenerationOutOfRange,
}

#[derive(Default, Clone, Copy)]
pub struct Handle32<const MIN_COUNT: u32> {
    value: u32,
}

impl<const MIN_COUNT: u32> std::fmt::Debug for Handle32<MIN_COUNT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .field("index", &self.index())
            .field("generation", &self.generation())
            .finish()
    }
}

impl<const MIN_COUNT: u32> Handle32<MIN_COUNT> {
    const INDEX_BITS: u32 = u32::BITS - MIN_COUNT.leading_zeros();

    const INDEX_MAX: u32 = (1 << Self::INDEX_BITS) - 1;
    const GENERATION_MAX: u32 = !Self::INDEX_MAX;

    pub fn new(value: usize, generation: usize) -> Result<Self, Error> {
        if value > Self::INDEX_MAX as usize {
            return Err(Error::IndexOutOfRange);
        }

        if generation > Self::GENERATION_MAX as usize {
            return Err(Error::GenerationOutOfRange);
        }

        let i = value as u32;
        let g = generation as u32;

        Ok(Self {
            value: i | (g << Self::INDEX_BITS),
        })
    }

    pub fn new_index(index: usize) -> Result<Self, Error> {
        if index > Self::INDEX_MAX as usize {
            return Err(Error::IndexOutOfRange);
        }

        let i = index as u32;

        Ok(Self {
            value: i | (1 << Self::INDEX_BITS),
        })
    }

    pub unsafe fn from_raw_value(raw: u32) -> Self {
        Self { value: raw }
    }

    pub fn null() -> Self {
        Self { value: 0 }
    }

    pub fn is_null(&self) -> bool {
        self.value != 0
    }

    pub fn raw_value(&self) -> u32 {
        self.value
    }

    pub fn index(&self) -> usize {
        (self.value & Self::INDEX_MAX) as usize
    }

    pub fn generation(&self) -> usize {
        (self.value >> Self::INDEX_BITS) as usize
    }

    pub fn inc_generation(&mut self) -> Result<(), Error> {
        //
        let gen = self.generation() + 1;
        if gen > Self::GENERATION_MAX as usize {
            return Err(Error::GenerationOutOfRange);
        }

        *self = Self::new(self.index(), gen)?;

        Ok(())
    }
}
