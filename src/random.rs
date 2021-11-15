pub struct RandomGenerator {
    numbers: Vec<usize>,
    index: usize,
}

impl RandomGenerator {
    pub fn new() -> RandomGenerator {
        RandomGenerator {
            numbers: Vec::new(),
            index: 0,
        }
    }

    /**
     * After this function self.numbers will have at least n entries.
     */
    fn define(&mut self, n: usize) {
        while self.numbers.len() < n {
            self.numbers.push(rand::random());
        }
    }

    pub fn next(&mut self) -> usize {
        self.index += 1;
        self.get()
    }

    pub fn get(&mut self) -> usize {
        self.get_index(self.index)
    }

    pub fn get_index(&mut self, index: usize) -> usize {
        self.define(index+1);
        *self.numbers.get(index).unwrap()
    }

    #[allow(unused)]
    pub fn get_offset(&mut self, offset: u64) -> Option<usize> {
        let index = (self.index as u64).checked_add(offset)?;
        Some(self.get_index(index as usize))
    }

    pub fn get_offset_unchecked(&mut self, offset: usize) -> usize {
        let index = self.index + offset;
        self.get_index(index)
    }
}